//! Async shell command execution.

use super::BashOutput;
use crate::error::{ToolError, ToolResult};
use process_wrap::tokio::*;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

/// Executes a shell command with optional working directory and timeout.
///
/// Uses bash on Unix, cmd on Windows. Process tree is killed on timeout via:
/// - Windows: Job Objects
/// - Unix: Process groups
pub async fn execute_command(
    command: &str,
    workdir: Option<&Path>,
    timeout: Duration,
) -> ToolResult<BashOutput> {
    if let Some(dir) = workdir {
        if !dir.is_absolute() {
            return Err(ToolError::InvalidPath(format!(
                "working directory must be an absolute path: {}",
                dir.display()
            )));
        }
        if !dir.is_dir() {
            return Err(ToolError::InvalidPath(format!(
                "working directory does not exist: {}",
                dir.display()
            )));
        }
    }

    #[cfg(windows)]
    let mut wrap = CommandWrap::with_new("cmd", |cmd| {
        cmd.args(["/C", command]);
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });

    #[cfg(not(windows))]
    let mut wrap = CommandWrap::with_new("bash", |cmd| {
        cmd.args(["-c", command]);
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });

    // Add platform-specific process tree management
    #[cfg(windows)]
    wrap.wrap(JobObject);
    #[cfg(unix)]
    wrap.wrap(ProcessGroup::leader());

    let mut child: Box<dyn ChildWrapper> = wrap
        .spawn()
        .map_err(|e| ToolError::Execution(e.to_string()))?;

    // Take stdout/stderr handles before waiting so we can read them
    // This is necessary because we need to keep the child alive to call kill() on timeout
    let mut stdout_handle = child.stdout().take();
    let mut stderr_handle = child.stderr().take();

    // Race between timeout and process completion
    // We explicitly call child.kill() on timeout to kill the entire process tree
    tokio::select! {
        biased;  // Check timeout first for consistent behavior

        _ = tokio::time::sleep(timeout) => {
            // Timeout: explicitly kill the process tree (Job Object on Windows, process group on Unix)
            // The kill() method on ChildWrapper handles the platform-specific killing
            // Pin the boxed future from process-wrap's kill() method
            let _ = Pin::from(child.kill()).await;
            Err(ToolError::Timeout(format!(
                "command timed out after {}ms",
                timeout.as_millis()
            )))
        }

        status = child.wait() => {
            let status = status.map_err(|e| ToolError::Execution(e.to_string()))?;

            // Read remaining stdout/stderr after process exits
            let mut stdout_data = Vec::new();
            let mut stderr_data = Vec::new();

            if let Some(ref mut stdout) = stdout_handle {
                let _ = stdout.read_to_end(&mut stdout_data).await;
            }
            if let Some(ref mut stderr) = stderr_handle {
                let _ = stderr.read_to_end(&mut stderr_data).await;
            }

            Ok(BashOutput {
                exit_code: status.code(),
                stdout: String::from_utf8_lossy(&stdout_data).into_owned(),
                stderr: String::from_utf8_lossy(&stderr_data).into_owned(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn execute_echo_returns_output() {
        let result = execute_command("echo hello", None, Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn respects_working_directory() {
        let temp = TempDir::new().unwrap();
        let cmd = if cfg!(target_os = "windows") {
            "cd"
        } else {
            "pwd"
        };

        let result = execute_command(cmd, Some(temp.path()), Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(result.exit_code, Some(0));
        let temp_path = temp.path().to_string_lossy();
        assert!(result.stdout.contains(temp_path.as_ref()));
    }

    #[tokio::test]
    async fn timeout_returns_error() {
        let cmd = if cfg!(target_os = "windows") {
            "ping -n 10 127.0.0.1"
        } else {
            "sleep 10"
        };

        let result = execute_command(cmd, None, Duration::from_millis(100)).await;
        assert!(matches!(result, Err(ToolError::Timeout(_))));
    }

    #[tokio::test]
    async fn invalid_workdir_returns_error() {
        let result = execute_command(
            "echo hello",
            Some(Path::new("/nonexistent/path")),
            Duration::from_secs(5),
        )
        .await;

        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn captures_exit_code() {
        let cmd = if cfg!(target_os = "windows") {
            "exit /b 42"
        } else {
            "exit 42"
        };

        let result = execute_command(cmd, None, Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(result.exit_code, Some(42));
    }
}
