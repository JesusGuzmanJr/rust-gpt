use {
    crate::{scheduler, thread::ThreadId, user::UserId},
    anyhow::Result,
    language_model::models::{ModelId, ModelInfo},
    std::sync::{LazyLock, OnceLock, RwLock},
};

#[derive(Debug, Clone)]
pub(crate) struct InferenceRequest {
    pub(crate) model_id: ModelId,
    pub(crate) temperature: f32,
}
