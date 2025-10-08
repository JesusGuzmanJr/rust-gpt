export RUST_LOG := env("RUST_LOG", "info")
set positional-arguments

[private]
default:
    #!/usr/bin/env bash
    just --list --unsorted

# Run the specified binary
run bin *args:
    #!/usr/bin/env bash
    mkdir -p target/logs/{{bin}}
    cargo run --release --bin {{bin}} -- "${@:2}"

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast "${@:2}" -- --color always --no-capture
