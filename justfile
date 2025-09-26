[private]
@default:
    just --list --unsorted

run:
    #!/usr/bin/env bash
    mkdir -p target/cuda
    nvcc src/app.cu -o target/cuda/app
    ./target/cuda/app
