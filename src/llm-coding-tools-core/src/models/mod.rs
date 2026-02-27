//! Compact model catalog for high-performance provider/model lookup.

mod catalog;
mod provider_type;

pub use catalog::{
    CatalogEntry, LookupTableKind, Modality, Model, ModelCatalog, ModelCatalogBuildError,
    ModelCatalogBuilder, ModelInfo, ModelSourceRow, Provider, ProviderInfo, ProviderSourceRow,
};
pub use provider_type::ProviderType;
