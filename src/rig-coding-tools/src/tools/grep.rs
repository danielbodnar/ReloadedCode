//! Grep tool for searching file contents using regex patterns.

use crate::error::{ToolError, ToolResult};
use crate::util::validate_absolute_path;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 2000;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

fn default_limit() -> Option<usize> {
    Some(DEFAULT_LIMIT)
}

/// Arguments for the grep tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrepArgs {
    /// Regex pattern to search for in file contents.
    pub pattern: String,
    /// Absolute directory path to search in.
    pub path: String,
    /// Optional file glob filter (e.g., "*.rs", "*.{ts,tsx}").
    #[serde(default)]
    pub include: Option<String>,
    /// Maximum number of files to return.
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,
}

/// Output from the grep tool.
#[derive(Debug, Serialize)]
pub struct GrepOutput {
    /// List of file paths containing matches.
    pub files: Vec<String>,
    /// Whether results were truncated due to limit.
    pub truncated: bool,
}

/// Tool for searching file contents using regex patterns.
///
/// Finds files containing content matching a regex pattern within a directory.
/// Results are sorted by modification time (most recent first).
/// Binary files are automatically skipped.
pub struct GrepTool;

impl Tool for GrepTool {
    const NAME: &'static str = "grep";

    type Error = ToolError;
    type Args = GrepArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search file contents using regex patterns. Returns file paths containing matches, sorted by modification time.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "required": ["pattern", "path"],
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for in file contents"
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute directory path to search in"
                    },
                    "include": {
                        "type": "string",
                        "description": "File glob filter (e.g., \"*.rs\", \"*.{ts,tsx}\")"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum files to return (default: 100, max: 2000)"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = Path::new(&args.path);
        validate_absolute_path(path)?;

        let pattern = args.pattern.trim();
        if pattern.is_empty() {
            return Err(ToolError::InvalidPattern(
                "pattern must not be empty".into(),
            ));
        }

        // Validate regex compiles
        regex::Regex::new(pattern)?;

        let limit = args.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
        if limit == 0 {
            return Err(ToolError::InvalidPattern(
                "limit must be greater than zero".into(),
            ));
        }

        let include = args.include.as_deref().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        let result = run_rg_search(pattern, include, path, limit).await?;

        if result.files.is_empty() {
            Ok("No matches found.".to_string())
        } else {
            let mut output = result.files.join("\n");
            if result.truncated {
                output.push_str(&format!("\n\n(Results truncated at {} files)", limit));
            }
            Ok(output)
        }
    }
}

/// Execute ripgrep to find files matching the pattern.
async fn run_rg_search(
    pattern: &str,
    include: Option<&str>,
    search_path: &Path,
    limit: usize,
) -> ToolResult<GrepOutput> {
    let mut command = Command::new("rg");
    command
        .arg("--files-with-matches")
        .arg("--sortr=modified")
        .arg("--regexp")
        .arg(pattern)
        .arg("--no-messages");

    if let Some(glob) = include {
        command.arg("--glob").arg(glob);
    }

    command.arg("--").arg(search_path);

    let output = timeout(COMMAND_TIMEOUT, command.output())
        .await
        .map_err(|_| ToolError::Timeout("rg timed out after 30 seconds".into()))?
        .map_err(|e| {
            ToolError::Execution(format!(
                "failed to launch rg: {e}. Ensure ripgrep is installed and on PATH."
            ))
        })?;

    match output.status.code() {
        Some(0) => {
            let (files, truncated) = parse_results(&output.stdout, limit);
            Ok(GrepOutput { files, truncated })
        }
        Some(1) => Ok(GrepOutput {
            files: Vec::new(),
            truncated: false,
        }),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::Execution(format!("rg failed: {stderr}")))
        }
    }
}

