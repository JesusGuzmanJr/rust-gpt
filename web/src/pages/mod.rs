use {
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

mod chat;
mod not_found;

pub(crate) use {chat::*, not_found::*};

pub(crate) fn page(title: &str, content: Markup) -> impl IntoResponse {
    html_page(StatusCode::OK, title, content)
}

pub(crate) fn error_page(
    status_code: StatusCode,
    title: &str,
    content: Markup,
) -> impl IntoResponse {
    html_page(status_code, title, content)
}

fn html_page(status_code: StatusCode, title: &str, content: Markup) -> impl IntoResponse {
    (
        status_code,
        crate::DEFAULT_HEADERS,
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    title { (title) };
                    link rel="stylesheet" href="style.css";
                    script async src="htmx.min.js" {}
                }
                body {
                    (content);
                }
            }
        },
    )
}
