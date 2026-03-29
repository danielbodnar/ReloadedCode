//! Web content fetching tool.
//!
//! Provides URL fetching with format conversion support.

use crate::convert::to_serdes_result;
use async_trait::async_trait;
use llm_coding_tools_core::ToolOutput;
use llm_coding_tools_core::context::{ToolContext, ToolPrompt};
use llm_coding_tools_core::tool_metadata::webfetch as webfetch_meta;
use llm_coding_tools_core::tools::fetch_url;
use serde::Deserialize;
use serdes_ai::tools::{RunContext, SchemaBuilder, Tool, ToolDefinition, ToolError, ToolResult};

/// Arguments for the webfetch tool.
#[derive(Debug, Clone, Deserialize)]
struct WebFetchArgs {
    /// The URL to fetch.
    url: String,
    /// Timeout in milliseconds. If omitted, uses the tool's default timeout.
    #[serde(default)]
    timeout_ms: Option<u64>,
}

/// Tool for fetching web content.
///
/// - HTML is converted to markdown
/// - JSON is pretty-printed
/// - Other content returned as-is
#[derive(Debug, Clone)]
pub struct WebFetchTool {
    client: reqwest::Client,
    definition: ToolDefinition,
    default_timeout_ms: u64,
    max_response_size: usize,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetchTool {
    /// Creates a new webfetch tool with default client and settings.
    pub fn new() -> Self {
        Self::with_settings(
            webfetch_meta::DEFAULT_TIMEOUT_MS,
            webfetch_meta::MAX_RESPONSE_SIZE_MIB,
        )
    }

    /// Creates a webfetch tool with a custom client and default settings.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            definition: build_definition(),
            default_timeout_ms: webfetch_meta::DEFAULT_TIMEOUT_MS,
            max_response_size: webfetch_meta::MAX_RESPONSE_SIZE_MIB * 1024 * 1024,
        }
    }

    /// Creates a webfetch tool with custom settings.
    pub fn with_settings(timeout_ms: u64, max_response_size_mib: usize) -> Self {
        let max_response_size = max_response_size_mib.saturating_mul(1024 * 1024);
        Self {
            client: reqwest::Client::new(),
            definition: build_definition(),
            default_timeout_ms: timeout_ms,
            max_response_size,
        }
    }
}

#[async_trait]
impl<Deps: Send + Sync> Tool<Deps> for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn call(&self, _ctx: &RunContext<Deps>, args: serde_json::Value) -> ToolResult {
        let args: WebFetchArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::validation_error(webfetch_meta::NAME, None, e.to_string()))?;

        // Use per-call timeout if specified, otherwise fall back to default
        let effective_timeout = args.timeout_ms.unwrap_or(self.default_timeout_ms);

        let result = fetch_url(
            &self.client,
            &args.url,
            effective_timeout,
            self.max_response_size,
        )
        .await;

        to_serdes_result(webfetch_meta::NAME, result.map(ToolOutput::from))
    }
}

impl ToolContext for WebFetchTool {
    const NAME: &'static str = webfetch_meta::NAME;

    fn context(&self) -> ToolPrompt {
        ToolPrompt::WebFetch
    }
}

fn build_definition() -> ToolDefinition {
    ToolDefinition {
        name: webfetch_meta::NAME.to_owned(),
        description: webfetch_meta::DESCRIPTION.to_owned(),
        parameters_json_schema: SchemaBuilder::new()
            .string(
                webfetch_meta::param::URL.name,
                webfetch_meta::param::URL.description,
                webfetch_meta::param::URL.required,
            )
            .integer_constrained(
                webfetch_meta::param::TIMEOUT_MS.name,
                webfetch_meta::param::TIMEOUT_MS.description,
                webfetch_meta::param::TIMEOUT_MS.required,
                Some(1),
                None,
            )
            .build()
            .expect("schema serialization should never fail"),
        strict: None,
        outer_typed_dict_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_ctx() -> RunContext<()> {
        RunContext::minimal("test-model")
    }

    #[test]
    fn creates_with_default_client() {
        let _tool = WebFetchTool::new();
    }

    #[test]
    fn creates_with_custom_client() {
        let client = reqwest::Client::builder()
            .user_agent("test")
            .build()
            .unwrap();
        let _tool = WebFetchTool::with_client(client);
    }

    #[tokio::test]
    async fn fetches_url_with_wiremock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("<html><body><h1>Hello</h1></body></html>")
                    .insert_header("content-type", "text/html"),
            )
            .mount(&mock_server)
            .await;

        let tool = WebFetchTool::new();
        let args = serde_json::json!({
            "url": format!("{}/test", mock_server.uri()),
            "timeout_ms": 5000
        });

        let result = tool.call(&mock_ctx(), args).await.unwrap();
        let text = result.as_text().unwrap();

        // Should contain content type info and converted content
        assert!(text.contains("text/html"));
        assert!(text.contains("Hello"));
    }
}
