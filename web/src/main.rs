use {
    anyhow::{Context, Result},
    axum::{
        Router,
        http::{HeaderName, HeaderValue, header},
        response::{IntoResponse, Redirect, Response},
        routing::get,
    },
    bytesize::ByteSize,
    const_format::formatcp,
    std::thread,
    tower::service_fn,
    tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer},
    tracing::*,
};

mod auth;
mod chat;
mod config;
mod datetime;
mod error;
mod http;
mod mailer;
mod pages;
mod persistence;
mod svg;
mod user;
mod verification;

/// Set low to prevent abuse.
const MAX_REQUEST_BODY_SIZE: ByteSize = ByteSize::kib(32);

const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 2] =
    [http::CACHE_PUBLICLY_15_MIN, http::HTML_CONTENT_TYPE];

const PROJECT_NAME: &str = "rust-gpt";
const PROJECT_URL: &str = formatcp!("https://{PROJECT_NAME}.marzipanclub.com");

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::AppConfig::from_config_path()?;

    auth::init(config.auth);
    persistence::init(&config.persistence)?;
    mailer::init(config.mailer).await?;

    let bind_address = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    println!(
        "{}",
        verification::verification_email(
            user::Name::new("Jesus Guzman, Jr."),
            "https://www.google.com".to_string(),
        )?
    );

    std::process::exit(0);

    // mailer::send_email(
    //     &user::EmailAddress::new("jesusguzmanjr@icloud.com"),
    //     "Verify your email address",
    //     auth::verification_email(
    //         user::Name::new("Jesus Guzman, Jr."),
    //         "https://www.google.com".to_string(),
    //     )?,
    //     lettre::message::header::ContentType::TEXT_HTML,
    // )
    // .await?;

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
            .layer(axum_htmx::AutoVaryLayer)
            .layer(axum::extract::DefaultBodyLimit::max(
                MAX_REQUEST_BODY_SIZE.as_u64() as _,
            ))
            .layer(tower_governor::GovernorLayer::new({
                // Allow bursts with up to ten requests per IP address
                // and replenishes one element every two seconds
                let governor_config = tower_governor::governor::GovernorConfigBuilder::default()
                    .per_second(3)
                    .burst_size(10)
                    .finish()
                    .context("invalid rate limiting governor configuration")?;

                let governor_limiter = governor_config.limiter().clone();

                thread::spawn(move || {
                    loop {
                        thread::sleep(std::time::Duration::from_secs(20));
                        governor_limiter.retain_recent();
                    }
                });

                governor_config
            }))
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
