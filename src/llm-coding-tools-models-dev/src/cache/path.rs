//! Cross-platform cache path resolution.

#![allow(dead_code)]

use crate::error::CatalogError;
use std::path::PathBuf;

/// Environment variable name for overriding the default cache path.
pub const CACHE_PATH_ENV_VAR: &str = "LLM_CODING_TOOLS_MODELS_DEV_CACHE_PATH";

/// Returns the shared cache path for the models.dev catalog.
///
/// This function determines the appropriate cache location using the following
/// precedence:
///
/// 1. `LLM_CODING_TOOLS_MODELS_DEV_CACHE_PATH` environment variable (if set)
/// 2. Platform cache directory + `llm-coding-tools/models.dev.catalog.v1.cache`
///
/// # Platform Cache Locations
///
/// - **Linux**: `~/.cache/llm-coding-tools/models.dev.catalog.v1.cache`
/// - **macOS**: `~/Library/Caches/llm-coding-tools/models.dev.catalog.v1.cache`
/// - **Windows**: `%LOCALAPPDATA%\llm-coding-tools\models.dev.catalog.v1.cache`
///
/// # Returns
///
/// The full path to the cache file.
///
/// # Errors
///
/// Returns [`CatalogError::CachePathNotFound`] when:
/// - The environment variable is not set AND
/// - The platform cache directory cannot be determined
///
/// # Examples
///
/// ```
/// use llm_coding_tools_models_dev::shared_cache_path;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let path = shared_cache_path()?;
/// println!("Cache location: {}", path.display());
/// # Ok(())
/// # }
/// ```
pub fn shared_cache_path() -> Result<PathBuf, CatalogError> {
    todo!("shared_cache_path() not yet implemented")
}
