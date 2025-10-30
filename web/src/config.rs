use {
    crate::{hash::HashKey, mailer::MailerConfig},
    anyhow::{Context, Result},
    serde::Deserialize,
    std::path::PathBuf,
};

/// The root configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct AppConfig {
    /// The key for hashing data in hex.
    pub(crate) hash_key: HashKey,

    /// The embedded database path.
    pub(crate) db_path: PathBuf,

    /// The SMTP server configuration for sending and verifying emails.
    pub(crate) mailer: MailerConfig,

    /// Sets the TLS configuration.
    ///
    /// If provided, listens on port 443 with TLS and redirects port http
    /// requests on port 80 to 443.
    pub(crate) tls: Option<TlsCertPaths>,
}

/// The paths to the tls certificate and private key.
#[derive(Debug, Deserialize)]
pub(crate) struct TlsCertPaths {
    /// The filepath to the tls certificate.
    pub(crate) cert: PathBuf,

    /// The filepath to the tls private key.
    pub(crate) key: PathBuf,
}

impl AppConfig {
    pub(crate) fn from_config_path() -> Result<Self> {
        let path =
            std::env::var("CONFIG").with_context(|| "CONFIG environment variable is not set")?;

        ron::from_str(
            &std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read file at {path:?}"))?,
        )
        .with_context(|| format!("failed to parse {path:?}"))
    }
}
