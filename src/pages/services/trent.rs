//! TRENT - Trusted Random Entropy page handler
//!
//! Trusted third party random number generator for drawings, contests, and lotteries.
//! This is a thin HTTP adapter: form parsing, orchestration, and template rendering.
//! All business logic (validation, printout generation, temp files) lives in libs/trent.
//!
//! Port of defuse.ca/src/pages/services/trustedthirdparty.php

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, FormField, PageHandler, PostBody};
use crate::libs::trent::{self, DrawingParams, FileInput};

// =============================================================================
// Types
// =============================================================================

/// Display info for an uploaded file on the confirmation page: the original
/// filename, human-readable size, and SHA-256 hash.
struct FileInfo {
    name: String,
    size: String,
    sha256: String,
}

impl Default for FileInfo {
    fn default() -> Self {
        Self {
            name: "NO FILE".to_string(),
            size: "NO FILE".to_string(),
            sha256: "NO FILE".to_string(),
        }
    }
}

/// Shown after a drawing number is successfully reserved. Contains the
/// drawing number, generated password, scheduled drawing date, and permalink.
struct ReservationInfo {
    drawing_num: i32,
    password: String,
    drawing_date: String,
    url: String,
}

/// Shown on the confirmation page (step 1 of create) so the user can review
/// their parameters before finalizing. Wraps DrawingParams (with file hashes
/// filled in by `process_file`) plus display-only file metadata.
struct ConfirmationInfo {
    params: DrawingParams,
    file_infos: [FileInfo; 3],
}

/// Shown after a drawing is successfully completed. Contains the drawing
/// number and a permalink to view the results.
struct CompletionInfo {
    drawing_num: i32,
    url: String,
}

/// Result of looking up a drawing by number for the GET view.
enum DrawingView {
    /// Drawing doesn't exist
    NotFound(i32),
    /// Drawing exists but the review period hasn't elapsed yet
    Pending {
        drawing_num: i32,
        draw_date: String,
    },
    /// Drawing is complete; printout contains the random results
    Complete {
        drawing_num: i32,
        userprintout: String,
        printout: String,
    },
}

/// Raw form data from the TRENT HTML form, deserialized from either
/// URL-encoded or multipart POST bodies. All fields are strings because
/// they come from user input and are parsed/validated in `handle_create`.
#[derive(Deserialize, Default)]
struct TrentForm {
    #[serde(default)]
    makedrawingnumber: Option<String>,
    #[serde(default)]
    create: Option<String>,
    #[serde(default)]
    prereview: String,
    #[serde(default)]
    drawingnumber: String,
    #[serde(default)]
    passcode: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    randlines1: String,
    #[serde(default)]
    randlines2: String,
    #[serde(default)]
    randlines3: String,
    #[serde(default)]
    chosentwice: String,
    #[serde(default)]
    lowval: String,
    #[serde(default)]
    highval: String,
    #[serde(default)]
    numgen: String,
    #[serde(default)]
    confirmed: String,
    #[serde(default)]
    file1hash: String,
    #[serde(default)]
    file2hash: String,
    #[serde(default)]
    file3hash: String,
}

/// The main page template. Populated by the handler methods and rendered
/// via Askama. At most one of `drawing_view`, `reservation`, `confirmation`,
/// `completion`, or `error` will be set per request.
#[derive(Template)]
#[template(path = "pages/services/trustedthirdparty.html")]
struct TrentPage {
    ctx: PageContext,
    current_time: String,
    drawing_view: Option<DrawingView>,
    reservation: Option<ReservationInfo>,
    confirmation: Option<ConfirmationInfo>,
    completion: Option<CompletionInfo>,
    error: Option<String>,
    /// When set, the form fields are repopulated with these values after a
    /// validation error so the user doesn't have to re-enter everything.
    form_values: Option<DrawingParams>,
}

// =============================================================================
// Page handler
// =============================================================================

pub struct Handler;

impl PageHandler for Handler {
    fn get(&self, ctx: PageContext, _state: &AppState) -> BoxFuture {
        Box::pin(async move {
            let query = ctx.query_string.as_deref().unwrap_or("");
            let params: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            let mut page = TrentPage::new(ctx);

            // Check if viewing a specific drawing
            if let Some(drawingnum_str) = params.get("drawingnum") {
                match drawingnum_str.parse::<i32>() {
                    Ok(drawing_num) => page.handle_view_drawing(drawing_num).await,
                    Err(_) => page.error = Some("Invalid drawing number.".to_string()),
                }
            }

            page.into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            let mut page = TrentPage::new(ctx);

            // There are THREE valid POST requests:
            // 1. When the user is reserving their drawing number.
            // 2. When the user is submitting their drawing info. <-- may have files
            // 3. When the user is confirming their drawing info.

            match body {
                PostBody::UrlEncoded(bytes) => {
                    page.handle_urlencoded_post(&bytes).await;
                }
                PostBody::Multipart { fields } => {
                    page.handle_multipart_post(fields).await;
                }
            }

            page.into_response()
        }))
    }
}

// =============================================================================
// TrentPage implementation
// =============================================================================

