use std::process::{Command, Output};

/// Create the test output directory if it doesn't exist.
pub(crate) fn create_test_output_dir() {
    if std::fs::metadata("target/test").is_ok() {
        std::fs::remove_dir_all("target/test").expect("failed to remove test output directory");
    }
    std::fs::create_dir_all("target/test").expect("failed to create test output directory");
}

/// Compile the specified binary in release mode.
pub(crate) fn compile_bin(bin: &str) {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    cmd.arg("--release");
    cmd.arg("--bin");
    cmd.arg(bin);
    let output = cmd.output().expect("failed to run cargo build");

    if !output.status.success() {
        eprintln!("cargo build failed for binary '{}'", bin);
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("failed to compile binary '{}'", bin);
    }
}

/// Assert that the given process output is successful.
pub(crate) fn assert_success(output: &Output) {
    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("failed with status: {}", output.status);
    }
}
