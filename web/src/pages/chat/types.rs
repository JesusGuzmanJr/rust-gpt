use {
    crate::{
        hash::GlassVault,
        message::{Feedback, MessageId, UserMessageContent},
        thread::{Thread, ThreadId, ThreadTitle},
    },
    chrono::{DateTime, Utc},
    garde::Validate,
    language_model::models::{LANGUAGE_MODEL_0_INFO, LANGUAGE_MODEL_1_INFO, ModelInfo},
    serde::Deserialize,
    strum::{Display, EnumIter},
};

/// The Unicode End of Transmission (EOT) character U+0004.
pub(super) const END_OF_TRANSMISSION: &str = "\u{4}";

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

#[derive(Debug, Display, Default, EnumIter, Deserialize)]
pub(super) enum ModelSelection {
    #[default]
    Model0,
    Model1,
}

impl From<ModelSelection> for ModelInfo {
    fn from(model_option: ModelSelection) -> Self {
        match model_option {
            ModelSelection::Model0 => LANGUAGE_MODEL_0_INFO,
            ModelSelection::Model1 => LANGUAGE_MODEL_1_INFO,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ModelQuery {
    // must match the "name" attribute
    pub(super) model: ModelSelection,
}

#[derive(Debug, Validate, Deserialize)]
pub(super) struct SendForm {
    #[garde(dive)]
    pub(super) content: UserMessageContent,
    #[garde(skip)]
    pub(super) current_thread_id: GlassVault<ThreadId>,
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
