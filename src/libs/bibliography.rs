//! Bibliography system for academic-style citations.
//!
//! Matches PHP's Bibliography.php output exactly.

use super::util::html_escape;

/// A single reference in the bibliography.
#[derive(Debug, Clone)]
pub struct Reference {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub date: String,
    pub url: String,
}

/// Bibliography for academic-style citations.
///
/// Created immutably with all references, then used in templates
/// to generate citation links and the bibliography section.
#[derive(Debug, Clone)]
pub struct Bibliography {
    references: Vec<Reference>,
}

impl Bibliography {
    /// Create a new bibliography from a list of references.
    ///
    /// Each tuple is (key, title, authors, date, url).
    pub fn new(refs: &[(&str, &str, &str, &str, &str)]) -> Self {
        let references = refs
            .iter()
            .map(|(key, title, authors, date, url)| Reference {
                key: key.to_string(),
                title: title.to_string(),
                authors: authors.to_string(),
                date: date.to_string(),
                url: url.to_string(),
            })
            .collect();
        Bibliography { references }
    }

    /// Generate a citation link for inline use.
    /// Outputs: <sup><a href="#cite_KEY">[KEY]</a></sup>
    pub fn cite(&self, key: &str) -> String {
        let safe_key = html_escape(key);
        if self.references.iter().any(|r| r.key == key) {
            format!(
                "<sup><a href=\"#cite_{}\">[{}]</a></sup>",
                safe_key, safe_key
            )
        } else {
            "<sup>ERROR: INVALID KEY</sup>".to_string()
        }
    }

    /// Render the full bibliography section.
    /// Call this at the end of the page.
    pub fn render(&self) -> String {
        let mut html = String::new();
        html.push_str("<div id=\"references\">");
        html.push_str("<h2>References and Notes</h2>");

        // Sort references by key
        let mut sorted_refs: Vec<_> = self.references.iter().collect();
        sorted_refs.sort_by(|a, b| a.key.cmp(&b.key));

        for r in sorted_refs {
            let safe_key = html_escape(&r.key);
            let safe_title = html_escape(&r.title);
            let safe_authors = html_escape(&r.authors);
            let safe_date = html_escape(&r.date);
            let safe_url = html_escape(&r.url);

            html.push_str("<div class=\"ref_item\">");
            html.push_str(&format!(
                "<a name=\"cite_{}\"></a>{}. <span id=\"cite_{}\">{safe_authors}. {safe_date}. <a href=\"{safe_url}\">{safe_title}.</a></span>",
                safe_key, safe_key, safe_key
            ));
            html.push_str("</div>");
        }

        html.push_str("</div>");
        html
    }
}
