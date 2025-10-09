use {
    crate::common::assert_success,
    rust_gpt::{
        Bincode,
        tokenization::{Token, TokenizerModel},
    },
    std::process::Command,
};

mod common;

#[test]
fn test_train_tokenizer() {
    common::create_test_output_dir();
    common::compile_bin("train_tokenizer");

    let regex = r"\p{L}+ ?";
    assert_success(
        &Command::new("just")
            .arg("run")
            .arg("create_tokenizer")
            .arg("--input-dir")
            .arg("training-data")
            .arg("--tokenizer-file")
            .arg("target/test/test-tokenizer")
            .arg("--training-config-file")
            .arg("target/test/test-training-config")
            .arg("--vocab-size")
            .arg("264")
            .arg("--regex")
            .arg(regex)
            .output()
            .expect("failed to run create_tokenizer"),
    );

    let regex = r"\p{L}+ ?";
    assert_success(
        &Command::new("just")
            .arg("run")
            .arg("train_tokenizer")
            .arg("--training-config-file")
            .arg("target/test/test-training-config")
            .output()
            .expect("failed to run train_tokenizer"),
    );

    let control_model = TokenizerModel {
        pre_tokenization_regex: regex.to_string(),
        merges: vec![
            Token::from_slice(b"er"),
            Token::from_slice(b"er "),
            Token::from_slice(b"ne"),
            Token::from_slice(b"new"),
            Token::from_slice(b"lo"),
            Token::from_slice(b"low"),
            Token::from_slice(b"newer "),
            Token::from_slice(b"low "),
        ],
    };

    let model =
        TokenizerModel::from_bytes(&std::fs::read("target/test/test-tokenizer").unwrap()).unwrap();

    assert_eq!(model, control_model);
}
