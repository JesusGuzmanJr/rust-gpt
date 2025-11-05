use {
    crate::svg,
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

pub(crate) mod about;
pub(crate) mod chat;
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

pub(crate) fn not_found_page() -> impl IntoResponse {
    error_page(
        StatusCode::NOT_FOUND,
        "Not Found",
        "404",
        "Page Not Found",
        "The page you're looking for doesn't exist or has been moved.",
    )
}

pub(crate) fn internal_server_error_page(message: &str) -> impl IntoResponse {
    error_page(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Server Error",
        "500",
        "Internal Server Error",
        message,
    )
}

fn error_page(
    status_code: StatusCode,
    title: &str,
    h1: &str,
    h2: &str,
    message: &str,
) -> impl IntoResponse {
    html_page(
        status_code,
        title,
        html! {
            div.error-page__container {
                h1.error-page__code { (h1) }
                h2.error-page__title { (h2) }
                p.error-page__text {
                    (message)
                }
                a.button.button--primary href="/" {
                    (svg::home(16, 16))
                    span { "Go Home" }
                }
            }
        },
    )
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
