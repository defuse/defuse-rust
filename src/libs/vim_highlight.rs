//! Vim-based syntax highlighting.
//!
//! This module provides syntax highlighting using Vim's TOhtml feature.
//! It's a Rust port of VimHighlight.php from the original site.
//!
//! The highlighting is done by running vim in batch mode to convert
//! source code to HTML with syntax highlighting.

use md5::{Md5, Digest};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use tempfile::NamedTempFile;
use tracing::{debug, warn};

/// Cache directory for highlighted output, derived from STORAGE_PATH env var.
/// Falls back to /storage/vimhl if STORAGE_PATH is not set (for backwards compatibility).
fn cache_dir() -> &'static Path {
    static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
    CACHE_DIR.get_or_init(|| {
        match std::env::var("STORAGE_PATH") {
            Ok(storage_path) => PathBuf::from(storage_path).join("vimhl"),
            Err(_) => PathBuf::from("/storage/vimhl"),
        }
    })
}

const CACHE_SUFFIX: &str = ".highlighted.html";

/// Vim-based syntax highlighter
#[derive(Debug, Clone)]
pub struct VimHighlight {
    /// Whether to cache the result (vim is slow)
    pub caching: bool,
    /// Color scheme to use (passed to :colorscheme)
    pub color_scheme: String,
    /// File type / language (None = let vim auto-detect)
    pub file_type: Option<String>,
    /// Whether to show line numbers
    pub show_lines: bool,
    /// Use CSS instead of inline font tags
    pub use_css: bool,
    /// The vim command to use ("vim" or "gvim")
    vim_command: String,
}

impl Default for VimHighlight {
    fn default() -> Self {
        Self {
            caching: false,
            color_scheme: "default".to_string(),
            file_type: None,
            show_lines: true,
            use_css: false,
            vim_command: "vim".to_string(),
        }
    }
}

impl VimHighlight {
    /// Create a new VimHighlight with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with the standard settings used by printHlString
    pub fn for_print_hl_string(file_type: &str, show_lines: bool) -> Self {
        Self {
            caching: true,
            color_scheme: "default".to_string(),
            file_type: Some(file_type.to_string()),
            show_lines,
            use_css: true,
            vim_command: "vim".to_string(),
        }
    }

    /// Set the vim command ("vim", "vi", or "gvim")
    pub fn set_vim_command(&mut self, cmd: &str) {
        let cmd_lower = cmd.to_lowercase();
        if cmd_lower == "vi" || cmd_lower == "vim" || cmd_lower == "gvim" {
            self.vim_command = cmd_lower;
        } else {
            warn!("Invalid vim command: {}", cmd);
        }
    }

    /// Process a string and return syntax-highlighted HTML
    ///
    /// If `body_only` is true, returns just the <pre>...</pre> content
    /// (when use_css is true) or the inner body content (when use_css is false).
    pub fn process_text(&self, text: &str, body_only: bool) -> Result<String, VimHighlightError> {
        // Write the string to a temp file for vim to read
        let mut input_file = NamedTempFile::new()
            .map_err(|e| VimHighlightError::IoError(format!("Failed to create temp file: {}", e)))?;
        input_file.write_all(text.as_bytes())
            .map_err(|e| VimHighlightError::IoError(format!("Failed to write temp file: {}", e)))?;
        let input_path = input_file.path();

        // Generate cache path based on md5 of content
        // Only use cache if the cache directory exists and is writable
        let cache_path = if self.caching {
            let cache_dir_path = cache_dir();
            if cache_dir_path.exists() {
                let hash = self.compute_cache_key(text);
                Some(cache_dir_path.join(format!("string-{}{}", hash, CACHE_SUFFIX)))
            } else {
                // Cache dir doesn't exist, disable caching for this call
                debug!("Cache directory {:?} doesn't exist, skipping cache", cache_dir_path);
                None
            }
        } else {
            None
        };

        // For strings, ignore mtime check since we just created the temp file
        self.run_vim(input_path, cache_path.as_deref(), body_only, true)
    }

    /// Process a file and return syntax-highlighted HTML
    pub fn process_file(&self, input_path: &Path, body_only: bool) -> Result<String, VimHighlightError> {
        if !input_path.exists() {
            return Err(VimHighlightError::FileNotFound(input_path.display().to_string()));
        }

        let cache_path = if self.caching {
            let real_path = input_path.canonicalize()
                .map_err(|e| VimHighlightError::IoError(e.to_string()))?;
            let hash = format!("{:x}", Md5::digest(real_path.to_string_lossy().as_bytes()));
            Some(cache_dir().join(format!("path-{}{}", hash, CACHE_SUFFIX)))
        } else {
            None
        };

        self.run_vim(input_path, cache_path.as_deref(), body_only, false)
    }

