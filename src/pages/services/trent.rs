//! TRENT - Trusted Random Entropy page handler
//!
//! Trusted third party random number generator for drawings, contests, and lotteries.
//!
//! Port of defuse.ca/src/pages/services/trustedthirdparty.php

use askama::Template;
use axum::response::IntoResponse;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::app_state::AppState;
use crate::context::PageContext;
use crate::handler::{BoxFuture, FormField, PageHandler, PostBody};
use crate::libs::trent;

/// Maximum file size: 10MB
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum name/description size: 1MB each
const MAX_TEXT_FIELD_SIZE: usize = 1024 * 1024;

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
                let drawing_num: i32 = drawingnum_str.parse().unwrap_or(0);
                page.handle_view_drawing(drawing_num).await;
            }

            page.into_response()
        })
    }

    fn post(&self, ctx: PageContext, _state: &AppState, body: PostBody) -> Option<BoxFuture> {
        Some(Box::pin(async move {
            let mut page = TrentPage::new(ctx);

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

/// Reservation success info
struct ReservationInfo {
    drawing_num: i32,
    password: String,
    drawing_date: String,
    url: String,
}

/// Confirmation page info (step 1 of create)
struct ConfirmationInfo {
    params: DrawingParams,
    file_infos: [FileInfo; 3],
}

/// File info for confirmation display
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

/// Completion success info
struct CompletionInfo {
    drawing_num: i32,
    url: String,
}

/// Per-file input for a drawing (hash, content, and number of random lines)
#[derive(Clone)]
struct FileInput {
    hash: String,
    content: Option<Vec<u8>>,
    randlines: i32,
}

/// Validated parameters for creating/completing a drawing
#[derive(Clone)]
struct DrawingParams {
    drawing_num: i32,
    passcode: String,
    name: String,
    description: String,
    files: [FileInput; 3],
    lowval: i32,
    highval: i32,
    numgen: i32,
    chosentwice: bool,
}

/// Drawing view info
enum DrawingView {
    /// Drawing doesn't exist
    NotFound(i32),
    /// Drawing exists but not complete
    Pending {
        drawing_num: i32,
        draw_date: String,
    },
    /// Drawing is complete
    Complete {
        drawing_num: i32,
        userprintout: String,
        printout: String,
    },
}

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
    form_values: Option<DrawingParams>,
}

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

    /// Handle URL-encoded POST (confirmation step 2, or simple forms)
    async fn handle_urlencoded_post(&mut self, bytes: &[u8]) {
        let form: TrentForm = serde_urlencoded::from_bytes(bytes).unwrap_or_default();

        if form.makedrawingnumber.is_some() {
            self.handle_reserve(&form).await;
        } else if form.create.is_some() {
            self.handle_create(&form, HashMap::new()).await;
        }
    }

    /// Handle multipart POST (file uploads in create step 1)
    async fn handle_multipart_post(&mut self, fields: Vec<FormField>) {
        // Parse form fields into TrentForm
        let mut form = TrentForm::default();
        let mut files: HashMap<String, (String, Vec<u8>)> = HashMap::new();

        for field in fields {
            match field.name.as_str() {
                "makedrawingnumber" => form.makedrawingnumber = Some(String::new()),
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
                _ => {}
            }
        }

        if form.makedrawingnumber.is_some() {
            self.handle_reserve(&form).await;
        } else if form.create.is_some() {
            self.handle_create(&form, files).await;
        }
    }

    /// Handle drawing reservation
    async fn handle_reserve(&mut self, form: &TrentForm) {
        let review_time: u32 = form.prereview.parse().unwrap_or(0);

        match trent::reserve_drawing(review_time).await {
            Ok((drawing_num, password)) => {
                let drawing_date = trent::format_date(trent::now() + review_time);
                let url = format!(
                    "{}/trustedthirdparty.htm?drawingnum={}",
                    self.ctx.url_prefix, drawing_num
                );
                self.reservation = Some(ReservationInfo {
                    drawing_num,
                    password,
                    drawing_date,
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
        // Build DrawingParams early so it's available for error recovery
        let mut params = DrawingParams {
            drawing_num: form.drawingnumber.parse().unwrap_or(0),
            passcode: form.passcode.trim().to_string(),
            name: form.name.trim().to_string(),
            description: form.description.trim().to_string(),
            files: [
                FileInput { hash: form.file1hash.clone(), content: None, randlines: form.randlines1.parse().unwrap_or(0) },
                FileInput { hash: form.file2hash.clone(), content: None, randlines: form.randlines2.parse().unwrap_or(0) },
                FileInput { hash: form.file3hash.clone(), content: None, randlines: form.randlines3.parse().unwrap_or(0) },
            ],
            lowval: form.lowval.parse().unwrap_or(0),
            highval: form.highval.parse().unwrap_or(0),
            numgen: form.numgen.parse().unwrap_or(0),
            chosentwice: form.chosentwice == "true",
        };

        // Validate name and description size
        if params.name.len() > MAX_TEXT_FIELD_SIZE || params.description.len() > MAX_TEXT_FIELD_SIZE {
            self.set_error("Name and description must each be less than 1 MB.".to_string(), &params);
            return;
        }

        // Validate that name and description contain only Latin-1 characters.
        // The database uses latin1 charset, so characters outside 0-255 will fail.
        if !is_latin1_safe(&params.name) || !is_latin1_safe(&params.description) {
            self.set_error(
                "Name and description can only contain Latin-1 characters (standard Western European letters, numbers, and symbols). \
                 Emojis, Chinese/Japanese/Korean characters, and other special Unicode characters are not supported.".to_string(),
                &params,
            );
            return;
        }

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

        // Validate password
        let password_hash = trent::hash_password(&params.passcode);
        if password_hash != drawing.passwordhash {
            self.set_error(format!("Incorrect password for drawing #{}.", params.drawing_num), &params);
            return;
        }

        // Check if already complete
        if drawing.complete {
            self.set_error(format!(
                "The random numbers for drawing #{} have already been chosen.",
                params.drawing_num
            ), &params);
            return;
        }

        // Check review period
        let drawing_time = drawing.starttime + drawing.reviewtime;
        if trent::now() < drawing_time {
            let date = trent::format_date(drawing_time);
            self.set_error(format!(
                "The review period for drawing #{} is not complete. You will be able to do the drawing after {}",
                params.drawing_num, date
            ), &params);
            return;
        }

        // Validate range
        if params.lowval >= params.highval && params.numgen != 0 {
            self.set_error("The number range is invalid.".to_string(), &params);
            return;
        }

        // Check for negative values
        if params.numgen < 0 || params.files.iter().any(|f| f.randlines < 0) {
            self.set_error("We couldn't possibly generate a NEGATIVE amount of random numbers...".to_string(), &params);
            return;
        }

        // Check max values
        if params.numgen > 1000 || params.files.iter().any(|f| f.randlines > 1000) {
            self.set_error("Sorry, we can only generate 1000 random numbers at a time.".to_string(), &params);
            return;
        }

        // Check file sizes (for uploaded files)
        for (_, (_, data)) in &files {
            if data.len() > MAX_FILE_SIZE {
                self.set_error("Sorry, maximum file size is 10MB.".to_string(), &params);
                return;
            }
        }

        // Resolve file contents (either from upload or from temp storage)
        let field_names = ["file1", "file2", "file3"];
        for i in 0..3 {
            let content = self.get_file_content(
                field_names[i], &params.files[i].hash, params.drawing_num, &files,
            ).await;
            params.files[i].content = content;
        }

        // Check that files are provided when random lines are requested
        for file in &params.files {
            if file.randlines > 0 && file.content.is_none() {
                self.set_error("Please upload a file for each set of random lines requested.".to_string(), &params);
                return;
            }
        }

        // Check if files have enough lines (when no_line_repeat)
        if !params.chosentwice {
            for file in &params.files {
                if !Self::enough_lines(&file.content, file.randlines as usize) {
                    self.set_error(
                        "One of the files doesn't have enough lines to be able to choose the requested number of lines."
                            .to_string(),
                        &params,
                    );
                    return;
                }
            }
        }

        // Is this the confirmation step?
        if form.confirmed == "true" {
            self.complete_drawing(&params).await;

            // Clean up temp files now that the drawing is complete (or failed)
            for file in &params.files {
                delete_temp_file(params.drawing_num, &file.hash).await;
            }
        } else {
            self.show_confirmation(params, &files).await;
        }
    }

    /// Get file content from upload or temp storage
    async fn get_file_content(
        &self,
        field_name: &str,
        hash: &str,
        drawing_num: i32,
        files: &HashMap<String, (String, Vec<u8>)>,
    ) -> Option<Vec<u8>> {
        // First check if we have an uploaded file
        if let Some((_, data)) = files.get(field_name) {
            return Some(data.clone());
        }

        // Check if we have a hash and can load from temp
        if is_sha256_hex(hash) {
            let path = temp_path(drawing_num, hash);
            if let Ok(data) = tokio::fs::read(&path).await {
                return Some(data);
            }
        }

        None
    }

    /// Check if file has enough lines
    fn enough_lines(content: &Option<Vec<u8>>, num: usize) -> bool {
        match content {
            None => true, // No file = enough lines (no lines requested)
            Some(_) if num == 0 => true,
            Some(data) => trent::count_lines(data) >= num,
        }
    }

    /// Show confirmation page and save files to temp
    async fn show_confirmation(
        &mut self,
        mut params: DrawingParams,
        files: &HashMap<String, (String, Vec<u8>)>,
    ) {
        // Compute hashes, save files to temp, and update params with computed hashes
        let field_names = ["file1", "file2", "file3"];
        let mut file_infos = [FileInfo::default(), FileInfo::default(), FileInfo::default()];
        for i in 0..3 {
            let (hash, info) = self.process_file(field_names[i], params.drawing_num, files, &params.files[i].content).await;
            params.files[i].hash = hash;
            file_infos[i] = info;
        }

        self.confirmation = Some(ConfirmationInfo { params, file_infos });
    }

    /// Process a file: compute hash, save to temp, return (hash, info)
    async fn process_file(
        &self,
        field_name: &str,
        drawing_num: i32,
        files: &HashMap<String, (String, Vec<u8>)>,
        content: &Option<Vec<u8>>,
    ) -> (String, FileInfo) {
        if let Some((filename, data)) = files.get(field_name) {
            let hash = hex::encode(Sha256::digest(data));
            let size = trent::format_bytes(data.len() as u64);

            // Save to temp
            let path = temp_path(drawing_num, &hash);
            if let Err(e) = tokio::fs::write(&path, data).await {
                tracing::error!("Failed to write temp file: {}", e);
            }

            (
                hash.clone(),
                FileInfo {
                    name: filename.clone(),
                    size,
                    sha256: hash,
                },
            )
        } else if let Some(data) = content {
            // File was loaded from temp (hash already known)
            let hash = hex::encode(Sha256::digest(data));
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

    /// Complete the drawing and save results
    async fn complete_drawing(&mut self, params: &DrawingParams) {
        let no_line_repeat = !params.chosentwice;

        // Build user printout
        let userprintout = format!("NAME: {}\nDESCRIPTION:\n{}", params.name, params.description);

        // Build printout
        let mut printout = String::new();
        printout.push_str(&format!("DRAWING NUMBER: {}\n", params.drawing_num));
        printout.push_str(&format!("DRAWING DATE: {}\n", trent::format_date(trent::now())));
        printout.push_str(&format!("AMOUNT OF NUMBERS: {}\n", params.numgen));
        printout.push_str(&format!("RANGE: {} TO {}\n\n", params.lowval, params.highval));

        // Files
        for (i, file) in params.files.iter().enumerate() {
            let file_num = (i + 1) as u8;
            if is_sha256_hex(&file.hash) {
                if let Some(content) = &file.content {
                    printout.push_str(&format!("FILE{} SHA256: {}\n\n", file_num, file.hash));
                    printout.push_str(&trent::get_random_lines_output(
                        content,
                        file.randlines as usize,
                        no_line_repeat,
                        file_num,
                    ));
                }
            }
        }

        // Random numbers
        for i in 1..=params.numgen {
            let random_bytes = trent::generate_random_bytes();
            let randnum = trent::select_random_number(&random_bytes, params.lowval as i64, params.highval as i64);
            printout.push_str(&format!("RANDOM NUMBER NUMBER {}: {}\n", i, randnum));
        }

        // Save to database
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

/// Check if a string is a valid SHA256 hex hash (64 hex characters)
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Build the temp file path for a given drawing and content hash.
/// Includes the drawing number so concurrent drawings don't share temp files.
fn temp_path(drawing_num: i32, hash: &str) -> String {
    format!("/tmp/trent-{}-{}", drawing_num, hash)
}

/// Delete a temp file by its hash (best-effort, ignores errors)
async fn delete_temp_file(drawing_num: i32, hash: &str) {
    if is_sha256_hex(hash) {
        let _ = tokio::fs::remove_file(temp_path(drawing_num, hash)).await;
    }
}

/// Check if a string contains only Latin-1 compatible characters (code points 0-255).
/// The database uses latin1 charset, so characters outside this range will cause errors.
fn is_latin1_safe(s: &str) -> bool {
    s.chars().all(|c| (c as u32) <= 255)
}

/// Form data for TRENT
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

/// Extension trait for FormField
trait FormFieldExt {
    fn data_as_string(&self) -> String;
}

impl FormFieldExt for FormField {
    fn data_as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}
