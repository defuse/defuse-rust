use super::util::html_escape;

/// A single reference entry in a bibliography.
pub enum Reference<'a> {
    /// A simple link reference (no author/date).
    Simple { title: &'a str, url: &'a str },
    /// A full academic citation.
    Full { authors: &'a str, date: &'a str, title: &'a str, url: &'a str },
}

/// Bibliography for academic-style citations.
#[derive(Debug, Clone)]
pub struct Bibliography {
    /// Rendered HTML for each reference, in order.
    references: Vec<String>,
}

impl Bibliography {
    /// Create a new bibliography from a list of references.
    ///
    /// Citations are numbered 1, 2, 3, ... based on position.
    pub fn new(refs: &[Reference]) -> Self {
        let references = refs.iter().map(|reference| {
            match reference {
                Reference::Simple { title, url } => {
                    let safe_title = html_escape(title);
                    let safe_url = html_escape(url);
                    format!("<a href=\"{safe_url}\">{safe_title}</a>")
                }
                Reference::Full { authors, date, title, url } => {
                    let safe_authors = html_escape(authors);
                    let safe_date = html_escape(date);
                    let safe_title = html_escape(title);
                    let safe_url = html_escape(url);
                    format!("{safe_authors}. {safe_date}. <a href=\"{safe_url}\">{safe_title}.</a>")
                }
            }
        }).collect();
        Bibliography { references }
    }

    /// Generate a citation link for inline use.
    /// Outputs: <sup><a href="#cite_N">[N]</a></sup>
    pub fn cite(&self, index: usize) -> String {
        if index >= 1 && index <= self.references.len() {
            let safe_key = html_escape(&index.to_string());
            format!(
                "<sup><a href=\"#cite_{safe_key}\">[{safe_key}]</a></sup>",
            )
        } else {
            "<sup>ERROR: INVALID KEY</sup>".to_string()
        }
    }

    /// Render the full bibliography section as HTML
    pub fn render(&self) -> String {
        let mut html = String::new();
        html.push_str("<div id=\"references\">");
        html.push_str("<h2>References and Notes</h2>");

        for (i, ref_html) in self.references.iter().enumerate() {
            let safe_key = html_escape(&(i + 1).to_string());
            html.push_str("<div class=\"ref_item\">");
            html.push_str(&format!(
                "<a name=\"cite_{}\"></a>{}. <span id=\"cite_{}\">{}</span>",
                safe_key, safe_key, safe_key, ref_html
            ));
            html.push_str("</div>");
        }

        html.push_str("</div>");
        html
    }
}
