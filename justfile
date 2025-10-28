export RUST_LOG := env("RUST_LOG", "info")
set positional-arguments

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast "$@" -- --color always --no-capture

# Generate a random blake3 key
generate-blake3-key:
    #!/usr/bin/env bash
    openssl rand -hex 32