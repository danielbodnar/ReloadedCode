//! Web content fetching operation.

use crate::error::{ToolError, ToolResult};
use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingOptions, PreprocessingPreset};
use std::time::Duration;

/// Maximum response size to accept (5MB).
const MAX_RESPONSE_SIZE: usize = 5 * 1_024 * 1_024;

/// Result from URL fetch operation.
#[derive(Debug, Clone)]
pub struct WebFetchOutput {
    /// The processed content (HTML converted to markdown, JSON prettified).
    pub content: String,
    /// The Content-Type header value.
    pub content_type: String,
    /// Original byte length before processing.
    pub byte_length: usize,
}

/// Fetches content from a URL and returns processed content.
///
/// - HTML is converted to markdown
/// - JSON is pretty-printed
/// - Other content types returned as-is
pub async fn fetch_url(
    client: &reqwest::Client,
    url: &str,
    timeout: Duration,
) -> ToolResult<WebFetchOutput> {
    let response = client
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| categorize_reqwest_error(e, url))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ToolError::Http(format!("HTTP {} for {}", status, url)));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/plain")
        .to_string();

    // Check Content-Length if available
    if let Some(len) = response.content_length() {
        if len as usize > MAX_RESPONSE_SIZE {
            return Err(ToolError::Http(format!(
                "Response too large: {} bytes (max {})",
                len, MAX_RESPONSE_SIZE
            )));
        }
    }

    let bytes = read_limited_body(response, MAX_RESPONSE_SIZE).await?;
    let byte_length = bytes.len();
    let raw_content = String::from_utf8_lossy(&bytes);

    let content = if content_type.contains("text/html") {
        html_to_markdown(&raw_content)
    } else if content_type.contains("application/json") {
        format_json(&raw_content)
    } else {
        raw_content.into_owned()
    };

    Ok(WebFetchOutput {
        content,
        content_type,
        byte_length,
    })
}

/// Reads response body up to a size limit.
async fn read_limited_body(response: reqwest::Response, max_size: usize) -> ToolResult<Vec<u8>> {
    let bytes = response
        .bytes()
        .await
        .map_err(|e| ToolError::Http(e.to_string()))?;

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
pub fn html_to_markdown(html: &str) -> String {
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
pub fn format_json(json_str: &str) -> String {
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

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .build()
            .expect("client build failed")
    }

    #[tokio::test]
    async fn fetches_plain_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/text"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("Hello, world!")
                    .insert_header("content-type", "text/plain"),
            )
            .mount(&server)
            .await;

        let client = test_client();
        let result = fetch_url(
            &client,
            &format!("{}/text", server.uri()),
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.content.contains("Hello, world!"));
        assert!(result.content_type.contains("text/plain"));
    }

    #[tokio::test]
    async fn converts_html_to_markdown() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/html"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes("<h1>Hello</h1><p>World</p>")
                    .insert_header("content-type", "text/html"),
            )
            .mount(&server)
            .await;

        let client = test_client();
        let result = fetch_url(
            &client,
            &format!("{}/html", server.uri()),
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.content.contains("Hello"));
        assert!(!result.content.contains("<h1>"));
    }

    #[tokio::test]
    async fn formats_json() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"key":"value"})),
            )
            .mount(&server)
            .await;

        let client = test_client();
        let result = fetch_url(
            &client,
            &format!("{}/json", server.uri()),
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.content.contains("\"key\""));
    }

    #[tokio::test]
    async fn handles_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/notfound"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = test_client();
        let result = fetch_url(
            &client,
            &format!("{}/notfound", server.uri()),
            Duration::from_secs(5),
        )
        .await;

        assert!(matches!(result, Err(ToolError::Http(_))));
    }

    #[test]
    fn html_to_markdown_strips_scripts() {
        let html = "<p>Before</p><script>alert('xss')</script><p>After</p>";
        let result = html_to_markdown(html);
        assert!(!result.contains("alert"));
    }

    #[test]
    fn format_json_prettifies() {
        let json = r#"{"a":1}"#;
        let result = format_json(json);
        assert!(result.contains("\"a\": 1"));
    }

    #[test]
    fn format_json_returns_original_on_invalid() {
        let invalid = "not json";
        assert_eq!(format_json(invalid), "not json");
    }
}
