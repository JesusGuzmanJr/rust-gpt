[private]
@default:
    #!/usr/bin/env bash
    just --list --unsorted

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast {{args}} -- --color always --no-capture
