use askama::Template;
use askama_axum::IntoResponse;
use std::path::Path;

use crate::context::PageContext;
use crate::vim_highlight::{highlight_string, highlight_file};

#[derive(Template)]
#[template(path = "pages/blind_birthday_attack.html")]
pub struct BlindBirthdayAttackPage {
    pub ctx: PageContext,
    pub output_highlighted: String,
    pub source_highlighted: String,
}

pub async fn get(ctx: PageContext) -> impl IntoResponse {
    // The program output (displayed as plain text with no line numbers)
    let output_text = r#"Closest collision so far: 1
Tree size: 0
Closest collision so far: 2
Tree size: 1
Closest collision so far: 3
Tree size: 4
Closest collision so far: 7
Tree size: 5
Closest collision so far: 9
Tree size: 11
Closest collision so far: 10
Tree size: 52
Closest collision so far: 14
Tree size: 86
Closest collision so far: 16
Tree size: 200
Closest collision so far: 17
Tree size: 347
Closest collision so far: 18
Tree size: 407
Closest collision so far: 19
Tree size: 679
Closest collision so far: 20
Tree size: 2895
Closest collision so far: 22
Tree size: 3256
Closest collision so far: 23
Tree size: 6067
Closest collision so far: 25
Tree size: 6678
Closest collision so far: 29
Tree size: 12976
Closest collision so far: 30
Tree size: 13006
Closest collision so far: 32
Tree size: 33425
Found a collision amongst 33425 in 445046 queries!
Message 1: 8db04aea6b6b8d3a80d93d7064ec78a0bd24a8cba3a56d3c2f8755d8a9b63a40
Message 2: 99cec515fff6c583134b0942c0e6381ebdb10c07b472f58fd74e1cb0fbb684b8"#;

    let output_highlighted = highlight_string(output_text, "text", false)
        .unwrap_or_else(|e| format!("<pre>Error highlighting output: {}</pre>", e));

    let source_path = Path::new("static/source/blind-birthday.rb");
    let source_highlighted = highlight_file(source_path, false)
        .unwrap_or_else(|e| format!("<pre>Error highlighting source: {}</pre>", e));

    BlindBirthdayAttackPage {
        ctx,
        output_highlighted,
        source_highlighted,
    }
}
