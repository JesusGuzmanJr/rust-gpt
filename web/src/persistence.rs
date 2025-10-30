use {
    anyhow::Result,
    native_db::{Database, Models},
    std::{path::Path, sync::OnceLock},
    tracing::*,
};

static MODELS: OnceLock<Models> = OnceLock::new();
static DB: OnceLock<Database<'static>> = OnceLock::new();

/// Get a reference to the database.
pub(crate) fn db() -> &'static Database<'static> {
    DB.get().expect("db not initialized")
}

#[deny(dead_code)]
pub(super) fn init(db_path: &Path) -> Result<()> {
    let mut models: Models = Models::new();
    crate::user::define(&mut models)?;
    crate::thread::define(&mut models)?;

    drop(MODELS.set(models));
    let mut db =
        native_db::Builder::new().create(MODELS.get().expect("models not initialized"), db_path)?;

    info!(db_path = %db_path.display(), "compacting database");
    db.compact()?;

    drop(DB.set(db));

    Ok(())
}
