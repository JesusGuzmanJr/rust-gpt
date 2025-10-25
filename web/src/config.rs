use {
    crate::{auth::AuthConfig, mailer::MailerConfig, persistence::PersistenceConfig},
    anyhow::{Context, Result},
    serde::Deserialize,
};

/// The root configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct AppConfig {
    pub(crate) persistence: PersistenceConfig,
    pub(crate) auth: AuthConfig,
    pub(crate) mailer: MailerConfig,
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
