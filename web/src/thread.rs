use {
    crate::{
        message::{Message, MessageKey},
        persistence::db,
        user::{User, UserId},
    },
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
        pub(crate) title: ThreadTitle,
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
    pub(crate) fn new(user_id: UserId, title: ThreadTitle) -> Self {
        Thread {
            id: ThreadId::new(),
            user_id,
            title,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[instrument]
    pub(crate) async fn by_id(thread_id: ThreadId) -> Result<Option<Self>> {
        spawn_blocking(move || {
            debug!("getting thread by id");
            Ok(db().r_transaction()?.get().primary::<Thread>(thread_id)?)
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn save(mut self) -> Result<()> {
        spawn_blocking(move || {
            debug!("saving thread");
            let rw = db().rw_transaction()?;

            // assert that the user exists
            if rw.get().primary::<User>(self.user_id)?.is_none() {
                anyhow::bail!("user not found");
            }

            self.updated_at = Utc::now();
            rw.upsert(self)?;
            rw.commit()
                .context("failed to commit transaction that saves thread")?;

            Ok(())
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn get_all(user_id: UserId) -> Result<Vec<Self>> {
        spawn_blocking(move || {
            debug!("getting all threads");
            let threads = db()
                .r_transaction()?
                .scan()
                .secondary(ThreadKey::user_id)?
                .start_with(user_id)?
                .filter_map(|result| {
                    if let Err(error) = &result {
                        warn!(%user_id, ?error, "failed to get thread");
                    }
                    result.ok()
                })
                .collect::<Vec<_>>();
            debug!(%user_id, len = threads.len(), "got all threads");
            Ok(threads)
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn delete(thread_id: ThreadId) -> Result<()> {
        spawn_blocking(move || {
            debug!("deleting thread");
            let rw = db().rw_transaction()?;

            let messages: Vec<Message> = rw
                .scan()
                .secondary(MessageKey::thread_id)?
                .start_with(thread_id)?
                .filter_map(|result| {
                    if let Err(error) = &result {
                        warn!(%thread_id, ?error, "failed to get message");
                    }
                    result.ok()
                })
                .collect();

            let thread = rw
                .get()
                .primary::<Thread>(thread_id)?
                .context("thread not found")?;

            for message in messages {
                if let Err(error) = rw.remove(message) {
                    warn!(%thread_id, ?error, "failed to delete message");
                }
            }

            rw.remove(thread)?;
            rw.commit()
                .context("failed to commit transaction that deletes thread")?;

            Ok(())
        })
        .await?
    }
}