impl TrentPage {
    fn new(ctx: PageContext) -> Self {
        Self {
            ctx,
            current_time: trent::format_date(trent::now()),
            drawing_view: None,
            reservation: None,
            confirmation: None,
            completion: None,
            error: None,
            form_values: None,
        }
    }

    /// Handle viewing a drawing by number
    async fn handle_view_drawing(&mut self, drawing_num: i32) {
        match trent::get_drawing(drawing_num).await {
            Ok(Some(drawing)) => {
                if drawing.complete {
                    self.drawing_view = Some(DrawingView::Complete {
                        drawing_num,
                        userprintout: drawing.userprintout,
                        printout: drawing.printout,
                    });
                } else {
                    let draw_date = trent::format_date(drawing.starttime + drawing.reviewtime);
                    self.drawing_view = Some(DrawingView::Pending {
                        drawing_num,
                        draw_date,
                    });
                }
            }
            Ok(None) => {
                self.drawing_view = Some(DrawingView::NotFound(drawing_num));
            }
            Err(e) => {
                tracing::error!("Database error viewing drawing: {}", e);
                self.error = Some("Database error. Please try again.".to_string());
            }
        }
    }

    /// Handle URL-encoded POST (reservation or confirmation, creation is multipart)
    async fn handle_urlencoded_post(&mut self, bytes: &[u8]) {
        let form: TrentForm = serde_urlencoded::from_bytes(bytes).unwrap_or_default();

        if form.makedrawingnumber.is_some() {
            self.handle_reserve(&form).await;
        } else if form.create.is_some() {
            self.handle_create(&form, HashMap::new()).await;
        } else {
            self.error = Some("Invalid form submission.".to_string());
        }
    }

    /// Handle multipart POST (file uploads in drawing creation)
    async fn handle_multipart_post(&mut self, fields: Vec<FormField>) {
        let mut form = TrentForm::default();
        let mut files: HashMap<String, (String, Vec<u8>)> = HashMap::new();

        for field in fields {
            match field.name.as_str() {
                "create" => form.create = Some(String::new()),
                "prereview" => form.prereview = field.data_as_string(),
                "drawingnumber" => form.drawingnumber = field.data_as_string(),
                "passcode" => form.passcode = field.data_as_string(),
                "name" => form.name = field.data_as_string(),
                "description" => form.description = field.data_as_string(),
                "randlines1" => form.randlines1 = field.data_as_string(),
                "randlines2" => form.randlines2 = field.data_as_string(),
                "randlines3" => form.randlines3 = field.data_as_string(),
                "chosentwice" => form.chosentwice = field.data_as_string(),
                "lowval" => form.lowval = field.data_as_string(),
                "highval" => form.highval = field.data_as_string(),
                "numgen" => form.numgen = field.data_as_string(),
                "confirmed" => form.confirmed = field.data_as_string(),
                "file1hash" => form.file1hash = field.data_as_string(),
                "file2hash" => form.file2hash = field.data_as_string(),
                "file3hash" => form.file3hash = field.data_as_string(),
                "file1" | "file2" | "file3" => {
                    if let Some(filename) = &field.filename {
                        if !filename.is_empty() && !field.data.is_empty() {
                            files.insert(field.name.clone(), (filename.clone(), field.data.to_vec()));
                        }
                    }
                }
                _ => {
                    self.error = Some(format!("Unrecognized form field: {}", field.name));
                    return;
                }
            }
        }

        if form.create.is_some() {
            self.handle_create(&form, files).await;
        } else {
            self.error = Some("Invalid form submission.".to_string());
        }
    }

    /// Handle drawing reservation
    async fn handle_reserve(&mut self, form: &TrentForm) {
        let review_time: u32 = form.prereview.parse().unwrap_or(0);

        match trent::reserve_drawing(review_time).await {
            Ok(result) => {
                let url = format!(
                    "{}/trustedthirdparty.htm?drawingnum={}",
                    self.ctx.url_prefix, result.drawing_num
                );
                self.reservation = Some(ReservationInfo {
                    drawing_num: result.drawing_num,
                    password: result.password,
                    drawing_date: result.drawing_date,
                    url,
                });
            }
            Err(e) => {
                tracing::error!("Database error reserving drawing: {}", e);
                self.error = Some("Database error. Please try again.".to_string());
            }
        }
    }

    /// Set an error message and repopulate form values
    fn set_error(&mut self, msg: String, params: &DrawingParams) {
        self.error = Some(msg);
        self.form_values = Some(params.clone());
    }

    /// Handle drawing creation (both confirmation step and completion)
    async fn handle_create(
        &mut self,
        form: &TrentForm,
        files: HashMap<String, (String, Vec<u8>)>,
    ) {
        // Parse numeric fields — empty is 0, non-numeric is an error
        let drawing_num = match parse_int(&form.drawingnumber) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let randlines1 = match parse_int(&form.randlines1) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let randlines2 = match parse_int(&form.randlines2) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let randlines3 = match parse_int(&form.randlines3) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let lowval = match parse_int(&form.lowval) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let highval = match parse_int(&form.highval) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };
        let numgen = match parse_int(&form.numgen) {
            Ok(v) => v,
            Err(e) => { self.error = Some(e); return; }
        };

