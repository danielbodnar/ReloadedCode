#!/usr/bin/env bash
# Post-change verification script
# All steps must pass without warnings
# Keep in sync with verify.ps1
#
# Note: llm-coding-tools-serdesai is async-only (implements async Tool traits).
# The blocking feature only applies to llm-coding-tools-core.

set -e

echo "Building..."
cargo build -p llm-coding-tools-core
cargo build -p llm-coding-tools-agents --quiet
cargo build -p llm-coding-tools-serdesai --quiet

echo "Testing..."
cargo test -p llm-coding-tools-core
cargo test -p llm-coding-tools-agents --quiet
cargo test -p llm-coding-tools-serdesai --quiet

echo "Clippy..."
cargo clippy -p llm-coding-tools-core -- -D warnings
cargo clippy -p llm-coding-tools-agents --quiet -- -D warnings
cargo clippy -p llm-coding-tools-serdesai --quiet -- -D warnings

echo "Testing blocking feature..."
cargo test -p llm-coding-tools-core --no-default-features --features blocking --quiet

echo "Docs..."
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

echo "Formatting..."
cargo fmt --all

echo "Publish dry-run..."
cargo publish --dry-run -p llm-coding-tools-core --quiet
cargo publish --dry-run -p llm-coding-tools-agents --quiet
cargo publish --dry-run -p llm-coding-tools-serdesai --quiet

echo "All checks passed!"
