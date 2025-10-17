use {
    anyhow::Result,
    axum::{Router, response::IntoResponse, routing::get},
    maud::html,
    tracing::*,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let bind_address = "127.0.0.1:3000";
    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(%bind_address, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

// basic handler that responds with a static string
async fn root() -> impl IntoResponse {
    info!("Root endpoint called");
    html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
    }
}
