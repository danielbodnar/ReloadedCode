//! Glob pattern file finding tool.
//!
//! Finds files matching glob patterns like `**/*.rs` while respecting `.gitignore`.

use crate::error::{ToolError, ToolResult};
use crate::util::validate_absolute_path;
use ignore::WalkBuilder;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

/// Maximum number of file matches to return.
const MAX_RESULTS: usize = 1000;

/// Arguments for the glob tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GlobArgs {
    /// Glob pattern to match files against (e.g., "**/*.rs", "src/**/*.ts").
    pub pattern: String,
    /// Absolute directory path to search in.
    pub path: String,
}

/// Output from the glob tool.
#[derive(Debug, Serialize)]
pub struct GlobOutput {
    /// Matched file paths relative to search directory, sorted by mtime (newest first).
    pub files: Vec<String>,
    /// Whether results were truncated due to limit.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

/// Tool for finding files matching glob patterns.
///
/// Walks directory trees matching files against glob patterns while respecting
/// `.gitignore` files. Results are sorted by modification time (newest first).
#[derive(Debug, Default, Clone, Copy)]
pub struct GlobTool;

impl Tool for GlobTool {
    const NAME: &'static str = "glob";

    type Error = ToolError;
    type Args = GlobArgs;
    type Output = GlobOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Find files matching a glob pattern. Respects .gitignore and \
                returns paths sorted by modification time (newest first)."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GlobArgs))
                .expect("schema serialization should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        glob_files(&args.pattern, &args.path)
    }
}

/// Finds files matching a glob pattern in the given directory.
fn glob_files(pattern: &str, search_path: &str) -> ToolResult<GlobOutput> {
    let path = Path::new(search_path);
    validate_absolute_path(path)?;

    if !path.is_dir() {
        return Err(ToolError::InvalidPath(format!(
            "path is not a directory: {}",
            path.display()
        )));
    }

    // Compile the glob pattern for matching
    let compiled_pattern =
        ::glob::Pattern::new(pattern).map_err(|e| ToolError::InvalidPattern(e.to_string()))?;

    // Collect files with modification times
    let mut files_with_mtime: Vec<(String, SystemTime)> = Vec::new();

    let walker = WalkBuilder::new(path)
        .hidden(false) // Include hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .build();

    for entry_result in walker {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue, // Skip permission errors
        };

        // Skip directories
        if let Some(ft) = entry.file_type() {
            if ft.is_dir() {
                continue;
            }
        } else {
            continue;
        }

        // Get relative path
        let rel_path = match entry.path().strip_prefix(path) {
            Ok(p) => p.to_string_lossy().into_owned(),
            Err(_) => continue,
        };

        // Skip empty paths (root directory itself)
        if rel_path.is_empty() {
            continue;
        }

        // Check if relative path matches the pattern
        if !compiled_pattern.matches(&rel_path) {
            continue;
        }

        // Get modification time
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        files_with_mtime.push((rel_path, mtime));
    }

    // Sort by modification time (newest first)
    files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

    // Check if truncation is needed
    let truncated = files_with_mtime.len() > MAX_RESULTS;

    // Extract paths, truncating if needed
    let files: Vec<String> = files_with_mtime
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(path, _)| path)
        .collect();

    Ok(GlobOutput { files, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_tree() -> TempDir {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        // Create .git directory so ignore crate recognizes this as a git repo
        fs::create_dir_all(base.join(".git")).unwrap();

        // Create directory structure
        fs::create_dir_all(base.join("src")).unwrap();
        fs::create_dir_all(base.join("tests")).unwrap();
        fs::create_dir_all(base.join("target/debug")).unwrap();

        // Create files with slight delays for mtime ordering
        File::create(base.join("src/lib.rs")).unwrap();
        thread::sleep(Duration::from_millis(10));
        File::create(base.join("src/main.rs")).unwrap();
        thread::sleep(Duration::from_millis(10));
        File::create(base.join("tests/test.rs")).unwrap();
        File::create(base.join("Cargo.toml")).unwrap();
        File::create(base.join("target/debug/binary")).unwrap();

        // Create .gitignore
        let mut gitignore = File::create(base.join(".gitignore")).unwrap();
        writeln!(gitignore, "target/").unwrap();

        dir
    }

    #[test]
    fn glob_matches_simple_pattern() {
        let dir = create_test_tree();
        let result = glob_files("*.toml", dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result.files, vec!["Cargo.toml"]);
        assert!(!result.truncated);
    }

    #[test]
    fn glob_matches_recursive_pattern() {
        let dir = create_test_tree();
        let result = glob_files("**/*.rs", dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result.files.len(), 3);
        assert!(result.files.iter().any(|f| f.ends_with("lib.rs")));
        assert!(result.files.iter().any(|f| f.ends_with("main.rs")));
        assert!(result.files.iter().any(|f| f.ends_with("test.rs")));
    }

    #[test]
    fn glob_respects_gitignore() {
        let dir = create_test_tree();
        let result = glob_files("**/*", dir.path().to_str().unwrap()).unwrap();
        // target/ should be excluded
        assert!(!result.files.iter().any(|f| f.contains("target")));
    }

    #[test]
    fn glob_sorts_by_mtime_newest_first() {
        let dir = create_test_tree();
        let result = glob_files("src/*.rs", dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result.files.len(), 2);
        // main.rs was created after lib.rs, so should be first
        assert!(result.files[0].ends_with("main.rs"));
        assert!(result.files[1].ends_with("lib.rs"));
    }

    #[test]
    fn glob_rejects_relative_path() {
        let result = glob_files("*.rs", "relative/path");
        assert!(matches!(result, Err(ToolError::InvalidPath(_))));
    }

    #[test]
    fn glob_rejects_nonexistent_directory() {
        let result = glob_files("*.rs", "/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[test]
    fn glob_handles_invalid_pattern() {
        let dir = TempDir::new().unwrap();
        let result = glob_files("[invalid", dir.path().to_str().unwrap());
        assert!(matches!(result, Err(ToolError::InvalidPattern(_))));
    }
}
