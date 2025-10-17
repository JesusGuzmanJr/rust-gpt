use {
    anyhow::Result,
    axum::{
        Router,
        http::{HeaderValue, header},
        response::IntoResponse,
        routing::get,
    },
    maud::html,
    tracing::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let bind_address = "127.0.0.1:3000";
    let app = Router::new()
        .route("/style.css", get(style))
        .route("/", get(root));

    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(%bind_address, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    info!("Root endpoint called");
    let r = html! {
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
    };
    println!("{}", r.clone().into_string());
    r
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