    /// Compute cache key from content and settings
    fn compute_cache_key(&self, content: &str) -> String {
        format!("{:x}", Md5::digest(content.as_bytes()))
    }

    /// Encode current settings for cache validation
    fn encode_info(&self) -> String {
        // Match PHP's serialize format for cache validation
        // We store: color_scheme, file_type, show_lines, use_css, vim_command
        format!(
            "color_scheme:{};file_type:{};show_lines:{};use_css:{};vim_command:{}",
            self.color_scheme,
            self.file_type.as_deref().unwrap_or(""),
            self.show_lines,
            self.use_css,
            self.vim_command
        )
    }

    /// Run vim to generate HTML
    fn run_vim(
        &self,
        input_path: &Path,
        cache_path: Option<&Path>,
        body_only: bool,
        ignore_mtime: bool,
    ) -> Result<String, VimHighlightError> {
        // Check cache first
        if self.caching {
            if let Some(cache_path) = cache_path {
                if let Some(cached) = self.check_cache(cache_path, input_path, ignore_mtime)? {
                    debug!("Cache hit: {:?}", cache_path);
                    return Ok(if body_only {
                        self.extract_body(&cached)?
                    } else {
                        self.strip_info(&cached)
                    });
                }
            }
        }

        // Create output temp file
        let output_file = NamedTempFile::new()
            .map_err(|e| VimHighlightError::IoError(e.to_string()))?;
        let output_path = output_file.path();

        // Build vim command
        let colorscheme_cmd = format!("colo {}", self.color_scheme);
        let filetype_cmd = self.file_type.as_ref()
            .map(|ft| format!("set filetype={}", ft));
        let number_cmd = if self.show_lines { "set number" } else { "set nonumber" };
        let use_css_val = if self.use_css { "1" } else { "0" };
        let write_cmd = format!("w! {}", output_path.display());

        let mut args = vec![
            // Don't connect to X; pretend we have xterm with 256 colors
            "-X", "-T", "xterm",
            "-c", "set t_Co=256",
            // Disable plugins, swap file, and wildcard expansion
            "--noplugin", "-n", "--literal",
            // Enable syntax highlighting
            "-f", "-c", "syn on",
        ];

        // Set html_use_css
        let css_cmd = format!("let html_use_css = {}", use_css_val);
        args.push("-c");
        args.push(&css_cmd);

        // Set colorscheme
        args.push("-c");
        args.push(&colorscheme_cmd);

        // Set filetype if specified
        let ft_cmd_storage;
        if let Some(ref ft_cmd) = filetype_cmd {
            ft_cmd_storage = ft_cmd.clone();
            args.push("-c");
            args.push(&ft_cmd_storage);
        }

        // Set line numbers
        args.push("-c");
        args.push(number_cmd);

        // Set tab width to 8 and preserve literal tabs in HTML output
        // (newer vim versions expand tabs by default when html_use_css=1)
        args.push("-c");
        args.push("set tabstop=8 | let g:html_expand_tabs = 0");

        // Generate HTML, write it, then quit both buffers
        args.push("-c");
        args.push("run! syntax/2html.vim");
        args.push("-c");
        args.push(&write_cmd);
        args.push("-c");
        args.push("q! | q!");

        // Input file
        args.push(input_path.to_str().ok_or_else(|| {
            VimHighlightError::IoError("Invalid input path".to_string())
        })?);

        debug!("Running vim with args: {:?}", args);

        // Run vim
        let output = Command::new(&self.vim_command)
            .args(&args)
            .output()
            .map_err(|e| VimHighlightError::VimError(format!("Failed to run vim: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Vim exited with status {}: {}", output.status, stderr);
            // Don't fail - vim often exits with non-zero even when successful
        }

        // Read the generated HTML
        let html = fs::read_to_string(output_path)
            .map_err(|e| VimHighlightError::IoError(format!("Failed to read vim output at {:?}: {}", output_path, e)))?;

        // Extract body if requested
        let result = if body_only {
            self.extract_body(&html)?
        } else {
            html.clone()
        };

        // Cache the result
        if self.caching {
            if let Some(cache_path) = cache_path {
                self.write_cache(cache_path, &html)?;
            }
        }

        Ok(result)
    }

    /// Check cache and return cached content if valid
    fn check_cache(
        &self,
        cache_path: &Path,
        input_path: &Path,
        ignore_mtime: bool,
    ) -> Result<Option<String>, VimHighlightError> {
        if !cache_path.exists() {
            return Ok(None);
        }

        // Check mtime unless ignored
        if !ignore_mtime {
            let cache_mtime = fs::metadata(cache_path)
                .and_then(|m| m.modified())
                .ok();
            let input_mtime = fs::metadata(input_path)
                .and_then(|m| m.modified())
                .ok();

            match (cache_mtime, input_mtime) {
                (Some(cache_t), Some(input_t)) if cache_t <= input_t => {
                    return Ok(None); // Cache is stale
                }
                _ => {}
            }
        }

        // Read and validate cache
        let cached = fs::read_to_string(cache_path)
            .map_err(|e| VimHighlightError::IoError(e.to_string()))?;

        // Extract and validate settings from cache
        if let Some(info) = self.extract_info(&cached) {
            let current_info = self.encode_info();
            if info == current_info {
                return Ok(Some(cached));
            }
        }

        Ok(None)
    }

    /// Write to cache with settings info appended
    fn write_cache(&self, cache_path: &Path, html: &str) -> Result<(), VimHighlightError> {
        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| VimHighlightError::IoError(e.to_string()))?;
        }

