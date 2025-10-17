export RUST_LOG := env("RUST_LOG", "info")
set positional-arguments

# Run the specified binary
run bin *args:
    #!/usr/bin/env bash
    cargo run --release --bin {{bin}} -- "${@:2}"

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast "$@" -- --color always --no-capture
