# Post-change verification script
# All steps must pass without warnings

$ErrorActionPreference = "Stop"

Write-Host "Building..."
cargo build -p llm-coding-tools-core
cargo build -p llm-coding-tools-rig --quiet
cargo build -p llm-coding-tools-serdesai --quiet

Write-Host "Testing..."
cargo test -p llm-coding-tools-core
cargo test -p llm-coding-tools-rig --quiet
cargo test -p llm-coding-tools-serdesai --quiet

Write-Host "Clippy..."
cargo clippy -p llm-coding-tools-core -- -D warnings
cargo clippy -p llm-coding-tools-rig --quiet -- -D warnings
cargo clippy -p llm-coding-tools-serdesai --quiet -- -D warnings

Write-Host "Testing blocking feature..."
cargo test -p llm-coding-tools-core --no-default-features --features blocking --quiet

Write-Host "Docs..."
cargo doc --workspace --no-deps --quiet

Write-Host "Formatting..."
cargo fmt --all

Write-Host "All checks passed!"
