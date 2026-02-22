use comrak::{markdown_to_html, Options};

/// Render a README markdown string to HTML, stripping the leading `# ...` heading
/// if present (since pages provide their own `<h1>`), and shifting all heading
/// levels down by one (h1->h2, h2->h3, etc.) so they nest under the page heading.
pub fn render_readme(md: &str) -> String {
    let body = match md.strip_prefix("# ") {
        Some(rest) => rest.split_once('\n').map_or("", |(_, after)| after),
        None => md,
    };

    // Strip badge lines (e.g. build status, codecov, crates.io, docs.rs)
    let body: String = body
        .lines()
        .filter(|line| !line.trim_start().starts_with("[!["))
        .collect::<Vec<_>>()
        .join("\n");

    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;

    let html = markdown_to_html(&body, &options);
    demote_headings(&html)
}

/// Shift all HTML heading levels down by one (h1->h2, ..., h5->h6, h6 stays h6).
fn demote_headings(html: &str) -> String {
    // Replace from h5->h6 down to h1->h2 to avoid double-replacing
    let mut result = html.to_string();
    for level in (1..=5).rev() {
        let from_open = format!("<h{}", level);
        let to_open = format!("<h{}", level + 1);
        let from_close = format!("</h{}", level);
        let to_close = format!("</h{}", level + 1);
        result = result.replace(&from_open, &to_open).replace(&from_close, &to_close);
    }
    result
}
