# rig-coding-tools

Basic coding tools for rig based LLM agents

# Project Structure

- `coding-tools-core/` - Framework-agnostic core library
  - `src/operations/` - Core operation implementations (read, write, edit, glob, grep, bash, etc.)
  - `src/path/` - Path resolution (absolute and allowed)
  - `src/error.rs` - Unified error types
  - `src/output.rs` - Tool output formatting
  - `src/util.rs` - Shared utilities
- `coding-tools-rig/` - Rig framework Tool implementations
   - `src/absolute/` - Unrestricted file system tools
   - `src/allowed/` - Sandboxed file system tools
   - `src/bash.rs`, `src/task.rs`, etc. - Standalone tools

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
```

For individual crates:
```bash
cargo publish --dry-run -p coding-tools-core --quiet
cargo publish --dry-run -p coding-tools-rig --quiet
```
