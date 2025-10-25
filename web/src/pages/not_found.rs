use {crate::svg, axum::response::IntoResponse, maud::html};

pub(crate) fn page() -> impl IntoResponse {
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
                    (svg::home(16, 16))
                    span { "Go Home" }
                }
            }
        },
    )
}
