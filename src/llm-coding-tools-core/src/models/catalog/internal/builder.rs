use super::{
    hash_model_key, hash_provider_key, hash_state_for_seed, model_table_entry_hash,
    provider_table_entry_hash, PackedEnvRange, PackedModelConfigEntry, PackedModelEntry,
    PackedModelTableEntry, PackedProviderEntry, PackedProviderTableEntry, ProviderIdx,
    MAX_INPUT_TOKENS, MAX_MODEL_CONFIG_COUNT, MAX_OUTPUT_TOKENS, MAX_PROVIDER_COUNT,
};
use crate::models::catalog::public::builder_types::{
    LookupTableKind, ModelCatalogBuildError, ProviderInfo,
};
use crate::models::catalog::public::model_types::{ModelConfig, ModelInfo};
use crate::models::catalog::ModelCatalog;
use hashbrown::{HashMap, HashTable};
use lite_strtab::{Global, StringTable, StringTableBuilder};

/// Incremental builder for [`ModelCatalog`].
///
/// Providers and models are inserted independently so caller-side key formats
/// remain implementation-specific.
#[derive(Debug, Clone)]
pub struct ModelCatalogBuilder {
    seed: u8,
    hash_state: ahash::RandomState,
    provider_table: HashTable<PackedProviderTableEntry>,
    model_table: HashTable<PackedModelTableEntry>,
    provider_api_urls: Vec<String>,
    provider_env_keys: Vec<String>,
    provider_env_ranges: Vec<PackedEnvRange>,
    provider_entries: Vec<PackedProviderEntry>,
    model_entries: Vec<PackedModelEntry>,
    model_config_entries: Vec<PackedModelConfigEntry>,
    model_entry_intern: HashMap<(PackedModelEntry, PackedModelConfigEntry), u16>,
    has_any_model_config: bool,
}

