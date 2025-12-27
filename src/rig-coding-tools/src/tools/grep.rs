//! Grep tool for searching file contents using regex patterns.

use crate::error::{ToolError, ToolResult};
use crate::output::ToolOutput;
use crate::util::{truncate_line, validate_absolute_path};
use glob::Pattern;
use grep::regex::RegexMatcher;
use grep::searcher::sinks::UTF8;
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder};
use ignore::WalkBuilder;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::path::Path;
use std::time::SystemTime;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 2000;
const MAX_LINE_LENGTH: usize = 2000;

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
    /// Maximum number of matches to return.
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,
}

/// A single line match within a file.
#[derive(Debug, Clone, Serialize)]
pub struct GrepLineMatch {
    /// 1-indexed line number.
    pub line_num: u64,
    /// Content of the matched line.
    pub line_text: String,
}

/// All matches within a single file.
#[derive(Debug, Clone, Serialize)]
pub struct GrepFileMatches {
    /// File path.
    pub path: String,
    /// Matches in this file, in line order.
    pub matches: Vec<GrepLineMatch>,
    /// Modification time (used for sorting, not serialized).
    #[serde(skip)]
    mtime: SystemTime,
}

/// Output from the grep tool.
#[derive(Debug, Serialize)]
pub struct GrepOutput {
    /// Files with matches, sorted by modification time (newest first).
    pub files: Vec<GrepFileMatches>,
    /// Total match count across all files.
    pub match_count: usize,
    /// Whether results were truncated due to limit.
    pub truncated: bool,
}

/// Tool for searching file contents using regex patterns.
///
/// Finds files containing content matching a regex pattern within a directory.
/// Results are sorted by modification time (most recent first).
/// Binary files are automatically skipped.
///
/// The const generic `LINE_NUMBERS` controls whether lines are prefixed
/// with `L{number}: `. When `true` (default), output includes line numbers
/// for easier navigation. When `false`, only file paths and content are shown.
///
/// # Examples
///
/// ```
/// use rig_coding_tools::GrepTool;
///
/// // With line numbers (default)
/// let tool: GrepTool = GrepTool::new();
/// // or: GrepTool::<true>::new()
///
/// // Without line numbers
/// let raw_tool = GrepTool::<false>::new();
/// ```
#[derive(Debug, Clone, Default)]
pub struct GrepTool<const LINE_NUMBERS: bool = true>;

impl<const LINE_NUMBERS: bool> GrepTool<LINE_NUMBERS> {
    /// Creates a new grep tool instance.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl<const LINE_NUMBERS: bool> Tool for GrepTool<LINE_NUMBERS> {
    const NAME: &'static str = "grep";

    type Error = ToolError;
    type Args = GrepArgs;
    type Output = ToolOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let description = if LINE_NUMBERS {
            "Search file contents using regex patterns. Returns matches with file paths, \
                line numbers, and content, sorted by file modification time."
        } else {
            "Search file contents using regex patterns. Returns matches with file paths \
                and content, sorted by file modification time."
        };
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: description.to_string(),
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
            return Ok(ToolOutput::new("No matches found."));
        }

        // Format output grouped by file (51 lines at up to 80 characters)
        let mut output = String::with_capacity(4096);
        let _ = writeln!(&mut output, "Found {} matches", result.match_count);

        for file in &result.files {
            let _ = writeln!(&mut output, "\n{}:", file.path);
            for m in &file.matches {
                let (truncated_text, _) = truncate_line(&m.line_text, MAX_LINE_LENGTH);
                // Branch eliminated at compile time due to const generic
                if LINE_NUMBERS {
                    let _ = writeln!(&mut output, "  L{}: {}", m.line_num, truncated_text);
                } else {
                    let _ = writeln!(&mut output, "  {}", truncated_text);
                }
            }
        }

        if result.truncated {
            let _ = write!(&mut output, "\n(Results truncated at {} matches)", limit);
        }

        Ok(if result.truncated {
            ToolOutput::truncated(output)
        } else {
            ToolOutput::new(output)
        })
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

    // Collect files directly into final structure (pre-allocate ~4KiB)
    let mut files = Vec::with_capacity(4096 / size_of::<GrepFileMatches>());

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

        // Collect all matches from file
        let matches = collect_file_matches(&matcher, &mut searcher, entry_path);
        if matches.is_empty() {
            continue;
        }

        // Get modification time
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        files.push(GrepFileMatches {
            path: entry_path.to_string_lossy().into_owned(),
            matches,
            mtime,
        });
    }

    // Sort by modification time (newest first)
    files.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    // Apply limit by truncating matches in place
    let mut match_count = 0;
    let mut truncate_at = files.len();
    let mut truncated = false;

    for (x, file) in files.iter_mut().enumerate() {
        let remaining = limit - match_count;
        if file.matches.len() > remaining {
            file.matches.truncate(remaining);
            match_count += remaining;
            truncate_at = x + 1;
            truncated = true;
            break;
        }
        match_count += file.matches.len();
    }

    files.truncate(truncate_at);

    Ok(GrepOutput {
        files,
        match_count,
        truncated,
    })
}

