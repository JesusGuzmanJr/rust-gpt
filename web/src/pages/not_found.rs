use {axum::response::IntoResponse, maud::html};

pub(crate) fn not_found() -> impl IntoResponse {
    super::error_page(
        axum::http::StatusCode::NOT_FOUND,
        "Not Found",
        html! {
            div.error-page__container {
                h1.error-page__code { "404" }
                h2.error-page__title { "Page Not Found" }
                p.error-page__text {
                    "The page you're looking for doesn't exist or has been moved."
                }
                a.button.button--primary href="/" {
                    svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                        path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z";
                        polyline points="9 22 9 12 15 12 15 22";
                    }
                    span { "Go Home" }
                }
            }
        },
    )
}
