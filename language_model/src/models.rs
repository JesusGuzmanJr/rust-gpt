use bytesize::ByteSize;

/// Basic information about a language model.
#[derive(Debug)]
pub struct ModelInfo {
    pub name: &'static str,
    pub vocabulary_size: usize,
    pub corpus_size: ByteSize,
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
