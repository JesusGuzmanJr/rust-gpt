use {
    axum::{http::StatusCode, response::IntoResponse},
    maud::{Markup, html},
};

pub(crate) mod chat;
pub(crate) mod not_found;

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
                    script src="htmx.min.js" {}
                    script {
                        (maud::PreEscaped(r#"
                        (function() {
                        document.cookie = `locale=${encodeURIComponent(navigator.language)}; path=/; SameSite=Strict`;
                        document.cookie = `timezone=${encodeURIComponent(Intl.DateTimeFormat().resolvedOptions().timeZone)}; path=/; SameSite=Strict`;
                        })();
                        "#))
                    }
                }
                body {
                    (content);
                }
            }
        },
    )
}
