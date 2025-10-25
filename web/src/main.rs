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

mod auth;
mod chat;
mod database;
mod datetime;
mod error;
mod http;
mod pages;
mod user;

const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 2] =
    [http::CACHE_PUBLICLY_15_MIN, http::HTML_CONTENT_TYPE];

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    auth::init([0u8; blake3::KEY_LEN]);
    database::init(std::path::Path::new("../target/web.db"))?;

    let bind_address = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    info!(%bind_address, "listening");
    axum::serve(
        listener,
        Router::new()
            .route("/", get(|| async { Redirect::to(pages::chat::PATH) }))
            .route(pages::chat::PATH, get(pages::chat::page))
            .route(pages::privacy::PATH, get(pages::privacy::page))
            .route(pages::signin::PATH, get(pages::signin::page))
            .route(pages::signup::PATH, get(pages::signup::page))
            .route(pages::terms::PATH, get(pages::terms::page))
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
                    include_bytes!(concat!(env!("OUT_DIR"), "/style.css")),
                )),
            )
            .nest(
                "/api",
                Router::new()
                    .merge(pages::chat::api())
                    .merge(pages::signin::api())
                    .merge(pages::signup::api()),
            )
            .fallback_service(
                ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")).fallback(service_fn(
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
            ))
            .layer(axum_htmx::AutoVaryLayer),
    )
    .await?;
    Ok(())
}
