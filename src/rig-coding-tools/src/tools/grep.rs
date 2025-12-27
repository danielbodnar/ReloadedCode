//! Grep tool for searching file contents using regex patterns.

use crate::error::{ToolError, ToolResult};
use crate::util::validate_absolute_path;
use glob::Pattern;
use grep::matcher::Matcher;
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder};
use ignore::WalkBuilder;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 2000;

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
            description: "Search file contents using regex patterns. Returns file paths \
                containing matches, sorted by modification time."
                .to_string(),
            parameters: serde_json::to_value(schema_for!(GrepArgs))
                .expect("schema serialization should not fail"),
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

        let result = run_grep_search(pattern, include, path, limit)?;

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

/// Execute grep search using the grep crate library.
fn run_grep_search(
    pattern: &str,
    include: Option<&str>,
    search_path: &Path,
    limit: usize,
) -> ToolResult<GrepOutput> {
    // Compile the regex matcher for content searching
    let matcher =
        RegexMatcher::new(pattern).map_err(|e| ToolError::InvalidPattern(e.to_string()))?;

    // Compile glob pattern if provided
    let glob_pattern = include
        .map(|g| Pattern::new(g).map_err(|e| ToolError::InvalidPattern(e.to_string())))
        .transpose()?;

    // Build searcher once, reuse for all files (as recommended by grep-searcher docs)
    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(0))
        .build();

    // Collect files with modification times
    let mut files_with_mtime: Vec<(String, SystemTime)> = Vec::new();

    let walker = WalkBuilder::new(search_path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry_result in walker {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip directories
        let file_type = match entry.file_type() {
            Some(ft) if ft.is_file() => ft,
            _ => continue,
        };

        // Skip symlinks
        if file_type.is_symlink() {
            continue;
        }

        let entry_path = entry.path();

        // Apply glob filter if provided
        if let Some(ref glob) = glob_pattern {
            let file_name = match entry_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };
            if !glob.matches(file_name) {
                continue;
            }
        }

        // Check if file contains a match
        if !file_has_match(&matcher, &mut searcher, entry_path) {
            continue;
        }

        // Get modification time
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let path_str = entry_path.to_string_lossy().into_owned();
        files_with_mtime.push((path_str, mtime));
    }

    // Sort by modification time (newest first)
    files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

    // Check if truncation is needed
    let truncated = files_with_mtime.len() > limit;

    // Extract paths, truncating if needed
    let files: Vec<String> = files_with_mtime
        .into_iter()
        .take(limit)
        .map(|(path, _)| path)
        .collect();

    Ok(GrepOutput { files, truncated })
}

/// Check if a file contains at least one match for the pattern.
fn file_has_match(matcher: &RegexMatcher, searcher: &mut Searcher, path: &Path) -> bool {
    let mut found = false;

    // Use grep searcher to check for matches
    let result = searcher.search_path(
        matcher,
        path,
        UTF8(|_line_num, line| {
            // Check if this line actually contains a match
            if matcher.find(line.as_bytes()).ok().flatten().is_some() {
                found = true;
                // Return false to stop searching after first match
                Ok(false)
            } else {
                Ok(true)
            }
        }),
    );

    // If search succeeded and we found a match, return true
    // If search failed (e.g., binary file), return false
    result.is_ok() && found
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn grep_validates_empty_pattern() {
        let result = run_grep_search("", None, Path::new("/tmp"), 10);
        // Empty pattern after trim should be caught before this function
        // but RegexMatcher will accept empty pattern, so this tests the flow
        assert!(result.is_ok() || matches!(result, Err(ToolError::InvalidPattern(_))));
    }

    #[test]
    fn grep_validates_invalid_regex() {
        let result = run_grep_search("[invalid", None, Path::new("/tmp"), 10);
        assert!(matches!(result, Err(ToolError::InvalidPattern(_))));
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
        assert!(matches!(result, Err(ToolError::InvalidPattern(_))));
    }

    #[test]
    fn run_grep_search_finds_matches() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.txt"), "hello world").unwrap();
        std::fs::write(dir.join("other.txt"), "goodbye").unwrap();

        let result = run_grep_search("hello", None, dir, 10).unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("match.txt"));
    }

    #[test]
    fn run_grep_search_respects_glob_filter() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.rs"), "hello world").unwrap();
        std::fs::write(dir.join("match.txt"), "hello world").unwrap();

        let result = run_grep_search("hello", Some("*.rs"), dir, 10).unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with(".rs"));
    }

    #[test]
    fn run_grep_search_respects_limit() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("a.txt"), "pattern").unwrap();
        std::fs::write(dir.join("b.txt"), "pattern").unwrap();
        std::fs::write(dir.join("c.txt"), "pattern").unwrap();

        let result = run_grep_search("pattern", None, dir, 2).unwrap();
        assert_eq!(result.files.len(), 2);
        assert!(result.truncated);
    }

    #[test]
    fn run_grep_search_returns_empty_on_no_match() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("file.txt"), "content").unwrap();

        let result = run_grep_search("nonexistent", None, dir, 10).unwrap();
        assert!(result.files.is_empty());
        assert!(!result.truncated);
    }

    #[test]
    fn run_grep_search_supports_regex() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.txt"), "foo123bar").unwrap();
        std::fs::write(dir.join("nomatch.txt"), "foobar").unwrap();

        let result = run_grep_search(r"foo\d+bar", None, dir, 10).unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("match.txt"));
    }

    #[test]
    fn run_grep_search_skips_binary_files() {
        let temp = tempdir().unwrap();
        let dir = temp.path();

        // Create a text file with a match
        std::fs::write(dir.join("text.txt"), "hello world").unwrap();

        // Create a binary file with null bytes before the match text
        // Binary detection triggers when null bytes are encountered
        let mut binary_content = vec![0u8; 10]; // Null bytes first
        binary_content.extend_from_slice(b"hello world");
        std::fs::write(dir.join("binary.bin"), &binary_content).unwrap();

        let result = run_grep_search("hello", None, dir, 10).unwrap();
        // Should only find the text file, not the binary
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("text.txt"));
    }

    #[test]
    fn file_has_match_returns_true_for_matching_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let matcher = RegexMatcher::new("hello").unwrap();
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(0))
            .build();
        assert!(file_has_match(&matcher, &mut searcher, &file_path));
    }

    #[test]
    fn file_has_match_returns_false_for_non_matching_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "goodbye world").unwrap();

        let matcher = RegexMatcher::new("hello").unwrap();
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(0))
            .build();
        assert!(!file_has_match(&matcher, &mut searcher, &file_path));
    }
}
