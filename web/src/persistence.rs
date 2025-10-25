use {
    anyhow::Result,
    native_db::{Database, Models},
    serde::Deserialize,
    std::{path::PathBuf, sync::OnceLock},
};

static MODELS: OnceLock<Models> = OnceLock::new();
static DB: OnceLock<Database<'static>> = OnceLock::new();

/// Get a reference to the database.
pub(crate) fn db() -> &'static Database<'static> {
    DB.get().expect("db not initialized")
}

/// The persistence configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct PersistenceConfig {
    /// The embedded database path.
    path: PathBuf,
}

#[deny(dead_code)]
pub(super) fn init(config: &PersistenceConfig) -> Result<()> {
    let mut models: Models = Models::new();
    crate::user::define(&mut models)?;

    drop(MODELS.set(models));
    let db = native_db::Builder::new()
        .create(MODELS.get().expect("models not initialized"), &config.path)?;

    drop(DB.set(db));

    Ok(())
}
