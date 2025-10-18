use {
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

mod chat;
mod not_found;

pub(crate) use {chat::*, not_found::*};

pub(crate) const STYLE_SHEET_PATH: &str = "style.css";

pub(crate) fn page(title: &str, content: Markup) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        crate::DEFAULT_HEADERS,
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    title { (title) };
                    link rel="stylesheet" href=(STYLE_SHEET_PATH);
                }
                body {
                    (content)
                }
            }
        },
    )
}