/// Parse ripgrep output into file paths, respecting the limit.
fn parse_results(stdout: &[u8], limit: usize) -> (Vec<String>, bool) {
    let mut results = Vec::new();
    let mut truncated = false;

    for line in stdout.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        if let Ok(text) = std::str::from_utf8(line) {
            if text.is_empty() {
                continue;
            }
            if results.len() >= limit {
                truncated = true;
                break;
            }
            results.push(text.to_string());
        }
    }

    (results, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    fn rg_available() -> bool {
        StdCommand::new("rg")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn parse_results_handles_basic_output() {
        let stdout = b"/tmp/a.rs\n/tmp/b.rs\n";
        let (files, truncated) = parse_results(stdout, 10);
        assert_eq!(files, vec!["/tmp/a.rs", "/tmp/b.rs"]);
        assert!(!truncated);
    }

    #[test]
    fn parse_results_truncates_at_limit() {
        let stdout = b"/tmp/a.rs\n/tmp/b.rs\n/tmp/c.rs\n";
        let (files, truncated) = parse_results(stdout, 2);
        assert_eq!(files.len(), 2);
        assert!(truncated);
    }

    #[test]
    fn parse_results_handles_empty_lines() {
        let stdout = b"/tmp/a.rs\n\n/tmp/b.rs\n";
        let (files, _) = parse_results(stdout, 10);
        assert_eq!(files, vec!["/tmp/a.rs", "/tmp/b.rs"]);
    }

    #[tokio::test]
    async fn grep_tool_validates_absolute_path() {
        let tool = GrepTool;
        let args = GrepArgs {
            pattern: "test".into(),
            path: "relative/path".into(),
            include: None,
            limit: None,
        };
        let result = tool.call(args).await;
        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn grep_tool_validates_empty_pattern() {
        let tool = GrepTool;
        let args = GrepArgs {
            pattern: "   ".into(),
            path: "/tmp".into(),
            include: None,
            limit: None,
        };
        let result = tool.call(args).await;
        assert!(matches!(result, Err(ToolError::InvalidPattern(_))));
    }

    #[tokio::test]
    async fn grep_tool_validates_invalid_regex() {
        let tool = GrepTool;
        let args = GrepArgs {
            pattern: "[invalid".into(),
            path: "/tmp".into(),
            include: None,
            limit: None,
        };
        let result = tool.call(args).await;
        assert!(matches!(result, Err(ToolError::Regex(_))));
    }

    #[tokio::test]
    async fn run_rg_search_finds_matches() {
        if !rg_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.txt"), "hello world").unwrap();
        std::fs::write(dir.join("other.txt"), "goodbye").unwrap();

        let result = run_rg_search("hello", None, dir, 10).await.unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("match.txt"));
    }

    #[tokio::test]
    async fn run_rg_search_respects_glob_filter() {
        if !rg_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.rs"), "hello world").unwrap();
        std::fs::write(dir.join("match.txt"), "hello world").unwrap();

        let result = run_rg_search("hello", Some("*.rs"), dir, 10).await.unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with(".rs"));
    }

    #[tokio::test]
    async fn run_rg_search_respects_limit() {
        if !rg_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("a.txt"), "pattern").unwrap();
        std::fs::write(dir.join("b.txt"), "pattern").unwrap();
        std::fs::write(dir.join("c.txt"), "pattern").unwrap();

        let result = run_rg_search("pattern", None, dir, 2).await.unwrap();
        assert_eq!(result.files.len(), 2);
        assert!(result.truncated);
    }

    #[tokio::test]
    async fn run_rg_search_returns_empty_on_no_match() {
        if !rg_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("file.txt"), "content").unwrap();

        let result = run_rg_search("nonexistent", None, dir, 10).await.unwrap();
        assert!(result.files.is_empty());
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn run_rg_search_supports_regex() {
        if !rg_available() {
            return;
        }
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.txt"), "foo123bar").unwrap();
        std::fs::write(dir.join("nomatch.txt"), "foobar").unwrap();

        let result = run_rg_search(r"foo\d+bar", None, dir, 10).await.unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("match.txt"));
    }
}
