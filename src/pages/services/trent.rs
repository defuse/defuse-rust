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
    drawing_num: i32,
    passcode: String,
    name: String,
    description: String,
    file1_info: FileInfo,
    file2_info: FileInfo,
    file3_info: FileInfo,
    randlines1: i32,
    randlines2: i32,
    randlines3: i32,
    file1hash: String,
    file2hash: String,
    file3hash: String,
    lowval: i32,
    highval: i32,
    numgen: i32,
    chosentwice: bool,
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

/// Form values for re-populating the form on validation errors
#[derive(Default)]
struct FormValues {
    drawing_num: String,
    passcode: String,
    name: String,
    description: String,
    lowval: String,
    highval: String,
    numgen: String,
    randlines1: String,
    randlines2: String,
    randlines3: String,
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
    form_values: Option<FormValues>,
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
                self.drawing_view = Some(DrawingView::NotFound(drawing_num));
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

    /// Handle drawing creation (both confirmation step and completion)
    async fn handle_create(
        &mut self,
        form: &TrentForm,
        files: HashMap<String, (String, Vec<u8>)>,
    ) {
        let drawing_num: i32 = form.drawingnumber.parse().unwrap_or(0);
        let passcode = form.passcode.trim();
        let name = form.name.trim();
        let description = form.description.trim();

        let randlines1: i32 = form.randlines1.parse().unwrap_or(0);
        let randlines2: i32 = form.randlines2.parse().unwrap_or(0);
        let randlines3: i32 = form.randlines3.parse().unwrap_or(0);
        let lowval: i32 = form.lowval.parse().unwrap_or(0);
        let highval: i32 = form.highval.parse().unwrap_or(0);
        let numgen: i32 = form.numgen.parse().unwrap_or(0);
        let chosentwice = form.chosentwice == "true";
        let no_line_repeat = !chosentwice;

        // Get file paths/hashes
        let file1hash = form.file1hash.clone();
        let file2hash = form.file2hash.clone();
        let file3hash = form.file3hash.clone();

        // Validate that name and description contain only Latin-1 characters.
        // The database uses latin1 charset, so characters outside 0-255 will fail.
        if !is_latin1_safe(name) || !is_latin1_safe(description) {
            self.error = Some(
                "Name and description can only contain Latin-1 characters (standard Western European letters, numbers, and symbols). \
                 Emojis, Chinese/Japanese/Korean characters, and other special Unicode characters are not supported.".to_string()
            );
            self.form_values = Some(FormValues {
                drawing_num: drawing_num.to_string(),
                passcode: passcode.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                lowval: lowval.to_string(),
                highval: highval.to_string(),
                numgen: numgen.to_string(),
                randlines1: randlines1.to_string(),
                randlines2: randlines2.to_string(),
                randlines3: randlines3.to_string(),
                chosentwice,
            });
            return;
        }

        // Look up drawing
        let drawing = match trent::get_drawing(drawing_num).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                self.error = Some(format!("Drawing #{} does not exist.", drawing_num));
                return;
            }
            Err(e) => {
                tracing::error!("Database error: {}", e);
                self.error = Some(format!("Drawing #{} does not exist.", drawing_num));
                return;
            }
        };

        // Validate password
        let password_hash = trent::hash_password(passcode);
        if password_hash != drawing.passwordhash {
            self.error = Some(format!("Incorrect password for drawing #{}.", drawing_num));
            return;
        }

        // Check if already complete
        if drawing.complete {
            self.error = Some(format!(
                "The random numbers for drawing #{} have already been chosen.",
                drawing_num
            ));
            return;
        }

        // Check review period
        let drawing_time = drawing.starttime + drawing.reviewtime;
        if trent::now() < drawing_time {
            let date = trent::format_date(drawing_time);
            self.error = Some(format!(
                "The review period for drawing #{} is not complete. You will be able to do the drawing after {}",
                drawing_num, date
            ));
            return;
        }

        // Validate range
        if lowval >= highval && numgen != 0 {
            self.error = Some("The number range is invalid.".to_string());
            return;
        }

        // Check for negative values
        if numgen < 0 || randlines1 < 0 || randlines2 < 0 || randlines3 < 0 {
            self.error = Some("We couldn't possibly generate a NEGATIVE amount of random numbers...".to_string());
            return;
        }

        // Check max values
        if numgen > 1000 || randlines1 > 1000 || randlines2 > 1000 || randlines3 > 1000 {
            self.error = Some("Sorry, we can only generate 1000 random numbers at a time.".to_string());
            return;
        }

        // Check file sizes (for uploaded files)
        for (_, (_, data)) in &files {
            if data.len() > MAX_FILE_SIZE {
                self.error = Some("Sorry, maximum file size is 10MB.".to_string());
                return;
            }
        }

        // Get file contents (either from upload or from temp storage)
        let file1_content = self.get_file_content("file1", &file1hash, &files).await;
        let file2_content = self.get_file_content("file2", &file2hash, &files).await;
        let file3_content = self.get_file_content("file3", &file3hash, &files).await;

        // Check if files have enough lines (when no_line_repeat)
        if no_line_repeat {
            if !Self::enough_lines(&file1_content, randlines1 as usize)
                || !Self::enough_lines(&file2_content, randlines2 as usize)
                || !Self::enough_lines(&file3_content, randlines3 as usize)
            {
                self.error = Some(
                    "One of the files doesn't have enough lines to be able to choose the requested number of lines."
                        .to_string(),
                );
                return;
            }
        }

        // Is this the confirmation step?
        if form.confirmed == "true" {
            // Complete the drawing
            self.complete_drawing(
                drawing_num,
                name,
                description,
                &file1hash,
                &file2hash,
                &file3hash,
                &file1_content,
                &file2_content,
                &file3_content,
                randlines1,
                randlines2,
                randlines3,
                lowval,
                highval,
                numgen,
                no_line_repeat,
            )
            .await;
        } else {
            // Show confirmation page
            self.show_confirmation(
                drawing_num,
                passcode,
                name,
                description,
                &files,
                &file1_content,
                &file2_content,
                &file3_content,
                randlines1,
                randlines2,
                randlines3,
                lowval,
                highval,
                numgen,
                chosentwice,
            )
            .await;
        }
    }

    /// Get file content from upload or temp storage
    async fn get_file_content(
        &self,
        field_name: &str,
        hash: &str,
        files: &HashMap<String, (String, Vec<u8>)>,
    ) -> Option<Vec<u8>> {
        // First check if we have an uploaded file
        if let Some((_, data)) = files.get(field_name) {
            return Some(data.clone());
        }

        // Check if we have a hash and can load from temp
        if is_sha256_hex(hash) {
            let path = format!("/tmp/{}", hash);
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
        drawing_num: i32,
        passcode: &str,
        name: &str,
        description: &str,
        files: &HashMap<String, (String, Vec<u8>)>,
        file1_content: &Option<Vec<u8>>,
        file2_content: &Option<Vec<u8>>,
        file3_content: &Option<Vec<u8>>,
        randlines1: i32,
        randlines2: i32,
        randlines3: i32,
        lowval: i32,
        highval: i32,
        numgen: i32,
        chosentwice: bool,
    ) {
        // Compute hashes and save files to temp
        let (file1hash, file1_info) = self.process_file("file1", files, file1_content).await;
        let (file2hash, file2_info) = self.process_file("file2", files, file2_content).await;
        let (file3hash, file3_info) = self.process_file("file3", files, file3_content).await;

        self.confirmation = Some(ConfirmationInfo {
            drawing_num,
            passcode: passcode.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            file1_info,
            file2_info,
            file3_info,
            randlines1,
            randlines2,
            randlines3,
            file1hash,
            file2hash,
            file3hash,
            lowval,
            highval,
            numgen,
            chosentwice,
        });
    }

    /// Process a file: compute hash, save to temp, return (hash, info)
    async fn process_file(
        &self,
        field_name: &str,
        files: &HashMap<String, (String, Vec<u8>)>,
        content: &Option<Vec<u8>>,
    ) -> (String, FileInfo) {
        if let Some((filename, data)) = files.get(field_name) {
            let hash = hex::encode(Sha256::digest(data));
            let size = trent::format_bytes(data.len() as u64);

            // Save to temp
            let path = format!("/tmp/{}", hash);
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
    #[allow(clippy::too_many_arguments)]
    async fn complete_drawing(
        &mut self,
        drawing_num: i32,
        name: &str,
        description: &str,
        file1hash: &str,
        file2hash: &str,
        file3hash: &str,
        file1_content: &Option<Vec<u8>>,
        file2_content: &Option<Vec<u8>>,
        file3_content: &Option<Vec<u8>>,
        randlines1: i32,
        randlines2: i32,
        randlines3: i32,
        lowval: i32,
        highval: i32,
        numgen: i32,
        no_line_repeat: bool,
    ) {
        // Build user printout
        let userprintout = format!("NAME: {}\nDESCRIPTION:\n{}", name, description);

        // Build printout
        let mut printout = String::new();
        printout.push_str(&format!("DRAWING NUMBER: {}\n", drawing_num));
        printout.push_str(&format!("DRAWING DATE: {}\n", trent::format_date(trent::now())));
        printout.push_str(&format!("AMOUNT OF NUMBERS: {}\n", numgen));
        printout.push_str(&format!("RANGE: {} TO {}\n\n", lowval, highval));

        // File 1
        if is_sha256_hex(file1hash) {
            if let Some(content) = file1_content {
                printout.push_str(&format!("FILE1 SHA256: {}\n\n", file1hash));
                printout.push_str(&trent::get_random_lines_output(
                    content,
                    randlines1 as usize,
                    no_line_repeat,
                    1,
                ));
            }
        }

        // File 2
        if is_sha256_hex(file2hash) {
            if let Some(content) = file2_content {
                printout.push_str(&format!("FILE2 SHA256: {}\n\n", file2hash));
                printout.push_str(&trent::get_random_lines_output(
                    content,
                    randlines2 as usize,
                    no_line_repeat,
                    2,
                ));
            }
        }

        // File 3
        if is_sha256_hex(file3hash) {
            if let Some(content) = file3_content {
                printout.push_str(&format!("FILE3 SHA256: {}\n\n", file3hash));
                printout.push_str(&trent::get_random_lines_output(
                    content,
                    randlines3 as usize,
                    no_line_repeat,
                    3,
                ));
            }
        }

        // Random numbers
        for i in 1..=numgen {
            let random_bytes = trent::generate_random_bytes();
            let randnum = trent::select_random_number(&random_bytes, lowval as i64, highval as i64);
            printout.push_str(&format!("RANDOM NUMBER NUMBER {}: {}\n", i, randnum));
        }

        // Save to database
        if let Err(e) = trent::complete_drawing(drawing_num, &printout, &userprintout).await {
            tracing::error!("Database error completing drawing: {}", e);
            self.error = Some("Database error. Please try again.".to_string());
            return;
        }

        let url = format!(
            "{}/trustedthirdparty.htm?drawingnum={}",
            self.ctx.url_prefix, drawing_num
        );
        self.completion = Some(CompletionInfo { drawing_num, url });
    }
}

/// Check if a string is a valid SHA256 hex hash (64 hex characters)
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
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
