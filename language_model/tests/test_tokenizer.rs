use {
    crate::common::assert_success,
    language_model::{
        tokenization::{Token, TokenizerModel},
        utils::Bincode,
    },
    std::process::Command,
};

mod common;

#[test]
fn test_train_tokenizer() {
    common::create_test_output_dir();
    common::compile_bin("train_tokenizer");

    assert_success(
        &Command::new("just")
            .arg("run")
            .arg("train_tokenizer")
            .arg("--config")
            .arg("tests/tokenizer-training-config.ron")
            .output()
            .expect("failed to run train_tokenizer"),
    );

    let control_model = TokenizerModel {
        pre_tokenization_regex: r"\p{L}+ ?".into(),
        merges: vec![
            Token::from_slice(b"r "), // exact BPE would have "er"
            Token::from_slice(b"er "),
            Token::from_slice(b"ne"),
            Token::from_slice(b"new"),
            Token::from_slice(b"ow"), // exact BPE would have "lo"
            Token::from_slice(b"low"),
            Token::from_slice(b"newer "),
            Token::from_slice(b"low "),
        ],
    };

    let model_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/test/tokenizer");
    let model = TokenizerModel::from_bytes(&std::fs::read(model_path).unwrap()).unwrap();

    assert_eq!(model, control_model);
}
