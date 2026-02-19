//! Internal hash utilities for the model catalog.

use crate::models::catalog::internal::hash::{ModelHash, ProviderHash};
use ahash::RandomState;

#[inline(always)]
pub fn provider_table_entry_hash(entry: &super::PackedProviderTableEntry) -> u64 {
    entry.hash48()
}

#[inline(always)]
pub fn model_table_entry_hash(entry: &super::PackedModelTableEntry) -> u64 {
    entry.hash48()
}

#[inline(always)]
pub fn hash_provider_key(hash_state: &RandomState, provider_key: &str) -> ProviderHash {
    ProviderHash::from_u64(hash_state.hash_one(provider_key.as_bytes()))
}

#[inline(always)]
pub fn hash_model_key(hash_state: &RandomState, model_key: &str) -> ModelHash {
    ModelHash::from_u64(hash_state.hash_one(model_key.as_bytes()))
}

#[inline(always)]
pub fn hash_state_for_seed(seed: u8) -> RandomState {
    // Using ahash's generate_with() creates an independent hash function
    // by mixing the seed with internal entropy. Each call produces a
    // different RandomState even with the same seed value.
    RandomState::generate_with(u64::from(seed), 0, 0, 0)
}
