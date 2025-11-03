use {
    crate::{
        persistence::db,
        thread::{Thread, ThreadId},
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
    /// A unique identifier for a chat message.
    pub(crate) MessageId
);

string_type!(
    /// The content of a chat message.
    #[derive(Validate)]
    pub(crate) UserMessageContent(#[garde(length(min = 1, max = 1024))])
);

string_type!(
    /// The content of a system message.
    pub(crate) SystemMessageContent
);

impl SystemMessageContent {
    /// The default system greeting message.
    pub(crate) fn greeting() -> Self {
        "Hello! How can I assist you today?".into()
    }
}

pub(crate) type Message = v1::Message;
pub(crate) type MessageKey = v1::MessageKey;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum Feedback {
    ThumbsUp,
    ThumbsDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Payload {
    UserMessage {
        content: UserMessageContent,
    },
    SystemMessage {
        content: SystemMessageContent,
        feedback: Option<Feedback>,
    },
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
        pub(crate) created_at: DateTime<Utc>,
        pub(crate) payload: Payload,
    }
}

pub(crate) fn define(models: &mut Models) -> Result<()> {
    models
        .define::<v1::Message>()
        .context("failed to define message v1 model")
}

impl Message {
    pub(crate) fn new(thread_id: ThreadId, payload: Payload) -> Self {
        Message {
            id: MessageId::new(),
            thread_id,
            created_at: Utc::now(),
            payload,
        }
    }

    pub(crate) async fn save(self) -> Result<()> {
        spawn_blocking(move || {
            let rw = db().rw_transaction()?;

            // assert that the thread exists
            if rw.get().primary::<Thread>(self.thread_id)?.is_none() {
                anyhow::bail!("thread not found");
            }

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

    pub(crate) async fn update_feedback(message_id: MessageId, feedback: Feedback) -> Result<()> {
        spawn_blocking(move || {
            let rw = db().rw_transaction()?;

            let mut message: Message =
                rw.get().primary(message_id)?.context("message not found")?;

            // Only system messages can have feedback
            if let Payload::SystemMessage { content, .. } = message.payload {
                message.payload = Payload::SystemMessage {
                    content,
                    feedback: Some(feedback),
                };

                rw.auto_update(message.clone())?;
                rw.commit()
                    .context("failed to commit transaction that updates message feedback")?;
            } else {
                anyhow::bail!("cannot set feedback on user messages");
            }

            Ok(())
        })
        .await?
    }

    pub(crate) async fn get_by_id(message_id: MessageId) -> Result<Option<Self>> {
        spawn_blocking(move || {
            let r = db().r_transaction()?;
            Ok(r.get().primary(message_id)?)
        })
        .await?
    }
}
