use {
    crate::{persistence::db, thread::ThreadId, user::UserId},
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
    /// A unique identifier for a chat message.
    pub(crate) MessageId
);

string_type!(
    /// The content of a chat message.
    #[derive(Validate)]
    pub(crate) MessageContent(#[garde(length(min = 1, max = 1024))])
);

pub(crate) type Message = v1::Message;
pub(crate) type MessageKey = v1::MessageKey;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum Author {
    System,
    User(UserId),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum Feedback {
    ThumbsUp,
    ThumbsDown,
}

pub(crate) mod v1 {
    use super::*;

    /// A chat message.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[native_model(id = 3, version = 1, with = native_model::bincode_2::Bincode)]
    #[native_db]
    pub(crate) struct Message {
        #[primary_key]
        pub(crate) id: MessageId,
        #[secondary_key]
        pub(crate) thread_id: ThreadId,
        pub(crate) author: Author,
        pub(crate) content: MessageContent,
        pub(crate) feedback: Option<Feedback>,
        pub(crate) created_at: DateTime<Utc>,
    }
}

pub(crate) fn define(models: &mut Models) -> Result<()> {
    models
        .define::<v1::Message>()
        .context("failed to define message v1 model")
}

impl Message {
    pub(crate) fn new(
        thread_id: ThreadId,
        author: Author,
        content: MessageContent,
        feedback: Option<Feedback>,
    ) -> Self {
        Message {
            id: MessageId::new(),
            thread_id,
            author,
            content,
            feedback,
            created_at: Utc::now(),
        }
    }

    pub(crate) async fn save(self) -> Result<()> {
        spawn_blocking(move || {
            let rw = db().rw_transaction()?;

            rw.insert(self)?;
            rw.commit()
                .context("failed to commit transaction that saves message")?;

            Ok(())
        })
        .await?
    }

    pub(crate) async fn get_all_messages(thread_id: ThreadId) -> Result<Vec<Self>> {
        spawn_blocking(move || {
            Ok(db()
                .r_transaction()?
                .scan()
                .secondary(MessageKey::thread_id)?
                .start_with(thread_id)?
                .filter_map(|result| {
                    if let Err(error) = &result {
                        warn!(%thread_id, %error, "failed to get messages");
                    }
                    result.ok()
                })
                .collect())
        })
        .await?
    }
}
