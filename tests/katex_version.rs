//! The KaTeX URLs in templates/base.html carry `?v=<version>`, because static files are
//! served without Cache-Control and browsers would otherwise keep running a cached copy
//! (or mix old JS with new CSS) after static/katex/ is upgraded. This checks that the
//! query string matches the KaTeX version actually vendored in static/katex/.

use regex::Regex;
use std::fs;
use std::path::Path;

fn read_repo_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e))
}

/// The version string KaTeX embeds in its minified build, e.g. `version:"0.19.1"`.
fn vendored_katex_version() -> String {
    let katex_js = read_repo_file("static/katex/katex.min.js");
    let version_pattern = Regex::new(r#"version:"([0-9]+\.[0-9]+\.[0-9]+)""#)
        .expect("version pattern is a valid regex");
    let versions: Vec<String> = version_pattern
        .captures_iter(&katex_js)
        .map(|captures| captures[1].to_string())
        .collect();
    assert_eq!(versions.len(), 1, "expected exactly one version string in katex.min.js, found {versions:?}");
    versions.into_iter().next().expect("length was just checked")
}

#[test]
fn katex_urls_carry_the_vendored_katex_version() {
    let version = vendored_katex_version();
    let template = read_repo_file("templates/base.html");
    // Only src="..." and href="..." values: the template also mentions static/katex/ in a comment.
    let url_pattern =
        Regex::new(r#"(?:src|href)="(/katex/[^"]*)""#).expect("URL pattern is a valid regex");
    let urls: Vec<&str> = url_pattern
        .captures_iter(&template)
        .map(|captures| captures.get(1).expect("group 1 always participates").as_str())
        .collect();
    assert_eq!(
        urls,
        vec![
            format!("/katex/katex.min.css?v={version}"),
            format!("/katex/katex.min.js?v={version}"),
            format!("/katex/contrib/auto-render.min.js?v={version}"),
        ]
    );
}
