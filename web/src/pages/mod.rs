use {
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

mod chat;
mod not_found;

pub(crate) use {chat::*, not_found::*};

pub(crate) const STYLE_SHEET_PATH: &str = "style.css";

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
                    // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/base
                    base target="htmz";
                    title { (title) };
                    link rel="stylesheet" href=(STYLE_SHEET_PATH);
                }
                body {
                    (content);
                    // https://leanrada.com/htmz/
                    iframe hidden name="htmz" onload="setTimeout(()=>document.querySelector(contentWindow.location.hash||null)?.replaceWith(...contentDocument.body.childNodes))" {}
                }
            }
        },
    )
}
