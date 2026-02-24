//! Efficient catalog/registry of providers and models sourced
//! from places like 'models.dev'. Contains bare minimum of information
//! required for usage.
//!
//! For instance; a model entry like `synthetic/hf:moonshotai/Kimi-K2.5` may be
//! split into:
//!
//! Provider: 'synthetic'
//! Model: 'hf:moonshotai/Kimi-K2.5'
//!
//! Internally the `provider`(s) and `model`(s) are stored in separate tables;
//! with friendly APIs to return those back combined when needed.
//!
//! # Why split provider and model tables?
//!
//! Many providers share the same models. Although they may sometimes be renamed,
//! e.g. `Kimi-K2.5` vs `hf:moonshotai/Kimi-K2.5`; they often have identical
//! metadata. (token limits, modalities, etc.)
//!
//! Given a snapshot of models.dev from 20th of February 2026, we have:
//!
//! - Unique model IDs: 1,669
//! - Unique model configurations: 552
//!
//! We optimize for this case hashtables of `hash -> index` 😉
//!
//! # Memory Optimizations
//!
//! To save on memory, we don't actually store the original strings for provider
//! or model anywhere. The typical use case is that a user has a given provider &
//! model ID e.g. `synthetic/hf:moonshotai/Kimi-K2.5` and just needs to pull
//! up metadata for it. e.g. when `model` is specified in an agent file.
//!
//! Instead, we provide a guarantee that a *VALID* user provided provider and
//! model key will always hash to unique values (0 collisions). Since the
//! `ModelCatalog` is usually constructed once at startup, this is something
//! we can practically guarantee. (negligible failure probability)
//!
//! Sometimes this concept is referred to as a 'perfect hash', elsewhere.
//!
//! ## Hash Collision Probabilities
//!
//! Currently `ProviderTable` and `ModelTable` use 48 bits for the hash.
//!
//! | Odds of collision | # 48-bit hash values |
//! | ----------------- | -------------------: |
//! | 1 in 2            |           19,753,663 |
//! | 1 in 10           |            7,701,474 |
//! | 1 in 100          |            2,378,621 |
//! | 1 in 1,000        |              750,488 |
//! | 1 in 10,000       |              237,272 |
//! | 1 in 100,000      |               75,031 |
//! | 1 in 1 million    |               23,727 |
//! | 1 in 10 million   |                7,503 |
//! | 1 in 100 million  |                2,373 |
//! | 1 in 1 billion    |                  751 |
//! | 1 in 10 billion   |                  238 |
//! | 1 in 100 billion  |                   76 |
//! | 1 in 1 trillion   |                   24 |
//! | 1 in 10 trillion  |                    8 |
//!
//! Today's probabilities of 'at least 1 collision' are:
//!
//! - `ProviderTable`: 96 entries, 48-bit hash -> about `1 in 61 billion`
//! - `ModelTable`: 1,669 entries, 48-bit hash -> about `1 in 202 million`
//!
//! Note: Above assumes a 'perfect' hash function with uniformly distributed output.
//!       While such function does not exist in practice, 'ahash' which I used
//!       here has very good distribution and comes fairly close.
//!
//! ## Reseeding
//!
//! As an additional safety measure, re-seeding is also supported.
//! i.e. Using alternative seeds for hashing.
//!
//! ProviderTable (96 entries, 48-bit):
//!
//! | Seeds | Odds of failure      |
//! | ----- | -------------------: |
//! | 1     | 1 in 62 billion      |
//! | 2     | 1 in 3.8 quintillion |
//! | 4     | 1 in 1.4 x 10^43     |
//! | 8     | 1 in 2.1 x 10^86     |
//! | 16    | 1 in 4.4 x 10^172    |
//!
//! ModelTable (1,669 entries, 48-bit):
//!
//! | Seeds | Odds of failure      |
//! | ----- | -------------------: |
//! | 1     | 1 in 202 million     |
//! | 2     | 1 in 41 quadrillion  |
//! | 4     | 1 in 1.7 x 10^33     |
//! | 8     | 1 in 2.8 x 10^66     |
//! | 16    | 1 in 7.8 x 10^132    |
//!
//! This basically seals the deal, ensuring a collision will never happen.
//!
//! As a point of reference, there are estimated to be 10^78 to 10^82 atoms in
//! the observable universe.
//!
//! # Numeric Limits
//!
//! | Limit                     |       Value | Description                                      |
//! | ------------------------- | ----------: | ------------------------------------------------ |
//! | Max providers             |      65,536 | Addressable by 16-bit provider index             |
//! | Max model configs         |      65,536 | Addressable by 16-bit model configuration index  |
//! | Max provider env vars     |      16,384 | Per-provider env-var pool offset (14-bit)        |
//! | Max env vars per provider |           3 | Count field in provider entry (2-bit)            |
//! | Max input tokens          | 536,870,911 | 29-bit packed field (≈536M)                      |
//! | Max output tokens         | 134,217,727 | 27-bit packed field (≈134M)                      |
//! | Hash bits retained        |          48 | Truncated from 64-bit ahash output               |
//! | Max reseed attempts       |          16 | Number of alternative hash seeds                 |
//!
//! Note: There's technically 16 bits per provider, but only 14 bits for provider env var.
//! Since each provider typically has 1 env var; that means 14 bits for provider, effectively.
//!
//! # Detailed Memory Layout
//!
//! This layout is optimized for scenarios where many providers host overlapping
//! models. Memory usage numbers below are from models.dev snapshot (Feb 20, 2026):
//!
//! ## Lookup Tables (Hash -> Index)
//!
//! - `ProviderTable`: `8 bytes * 96 = 768 bytes`
//!   - `48-bit` provider hash (truncated ahash)
//!   - `16-bit` provider index
//! - `ModelTable`: `8 bytes * 1,669 = 13,352 bytes`
//!   - `48-bit` model hash (truncated ahash)
//!   - `16-bit` model-configuration index
//!
//! ## Metadata Storage
//!
//! - `ModelEntry`: `8 bytes * 552 = 4,416 bytes`
//!   - `8-bit` modalities
//!   - `27-bit` max output (`134m` token range)
//!   - `29-bit` max input (`536m` token range)
//! - Optional `ModelConfigEntry`: `4 bytes * 552 = 2,208 bytes`
//!   - `16-bit` `top_p` fixed4 (`u16::MAX` sentinel means `None`)
//!   - `16-bit` `temperature` fixed4 (`u16::MAX` sentinel means `None`)
//! - `ProviderEntry`: `4 bytes * 96 = 384 bytes`
//!   - `8-bit` [`crate::models::ProviderType`]
//!   - `14-bit` env-var start index
//!   - `2-bit` env-var count
//!
//! **Total: ~21 KB** for the entire catalog (provider + model metadata).
//!
//! ## Deduplication
//!
//! The key insight is that `ModelTable` keys can point to shared
//! `ModelEntry` / `ModelConfigEntry` rows. When multiple providers host the
//! same model, we only store the metadata once. This is why we have 1,669
//! model keys but only 552 unique model configurations.

