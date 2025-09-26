[private]
@default:
    just --list --unsorted

# Run tests
test:
    cargo test --release --quiet --no-fail-fast -- --color always --no-capture
