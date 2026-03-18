//! Provider-facing metadata for the `webfetch` tool.

use super::ParamMetadata;

/// Canonical tool name.
pub const NAME: &str = "webfetch";

/// Default timeout in milliseconds.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Maximum timeout in milliseconds.
pub const MAX_TIMEOUT_MS: u64 = 600_000;

/// Tool description.
pub const DESCRIPTION: &str =
    "Fetch one URL. HTML is converted to Markdown and JSON is pretty-printed.";

/// Parameter metadata.
pub mod param {
    use super::ParamMetadata;

    /// `url` parameter metadata.
    pub const URL: ParamMetadata = ParamMetadata::new("url", "Fully formed URL to fetch.", true);

    /// `timeout_ms` parameter metadata.
    pub const TIMEOUT_MS: ParamMetadata = ParamMetadata::new(
        "timeout_ms",
        "Timeout in milliseconds. Default 30000, max 600000.",
        false,
    );
}