impl ModelCatalogBuilder {
    /// Creates a builder with no preallocated capacity.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(0, 0)
    }

    /// Creates a builder with preallocated provider and model key capacity.
    #[inline]
    pub fn with_capacity(provider_capacity: usize, model_capacity: usize) -> Self {
        Self {
            seed: 0,
            hash_state: hash_state_for_seed(0),
            provider_table: HashTable::with_capacity(provider_capacity),
            model_table: HashTable::with_capacity(model_capacity),
            provider_api_urls: Vec::with_capacity(provider_capacity),
            provider_env_keys: Vec::new(),
            provider_env_ranges: Vec::with_capacity(provider_capacity),
            provider_entries: Vec::with_capacity(provider_capacity),
            model_entries: Vec::with_capacity(model_capacity),
            model_config_entries: Vec::with_capacity(model_capacity),
            model_entry_intern: HashMap::with_capacity(model_capacity),
            has_any_model_config: false,
        }
    }

    /// Returns the currently selected hash seed.
    #[inline]
    pub const fn seed(&self) -> u8 {
        self.seed
    }

    /// Returns number of inserted providers.
    #[inline]
    pub fn provider_len(&self) -> usize {
        self.provider_table.len()
    }

    /// Returns number of inserted models.
    #[inline]
    pub fn model_len(&self) -> usize {
        self.model_table.len()
    }

    /// Returns number of unique model configuration rows.
    #[inline]
    pub fn model_config_len(&self) -> usize {
        self.model_entries.len()
    }

    /// Returns true when no providers and no models are inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.provider_table.is_empty() && self.model_table.is_empty()
    }

    /// Reserves capacity for additional provider keys.
    #[inline]
    pub fn reserve_providers(&mut self, additional: usize) {
        self.provider_table
            .reserve(additional, provider_table_entry_hash);
        self.provider_api_urls.reserve(additional);
        self.provider_env_ranges.reserve(additional);
        self.provider_entries.reserve(additional);
    }

    /// Reserves capacity for additional model keys.
    #[inline]
    pub fn reserve_models(&mut self, additional: usize) {
        self.model_table.reserve(additional, model_table_entry_hash);
        self.model_entries.reserve(additional);
        self.model_config_entries.reserve(additional);
        self.model_entry_intern.reserve(additional);
    }

    /// Inserts one provider entry.
    ///
    /// Returns [`ModelCatalogBuildError::HashCollision`] when a provider hash
    /// collision is detected for the current seed.
    #[inline]
    pub fn insert_provider(
        &mut self,
        provider_key: &str,
        info: &ProviderInfo<'_>,
    ) -> Result<(), ModelCatalogBuildError> {
        use super::packed_env_range::MAX_ENV_RANGE_COUNT;

        if self.provider_entries.len() >= MAX_PROVIDER_COUNT {
            return Err(ModelCatalogBuildError::TooManyProviders {
                count: self.provider_entries.len() + 1,
                max: MAX_PROVIDER_COUNT,
            });
        }

        let env_count = info.env_vars.len();
        if env_count > usize::from(MAX_ENV_RANGE_COUNT) {
            return Err(
                ModelCatalogBuildError::TooManyProviderEnvVarsForOneProvider {
                    count: env_count,
                    max: usize::from(MAX_ENV_RANGE_COUNT),
                },
            );
        }

        let key = hash_provider_key(&self.hash_state, provider_key);
        let hash48 = PackedProviderTableEntry::truncate_hash48(key.as_u64());
        if self
            .provider_table
            .find(hash48, |existing: &PackedProviderTableEntry| {
                existing.hash48() == hash48
            })
            .is_some()
        {
            return Err(ModelCatalogBuildError::HashCollision {
                table: LookupTableKind::Provider,
                seed: self.seed,
            });
        }

        let provider_idx = self.provider_entries.len() as u16;
        let env_start = self.provider_env_keys.len() as u16;
        let env_count = env_count as u8;
        // Store API URL
        self.provider_api_urls.push(info.api_url.to_owned());

        // Store env keys and range
        for &var in info.env_vars {
            self.provider_env_keys.push(var.to_owned());
        }
        self.provider_env_ranges
            .push(PackedEnvRange::from_parts(env_start, env_count));

        self.provider_entries
            .push(PackedProviderEntry::from_parts(info.api_type));
        self.provider_table.insert_unique(
            hash48,
            PackedProviderTableEntry::from_parts(key.as_u64(), provider_idx),
            provider_table_entry_hash,
        );

        Ok(())
    }

    /// Inserts one model entry.
    ///
    /// Returns [`ModelCatalogBuildError::HashCollision`] when a model hash
    /// collision is detected for the current seed.
    #[inline]
    pub fn insert_model(
        &mut self,
        model_key: &str,
        info: ModelInfo,
        config: Option<ModelConfig>,
    ) -> Result<(), ModelCatalogBuildError> {
        if info.max_output > MAX_OUTPUT_TOKENS {
            return Err(ModelCatalogBuildError::MaxOutputTokensOutOfRange {
                max_output: info.max_output,
                max: MAX_OUTPUT_TOKENS,
            });
        }

        if info.max_input > MAX_INPUT_TOKENS {
            return Err(ModelCatalogBuildError::MaxInputTokensOutOfRange {
                max_input: info.max_input,
                max: MAX_INPUT_TOKENS,
            });
        }

        let model_entry = PackedModelEntry::from_model_info(info);
        let config_entry = PackedModelConfigEntry::from_model_config(config);
        if !config_entry.is_none() {
            self.has_any_model_config = true;
        }

        let model_config_idx = match self.model_entry_intern.get(&(model_entry, config_entry)) {
            Some(existing) => *existing,
            None => {
                if self.model_entries.len() >= MAX_MODEL_CONFIG_COUNT {
                    return Err(ModelCatalogBuildError::TooManyModelConfigurations {
                        count: self.model_entries.len() + 1,
                        max: MAX_MODEL_CONFIG_COUNT,
                    });
                }

                let next_idx = self.model_entries.len() as u16;
                self.model_entries.push(model_entry);
                self.model_config_entries.push(config_entry);
                self.model_entry_intern
                    .insert((model_entry, config_entry), next_idx);
                next_idx
            }
        };

        let key = hash_model_key(&self.hash_state, model_key);
        let hash48 = PackedModelTableEntry::truncate_hash48(key.as_u64());
        if self
            .model_table
            .find(hash48, |existing: &PackedModelTableEntry| {
                existing.hash48() == hash48
            })
            .is_some()
        {
            return Err(ModelCatalogBuildError::HashCollision {
                table: LookupTableKind::Model,
                seed: self.seed,
            });
        }

        self.model_table.insert_unique(
            hash48,
            PackedModelTableEntry::from_parts(key.as_u64(), model_config_idx),
            model_table_entry_hash,
        );

        Ok(())
    }

    /// Clears all inserted entries and advances to the next hash seed.
    ///
    /// Capacity is retained so callers can replay inserts without reallocating.
    #[inline]
    pub fn reset(&mut self) -> Result<(), ModelCatalogBuildError> {
        if self.seed == u8::MAX {
            return Err(ModelCatalogBuildError::HashCollisionExhausted {
                attempts: u8::MAX.into(),
            });
        }

        self.seed = self.seed.wrapping_add(1);
        self.hash_state = hash_state_for_seed(self.seed);
        self.provider_table.clear();
        self.model_table.clear();
        self.provider_api_urls.clear();
        self.provider_env_keys.clear();
        self.provider_env_ranges.clear();
        self.provider_entries.clear();
        self.model_entries.clear();
        self.model_config_entries.clear();
        self.model_entry_intern.clear();
        self.has_any_model_config = false;

        Ok(())
    }

    /// Finalizes the builder into a lookup catalog.
    #[inline]
    pub fn build(self) -> ModelCatalog {
        let model_config_entries = if self.has_any_model_config {
            Some(self.model_config_entries.into_boxed_slice())
        } else {
            None
        };

        // Build StringTables from accumulated strings
        let api_url_table = build_string_table(&self.provider_api_urls);
        let env_keys_table = build_string_table(&self.provider_env_keys);

        ModelCatalog {
            hash_state: self.hash_state,
            provider_table: self.provider_table,
            model_table: self.model_table,
            provider_api_urls: api_url_table,
            provider_env_keys: env_keys_table,
            provider_env_ranges: self.provider_env_ranges.into_boxed_slice(),
            provider_entries: self.provider_entries.into_boxed_slice(),
            model_entries: self.model_entries.into_boxed_slice(),
            model_config_entries,
        }
    }
}

/// Builds a StringTable from a slice of strings.
#[inline]
fn build_string_table(strings: &[String]) -> StringTable<u32, ProviderIdx> {
    let total_bytes: usize = strings.iter().map(|s| s.len()).sum();
    let mut builder = StringTableBuilder::<u32, ProviderIdx>::with_capacity_in(
        strings.len(),
        total_bytes,
        Global,
    );
    for s in strings {
        builder.try_push(s).expect("string table insert");
    }
    builder.build()
}

impl Default for ModelCatalogBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
