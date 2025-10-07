export RUST_LOG := env("RUST_LOG", "info")

[private]
@default:
    #!/usr/bin/env bash
    just --list --unsorted

# Run the specified binary
run bin *args:
    #!/usr/bin/env bash
    mkdir -p target/logs/{{bin}}
    cargo run --release --bin {{bin}} -- {{args}} |& tee target/logs/{{bin}}/$(date +%Y-%m-%d_%H-%M-%S).log

# Run tests
test *args:
    #!/usr/bin/env bash
    cargo test --release --quiet --no-fail-fast {{args}} -- --color always --no-capture
