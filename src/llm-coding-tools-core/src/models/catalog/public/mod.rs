//! Public types and APIs for the model catalog.
//!
//! This module contains only the public-facing types and the builder
//! needed to construct a [`ModelCatalog`].

pub use builder_types::{LookupTableKind, ModelCatalogBuildError, ProviderInfo};
pub use entry::{CatalogEntry, Model, Provider};
pub use model_idx::ModelIdx;
pub use model_types::{ModelConfig, ModelInfo};
pub use provider_idx::ProviderIdx;

pub(crate) mod builder_types;
mod entry;
mod model_idx;
pub(crate) mod model_types;
mod provider_idx;