pub use internal::ModelCatalogBuilder;
pub use public::{
    CatalogEntry, LookupTableKind, Model, ModelCatalogBuildError, ModelConfig, ModelInfo, Provider,
    ProviderInfo,
};

// Internal implementation details - not part of public API
mod internal;

// Public types and lookup results
mod public;

use ahash::RandomState;
use hashbrown::HashTable;
use internal::{
    hash_model_key, hash_provider_key, ModelIdx, PackedEnvRange, PackedModelConfigEntry,
    PackedModelEntry, PackedModelTableEntry, PackedProviderEntry, PackedProviderTableEntry,
    ProviderHash, ProviderIdx,
};
use lite_strtab::{StringId, StringTable};

/// Runtime lookup catalog with split provider and model tables.
///
/// See module-level documentation for design rationale, memory layout,
/// and numeric limits.
pub struct ModelCatalog {
    /// Precomputed hash state for the selected seed.
    hash_state: RandomState,
    /// Provider key lookup table.
    provider_table: HashTable<PackedProviderTableEntry>,
    /// Model key lookup table.
    model_table: HashTable<PackedModelTableEntry>,
    /// Provider API URLs indexed by provider index.
    provider_api_urls: StringTable<u32, ProviderIdx>,
    /// Provider env keys grouped in a string table.
    provider_env_keys: StringTable<u32, ProviderIdx>,
    /// Env key ranges (start, count) indexed by provider index.
    provider_env_ranges: Box<[PackedEnvRange]>,
    /// Packed provider metadata indexed by provider index.
    provider_entries: Box<[PackedProviderEntry]>,
    /// Packed deduplicated model metadata indexed by model-configuration index.
    model_entries: Box<[PackedModelEntry]>,
    /// Optional packed model sampling sidecars indexed by model-configuration index.
    model_config_entries: Option<Box<[PackedModelConfigEntry]>>,
}

