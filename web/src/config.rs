use {
    crate::{hash::HashKey, mailer::MailerConfig},
    anyhow::{Context, Result},
    serde::{Deserialize, Deserializer},
    std::{
        io::BufReader,
        path::{Path, PathBuf},
    },
};

pub(crate) type TlsConfig = rustls::ServerConfig;

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
    #[serde(deserialize_with = "parse_tls_config")]
    pub(crate) tls: Option<TlsConfig>,
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

/// The paths to the tls certificate and private key.
#[derive(Deserialize)]
struct TlsCertPaths {
    /// The filepath to the tls certificate.
    cert: PathBuf,

    /// The filepath to the tls private key.
    cert_key: PathBuf,
}

fn parse_tls_config<'de, D>(deserializer: D) -> Result<Option<TlsConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    let tls: Option<TlsCertPaths> = Deserialize::deserialize(deserializer)?;
    tls.map(|tls| create_tls(&tls.cert, &tls.cert_key).map_err(serde::de::Error::custom))
        .transpose()
}

fn create_tls(cert: &Path, cert_key: &Path) -> Result<TlsConfig> {
    let open = |path| std::fs::File::open(path).with_context(|| format!("error opening {path:?}"));
    let cert_chain = rustls_pemfile::certs(&mut BufReader::new(open(cert)?))
        .filter_map(|cert| cert.ok())
        .collect();

    let key = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(open(cert_key)?))
        .filter_map(|key| key.ok())
        .next()
        .context("no cert key found")?
        .into();

    TlsConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .context("couldn't create certificate chain")
}
