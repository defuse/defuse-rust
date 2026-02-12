use super::util::html_escape;
use std::collections::BTreeMap;

/// Bibliography for academic-style citations.
#[derive(Debug, Clone)]
pub struct Bibliography {
    /// Map of key -> rendered HTML for each reference
    references: BTreeMap<String, String>,
}

impl Bibliography {
    /// Create a new bibliography from a list of references.
    ///
    /// Each tuple is (key, title, authors, date, url).
    /// For general references (no author/date), pass empty strings for authors and date.
    pub fn new(refs: &[(&str, &str, &str, &str, &str)]) -> Self {
        // AUDIT: instead of passing a big array, add more-usable methods for adding citations
        let mut references = BTreeMap::new();
        for (key, title, authors, date, url) in refs {
            let safe_title = html_escape(title);
            let safe_url = html_escape(url);

            let html = if authors.is_empty() && date.is_empty() {
                // General reference: just <a href="url">text</a>
                format!("<a href=\"{safe_url}\">{safe_title}</a>")
            } else {
                // Full reference: authors. date. <a href="url">title.</a>
                let safe_authors = html_escape(authors);
                let safe_date = html_escape(date);
                format!("{safe_authors}. {safe_date}. <a href=\"{safe_url}\">{safe_title}.</a>")
            };

            references.insert(key.to_string(), html);
        }
        Bibliography { references }
    }

    /// Generate a citation link for inline use.
    /// Outputs: <sup><a href="#cite_KEY">[KEY]</a></sup>
    pub fn cite(&self, key: &str) -> String {
        let safe_key = html_escape(key);
        if self.references.contains_key(key) {
            format!(
                "<sup><a href=\"#cite_{}\">[{}]</a></sup>",
                safe_key, safe_key
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

        // BTreeMap is already sorted by key
        for (key, ref_html) in &self.references {
            let safe_key = html_escape(key);
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
