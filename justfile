export RUST_LOG := env("RUST_LOG", "info")
set positional-arguments

[private]
default:
    #!/usr/bin/env bash
    just --list --unsorted

# Run the specified binary
run bin *args:
    #!/usr/bin/env bash
    cargo run --release --bin {{bin}} -- "${@:2}"

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast "${@:2}" -- --color always --no-capture

# Install all binaries in /usr/local/bin
install:
    #!/usr/bin/env bash
    cargo build --release
    sudo chown :devs target/release/*
    export DST=/usr/local/bin/
    sudo cp target/release/export_tokenizer $DST
    sudo cp target/release/preprocess_cx $DST
    sudo cp target/release/preprocess_sow $DST
    sudo cp target/release/train_tokenizer $DST
