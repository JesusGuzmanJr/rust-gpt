use {
    crate::user::EmailAddress,
    anyhow::{Context, Result, bail},
    common::{key_type, string_type},
    lettre::{
        Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
        message::{Mailbox, header::ContentType},
        transport::smtp::PoolConfig,
    },
    serde::Deserialize,
    std::{sync::OnceLock, time::Duration},
};

const SENDER_NAME: &str = "Rust GPT Devs";
const SENDER_EMAIL: &str = "hello@marzipanclub.com";

/// Timeout for SMTP connection.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(3);

/// SMTP minimum idle connections.
const MIN_IDLE_CONNECTIONS: u32 = 3;

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
#[derive(Debug, Deserialize)]
pub(crate) struct MailerConfig {
    host: Host,
    port: u16,
    username: Username,
    password: Password,
}

/// Creates a SMTP transport and tests the connection.
#[deny(dead_code)]
pub(crate) async fn init(config: MailerConfig) -> Result<()> {
    let transport = build_smtp_transport(config)?;

    SENDER_EMAIL_ADDRESS
        .set(
            SENDER_EMAIL
                .parse()
                .context("invalid sender email address")?,
        )
        .expect("already initialized");

    tracing::info!("testing SMTP connection");
    if !transport
        .test_connection()
        .await
        .context("unable to test SMTP connection")?
    {
        bail!("SMTP connection failed");
    }
    tracing::info!("SMTP connection successful");

    SMTP_TRANSPORT.set(transport).expect("already initialized");

    Ok(())
}

fn build_smtp_transport(config: MailerConfig) -> Result<SmtpTransport> {
    Ok(SmtpTransport::starttls_relay(&config.host)
        .context("invalid SMTP host")?
        .pool_config(PoolConfig::default().min_idle(MIN_IDLE_CONNECTIONS))
        .timeout(Some(CONNECTION_TIMEOUT))
        .port(config.port)
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            config.username.0,
            config.password.0.clone(),
        ))
        .build())
}

/// Sends an email.
pub(crate) async fn send_email(
    email: &EmailAddress,
    subject: &str,
    body: String,
    content_type: ContentType,
) -> Result<(), anyhow::Error> {
    let response = smtp_transport()
        .send(
            Message::builder()
                .from(Mailbox::new(
                    Some(SENDER_NAME.to_string()),
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
