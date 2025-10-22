use {
    anyhow::Result,
    native_db::{Database, Models},
    std::{path::Path, sync::OnceLock},
};

static MODELS: OnceLock<Models> = OnceLock::new();
static DB: OnceLock<Database<'static>> = OnceLock::new();

/// Get a reference to the database.
pub(crate) fn db() -> &'static Database<'static> {
    DB.get().expect("db not initialized")
}

pub(super) fn init(path: &Path) -> Result<()> {
    let mut models = Models::new();
    crate::user::define(&mut models)?;

    drop(MODELS.set(models));
    let db =
        native_db::Builder::new().create(MODELS.get().expect("models not initialized"), path)?;

    drop(DB.set(db));

    Ok(())
}
