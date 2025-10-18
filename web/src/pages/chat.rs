use {super::page, axum::response::IntoResponse, maud::html};

pub(crate) async fn chat() -> impl IntoResponse {
    page(
        "Chat",
        html! {
            h1 { "Chat" }
        },
    )
}
