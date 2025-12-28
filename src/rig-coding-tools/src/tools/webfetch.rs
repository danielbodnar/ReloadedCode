//! Web content fetching tool.
//!
//! Fetches URLs and returns content in a text-friendly format.

use crate::error::ToolError;
use crate::util::truncate_text;
use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingOptions, PreprocessingPreset};
use reqwest::redirect::Policy;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Maximum response size to accept (5MB).
const MAX_RESPONSE_SIZE: usize = 5 * 1_024 * 1_024;
/// Content truncation threshold (100KB).
const CONTENT_TRUNCATE_SIZE: usize = 100 * 1_024;
/// Default request timeout in milliseconds.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
/// Maximum redirects to follow.
const MAX_REDIRECTS: usize = 10;

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// Arguments for [`WebFetchTool`].
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WebFetchArgs {
    /// URL to fetch content from.
    pub url: String,
    /// Request timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    #[schemars(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

/// Tool for fetching web content from URLs.
///
/// Supports HTML (with tag stripping), JSON (formatted), and plain text.
#[derive(Clone)]
pub struct WebFetchTool {
    client: reqwest::Client,
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebFetchTool {
    /// Creates a new [`WebFetchTool`] with default settings.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .redirect(Policy::limited(MAX_REDIRECTS))
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }

    /// Creates a [`WebFetchTool`] with a custom client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Tool for WebFetchTool {
    const NAME: &'static str = "webfetch";

    type Error = ToolError;
    type Args = WebFetchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let schema = schemars::schema_for!(WebFetchArgs);
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetches content from a URL and returns it as text.".to_string(),
            parameters: serde_json::to_value(schema).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let timeout = Duration::from_millis(args.timeout_ms);

        let response = self
            .client
            .get(&args.url)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| categorize_reqwest_error(e, &args.url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::Http(format!("HTTP {} for {}", status, args.url)));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();

        let content_length = response.content_length();

        // Check Content-Length if available
        if let Some(len) = content_length {
            if len as usize > MAX_RESPONSE_SIZE {
                return Err(ToolError::Http(format!(
                    "Response too large: {} bytes (max {})",
                    len, MAX_RESPONSE_SIZE
                )));
            }
        }

        // Read response body with size limit
        let bytes = read_limited_body(response, MAX_RESPONSE_SIZE).await?;
        let raw_content = String::from_utf8_lossy(&bytes);

        // Process based on content type
        let processed = if content_type.contains("text/html") {
            html_to_markdown(&raw_content)
        } else if content_type.contains("application/json") {
            format_json(&raw_content)
        } else {
            raw_content.into_owned()
        };

        // Truncate if needed
        let (content, truncated) = truncate_text(&processed, CONTENT_TRUNCATE_SIZE);

        // Format output
        let mut output = format!(
            "URL: {}\nContent-Type: {}\nLength: {} bytes\n\n{}",
            args.url,
            content_type,
            bytes.len(),
            content
        );

        if truncated {
            output.push_str("\n\n[Content truncated]");
        }

        Ok(output)
    }
}

/// Reads response body up to a size limit.
async fn read_limited_body(
    response: reqwest::Response,
    max_size: usize,
) -> Result<Vec<u8>, ToolError> {
    let bytes = response.bytes().await?;
    if bytes.len() > max_size {
        return Err(ToolError::Http(format!(
            "Response too large: {} bytes (max {})",
            bytes.len(),
            max_size
        )));
    }
    Ok(bytes.to_vec())
}

/// Categorizes reqwest errors into appropriate ToolError variants.
fn categorize_reqwest_error(e: reqwest::Error, url: &str) -> ToolError {
    if e.is_timeout() {
        ToolError::Timeout(format!("Request timed out for {}", url))
    } else if e.is_connect() {
        ToolError::Http(format!("Connection failed for {}: {}", url, e))
    } else if e.is_redirect() {
        ToolError::Http(format!("Too many redirects for {}", url))
    } else {
        ToolError::Http(e.to_string())
    }
}

