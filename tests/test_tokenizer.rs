use std::process::Command;

mod common;

#[test]
fn test_tokenizer() {
    common::create_test_output_dir();
    common::compile_bin("train_tokenizer");

    let output = Command::new("target/release/train_tokenizer")
        .arg("--input-dir")
        .arg("training-data")
        .arg("--output-file")
        .arg("target/test/tokenizer")
        .arg("--vocab-size")
        .arg("265")
        .output()
        .expect("failed to run train_tokenizer");

    common::assert_success(&output);
}
