//! TRENT - Trusted Random Entropy page handler
//!
//! Trusted third party random number generator for drawings, contests, and lotteries.
//! This is a thin HTTP adapter: form parsing, orchestration, and template rendering.
//! All business logic (validation, printout generation) lives in libs/trent.
//!
//! Port of defuse.ca/src/pages/services/trustedthirdparty.php

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, FormField, PageHandler, PostBody};
use crate::libs::trent::{self, DrawingParams, FileInput, ValidatedDrawing};

// =============================================================================
// Types
// =============================================================================

/// Display info for an uploaded file on the confirmation page: the original
/// filename, human-readable size, and SHA-256 hash.
struct FileInfo {
    name: String,
    size: String,
    sha256: String,
    warning: String,
}

impl Default for FileInfo {
    fn default() -> Self {
        Self {
            name: "NO FILE".to_string(),
            size: "NO FILE".to_string(),
            sha256: "NO FILE".to_string(),
            warning: String::new(),
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
/// their parameters before finalizing.
struct ConfirmationInfo {
    params: DrawingParams,
    file_infos: [FileInfo; 3],
}

/// Shown after a drawing is successfully completed. Contains the drawing
/// number and a permalink to view the results.
struct CompletionInfo {
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
#[derive(Deserialize, Default, Clone)]
struct TrentForm {
    #[serde(default)]
    makedrawingnumber: Option<String>,
    #[serde(default)]
    create: Option<String>,
    #[serde(default)]
    cancel: Option<String>,
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
    /// When set, the form fields are repopulated with the user's raw input
    /// after a validation error so they don't have to re-enter everything.
    form_values: Option<TrentForm>,
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

            // Parse the POST body into a form + uploaded files, regardless of encoding
            let (form, files) = match parse_post_body(body) {
                Ok(parsed) => parsed,
                Err(msg) => {
                    page.error = Some(msg);
                    return page.into_response();
                }
            };

            // Dispatch to the appropriate handler:
            // 1. Reserve a drawing number
            // 2. Create/confirm a drawing (with shared validation, branching at the end)
            if form.makedrawingnumber.is_some() {
                page.handle_reserve(&form).await;
            } else if form.cancel.is_some() {
                page.set_error("Drawing cancelled. You may make corrections below and then re-submit the form.".to_string(), &form);
            } else if form.create.is_some() {
                page.handle_create(&form, files).await;
            } else {
                page.error = Some("Invalid form submission.".to_string());
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

    /// Handle viewing a drawing by number (GET ?drawingnum=N)
    async fn handle_view_drawing(&mut self, drawing_num: i32) {
        match trent::get_drawing(drawing_num).await {
            Ok(Some(drawing)) => {
                assert!(drawing_num == drawing.drawingnum);
                if drawing.complete {
                    self.drawing_view = Some(DrawingView::Complete {
                        drawing_num,
                        userprintout: drawing.userprintout,
                        printout: drawing.printout,
                    });
                } else {
                    let draw_date = trent::format_date(drawing.draw_date());
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

    /// Handle drawing reservation
    async fn handle_reserve(&mut self, form: &TrentForm) {
        // TODO: validate review_time against the allowed dropdown values.
        // Currently any u32 is accepted; a large value can overflow draw_date()
        // (starttime + reviewtime wraps in release mode). Not a security issue
        // since instant (0) review is allowed, but it's sloppy.
        let review_time: u32 = match form.prereview.parse() {
            Ok(v) => v,
            Err(_) => {
                self.set_error("Invalid review time.".to_string(), form);
                return;
            }
        };

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

    /// Set an error message and repopulate form values from the user's raw input
    fn set_error(&mut self, msg: String, form: &TrentForm) {
        self.error = Some(msg);
        self.form_values = Some(form.clone());
    }

    /// Handle drawing creation (both confirmation step and completion)
    async fn handle_create(
        &mut self,
        form: &TrentForm,
        files: HashMap<String, (String, Vec<u8>)>,
    ) {
        // Parse numeric fields — empty is 0, non-numeric is an error
        let drawing_num = match parse_int_or_empty(&form.drawingnumber) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let randlines1 = match parse_int_or_empty(&form.randlines1) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let randlines2 = match parse_int_or_empty(&form.randlines2) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let randlines3 = match parse_int_or_empty(&form.randlines3) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let lowval = match parse_int_or_empty(&form.lowval) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let highval = match parse_int_or_empty(&form.highval) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };
        let numgen = match parse_int_or_empty(&form.numgen) {
            Ok(v) => v,
            Err(e) => { self.set_error(e, form); return; }
        };

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

        // Load file contents: from temp storage (confirmed) or uploads (initial)
        if form.confirmed == "true" {
            for i in 0..3 {
                params.files[i].content = load_temp_file(
                    params.drawing_num, &params.files[i].hash,
                ).await;
            }
        } else {
            let field_names = ["file1", "file2", "file3"];
            for i in 0..3 {
                if let Some((_, data)) = files.get(field_names[i]) {
                    params.files[i].hash = trent::sha256_hex(data);
                    params.files[i].content = Some(data.clone());
                }
            }
        }

        // Validate everything: fetches drawing from DB, checks params + files
        let validated = match trent::validate_create_request(params).await {
            Ok(v) => v,
            Err(e) => {
                self.set_error(e.to_string(), form);
                return;
            }
        };

        // Act: finalize (confirmed) or show confirmation (initial)
        if form.confirmed == "true" {
            self.finalize_drawing(&validated).await;
        } else {
            if let Err(e) = save_files_to_temp(&validated.params).await {
                tracing::error!("Failed to write temp file: {}", e);
                self.set_error("Server error saving uploaded files. Please try again.".to_string(), form);
                return;
            }
            let file_infos = build_file_infos(&validated.params, &files);
            self.confirmation = Some(ConfirmationInfo { params: validated.params, file_infos });
            self.form_values = Some(form.clone());
        }
    }

    /// Complete the drawing and save to database
    async fn finalize_drawing(&mut self, validated: &ValidatedDrawing) {
        if let Err(e) = trent::complete_drawing(validated).await {
            // The only way this fails is a database error (connection failure,
            // or TOCTOU race where another request completed the drawing first).
            // We intentionally leave temp files around rather than deleting them,
            // so the user can retry. If an attacker can cause DB errors to fill
            // /tmp with temp files, they're already DoSing the site anyway.
            tracing::error!("Database error completing drawing: {}", e);
            self.error = Some("Database error. Please try again.".to_string());
            return;
        }

        delete_temp_files(validated.params.drawing_num, &validated.params.files).await;

        let drawing_num = validated.params.drawing_num;
        let url = format!(
            "{}/trustedthirdparty.htm?drawingnum={}",
            self.ctx.url_prefix, drawing_num
        );
        self.completion = Some(CompletionInfo { url });
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Parse a POST body (URL-encoded or multipart) into a TrentForm and uploaded files.
fn parse_post_body(body: PostBody) -> Result<(TrentForm, HashMap<String, (String, Vec<u8>)>), String> {
    match body {
        PostBody::UrlEncoded(bytes) => {
            let form: TrentForm = serde_urlencoded::from_bytes(&bytes).unwrap_or_default();
            Ok((form, HashMap::new()))
        }
        PostBody::Multipart { fields } => {
            let mut form = TrentForm::default();
            let mut files: HashMap<String, (String, Vec<u8>)> = HashMap::new();

            for field in fields {
                match field.name.as_str() {
                    "makedrawingnumber" => form.makedrawingnumber = Some(String::new()),
                    "create" => form.create = Some(String::new()),
                    "cancel" => form.cancel = Some(String::new()),
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
                            if !filename.is_empty() {
                                // Allow empty files to go through to validation so the user gets a useful error.
                                files.insert(field.name.clone(), (filename.clone(), field.data.to_vec()));
                            }
                        }
                    }
                    _ => {
                        return Err(format!("Unrecognized form field: {}", field.name));
                    }
                }
            }

            Ok((form, files))
        }
    }
}

/// Save uploaded files to temp storage. Hashes must already be set in params.
async fn save_files_to_temp(params: &DrawingParams) -> Result<(), std::io::Error> {
    for file in &params.files {
        if let Some(content) = &file.content {
            let path = temp_path(params.drawing_num, &file.hash);
            tokio::fs::write(&path, content).await?;
        }
    }
    Ok(())
}

/// Build display info for uploaded files on the confirmation page.
fn build_file_infos(
    params: &DrawingParams,
    files: &HashMap<String, (String, Vec<u8>)>,
) -> [FileInfo; 3] {
    let field_names = ["file1", "file2", "file3"];
    let mut file_infos = [FileInfo::default(), FileInfo::default(), FileInfo::default()];
    for i in 0..3 {
        if let Some((filename, data)) = files.get(field_names[i]) {
            let warning = if params.files[i].randlines == 0 {
                "This file will have no random lines selected. Only its SHA256 hash will be recorded.".to_string()
            } else {
                String::new()
            };
            file_infos[i] = FileInfo {
                name: filename.clone(),
                size: trent::format_bytes(data.len() as u64),
                sha256: params.files[i].hash.clone(),
                warning,
            };
        }
    }
    file_infos
}

/// Parse a form field as i32, treating empty string as 0.
/// Returns Err for non-empty non-numeric input.
fn parse_int_or_empty(s: &str) -> Result<i32, String> {
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

// =============================================================================
// Temp file management
// =============================================================================

/// Build the temp file path for a given drawing and content hash.
fn temp_path(drawing_num: i32, hash: &str) -> String {
    // Defense-in-depth against path traversal attacks.
    assert!(trent::is_sha256_hex(hash));
    format!("/tmp/trent-{}-{}", drawing_num, hash)
}

/// Load file content from temp storage. Returns None if the hash is invalid
/// or the file doesn't exist.
async fn load_temp_file(drawing_num: i32, hash: &str) -> Option<Vec<u8>> {
    if !trent::is_sha256_hex(hash) {
        return None;
    }
    tokio::fs::read(temp_path(drawing_num, hash)).await.ok()
}

/// Delete temp files for a completed drawing (best-effort).
async fn delete_temp_files(drawing_num: i32, files: &[FileInput]) {
    for file in files {
        if trent::is_sha256_hex(&file.hash) {
            let _ = tokio::fs::remove_file(temp_path(drawing_num, &file.hash)).await;
        }
    }
}
