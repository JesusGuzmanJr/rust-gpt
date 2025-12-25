use {
    crate::{
        hash::GlassVault,
        message::{Feedback, MessageId, UserMessageContent},
        thread::{Thread, ThreadId, ThreadTitle},
    },
    chrono::{DateTime, Utc},
    garde::Validate,
    language_model::models::ModelId,
    serde::Deserialize,
};

#[derive(Debug)]
pub(super) struct ThreadItem {
    pub(super) id: ThreadId,
    pub(super) title: ThreadTitle,
    pub(super) created_at: DateTime<Utc>,
    pub(super) preview: String,
    pub(super) is_active: bool,
}

impl ThreadItem {
    pub(super) fn from_thread(thread: Thread, preview: &str) -> ThreadItem {
        let preview = preview.chars().take(100).collect::<String>();
        ThreadItem {
            id: thread.id,
            title: thread.title,
            created_at: thread.created_at,
            preview: preview.chars().take(64).collect::<String>(),
            is_active: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ModelQuery {
    // must match the "name" attribute
    pub(super) model_id: ModelId,
}

#[derive(Debug, Validate, Deserialize)]
pub(super) struct SendForm {
    #[garde(dive)]
    pub(super) content: UserMessageContent,

    #[garde(skip)]
    pub(super) current_thread_id: GlassVault<ThreadId>,

    #[garde(skip)]
    pub(super) model_id: ModelId,

    #[garde(range(min = -1.0, max = 1.0))]
    pub(super) temperature: f32,
}

#[derive(Debug, Deserialize, Validate)]
pub(super) struct TitleForm {
    #[garde(dive)]
    pub(super) title: ThreadTitle,

    #[garde(skip)]
    pub(super) current_thread_id: GlassVault<ThreadId>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FeedbackForm {
    pub(super) message_id: GlassVault<MessageId>,
    pub(super) feedback: Feedback,
}

#[derive(Debug, Deserialize, Validate)]
pub(super) struct UpdateMessageForm {
    #[garde(dive)]
    pub(super) content: UserMessageContent,

    #[garde(skip)]
    pub(super) message_id: GlassVault<MessageId>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SelectForm {
    pub(super) thread_id: GlassVault<ThreadId>,
}

#[derive(Debug, Deserialize)]
pub(super) struct DeleteForm {
    pub(super) thread_id_to_delete: GlassVault<ThreadId>,
    pub(super) current_thread_id: GlassVault<ThreadId>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StreamQuery {
    pub(super) message_id: GlassVault<MessageId>,
}
