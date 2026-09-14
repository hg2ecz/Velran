use language_core::{Html, Redirect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppResponse {
    Html(Html),
    Json(String),
    Redirect(Redirect),
}
