[private]
@default:
    #!/usr/bin/env bash
    just --list --unsorted

# Preprocess the markdown files
preprocess *args:
    #!/usr/bin/env bash
    cargo run --release --bin preprocess -- {{args}}

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast {{args}} -- --color always --no-capture
