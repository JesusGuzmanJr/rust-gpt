use {
    bytesize::ByteSize,
    derive_more::Display,
    serde::Deserialize,
    std::{collections::HashMap, sync::LazyLock},
    strum::EnumIter,
};

/// A unique identifier for a language model.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Display, Default, EnumIter, Deserialize)]
pub enum ModelId {
    #[default]
    Model0,
    Model1,
}

/// Basic information about a language model.
#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub name: &'static str,
    pub vocabulary_size: usize,
    pub corpus_size: ByteSize,
}

static MODELS: LazyLock<HashMap<ModelId, ModelInfo>> = LazyLock::new(|| {
    let mut models = HashMap::new();
    models.insert(ModelId::Model0, LANGUAGE_MODEL_0_INFO);
    models.insert(ModelId::Model1, LANGUAGE_MODEL_1_INFO);
    models
});

impl From<ModelId> for ModelInfo {
    fn from(model_id: ModelId) -> Self {
        *MODELS.get(&model_id).expect("model not found")
    }
}

/// Experimental model 0.
pub const LANGUAGE_MODEL_0_INFO: ModelInfo = ModelInfo {
    name: "Model 0",
    vocabulary_size: 50_000,
    // du -sb /hdd/rust-gpt/pretraining-data/stanford-oval-wikipedia
    corpus_size: ByteSize::b(6133981193),
};

/// Experimental model 1.
pub const LANGUAGE_MODEL_1_INFO: ModelInfo = ModelInfo {
    name: "Model 1",
    vocabulary_size: 200_000,
    // du -sb /hdd/rust-gpt/pretraining-data
    corpus_size: ByteSize::b(4569311337315),
};
