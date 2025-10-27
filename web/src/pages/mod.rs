use {
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

pub(crate) mod chat;
pub(crate) mod not_found;
pub(crate) mod privacy;
pub(crate) mod signin;
pub(crate) mod signup;
pub(crate) mod terms;

/// Compiled and minified javascript.
mod scripts {
    include!(concat!(env!("OUT_DIR"), "/scripts.rs"));
}

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
                    link rel="stylesheet" href="/style.css";
                    script src="/htmx.min.js" {}
                    (scripts::main_script())
                }
                body {
                    (content);
                }
            }
        },
    )
}
