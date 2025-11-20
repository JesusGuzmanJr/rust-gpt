use {
    anyhow::{Context, Result},
    axum::{
        BoxError, Router,
        handler::HandlerWithoutStateExt,
        http::{HeaderName, HeaderValue, Uri, header, uri::Authority},
        response::{IntoResponse, Redirect, Response},
        routing::get,
    },
    axum_extra::extract::Host,
    bytesize::ByteSize,
    std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    },
    tower::{ServiceBuilder, service_fn, timeout::TimeoutLayer},
    tower_http::{
        catch_panic::CatchPanicLayer, compression::CompressionLayer, services::ServeDir,
        set_header::SetResponseHeaderLayer,
    },
    tracing::*,
};

mod auth;
mod config;
mod datetime;
mod error;
mod hash;
mod http;
mod internationalization;
mod job;
mod mailer;
mod message;
mod pages;
mod persistence;
mod scheduler;
mod svg;
mod thread;
mod user;

/// Set low to prevent abuse.
const MAX_REQUEST_BODY_SIZE: ByteSize = ByteSize::kib(16);

const DEFAULT_HEADERS: [(HeaderName, HeaderValue); 2] =
    [http::CACHE_PUBLICLY_15_MIN, http::HTML_CONTENT_TYPE];

const PROJECT_NAME: &str = "rust-gpt";
const PROJECT_URL: &str = "https://rust-gpt.marzipanclub.com";
const TEAM_EMAIL: &str = "hello@marzipanclub.com";

const HTTP_PORT: u16 = 3000;
const SRC_HTTP_PORT_REDIRECT: u16 = 3080;
const HTTPS_PORT: u16 = 3443;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_line_number(true)
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let config = config::AppConfig::from_config_path()?;

    hash::init(config.hash_key)?;
    persistence::init(&config.db_path)?;
    mailer::init(config.mailer).await?;

    let app = Router::new()
        .route("/", get(|| async { Redirect::to(pages::chat::PATH) }))
        .route(pages::about::PATH, get(pages::about::page))
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
                        pages::not_found_page().into_response(),
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
        .layer(
            ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |_: BoxError| async { axum::http::StatusCode::REQUEST_TIMEOUT },
                ))
                .layer(TimeoutLayer::new(REQUEST_TIMEOUT)),
        )
        .layer(tower_governor::GovernorLayer::new({
            // Allow bursts with up to ten requests per IP address
            // and replenishes one element every two seconds
            let governor_config = tower_governor::governor::GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(10)
                .finish()
                .context("invalid rate limiting governor configuration")?;

            let governor_limiter = governor_config.limiter().clone();

            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(20));
                    governor_limiter.retain_recent();
                }
            });

            governor_config
        }))
        .layer(CatchPanicLayer::custom(error::PanicResponder))
        .layer(CompressionLayer::new());

    let bind_address = if config.tls.is_some() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), HTTPS_PORT)
    } else {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), HTTP_PORT)
    };
    info!(%bind_address);

    if let Some(tls) = config.tls {
        let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(tls.cert, tls.key)
            .await
            .context("failed to parse tls cert and key")?;

        tokio::spawn(async move {
            if let Err(error) = redirect_http_to_https().await {
                error!(?error, "failed to redirect HTTP to HTTPS");
            }
        });

        axum_server::bind_rustls(bind_address, config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?;
    } else {
        axum_server::bind(bind_address)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?;
    };

    Ok(())
}

async fn redirect_http_to_https() -> Result<()> {
    fn make_https(host: &str, uri: Uri) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse()?);
        }

        let authority = host.parse::<Authority>()?;
        let bare_host = match authority.port() {
            Some(port_struct) => authority
                .as_str()
                .strip_suffix(port_struct.as_str())
                .context("suffix not found")?
                .strip_suffix(':')
                .context("colon not found")?,
            None => authority.as_str(),
        };

        parts.authority = Some(format!("{bare_host}:443").parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(?error, "failed to convert URI to HTTPS");
                Err(axum::http::StatusCode::BAD_REQUEST)
            }
        }
    };

    let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), SRC_HTTP_PORT_REDIRECT);
    info!(%bind_address);
    axum_server::bind(bind_address)
        .serve(redirect.into_make_service())
        .await?;

    Ok(())
}
