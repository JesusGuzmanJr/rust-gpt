use {
    crate::{persistence::db, user::UserId},
    anyhow::{Context, Result},
    chrono::{DateTime, Utc},
    common::{string_type, uuid_type},
    garde::Validate,
    native_db::{Models, ToKey, native_db},
    native_model::{Model, native_model},
    serde::{Deserialize, Serialize},
    tokio::task::spawn_blocking,
    tracing::*,
};

uuid_type!(
    /// A unique identifier for a chat thread.
    pub(crate) ThreadId
);

string_type!(
    /// The title of a chat thread.
    #[derive(Validate)]
    pub(crate) ThreadTitle(#[garde(length(min = 1, max = 32))])
);

impl ThreadTitle {
    pub(crate) fn new_chat_title() -> Self {
        "New Chat".into()
    }
}

pub(crate) type Thread = v1::Thread;
pub(crate) type ThreadKey = v1::ThreadKey;

pub(crate) mod v1 {
    use super::*;

    /// A chat thread.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[native_model(id = 2, version = 1, with = native_model::bincode_2::Bincode)]
    #[native_db]
    pub(crate) struct Thread {
        #[primary_key]
        pub(crate) id: ThreadId,
        #[secondary_key]
        pub(crate) user_id: UserId,
        pub(crate) thread_title: ThreadTitle,
        pub(crate) created_at: DateTime<Utc>,
        pub(crate) updated_at: DateTime<Utc>,
    }
}

pub(crate) fn define(models: &mut Models) -> Result<()> {
    models
        .define::<v1::Thread>()
        .context("failed to define thread v1 model")
}

impl Thread {
    pub(crate) fn new(user_id: UserId, thread_title: ThreadTitle) -> Self {
        Thread {
            id: ThreadId::new(),
            user_id,
            thread_title,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub(crate) async fn save(self) -> Result<()> {
        spawn_blocking(move || {
            let rw = db().rw_transaction()?;

            rw.insert(self)?;
            rw.commit()
                .context("failed to commit transaction that saves thread")?;

            Ok(())
        })
        .await?
    }

    pub(crate) async fn get_all(user_id: UserId) -> Result<Vec<Self>> {
        spawn_blocking(move || {
            Ok(db()
                .r_transaction()?
                .scan()
                .secondary(ThreadKey::user_id)?
                .start_with(user_id)?
                .filter_map(|result| {
                    if let Err(error) = &result {
                        warn!(%user_id, %error, "failed to get thread");
                    }
                    result.ok()
                })
                .collect())
        })
        .await?
    }

    pub(crate) async fn get_by_id(thread_id: ThreadId) -> Result<Option<Self>> {
        spawn_blocking(move || {
            let r = db().r_transaction()?;
            Ok(r.get().primary(thread_id)?)
        })
        .await?
    }

    pub(crate) async fn update_title(thread_id: ThreadId, new_title: ThreadTitle) -> Result<()> {
        spawn_blocking(move || {
            let rw = db().rw_transaction()?;

            let mut thread: Thread = rw.get().primary(thread_id)?.context("thread not found")?;

            thread.thread_title = new_title;
            thread.updated_at = Utc::now();

            rw.insert(thread)?;
            rw.commit()
                .context("failed to commit transaction that updates thread name")?;

            Ok(())
        })
        .await?
    }
}