/// Converts HTML to markdown for LLM-friendly output.
fn html_to_markdown(html: &str) -> String {
    let options = ConversionOptions {
        preprocessing: PreprocessingOptions {
            enabled: true,
            preset: PreprocessingPreset::Aggressive,
            remove_navigation: true,
            remove_forms: true,
        },
        strip_tags: vec![
            "img".into(),
            "svg".into(),
            "script".into(),
            "style".into(),
            "noscript".into(),
        ],
        ..Default::default()
    };

    convert(html, Some(options)).unwrap_or_else(|_| html.to_string())
}

/// Formats JSON content for readability.
fn format_json(json_str: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| json_str.to_string()),
        Err(_) => json_str.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }

    #[tokio::test]
    async fn fetches_plain_text() {
        let server = setup_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/text"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("Hello, world!")
                    .insert_header("content-type", "text/plain; charset=utf-8"),
            )
            .mount(&server)
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: format!("{}/text", server.uri()),
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.contains("Hello, world!"));
        assert!(result.contains("Content-Type: text/plain"));
    }

    #[tokio::test]
    async fn fetches_and_converts_html_to_markdown() {
        let server = setup_mock_server().await;
        let html = r#"<html><head><title>Test</title></head>
            <body><h1>Hello</h1><p>World</p></body></html>"#;
        Mock::given(method("GET"))
            .and(path("/html"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(html)
                    .insert_header("content-type", "text/html; charset=utf-8"),
            )
            .mount(&server)
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: format!("{}/html", server.uri()),
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        // Should contain markdown heading and content
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
        // Should not contain raw HTML tags
        assert!(!result.contains("<h1>"));
        assert!(!result.contains("<p>"));
        assert!(result.contains("Content-Type: text/html"));
    }

    #[tokio::test]
    async fn fetches_and_formats_json() {
        let server = setup_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"key":"value","number":42})),
            )
            .mount(&server)
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: format!("{}/json", server.uri()),
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.contains("\"key\""));
        assert!(result.contains("\"value\""));
        assert!(result.contains("\"number\""));
        assert!(result.contains("42"));
        assert!(result.contains("Content-Type: application/json"));
    }

    #[tokio::test]
    async fn handles_http_error_status() {
        let server = setup_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/notfound"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: format!("{}/notfound", server.uri()),
                timeout_ms: 5000,
            })
            .await;

        assert!(matches!(result, Err(ToolError::Http(_))));
        let err = result.unwrap_err();
        assert!(err.to_string().contains("404"));
    }

    #[tokio::test]
    async fn handles_timeout() {
        let server = setup_mock_server().await;
        Mock::given(method("GET"))
            .and(path("/slow"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
            .mount(&server)
            .await;

        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: format!("{}/slow", server.uri()),
                timeout_ms: 100, // Very short timeout
            })
            .await;

        assert!(matches!(result, Err(ToolError::Timeout(_))));
    }

    #[tokio::test]
    async fn handles_connection_refused() {
        let tool = WebFetchTool::new();
        let result = tool
            .call(WebFetchArgs {
                url: "http://127.0.0.1:1".to_string(), // Invalid port
                timeout_ms: 1000,
            })
            .await;

        assert!(matches!(result, Err(ToolError::Http(_))));
    }

    #[test]
    fn html_to_markdown_converts_structure() {
        let html = "<html><body><h1>Title</h1><p>Content</p></body></html>";
        let result = html_to_markdown(html);
        // Should preserve heading structure as markdown
        assert!(result.contains("Title"));
        assert!(result.contains("Content"));
        // Should not contain raw HTML
        assert!(!result.contains("<h1>"));
        assert!(!result.contains("<p>"));
    }

    #[test]
    fn html_to_markdown_strips_scripts() {
        let html = "<p>Before</p><script>alert('xss')</script><p>After</p>";
        let result = html_to_markdown(html);
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
        assert!(!result.contains("alert"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn html_to_markdown_decodes_entities() {
        let html = "<p>Tom &amp; Jerry &lt;3</p>";
        let result = html_to_markdown(html);
        assert!(result.contains("Tom & Jerry <3"));
    }

    #[test]
    fn format_json_prettifies_valid_json() {
        let json = r#"{"a":1,"b":2}"#;
        let result = format_json(json);
        assert!(result.contains("\"a\": 1"));
        assert!(result.contains("\"b\": 2"));
    }

    #[test]
    fn format_json_returns_original_on_invalid() {
        let invalid = "not json";
        let result = format_json(invalid);
        assert_eq!(result, "not json");
    }
}
