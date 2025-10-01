[private]
@default:
    #!/usr/bin/env bash
    just --list --unsorted

# Run the specified binary
run bin *args:
    #!/usr/bin/env bash
    cargo run --release --bin {{bin}} -- {{args}}

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast {{args}} -- --color always --no-capture