        // Build DrawingParams early so it's available for error recovery
        let mut params = DrawingParams {
            drawing_num,
            passcode: form.passcode.trim().to_string(),
            name: form.name.trim().to_string(),
            description: form.description.trim().to_string(),
            files: [
                FileInput { hash: form.file1hash.clone(), content: None, randlines: randlines1 },
                FileInput { hash: form.file2hash.clone(), content: None, randlines: randlines2 },
                FileInput { hash: form.file3hash.clone(), content: None, randlines: randlines3 },
            ],
            lowval,
            highval,
            numgen,
            chosentwice: form.chosentwice == "true",
        };

        // Look up drawing
        let drawing = match trent::get_drawing(params.drawing_num).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                self.set_error(format!("Drawing #{} does not exist.", params.drawing_num), &params);
                return;
            }
            Err(e) => {
                tracing::error!("Database error: {}", e);
                self.set_error(format!("Drawing #{} does not exist.", params.drawing_num), &params);
                return;
            }
        };

        // Validate parameters against the drawing record
        if let Err(e) = trent::validate_create_request(&params, &drawing) {
            self.set_error(e.to_string(), &params);
            return;
        }

        // Resolve file contents from uploads or temp storage
        let field_names = ["file1", "file2", "file3"];
        for i in 0..3 {
            params.files[i].content = resolve_file_content(
                field_names[i], &params.files[i].hash, params.drawing_num, &files,
            ).await;
        }

        // Validate file contents
        if let Err(e) = trent::validate_files(&params) {
            self.set_error(e.to_string(), &params);
            return;
        }

        if form.confirmed == "true" {
            self.finalize_drawing(&params).await;
            trent::delete_temp_files(params.drawing_num, &params.files).await;
        } else {
            self.show_confirmation(params, &files).await;
        }
    }

    /// Show confirmation page and save files to temp
    async fn show_confirmation(
        &mut self,
        mut params: DrawingParams,
        files: &HashMap<String, (String, Vec<u8>)>,
    ) {
        let field_names = ["file1", "file2", "file3"];
        let mut file_infos = [FileInfo::default(), FileInfo::default(), FileInfo::default()];
        for i in 0..3 {
            let (hash, info) = process_file(
                field_names[i], params.drawing_num, files, &params.files[i].content,
            ).await;
            params.files[i].hash = hash;
            file_infos[i] = info;
        }

        self.confirmation = Some(ConfirmationInfo { params, file_infos });
    }

    /// Complete the drawing: build printout via the library and save to database
    async fn finalize_drawing(&mut self, params: &DrawingParams) {
        let (printout, userprintout) = trent::build_printout(params);

        if let Err(e) = trent::complete_drawing(params.drawing_num, &printout, &userprintout).await {
            tracing::error!("Database error completing drawing: {}", e);
            self.error = Some("Database error. Please try again.".to_string());
            return;
        }

        let url = format!(
            "{}/trustedthirdparty.htm?drawingnum={}",
            self.ctx.url_prefix, params.drawing_num
        );
        self.completion = Some(CompletionInfo { drawing_num: params.drawing_num, url });
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Resolve file content from an upload or from temp storage.
async fn resolve_file_content(
    field_name: &str,
    hash: &str,
    drawing_num: i32,
    files: &HashMap<String, (String, Vec<u8>)>,
) -> Option<Vec<u8>> {
    if let Some((_, data)) = files.get(field_name) {
        return Some(data.clone());
    }
    trent::load_temp_file(drawing_num, hash).await
}

/// Process an uploaded file: compute its hash, save to temp, and build display info.
async fn process_file(
    field_name: &str,
    drawing_num: i32,
    files: &HashMap<String, (String, Vec<u8>)>,
    content: &Option<Vec<u8>>,
) -> (String, FileInfo) {
    if let Some((filename, data)) = files.get(field_name) {
        let hash = trent::sha256_hex(data);
        trent::save_temp_file(drawing_num, &hash, data).await;
        (
            hash.clone(),
            FileInfo {
                name: filename.clone(),
                size: trent::format_bytes(data.len() as u64),
                sha256: hash,
            },
        )
    } else if let Some(data) = content {
        let hash = trent::sha256_hex(data);
        (
            hash.clone(),
            FileInfo {
                name: "NO FILE".to_string(),
                size: trent::format_bytes(data.len() as u64),
                sha256: hash,
            },
        )
    } else {
        (String::new(), FileInfo::default())
    }
}

/// Parse a form field as i32, treating empty string as 0.
/// Returns Err for non-empty non-numeric input.
fn parse_int(s: &str) -> Result<i32, String> {
    if s.is_empty() {
        Ok(0)
    } else {
        s.parse::<i32>().map_err(|_| format!("'{}' is not a valid number.", s))
    }
}

/// Extension trait for FormField to extract string data
trait FormFieldExt {
    fn data_as_string(&self) -> String;
}

impl FormFieldExt for FormField {
    fn data_as_string(&self) -> String {
        // TODO: could this be losing info?
        String::from_utf8_lossy(&self.data).into_owned()
    }
}