        // Append settings info as HTML comment
        let info = self.encode_info();
        let cached_content = format!("{}\n<!-- {} -->\n", html, html_escape(&info));

        fs::write(cache_path, cached_content)
            .map_err(|e| VimHighlightError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Extract settings info from cached HTML
    fn extract_info(&self, cached: &str) -> Option<String> {
        // Find the last HTML comment
        let start = cached.rfind("<!--")?;
        let end = cached.rfind("-->")?;
        if start >= end {
            return None;
        }
        let info = &cached[start + 4..end];
        Some(html_unescape(info.trim()))
    }

    /// Strip the settings info comment from cached HTML
    fn strip_info(&self, cached: &str) -> String {
        if let Some(start) = cached.rfind("<!--") {
            cached[..start].trim_end().to_string()
        } else {
            cached.to_string()
        }
    }

    /// Extract just the body content from vim's HTML output
    fn extract_body(&self, html: &str) -> Result<String, VimHighlightError> {
        if self.use_css {
            // When using CSS, extract the <pre>...</pre> block
            let start = html.find("<pre")
                .ok_or_else(|| VimHighlightError::ParseError("No <pre> tag found".to_string()))?;
            let end = html.rfind("</pre>")
                .ok_or_else(|| VimHighlightError::ParseError("No </pre> tag found".to_string()))?;
            Ok(html[start..end + 6].to_string())
        } else {
            // When not using CSS, extract body content and wrap in div
            let body_start = html.find("<body")
                .ok_or_else(|| VimHighlightError::ParseError("No <body> tag found".to_string()))?;
            let body_end = html.rfind("</body>")
                .ok_or_else(|| VimHighlightError::ParseError("No </body> tag found".to_string()))?;

            // Extract bgcolor and text color from body tag
            let body_tag_end = html[body_start..].find('>')
                .map(|i| body_start + i + 1)
                .ok_or_else(|| VimHighlightError::ParseError("Malformed body tag".to_string()))?;

            let body_tag = &html[body_start..body_tag_end];

            // Parse bgcolor and text attributes
            let bgcolor = extract_attribute(body_tag, "bgcolor").unwrap_or("#000000");
            let textcolor = extract_attribute(body_tag, "text").unwrap_or("#ffffff");

            let inner = &html[body_tag_end..body_end];

            Ok(format!(
                r#"<div class="vimhighlight" style="color: {}; background-color: {};">{}</div>"#,
                textcolor, bgcolor, inner
            ))
        }
    }
}

/// Extract an attribute value from an HTML tag
fn extract_attribute<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let pattern = format!("{}=\"", attr);
    let start = tag.find(&pattern)? + pattern.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Simple HTML entity escaping
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Simple HTML entity unescaping
fn html_unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

/// Errors that can occur during vim highlighting
#[derive(Debug)]
pub enum VimHighlightError {
    IoError(String),
    VimError(String),
    FileNotFound(String),
    ParseError(String),
}

impl std::fmt::Display for VimHighlightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::VimError(msg) => write!(f, "Vim error: {}", msg),
            Self::FileNotFound(path) => write!(f, "File not found: {}", path),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for VimHighlightError {}

/// Convenience function matching PHP's printHlString
/// Returns HTML wrapped in <div class="vimhighlight">
pub fn highlight_string(text: &str, file_type: &str, show_lines: bool) -> Result<String, VimHighlightError> {
    let hl = VimHighlight::for_print_hl_string(file_type, show_lines);
    let body = hl.process_text(text, true)?;
    Ok(format!("<div class=\"vimhighlight\">{}\n</div>", body))
}

/// Convenience function matching PHP's printSourceFile
/// Returns HTML wrapped in <div class="vimhighlight">
pub fn highlight_file(path: &Path, show_lines: bool) -> Result<String, VimHighlightError> {
    let mut hl = VimHighlight::new();
    hl.caching = true;
    hl.color_scheme = "default".to_string();
    hl.show_lines = show_lines;
    hl.use_css = true;
    hl.set_vim_command("vim");
    // file_type is None - let vim auto-detect

    let body = hl.process_file(path, true)?;
    Ok(format!("<div class=\"vimhighlight\">{}\n</div>", body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_attribute() {
        let tag = r##"<body bgcolor="#1e1e1e" text="#dcdcdc">"##;
        assert_eq!(extract_attribute(tag, "bgcolor"), Some("#1e1e1e"));
        assert_eq!(extract_attribute(tag, "text"), Some("#dcdcdc"));
        assert_eq!(extract_attribute(tag, "nonexistent"), None);
    }

    #[test]
    fn test_html_escape_unescape() {
        let original = "foo & bar < baz > \"quoted\"";
        let escaped = html_escape(original);
        assert_eq!(escaped, "foo &amp; bar &lt; baz &gt; &quot;quoted&quot;");
        let unescaped = html_unescape(&escaped);
        assert_eq!(unescaped, original);
    }

    #[test]
    fn test_encode_info() {
        let hl = VimHighlight::for_print_hl_string("ruby", true);
        let info = hl.encode_info();
        assert!(info.contains("color_scheme:default"));
        assert!(info.contains("file_type:ruby"));
        assert!(info.contains("show_lines:true"));
        assert!(info.contains("use_css:true"));
        assert!(info.contains("vim_command:vim"));
    }

    #[test]
    fn test_default_settings() {
        let hl = VimHighlight::new();
        assert!(!hl.caching);
        assert_eq!(hl.color_scheme, "default");
        assert!(hl.file_type.is_none());
        assert!(hl.show_lines);
        assert!(!hl.use_css);
        assert_eq!(hl.vim_command, "vim");
    }

    #[test]
    fn test_print_hl_string_settings() {
        let hl = VimHighlight::for_print_hl_string("python", false);
        assert!(hl.caching);
        assert_eq!(hl.color_scheme, "default");
        assert_eq!(hl.file_type, Some("python".to_string()));
        assert!(!hl.show_lines);
        assert!(hl.use_css);
    }

    #[test]
    fn test_set_vim_command_valid() {
        let mut hl = VimHighlight::new();
        hl.set_vim_command("gvim");
        assert_eq!(hl.vim_command, "gvim");
        hl.set_vim_command("VI");
        assert_eq!(hl.vim_command, "vi");
    }

    #[test]
    fn test_set_vim_command_invalid() {
        let mut hl = VimHighlight::new();
        hl.set_vim_command("emacs"); // Should be ignored
        assert_eq!(hl.vim_command, "vim"); // Unchanged
    }

    // Integration test - requires vim to be installed
    #[test]
        fn test_process_text_basic() {
        let hl = VimHighlight::for_print_hl_string("ruby", false);
        let result = hl.process_text("puts 'hello'", true);
        if let Err(ref e) = result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let html = result.unwrap();
        eprintln!("HTML output:\n{}", html);
        assert!(html.contains("<pre"), "Missing <pre> in: {}", html);
        assert!(html.contains("</pre>"), "Missing </pre> in: {}", html);
        // Should contain the text (possibly with highlighting spans)
        assert!(html.contains("puts") || html.contains("hello"), "Missing text in: {}", html);
    }

    #[test]
        fn test_highlight_string() {
        let result = highlight_string("x = 1 + 2", "ruby", false);
        if let Err(ref e) = result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let html = result.unwrap();
        eprintln!("HTML output:\n{}", html);
        // Should be wrapped in vimhighlight div
        assert!(html.starts_with(r#"<div class="vimhighlight">"#), "Wrong start: {}", html);
        assert!(html.ends_with("</div>"), "Wrong end: {}", html);
    }

    // Tests matching PHP test script exactly
    #[test]
    fn test_php_comparison_simple_ruby() {
        // PHP Test 1: Simple Ruby without line numbers
        let mut hl = VimHighlight::new();
        hl.caching = false;
        hl.color_scheme = "default".to_string();
        hl.show_lines = false;
        hl.use_css = true;
        hl.file_type = Some("ruby".to_string());
        hl.set_vim_command("vim");

        let result = hl.process_text("puts 'hello'", true).unwrap();
        eprintln!("=== Test 1: Simple Ruby ===\n{}\n", result);

        // Expected PHP output (exact match):
        let expected = "<pre id='vimCodeElement'>\nputs <span class=\"Special\">'</span>hello<span class=\"Special\">'</span>\n</pre>";
        assert_eq!(result, expected, "Output does not match PHP");
    }

    #[test]
    fn test_php_comparison_ruby_with_lines() {
        // PHP Test 2: Ruby with line numbers
        let mut hl = VimHighlight::new();
        hl.caching = false;
        hl.color_scheme = "default".to_string();
        hl.show_lines = true;
        hl.use_css = true;
        hl.file_type = Some("ruby".to_string());
        hl.set_vim_command("vim");

        let result = hl.process_text("x = 1 + 2", true).unwrap();
        eprintln!("=== Test 2: Ruby with line numbers ===\n{}\n", result);

        // Expected PHP output (exact match):
        let expected = "<pre id='vimCodeElement'>\n<span id=\"L1\" class=\"LineNr\">1 </span>x = <span class=\"Constant\">1</span> + <span class=\"Constant\">2</span>\n</pre>";
        assert_eq!(result, expected, "Output does not match PHP");
    }

    #[test]
    fn test_php_comparison_multiline_ruby() {
        // PHP Test 3: Multi-line Ruby
        let mut hl = VimHighlight::new();
        hl.caching = false;
        hl.color_scheme = "default".to_string();
        hl.show_lines = false;
        hl.use_css = true;
        hl.file_type = Some("ruby".to_string());
        hl.set_vim_command("vim");

        let result = hl.process_text("def hello\n  puts 'Hello, World!'\nend", true).unwrap();
        eprintln!("=== Test 3: Multi-line Ruby ===\n{}\n", result);

        // Expected PHP output (exact match):
        let expected = "<pre id='vimCodeElement'>\n<span class=\"PreProc\">def</span> hello\n  puts <span class=\"Special\">'</span>Hello, World!<span class=\"Special\">'</span>\n<span class=\"PreProc\">end</span>\n</pre>";
        assert_eq!(result, expected, "Output does not match PHP");
    }

    #[test]
    fn test_php_comparison_plain_text() {
        // PHP Test 4: Plain text (no highlighting)
        let mut hl = VimHighlight::new();
        hl.caching = false;
        hl.color_scheme = "default".to_string();
        hl.show_lines = false;
        hl.use_css = true;
        hl.file_type = Some("text".to_string());
        hl.set_vim_command("vim");

        let result = hl.process_text("This is plain text\nWith multiple lines", true).unwrap();
        eprintln!("=== Test 4: Plain text ===\n{}\n", result);

        // Expected PHP output (exact match):
        let expected = "<pre id='vimCodeElement'>\nThis is plain text\nWith multiple lines\n</pre>";
        assert_eq!(result, expected, "Output does not match PHP");
    }

    #[test]
    fn test_php_comparison_html_entities() {
        // PHP Test 5: HTML entities in code
        let mut hl = VimHighlight::new();
        hl.caching = false;
        hl.color_scheme = "default".to_string();
        hl.show_lines = false;
        hl.use_css = true;
        hl.file_type = Some("ruby".to_string());
        hl.set_vim_command("vim");

        let result = hl.process_text("x = '<html>' && y > 0", true).unwrap();
        eprintln!("=== Test 5: HTML entities ===\n{}\n", result);

        // Expected PHP output (exact match):
        let expected = "<pre id='vimCodeElement'>\nx = <span class=\"Special\">'</span>&lt;html&gt;<span class=\"Special\">'</span> &amp;&amp; y &gt; <span class=\"Constant\">0</span>\n</pre>";
        assert_eq!(result, expected, "Output does not match PHP");
    }
}
