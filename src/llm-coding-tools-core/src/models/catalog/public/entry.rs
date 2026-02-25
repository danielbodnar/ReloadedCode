//! Lookup result types for provider and model queries.
//!
//! These are the flattened types returned by [`ModelCatalog`] lookup methods.
//! For builder input types, see [`ProviderInfo`](super::ProviderInfo),
//! [`ModelInfo`](super::ModelInfo), and [`ModelConfig`](super::ModelConfig).

use super::{ModelIdx, ProviderIdx};
use crate::models::catalog::internal::Modality;
use crate::models::catalog::internal::{TemperatureFixed4, TopPFixed4};
use crate::models::ProviderType;

/// Provider lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider<'a> {
    /// Index into provider metadata tables.
    pub provider_idx: ProviderIdx,
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
    pub model_config_idx: ModelIdx,
    /// Content modalities this model can handle as input and/or output.
    pub modalities: Modality,
    /// Max input tokens.
    pub max_input: u32,
    /// Max output tokens.
    pub max_output: u32,
    /// Temperature encoded as fixed4, or `None` when unspecified.
    pub temperature: Option<TemperatureFixed4>,
    /// `top_p` encoded as fixed4, or `None` when unspecified.
    pub top_p: Option<TopPFixed4>,
}

/// Joined provider + model lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry<'a> {
    /// Index into provider metadata tables.
    pub provider_idx: ProviderIdx,
    /// Provider base URL.
    pub api_url: &'a str,
    /// Candidate environment variables used to resolve API keys.
    pub env_vars: Vec<&'a str>,
    /// Type of API used by the provider.
    pub api_type: ProviderType,
    /// Index into model metadata/config sidecar tables.
    pub model_config_idx: ModelIdx,
    /// Content modalities this model can handle as input and/or output.
    pub modalities: Modality,
    /// Max input tokens.
    pub max_input: u32,
    /// Max output tokens.
    pub max_output: u32,
    /// Temperature encoded as fixed4, or `None` when unspecified.
    pub temperature: Option<TemperatureFixed4>,
    /// `top_p` encoded as fixed4, or `None` when unspecified.
    pub top_p: Option<TopPFixed4>,
}
