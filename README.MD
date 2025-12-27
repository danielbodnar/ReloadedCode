# rig-coding-tools

## Additional Setup Required!!

<!-- DELETE THIS SECTION AFTER COMPLETING SETUP -->

Additional setup might be required.
Once you do the stuff under this text, delete this section.

### Code Coverage
To setup code coverage for this project:

1. Go to [Codecov][codecov], add your project.
2. Go to [Settings -> Secrets and variables -> Actions][gh-actions-secrets] in your repo and add a repository secret named `CODECOV_TOKEN`.
3. Do the same in [Settings -> Secrets and variables -> Dependabot][gh-actions-dependabot].

The instructions on the Codecov page will provide the token to use.


### Setup API Key (crates.io)

You'll need to set up an API key to publish to `crates.io` on tag.

- Generate your API key in [crates.io (Account Settings -> API Tokens)][crates-io-key].
- Go to [Settings -> Secrets and variables -> Actions][gh-actions-secrets] in your repo and add environment secret named `CRATES_IO_TOKEN`.
- Paste your API key.




### Fill in README Documentation

This repository uses a two-level README structure:

| File | Purpose | What to Write |
|------|---------|---------------|
| `README.MD` (this file) | GitHub landing page | Project overview, quick start example, badges |
| [`src/rig-coding-tools/README.MD`](./src/rig-coding-tools/README.MD) | crates.io page | Detailed features, installation, comprehensive usage examples |

-----------------------

[![Crates.io](https://img.shields.io/crates/v/rig-coding-tools.svg)](https://crates.io/crates/rig-coding-tools)
[![Docs.rs](https://docs.rs/rig-coding-tools/badge.svg)](https://docs.rs/rig-coding-tools)
[![CI](https://github.com/Sewer56/rig-coding-tools/actions/workflows/rust.yml/badge.svg)](https://github.com/Sewer56/rig-coding-tools/actions)

Basic coding tools for rig based LLM agents

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
rig-coding-tools = "0.1.0"
```

```rust
// TODO: Add a minimal example showing the most common use case
```
See the [rig-coding-tools crate documentation](./src/rig-coding-tools/README.MD) for detailed usage.

## Crates

- [rig-coding-tools](./src/rig-coding-tools/README.MD): Core library <!-- TODO: Update description -->

## Developer Manual

For step-by-step development guidance, see the [Developer Manual](https://reloaded-project.github.io/reloaded-templates-rust/manual/).

## Contributing

We welcome contributions! See the [Contributing Guide](https://reloaded-project.github.io/reloaded-templates-rust/manual/#contributing) for details.

## License

Licensed under [Apache 2.0](./LICENSE).

[codecov]: https://app.codecov.io
[gh-actions-secrets]: https://github.com/Sewer56/rig-coding-tools/settings/secrets/actions
[gh-actions-dependabot]: https://github.com/Sewer56/rig-coding-tools/settings/secrets/dependabot
[gh-pages]: https://github.com/Sewer56/rig-coding-tools/settings/pages
[crates-io-key]: https://crates.io/settings/tokens
[nuget-key]: https://www.nuget.org/account/apikeys
