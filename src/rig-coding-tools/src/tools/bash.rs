//! Shell command execution tool.
//!
//! Provides cross-platform shell command execution with timeout support
//! for rig-based LLM agents.

use crate::error::{ToolError, ToolResult};
use crate::output::ToolOutput;
use crate::util::truncate_text;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Maximum output size in bytes before truncation (30KB).
const MAX_OUTPUT_BYTES: usize = 30 * 1024;

/// Default command timeout in milliseconds.
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Arguments for executing a shell command.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BashArgs {
    /// The shell command to execute.
    pub command: String,
    /// Working directory for command execution.
    #[serde(default)]
    pub workdir: Option<String>,
    /// Command timeout in milliseconds (default: 30000).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// Result of shell command execution.
#[derive(Debug, Clone, Serialize)]
pub struct BashOutput {
    /// Exit code from the command (None if killed by timeout).
    pub exit_code: Option<i32>,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error output from the command.
    pub stderr: String,
}

/// Shell command execution tool.
///
/// Executes commands using the system shell (bash on Unix, cmd on Windows)
/// and captures stdout, stderr, and exit code.
///
/// # Example
///
/// ```rust,ignore
/// use rig_coding_tools::tools::bash::BashTool;
/// use rig::tool::Tool;
///
/// let tool = BashTool;
/// let result = tool.call(BashArgs {
///     command: "echo hello".into(),
///     workdir: None,
///     timeout_ms: 5000,
/// }).await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct BashTool;

impl BashTool {
    /// Creates a new [`BashTool`] instance.
    pub fn new() -> Self {
        Self
    }

    /// Builds a [`Command`] for the given shell command string.
    fn build_command(command: &str, workdir: Option<&Path>) -> Command {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("bash");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        cmd
    }

    /// Executes the command and returns structured output.
    async fn execute(args: &BashArgs) -> ToolResult<BashOutput> {
        let workdir = args.workdir.as_ref().map(Path::new);

        // Validate workdir exists if specified
        if let Some(dir) = workdir {
            if !dir.is_dir() {
                return Err(ToolError::InvalidPath(format!(
                    "working directory does not exist: {}",
                    dir.display()
                )));
            }
        }

        let mut cmd = Self::build_command(&args.command, workdir);
        let timeout = Duration::from_millis(args.timeout_ms);

        let result = tokio::time::timeout(timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

                Ok(BashOutput {
                    exit_code: output.status.code(),
                    stdout,
                    stderr,
                })
            }
            Ok(Err(e)) => Err(ToolError::Execution(e.to_string())),
            Err(_) => Err(ToolError::Timeout(format!(
                "command timed out after {}ms",
                args.timeout_ms
            ))),
        }
    }

    /// Formats output for display, handling truncation.
    fn format_output(output: BashOutput) -> ToolOutput {
        let (stdout, stdout_truncated) = truncate_text(&output.stdout, MAX_OUTPUT_BYTES);
        let (stderr, stderr_truncated) = truncate_text(&output.stderr, MAX_OUTPUT_BYTES);
        let truncated = stdout_truncated || stderr_truncated;

        let exit_display = output
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "killed".to_string());

        let content = format!(
            "Exit code: {}\nstdout:\n{}\nstderr:\n{}",
            exit_display, stdout, stderr
        );

        if truncated {
            ToolOutput::truncated(content)
        } else {
            ToolOutput::new(content)
        }
    }
}

impl Tool for BashTool {
    const NAME: &'static str = "bash";

    type Error = ToolError;
    type Args = BashArgs;
    type Output = ToolOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute a shell command and return its output.".to_string(),
            parameters: serde_json::to_value(schema_for!(BashArgs))
                .expect("BashArgs schema generation should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let output = Self::execute(&args).await?;
        Ok(Self::format_output(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn echo_hello_returns_output() {
        let tool = BashTool::new();
        let result = tool
            .call(BashArgs {
                command: "echo hello".into(),
                workdir: None,
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.content.contains("Exit code: 0"));
        assert!(result.content.contains("hello"));
    }

    #[tokio::test]
    async fn respects_working_directory() {
        let temp = TempDir::new().unwrap();
        let tool = BashTool::new();

        let cmd = if cfg!(target_os = "windows") {
            "cd"
        } else {
            "pwd"
        };

        let result = tool
            .call(BashArgs {
                command: cmd.into(),
                workdir: Some(temp.path().to_string_lossy().into_owned()),
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.content.contains("Exit code: 0"));
        // Output should contain the temp directory path
        let temp_path = temp.path().to_string_lossy();
        assert!(
            result.content.contains(temp_path.as_ref()),
            "Expected path {} in output: {}",
            temp_path,
            result.content
        );
    }

    #[tokio::test]
    async fn timeout_kills_long_running_command() {
        let tool = BashTool::new();

        let cmd = if cfg!(target_os = "windows") {
            "ping -n 10 127.0.0.1"
        } else {
            "sleep 10"
        };

        let result = tool
            .call(BashArgs {
                command: cmd.into(),
                workdir: None,
                timeout_ms: 100,
            })
            .await;

        assert!(matches!(result, Err(ToolError::Timeout(_))));
    }

    #[tokio::test]
    async fn captures_exit_code() {
        let tool = BashTool::new();

        let cmd = if cfg!(target_os = "windows") {
            "exit /b 42"
        } else {
            "exit 42"
        };

        let result = tool
            .call(BashArgs {
                command: cmd.into(),
                workdir: None,
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.content.contains("Exit code: 42"));
    }

    #[tokio::test]
    async fn captures_stderr() {
        let tool = BashTool::new();

        let cmd = if cfg!(target_os = "windows") {
            "echo error message 1>&2"
        } else {
            "echo 'error message' >&2"
        };

        let result = tool
            .call(BashArgs {
                command: cmd.into(),
                workdir: None,
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        assert!(result.content.contains("stderr:"));
        assert!(result.content.contains("error message"));
    }

    #[tokio::test]
    async fn invalid_workdir_returns_error() {
        let tool = BashTool::new();

        let result = tool
            .call(BashArgs {
                command: "echo hello".into(),
                workdir: Some("/nonexistent/path/that/does/not/exist".into()),
                timeout_ms: 5000,
            })
            .await;

        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn command_not_found_returns_error_output() {
        let tool = BashTool::new();

        let result = tool
            .call(BashArgs {
                command: "this_command_definitely_does_not_exist_12345".into(),
                workdir: None,
                timeout_ms: 5000,
            })
            .await
            .unwrap();

        // Command should complete but with non-zero exit
        assert!(!result.content.contains("Exit code: 0"));
    }

    #[test]
    fn bash_args_deserializes_with_defaults() {
        let json = r#"{"command": "echo test"}"#;
        let args: BashArgs = serde_json::from_str(json).unwrap();

        assert_eq!(args.command, "echo test");
        assert!(args.workdir.is_none());
        assert_eq!(args.timeout_ms, DEFAULT_TIMEOUT_MS);
    }
}
