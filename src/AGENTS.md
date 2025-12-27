# rig-coding-tools

Basic coding tools for rig based LLM agents

# Project Structure

- `rig-coding-tools/` - Main library crate
  - `src/` - Library source code

# Code Guidelines

- Optimize for performance; use zero-cost abstractions, avoid allocations.
- Keep modules under 500 lines (excluding tests); split if larger.
- Place `use` inside functions only for `#[cfg]` conditional compilation.
- Prefer `core` over `std` where possible (`core::mem` over `std::mem`).

# Documentation Standards

- Document public items with `///`
- Add examples in docs where helpful
- Use `//!` for module-level docs
- Focus comments on "why" not "what"
- Use [`TypeName`] rustdoc links, not backticks.

# Post-Change Verification

All must pass without warnings:

```bash
cargo build --workspace --all-features --all-targets --quiet
cargo test --workspace --all-features --quiet
cargo clippy --workspace --all-features --quiet -- -D warnings
cargo doc --workspace --all-features --quiet
cargo fmt --all --quiet
cargo publish --dry-run --quiet
```
