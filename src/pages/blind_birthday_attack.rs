use askama::Template;
use askama_axum::IntoResponse;

use crate::context::PageContext;

#[derive(Template)]
#[template(path = "pages/blind_birthday_attack.html")]
pub struct BlindBirthdayAttackPage {
    pub ctx: PageContext,
}

pub async fn get(ctx: PageContext) -> impl IntoResponse {
    BlindBirthdayAttackPage { ctx }
}
