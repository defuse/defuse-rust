use comrak::{markdown_to_html, Options};

/// Render a README markdown string to HTML, stripping the leading `# ...` heading
/// if present (since pages provide their own `<h1>`).
pub fn render_readme(md: &str) -> String {
    let body = match md.strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map_or("", |(_, after)| after),
        None => md,
    };

    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;

    markdown_to_html(body, &options)
}
