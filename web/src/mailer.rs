use {
    crate::{TEAM_EMAIL, user::EmailAddress},
    anyhow::{Context, Result, bail},
    common::{key_type, string_type},
    lettre::{
        Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
        message::{Mailbox, header::ContentType},
        transport::smtp::PoolConfig,
    },
    quick_cache::sync::Cache,
    serde::Deserialize,
    std::{
        sync::{LazyLock, OnceLock},
        time::Duration,
    },
    tracing::*,
};

const SMTP_PORT: u16 = 587;

/// Timeout for SMTP connection.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);

/// SMTP minimum idle connections.
const MIN_IDLE_CONNECTIONS: u32 = 3;

/// Timeout for DNS MX record lookup.
const EMAIL_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

const _: () = assert!(
    EMAIL_CHECK_TIMEOUT.as_secs() < crate::REQUEST_TIMEOUT.as_secs(),
    "EMAIL_CHECK_TIMEOUT must be less than REQUEST_TIMEOUT"
);

type SmtpTransport = AsyncSmtpTransport<Tokio1Executor>;

static SMTP_TRANSPORT: OnceLock<SmtpTransport> = OnceLock::new();
static SENDER_EMAIL_ADDRESS: OnceLock<Address> = OnceLock::new();

fn sender_email_address() -> &'static Address {
    SENDER_EMAIL_ADDRESS.get().expect("not initialized")
}

/// Returns a reference to the SMTP transport.
#[inline]
fn smtp_transport() -> &'static SmtpTransport {
    SMTP_TRANSPORT.get().expect("not initialized")
}

string_type!(Host);
string_type!(Username);
key_type!(Password);
string_type!(SenderName);
string_type!(SenderEmail);

/// The SMTP server configuration.
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct MailerConfig {
    host: Host,
    username: Username,
    password: Password,
}

/// Creates a SMTP transport and tests the connection.
#[deny(dead_code)]
pub(crate) async fn init(config: MailerConfig) -> Result<()> {
    let transport = build_smtp_transport(config)?;

    SENDER_EMAIL_ADDRESS
        .set(TEAM_EMAIL.parse().context("invalid sender email address")?)
        .expect("already initialized");

    if !cfg!(debug_assertions) {
        tracing::info!("testing SMTP connection");
        if !transport
            .test_connection()
            .await
            .context("unable to test SMTP connection")?
        {
            bail!("SMTP connection failed");
        }
        tracing::info!("SMTP connection successful");
    }

    SMTP_TRANSPORT.set(transport).expect("already initialized");

    Ok(())
}

fn build_smtp_transport(config: MailerConfig) -> Result<SmtpTransport> {
    Ok(SmtpTransport::starttls_relay(&config.host)
        .context("invalid SMTP host")?
        .pool_config(PoolConfig::default().min_idle(MIN_IDLE_CONNECTIONS))
        .timeout(Some(CONNECTION_TIMEOUT))
        .port(SMTP_PORT)
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            config.username.0,
            config.password.0.clone(),
        ))
        .build())
}

/// Check if the MX records for an email address are valid.
pub(crate) async fn are_mx_records_valid(email: &EmailAddress) -> bool {
    // no need to cache these
    if !mailchecker::is_valid(email.as_str()) {
        tracing::warn!(%email, "email is not valid");
        return false;
    }

    static CACHE: LazyLock<Cache<EmailAddress, bool>> = LazyLock::new(|| Cache::new(1024));

    if let Some(is_sendable) = CACHE.get(email) {
        trace!(%email, is_sendable, "email returned from cache");
        is_sendable
    } else {
        let results = match tokio::time::timeout(
            EMAIL_CHECK_TIMEOUT,
            check_if_email_exists::check_email(&check_if_email_exists::CheckEmailInput::new(
                email.to_string(),
            )),
        )
        .await
        {
            Ok(results) => results,
            Err(_) => {
                tracing::warn!(%email, "email validation check timed out; assuming email might be valid");
                CACHE.insert(email.clone(), true);
                return true;
            }
        };

        // only check mx records; we don't want to ask the receiver's email server if
        // *we* can email them because we're on a lowly residential ISP
        let is_sendable = if results.syntax.is_valid_syntax
            && results
                .mx
                .as_ref()
                .map(|mx| mx.lookup.is_ok())
                .unwrap_or(false)
        {
            true
        } else {
            tracing::warn!(%email, mx = ?results.mx, "email is not sendable");
            false
        };

        CACHE.insert(email.clone(), is_sendable);
        trace!(%email, is_sendable, "email inserted into cache");
        is_sendable
    }
}

/// Sends an email.
pub(crate) async fn send_email(
    email: &EmailAddress,
    subject: &str,
    body: String,
    content_type: ContentType,
) -> Result<(), anyhow::Error> {
    if !are_mx_records_valid(email).await {
        bail!("MX records are not valid");
    }

    let response = smtp_transport()
        .send(
            Message::builder()
                .from(Mailbox::new(
                    Some(TEAM_EMAIL.to_string()),
                    sender_email_address().clone(),
                ))
                .to(email.as_str().parse()?)
                .subject(subject)
                .header(content_type)
                .body(body)?,
        )
        .await
        .context("unable to send email")?;

    if !response.is_positive() {
        tracing::error!(?response, "received an unsuccessful SMTP response");
        bail!("received an unsuccessful SMTP response");
    }

    Ok(())
}
