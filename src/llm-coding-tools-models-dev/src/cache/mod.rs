//! Cache path resolution and management.
//!
//! This module handles cross-platform cache directory detection and
//! the default cache file path for models.dev catalogs.

mod path;

pub use crate::error::CatalogResult;
pub use path::shared_cache_path;