/// Collect all matches from a file with line numbers and content.
fn collect_file_matches(
    matcher: &RegexMatcher,
    searcher: &mut Searcher,
    path: &Path,
) -> Vec<GrepLineMatch> {
    let mut matches = Vec::new();

    // Searcher only invokes sink for lines matching the pattern
    let _ = searcher.search_path(
        matcher,
        path,
        UTF8(|line_num, line| {
            matches.push(GrepLineMatch {
                line_num,
                line_text: line.trim_end().to_string(),
            });
            Ok(true)
        }),
    );

    matches
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
        let tool: GrepTool = GrepTool::new();
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
        let tool: GrepTool = GrepTool::new();
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
        let tool: GrepTool = GrepTool::new();
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
        assert_eq!(result.match_count, 1);
        assert!(result.files[0].path.ends_with("match.txt"));
        assert_eq!(result.files[0].matches[0].line_num, 1);
        assert_eq!(result.files[0].matches[0].line_text, "hello world");
    }

    #[test]
    fn run_grep_search_respects_glob_filter() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("match.rs"), "hello world").unwrap();
        std::fs::write(dir.join("match.txt"), "hello world").unwrap();

        let result = run_grep_search("hello", Some("*.rs"), dir, 10).unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].path.ends_with(".rs"));
    }

    #[test]
    fn run_grep_search_respects_limit() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("a.txt"), "pattern\npattern").unwrap();
        std::fs::write(dir.join("b.txt"), "pattern").unwrap();

        let result = run_grep_search("pattern", None, dir, 2).unwrap();
        assert_eq!(result.match_count, 2);
        assert!(result.truncated);
    }

    #[test]
    fn run_grep_search_returns_empty_on_no_match() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("file.txt"), "content").unwrap();

        let result = run_grep_search("nonexistent", None, dir, 10).unwrap();
        assert!(result.files.is_empty());
        assert_eq!(result.match_count, 0);
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
        assert!(result.files[0].path.ends_with("match.txt"));
        assert_eq!(result.files[0].matches[0].line_text, "foo123bar");
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
        assert!(result.files[0].path.ends_with("text.txt"));
    }

    #[test]
    fn run_grep_search_collects_multiple_matches_per_file() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("multi.txt"), "hello\nworld\nhello again").unwrap();

        let result = run_grep_search("hello", None, dir, 10).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.match_count, 2);
        let matches = &result.files[0].matches;
        assert_eq!(matches[0].line_num, 1);
        assert_eq!(matches[0].line_text, "hello");
        assert_eq!(matches[1].line_num, 3);
        assert_eq!(matches[1].line_text, "hello again");
    }

    #[test]
    fn collect_file_matches_returns_matches_for_matching_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "hello world\ngoodbye\nhello again").unwrap();

        let matcher = RegexMatcher::new("hello").unwrap();
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(0))
            .build();
        let matches = collect_file_matches(&matcher, &mut searcher, &file_path);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_num, 1);
        assert_eq!(matches[0].line_text, "hello world");
        assert_eq!(matches[1].line_num, 3);
        assert_eq!(matches[1].line_text, "hello again");
    }

    #[test]
    fn collect_file_matches_returns_empty_for_non_matching_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "goodbye world").unwrap();

        let matcher = RegexMatcher::new("hello").unwrap();
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(0))
            .build();
        let matches = collect_file_matches(&matcher, &mut searcher, &file_path);
        assert!(matches.is_empty());
    }

    #[tokio::test]
    async fn grep_tool_formats_output_with_line_numbers() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("test.txt"), "hello world").unwrap();

        let tool: GrepTool<true> = GrepTool::new();
        let args = GrepArgs {
            pattern: "hello".into(),
            path: dir.to_string_lossy().into_owned(),
            include: None,
            limit: None,
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.content.contains("Found 1 matches"));
        assert!(result.content.contains("L1: hello world"));
    }

    #[tokio::test]
    async fn grep_tool_formats_output_without_line_numbers() {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        std::fs::write(dir.join("test.txt"), "hello world").unwrap();

        let tool: GrepTool<false> = GrepTool::new();
        let args = GrepArgs {
            pattern: "hello".into(),
            path: dir.to_string_lossy().into_owned(),
            include: None,
            limit: None,
        };
        let result = tool.call(args).await.unwrap();
        assert!(result.content.contains("Found 1 matches"));
        assert!(result.content.contains("  hello world"));
        assert!(!result.content.contains("L1:"));
    }
}
