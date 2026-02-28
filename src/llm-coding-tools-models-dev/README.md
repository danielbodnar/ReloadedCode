# llm-coding-tools-models-dev

models.dev catalog ingestion with online-first sync and local cache fallback.

This crate loads provider/model data from models.dev and builds a
`llm_coding_tools_core::models::ModelCatalog`.

## Why this exists

If you run coding agents against many providers, you usually want all three:

- Fresh data when online.
- A reliable fallback when offline.
- A compact cache that is cheap to load.

That is the core goal here.

## What it does

- **Online-first sync**: Sends conditional requests with `If-None-Match` and reuses cache on `304 Not Modified`.
- **Implicit fallback**: If network sync fails, loads the last valid cache automatically.
- **Compact storage**: Stores cache as prelude + ETag + `zstd(bitcode(payload))`.
- **Minimal API**: Exposes `ModelsDevCatalog::load()` and `ModelsDevCatalog::load_at(...)`.

## Usage

```rust
use llm_coding_tools_models_dev::{CatalogLoadSource, ModelsDevCatalog};

async fn load_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let result = ModelsDevCatalog::load().await?;

    match result.source {
        CatalogLoadSource::Downloaded => println!("Downloaded fresh snapshot."),
        CatalogLoadSource::NotModifiedCache => println!("Cache is already up to date."),
        CatalogLoadSource::FallbackCache => println!("Network unavailable, using cached snapshot."),
    }

    if let Some(entry) = result.catalog.lookup("openai", "gpt-4") {
        println!("provider api url: {}", entry.0.api_url);
        println!("max input tokens: {}", entry.1.max_input);
    }

    Ok(())
}
```

## Cache location

By default, cache is stored in the platform cache directory:

- Linux: `~/.cache/llm-coding-tools/models.dev.catalog.v1.cache`
- macOS: `~/Library/Caches/llm-coding-tools/models.dev.catalog.v1.cache`
- Windows: `%LOCALAPPDATA%\llm-coding-tools\models.dev.catalog.v1.cache`

Set `LLM_CODING_TOOLS_MODELS_DEV_CACHE_PATH` to override this path.

## Feature flags

- `tokio` (default): async runtime support.

## License

Apache-2.0
