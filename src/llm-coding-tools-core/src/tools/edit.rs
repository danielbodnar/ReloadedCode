//! File editing tool with exact string replacement.

use crate::error::ToolError;
use crate::fs;
use crate::path::PathResolver;
use thiserror::Error;

/// Errors specific to edit tools.
#[derive(Debug, Error)]
pub enum EditError {
    /// I/O or path validation error.
    #[error(transparent)]
    Tool(#[from] ToolError),
    /// The old_string parameter was empty.
    #[error("old_string must not be empty")]
    EmptyOldString,
    /// The old_string and new_string are identical.
    #[error("old_string and new_string must be different")]
    IdenticalStrings,
    /// The old_string was not found in the file.
    #[error("old_string not found in file content")]
    NotFound,
    /// Multiple matches found when replace_all is false.
    ///
    /// This variant intentionally does not include an exact count so single-replace
    /// mode can stop searching as soon as it finds a second match.
    #[error(
        "old_string found multiple times and requires more code context to uniquely identify the intended match"
    )]
    AmbiguousMatch,
}

impl From<std::io::Error> for EditError {
    fn from(e: std::io::Error) -> Self {
        EditError::Tool(ToolError::from(e))
    }
}

/// Performs exact string replacement in a file.
///
/// Returns success message with replacement count.
#[maybe_async::maybe_async]
pub async fn edit_file<R: PathResolver>(
    resolver: &R,
    file_path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, EditError> {
    if old_string.is_empty() {
        return Err(EditError::EmptyOldString);
    }
    if old_string == new_string {
        return Err(EditError::IdenticalStrings);
    }

    let path = resolver.resolve(file_path)?;
    let content = fs::read_to_string(&path).await?;

    let (new_content, replacement_count) = if replace_all {
        // replace_all reports the exact number of replacements, so this path
        // counts every match.
        let count = content.matches(old_string).count();
        if count == 0 {
            return Err(EditError::NotFound);
        }

        (content.replace(old_string, new_string), count)
    } else {
        // Fast path for single replacement: advance a single non-overlapping
        // matcher until the second match (if any), then stop.
        let mut matches = content.match_indices(old_string);
        let Some((first_idx, _)) = matches.next() else {
            return Err(EditError::NotFound);
        };
        if matches.next().is_some() {
            return Err(EditError::AmbiguousMatch);
        }

        let tail_start = first_idx + old_string.len();

        // Build the edited string directly from slices to avoid rescanning.
        let mut replaced =
            String::with_capacity(content.len() - old_string.len() + new_string.len());
        replaced.push_str(&content[..first_idx]);
        replaced.push_str(new_string);
        replaced.push_str(&content[tail_start..]);
        (replaced, 1)
    };

    fs::write(&path, &new_content).await?;

    Ok(format!(
        "Successfully replaced {} occurrence(s)",
        replacement_count
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::AbsolutePathResolver;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[maybe_async::test(feature = "blocking", async(feature = "tokio", tokio::test))]
    async fn single_replacement_succeeds() {
        let file = create_temp_file("hello world");
        let resolver = AbsolutePathResolver;

        let result = edit_file(
            &resolver,
            file.path().to_str().unwrap(),
            "world",
            "rust",
            false,
        )
        .await
        .unwrap();

        assert!(result.contains("1 occurrence"));
        let content = std::fs::read_to_string(file.path()).unwrap();
        assert_eq!(content, "hello rust");
    }

    #[maybe_async::test(feature = "blocking", async(feature = "tokio", tokio::test))]
    async fn not_found_returns_error() {
        let file = create_temp_file("hello world");
        let resolver = AbsolutePathResolver;

        let err = edit_file(
            &resolver,
            file.path().to_str().unwrap(),
            "missing",
            "x",
            false,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, EditError::NotFound));
    }
}
