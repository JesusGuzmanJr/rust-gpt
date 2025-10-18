use {
    anyhow::Result,
    axum::{
        Router,
        body::Body,
        http::{HeaderName, HeaderValue, Request, StatusCode, header},
        response::{IntoResponse, Response},
        routing::get,
    },
    maud::{Markup, html},
    tower::service_fn,
    tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer},
    tracing::*,
};

/// 15 minutes public cache
const CACHE_CONTROL: (HeaderName, HeaderValue) = (
    header::CACHE_CONTROL,
    HeaderValue::from_static("public, max-age=900"), // 15 minutes
);

/// HTML content type
const HTML_CONTENT_TYPE: (HeaderName, HeaderValue) = (
    header::CONTENT_TYPE,
    HeaderValue::from_static("text/html; charset=utf-8"),
);

const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 2] = [CACHE_CONTROL, HTML_CONTENT_TYPE];

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let bind_address = "127.0.0.1:3000";

    let app = Router::new()
        .route("/style.css", get(style))
        .route("/", get(root))
        .fallback_service(
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/public")).fallback(service_fn(
                |_req: Request<Body>| async move {
                    Ok::<Response, std::convert::Infallible>(not_found().into_response())
                },
            )),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ));

    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(%bind_address, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn root() -> impl IntoResponse {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                title { "Rust GPT" }
            }
            body {
                h1 { "Hello, world!" }
                p.intro {
                    "This is an example of the "
                    a href="https://github.com/lambda-fairy/maud" { "Maud" }
                    " template language."
                }
            }
        }
    }
}

async fn style() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            ),
            CACHE_CONTROL,
        ],
        include_bytes!("../../target/style.css"),
    )
}

fn page(title: &str, content: Markup) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        DEFAULT_HEADERS,
        html! {
            (maud::DOCTYPE)
            html {
                head {
                    title { (title) };
                    link rel="stylesheet" href="style.css";
                }
                body {
                    (content)
                }
            }
        },
    )
}

fn not_found() -> impl IntoResponse {
    page(
        "Not Found",
        html! {
             div.not-found-container {
                div.not-found-content {
                    div.not-found-code { "404" }
                    h1.not-found-title { "Page Not Found" }
                    p.not-found-description {
                        "The page you're looking for doesn't exist or has been moved."
                    }
                    a.not-found-button href="/" {
                        // Home icon placeholder - you can add an SVG or icon font here
                        span.not-found-button-icon { "🏠" }
                        span { "Back to Home" }
                    }
                }
            }
        },
    )
}
