#!/usr/bin/env bash
# Post-change verification script
# All steps must pass without warnings

set -e

echo "Building..."
cargo build -p llm-coding-tools-core
cargo build -p llm-coding-tools-rig --quiet
cargo build -p llm-coding-tools-serdesai --quiet

echo "Testing..."
cargo test -p llm-coding-tools-core
cargo test -p llm-coding-tools-rig --quiet
cargo test -p llm-coding-tools-serdesai --quiet

echo "Clippy..."
cargo clippy -p llm-coding-tools-core -- -D warnings
cargo clippy -p llm-coding-tools-rig --quiet -- -D warnings
cargo clippy -p llm-coding-tools-serdesai --quiet -- -D warnings

echo "Testing blocking feature..."
cargo test -p llm-coding-tools-core --no-default-features --features blocking --quiet

echo "Docs..."
cargo doc --workspace --no-deps --quiet

echo "Formatting..."
cargo fmt --all

echo "All checks passed!"