impl ModelCatalog {
    /// Creates a builder with no preallocated capacity.
    #[inline]
    pub fn builder() -> ModelCatalogBuilder {
        ModelCatalogBuilder::new()
    }

    /// Creates a builder with preallocated provider and model key capacity.
    #[inline]
    pub fn builder_with_capacity(
        provider_capacity: usize,
        model_capacity: usize,
    ) -> ModelCatalogBuilder {
        ModelCatalogBuilder::with_capacity(provider_capacity, model_capacity)
    }

    /// Returns number of provider keys.
    #[inline]
    pub fn provider_len(&self) -> usize {
        self.provider_table.len()
    }

    /// Returns number of model keys.
    #[inline]
    pub fn model_len(&self) -> usize {
        self.model_table.len()
    }

    /// Returns number of unique model configuration rows.
    #[inline]
    pub fn model_config_len(&self) -> usize {
        self.model_entries.len()
    }

    /// Returns true when catalog has no providers and no models.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.provider_table.is_empty() && self.model_table.is_empty()
    }

    /// Looks up one provider by key.
    #[inline]
    pub fn lookup_provider(&self, provider_key: &str) -> Option<Provider<'_>> {
        let key = hash_provider_key(&self.hash_state, provider_key);
        self.lookup_provider_hash(key)
    }

    /// Looks up one provider by prehashed key.
    #[inline]
    fn lookup_provider_hash(&self, key: ProviderHash) -> Option<Provider<'_>> {
        let hash48 = PackedProviderTableEntry::truncate_hash48(key.as_u64());
        let entry = self
            .provider_table
            .find(hash48, |entry: &PackedProviderTableEntry| {
                entry.hash48() == hash48
            })?;
        self.provider_from_index(entry.provider_idx())
    }

    /// Looks up one model by key.
    #[inline]
    pub fn lookup_model(&self, model_key: &str) -> Option<Model> {
        let hash = hash_model_key(&self.hash_state, model_key);
        self.lookup_model_hash(hash)
    }

    /// Looks up one model by prehashed key.
    #[inline]
    fn lookup_model_hash(&self, hash: internal::hash::ModelHash) -> Option<Model> {
        let hash48 = PackedModelTableEntry::truncate_hash48(hash.as_u64());
        let entry = self
            .model_table
            .find(hash48, |entry: &PackedModelTableEntry| {
                entry.hash48() == hash48
            })?;
        self.model_from_index(entry.model_config_idx())
    }

    /// Looks up both provider and model independently and returns joined result.
    #[inline]
    pub fn lookup(&self, provider_key: &str, model_key: &str) -> Option<CatalogEntry<'_>> {
        let provider = self.lookup_provider(provider_key)?;
        let model = self.lookup_model(model_key)?;
        Some(CatalogEntry { provider, model })
    }

    #[inline]
    fn provider_from_index(&self, provider_idx: u16) -> Option<Provider<'_>> {
        let provider_idx_usize = usize::from(provider_idx);
        let packed = *self.provider_entries.get(provider_idx_usize)?;
        let provider_idx = ProviderIdx::new(provider_idx);
        let api_url = self.provider_api_urls.get(StringId::new(provider_idx))?;
        let range = self.provider_env_ranges.get(provider_idx_usize)?;
        let start = range.start();
        let count = range.count();

        let env_vars: Vec<&str> = if count == 0 {
            Vec::new()
        } else {
            let mut vars = Vec::with_capacity(usize::from(count));
            for i in 0..count {
                let idx = ProviderIdx::new(start + u16::from(i));
                if let Some(s) = self.provider_env_keys.get(StringId::new(idx)) {
                    vars.push(s);
                }
            }
            vars
        };

        Some(Provider {
            provider_idx,
            api_url,
            env_vars,
            api_type: packed.api_type(),
        })
    }

    #[inline]
    fn model_from_index(&self, model_config_idx: u16) -> Option<Model> {
        let idx = usize::from(model_config_idx);
        let info = self.model_entries.get(idx)?.into_model_info();
        let config = self
            .model_config_entries
            .as_ref()
            .and_then(|entries| entries.get(idx))
            .and_then(|entry| entry.into_model_config());

        Some(Model {
            model_config_idx: ModelIdx::new(model_config_idx),
            info,
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::catalog::internal::{Modality, TemperatureFixed4, TopPFixed4};
    use crate::models::catalog::public::builder_types::{
        LookupTableKind, ModelCatalogBuildError, ProviderInfo,
    };
    use crate::models::catalog::public::model_types::{ModelConfig, ModelInfo};
    use crate::models::ProviderType;

    fn provider<'a>(
        api_url: &'a str,
        env_vars: &'a [&'a str],
        api_type: ProviderType,
    ) -> ProviderInfo<'a> {
        ProviderInfo {
            api_url,
            env_vars,
            api_type,
        }
    }

    fn info(max_input: u32, max_output: u32) -> ModelInfo {
        ModelInfo {
            modalities: Modality::TEXT,
            max_input,
            max_output,
        }
    }

    #[test]
    fn lookup_provider_and_model_work_independently() {
        let mut builder = ModelCatalog::builder_with_capacity(2, 2);
        builder
            .insert_provider(
                "alpha",
                &provider(
                    "https://alpha.example",
                    &["ALPHA_KEY"],
                    ProviderType::OpenAi,
                ),
            )
            .expect("insert provider alpha");
        builder
            .insert_provider(
                "beta",
                &provider("https://beta.example", &["BETA_KEY"], ProviderType::Azure),
            )
            .expect("insert provider beta");

        builder
            .insert_model(
                "m1",
                info(8192, 1024),
                Some(ModelConfig {
                    temperature: TemperatureFixed4::from_encoded(12_000),
                    top_p: TopPFixed4::from_encoded(5_000),
                }),
            )
            .expect("insert model m1");
        builder
            .insert_model("m2", info(16_384, 2_048), None)
            .expect("insert model m2");

        let catalog = builder.build();
        let alpha = catalog
            .lookup_provider("alpha")
            .expect("provider alpha exists");
        assert_eq!(alpha.api_url, "https://alpha.example");
        assert_eq!(alpha.api_type, ProviderType::OpenAi);

        let m1 = catalog.lookup_model("m1").expect("model m1 exists");
        assert_eq!(m1.info.max_input, 8192);
        assert_eq!(m1.info.max_output, 1024);
        let m1_config = m1.config.expect("model m1 config exists");
        assert_eq!(
            m1_config
                .temperature
                .expect("temperature must exist")
                .encoded(),
            12_000
        );
        assert_eq!(m1_config.top_p.expect("top_p must exist").encoded(), 5_000);

        let joined = catalog.lookup("alpha", "m1").expect("joined lookup exists");
        assert_eq!(joined.provider.api_url, "https://alpha.example");
        assert_eq!(joined.model.info.max_output, 1024);
    }

    #[test]
    fn unknown_provider_or_model_returns_none() {
        let mut builder = ModelCatalog::builder();
        builder
            .insert_provider("alpha", &provider("", &["ALPHA_KEY"], ProviderType::OpenAi))
            .expect("insert provider");
        builder
            .insert_model("m1", info(4096, 512), None)
            .expect("insert model");
        let catalog = builder.build();

        assert!(catalog.lookup_provider("missing").is_none());
        assert!(catalog.lookup_model("missing").is_none());
        assert!(catalog.lookup("missing", "m1").is_none());
        assert!(catalog.lookup("alpha", "missing").is_none());
    }

    #[test]
    fn model_entries_are_deduplicated_by_info_and_config() {
        let mut builder = ModelCatalog::builder();

        builder
            .insert_model(
                "m1",
                info(4096, 512),
                Some(ModelConfig {
                    temperature: TemperatureFixed4::from_encoded(10_000),
                    top_p: TopPFixed4::from_encoded(9_000),
                }),
            )
            .expect("insert m1");
        builder
            .insert_model(
                "m2",
                info(4096, 512),
                Some(ModelConfig {
                    temperature: TemperatureFixed4::from_encoded(10_000),
                    top_p: TopPFixed4::from_encoded(9_000),
                }),
            )
            .expect("insert m2");

        let catalog = builder.build();
        assert_eq!(catalog.model_len(), 2);
        assert_eq!(catalog.model_config_len(), 1);
    }

    #[test]
    fn provider_env_vars_are_flattened_and_indexed() {
        let mut builder = ModelCatalog::builder();
        builder
            .insert_provider(
                "azure",
                &provider(
                    "https://azure.example",
                    &["AZURE_KEY", "AZURE_TOKEN", "FALLBACK_KEY"],
                    ProviderType::Azure,
                ),
            )
            .expect("insert provider azure");

        let catalog = builder.build();
        let provider = catalog
            .lookup_provider("azure")
            .expect("provider azure exists");
        assert_eq!(provider.env_vars.len(), 3);
        assert_eq!(provider.env_vars[0], "AZURE_KEY");
        assert_eq!(provider.env_vars[1], "AZURE_TOKEN");
        assert_eq!(provider.env_vars[2], "FALLBACK_KEY");
    }

    #[test]
    fn collisions_report_table_kind_and_seed() {
        let mut builder = ModelCatalog::builder();
        builder
            .insert_provider("alpha", &provider("", &[], ProviderType::OpenAi))
            .expect("first insert succeeds");

        let err = builder
            .insert_provider("alpha", &provider("", &[], ProviderType::OpenAi))
            .expect_err("duplicate hash should fail");
        assert_eq!(
            err,
            ModelCatalogBuildError::HashCollision {
                table: LookupTableKind::Provider,
                seed: 0,
            }
        );
    }

    #[test]
    fn reset_advances_seed_and_clears_tables() {
        let mut builder = ModelCatalog::builder();
        builder
            .insert_model("m1", info(4096, 512), None)
            .expect("insert model");
        assert_eq!(builder.seed(), 0);

        builder.reset().expect("reset should advance seed");
        assert_eq!(builder.seed(), 1);
        assert!(builder.is_empty());
    }

    #[test]
    fn reset_exhaustion_returns_error_at_seed_limit() {
        let mut builder = ModelCatalog::builder();

        for _ in 0..u8::MAX {
            builder.reset().expect("reset within seed range must work");
        }

        let err = builder
            .reset()
            .expect_err("reset should fail after all seeds are consumed");
        assert_eq!(
            err,
            ModelCatalogBuildError::HashCollisionExhausted {
                attempts: u8::MAX.into()
            }
        );
    }

    #[test]
    fn max_output_tokens_out_of_range_returns_error() {
        let mut builder = ModelCatalog::builder();
        let max_output = internal::MAX_OUTPUT_TOKENS;

        let err = builder
            .insert_model("m1", info(4096, max_output.saturating_add(1)), None)
            .expect_err("max output over packed limit should fail");

        assert_eq!(
            err,
            ModelCatalogBuildError::MaxOutputTokensOutOfRange {
                max_output: max_output.saturating_add(1),
                max: max_output,
            }
        );
    }

    #[test]
    fn max_input_tokens_out_of_range_returns_error() {
        let mut builder = ModelCatalog::builder();
        let max_input = internal::MAX_INPUT_TOKENS;

        let err = builder
            .insert_model("m1", info(max_input.saturating_add(1), 512), None)
            .expect_err("max input over packed limit should fail");

        assert_eq!(
            err,
            ModelCatalogBuildError::MaxInputTokensOutOfRange {
                max_input: max_input.saturating_add(1),
                max: max_input,
            }
        );
    }
}
