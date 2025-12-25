use {
    crate::{
        persistence::db,
        thread::{Thread, ThreadId},
    },
    anyhow::{Context, Result},
    chrono::{DateTime, Utc},
    common::{string_type, uuid_type},
    garde::Validate,
    maud::Markup,
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
    pub(crate) SystemMessageMarkdown
);

impl SystemMessageMarkdown {
    /// The default system greeting message.
    pub(crate) fn greeting() -> Self {
        Self("Hello! How can I assist you today?".into())
    }

    pub(crate) fn to_html(&self) -> Markup {
        maud::PreEscaped(markdown::to_html(&self.0))
    }
}

impl std::ops::AddAssign<&str> for SystemMessageMarkdown {
    fn add_assign(&mut self, other: &str) {
        self.0 += other;
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
    User {
        content: UserMessageContent,
    },
    System {
        content: SystemMessageMarkdown,
        feedback: Option<Feedback>,
    },
    PartialSystem {
        content: SystemMessageMarkdown,
    },
}

impl Payload {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Payload::User { content } => content.as_str(),
            Payload::System { content, .. } => content.as_str(),
            Payload::PartialSystem { content } => content.as_str(),
        }
    }
}

/// A system message.
#[derive(Debug)]
pub(crate) struct SystemMessage {
    pub(crate) id: MessageId,
    pub(crate) thread_id: ThreadId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) content: SystemMessageMarkdown,
    pub(crate) feedback: Option<Feedback>,
}

impl TryFrom<Message> for SystemMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self> {
        match message.payload {
            Payload::System { content, feedback } => Ok(SystemMessage {
                id: message.id,
                thread_id: message.thread_id,
                created_at: message.created_at,
                content,
                feedback,
            }),
            _ => anyhow::bail!("message is not a system message"),
        }
    }
}

/// A partial system message.
#[derive(Debug)]
pub(crate) struct PartialSystemMessage {
    pub(crate) id: MessageId,
    pub(crate) thread_id: ThreadId,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) content: SystemMessageMarkdown,
}

impl TryFrom<Message> for PartialSystemMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<Self> {
        match message.payload {
            Payload::PartialSystem { content } => Ok(PartialSystemMessage {
                id: message.id,
                thread_id: message.thread_id,
                created_at: message.created_at,
                content,
            }),
            _ => anyhow::bail!("message is not a partial system message"),
        }
    }
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

    #[instrument]
    pub(crate) async fn save(self) -> Result<()> {
        spawn_blocking(move || {
            debug!("saving message");
            let rw = db().rw_transaction()?;

            // assert that the thread exists
            if rw.get().primary::<Thread>(self.thread_id)?.is_none() {
                anyhow::bail!("thread not found");
            }

            rw.upsert(self)?;
            rw.commit()
                .context("failed to commit transaction that saves message")?;

            Ok(())
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn get_all_messages(thread_id: ThreadId) -> Result<Vec<Self>> {
        debug!("getting all messages");
        spawn_blocking(move || {
            Ok(db()
                .r_transaction()?
                .scan()
                .secondary(MessageKey::thread_id)?
                .start_with(thread_id)?
                .filter_map(|result| {
                    if let Err(error) = &result {
                        warn!(%thread_id, ?error, "failed to get messages");
                    }
                    result.ok()
                })
                .collect())
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn by_id(message_id: MessageId) -> Result<Self> {
        debug!("getting message");
        spawn_blocking(move || {
            db().r_transaction()?
                .get()
                .primary(message_id)?
                .context("message not found")
        })
        .await?
    }

    #[instrument]
    pub(crate) async fn update_feedback(message_id: MessageId, feedback: Feedback) -> Result<()> {
        spawn_blocking(move || {
            debug!("updating message feedback");
            let rw = db().rw_transaction()?;

            let mut message: Message =
                rw.get().primary(message_id)?.context("message not found")?;

            // Only system messages can have feedback
            if let Payload::System { content, .. } = message.payload {
                message.payload = Payload::System {
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

    #[instrument]
    pub(crate) async fn delete(self) -> Result<()> {
        spawn_blocking(move || {
            debug!("deleting message");
            let rw = db().rw_transaction()?;
            rw.remove(self)?;
            rw.commit()
                .context("failed to commit transaction that deletes message")?;
            Ok(())
        })
        .await?
    }
}
