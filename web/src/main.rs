use {
    anyhow::Result,
    axum::{
        Router,
        body::Body,
        http::{HeaderValue, Request, StatusCode, header},
        response::{IntoResponse, Response},
        routing::get,
    },
    maud::html,
    tower::service_fn,
    tower_http::services::ServeDir,
    tracing::*,
};

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
        );

    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(%bind_address, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Hello, world!" }
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

/// Get the CSS file
async fn style() -> impl IntoResponse {
    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/css; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=900"), // 15 minutes
            ),
        ],
        include_bytes!("../../target/style.css"),
    )
}

fn not_found() -> impl IntoResponse {
    let doc = html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "Not Found" }
            }
            body {
                h1 { "404 - Not Found" }
                p { "The requested resource could not be found." }
            }
        }
    };
    (StatusCode::NOT_FOUND, doc)
}
