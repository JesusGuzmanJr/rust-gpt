use {
    anyhow::Result,
    axum::{
        Router,
        http::{HeaderName, HeaderValue, header},
        response::{IntoResponse, Redirect, Response},
        routing::get,
    },
    tower::service_fn,
    tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer},
    tracing::*,
};

mod http;
mod pages;

const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 2] =
    [http::CACHE_PUBLICLY_15_MIN, http::HTML_CONTENT_TYPE];

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let bind_address = "127.0.0.1:3000";

    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    info!(%bind_address, "listening");
    axum::serve(
        listener,
        Router::new()
            .route("/", get(|| async { Redirect::to("/chat") }))
            .route("/chat", get(pages::chat::page))
            .route(
                "/style.css",
                get((
                    [
                        (
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/css; charset=utf-8"),
                        ),
                        http::CACHE_PUBLICLY_15_MIN,
                    ],
                    include_bytes!("../../target/style.css"),
                )),
            )
            .nest("/api", Router::new().merge(pages::chat::api()))
            .fallback_service(
                ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/public")).fallback(service_fn(
                    |_| async move {
                        Ok::<Response, std::convert::Infallible>(
                            pages::not_found::page().into_response(),
                        )
                    },
                )),
            )
            .layer(SetResponseHeaderLayer::if_not_present(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            )),
    )
    .await?;
    Ok(())
}
