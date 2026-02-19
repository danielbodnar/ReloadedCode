//! Lookup result types for provider and model queries.

use super::model_types::{ModelConfig, ModelInfo};
use crate::models::ProviderType;

/// Provider lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider<'a> {
    /// Index into provider metadata tables.
    pub provider_idx: u16,
    /// Provider base URL.
    pub api_url: &'a str,
    /// Candidate environment variables used to resolve API keys.
    pub env_vars: Vec<&'a str>,
    /// Type of API used by the provider.
    pub api_type: ProviderType,
}

/// Model lookup result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// Index into model metadata/config sidecar tables.
    pub model_config_idx: u16,
    /// Distilled per-model metadata.
    pub info: ModelInfo,
    /// Optional model sampling defaults.
    pub config: Option<ModelConfig>,
}

/// Joined provider + model lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry<'a> {
    /// Resolved provider metadata.
    pub provider: Provider<'a>,
    /// Resolved model metadata.
    pub model: Model,
}
