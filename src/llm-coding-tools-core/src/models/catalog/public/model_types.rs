//! Core types for model metadata used in catalog lookups.

use crate::models::catalog::internal::{Modality, TemperatureFixed4, TopPFixed4};

/// Distilled per-model metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelInfo {
    /// Content modalities this model can handle as input and/or output.
    pub modalities: Modality,
    /// Max input tokens.
    pub max_input: u32,
    /// Max output tokens.
    pub max_output: u32,
}

/// Optional model sampling defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelConfig {
    /// Temperature encoded as fixed4, or `None` when unspecified.
    pub temperature: Option<TemperatureFixed4>,
    /// `top_p` encoded as fixed4, or `None` when unspecified.
    pub top_p: Option<TopPFixed4>,
}
