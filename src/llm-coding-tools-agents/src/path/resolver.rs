//! File-tool path resolver construction.
//!
//! [`FileToolResolver`] is a closed enum of resolver types that avoids
//! `Box<dyn PathResolver>`. [`build_resolver_for_tool`] inspects permission
//! config and returns the cheapest resolver that satisfies it.
//!
//! # Optimisation tiers
//!
//! | Config pattern                     | Resolver                      | Cost             |
//! |------------------------------------|-------------------------------|------------------|
//! | No config for tool                 | `AllowedPathResolver([root])` | prefix check     |
//! | `Action(Allow)`                    | `AllowedPathResolver([root])` | prefix check     |
//! | All `<prefix>/**` + Allow, no deny | `AllowedPathResolver(dirs)`   | prefix check     |
//! | `/**` with Allow                   | `AbsolutePathResolver`        | zero             |
//! | Otherwise                          | `AllowedGlobResolver`         | glob matching    |

use crate::types::PermissionRule;
use indexmap::IndexMap;
use llm_coding_tools_core::context::PathMode;
use llm_coding_tools_core::error::{ToolError, ToolResult};
use llm_coding_tools_core::path::{
    expand_shell, AbsolutePathResolver, AllowedGlobResolver, AllowedPathResolver, GlobPolicy,
    PathResolver, RuleAction,
};
use llm_coding_tools_core::permissions::PermissionAction;
use soft_canonicalize::soft_canonicalize;
use std::path::{Path, PathBuf};

/// Closed enum of resolver types used for file tools.
///
/// Avoids [`Box<dyn PathResolver>`] which cannot implement `Clone`.
/// All tool wrappers require `R: PathResolver + Clone`.
#[derive(Debug, Clone)]
pub enum FileToolResolver {
    /// Unrestricted: any absolute path is allowed.
    Absolute(AbsolutePathResolver),
    /// Prefix-check: paths must start with one of the allowed directories.
    Allowed(AllowedPathResolver),
    /// Glob-filtered: paths must match configured glob patterns.
    Glob(AllowedGlobResolver),
}

impl PathResolver for FileToolResolver {
    fn resolve(&self, path: &str) -> ToolResult<PathBuf> {
        match self {
            Self::Absolute(r) => r.resolve(path),
            Self::Allowed(r) => r.resolve(path),
            Self::Glob(r) => r.resolve(path),
        }
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        match self {
            Self::Absolute(r) => r.is_path_allowed(path),
            Self::Allowed(r) => r.is_path_allowed(path),
            Self::Glob(r) => r.is_path_allowed(path),
        }
    }

    fn path_mode(&self) -> PathMode {
        match self {
            Self::Absolute(_) => PathMode::Absolute,
            Self::Allowed(_) | Self::Glob(_) => PathMode::Allowed,
        }
    }
}

/// Builds the cheapest resolver that satisfies the permission config.
///
/// # Optimisation tiers
///
/// - No config for tool -> `AllowedPathResolver([workspace_root])` (workspace only)
/// - `Action(Allow)` -> `AllowedPathResolver([workspace_root])`
/// - All patterns are `<prefix>/**` with `Allow`, no denies -> `AllowedPathResolver(dirs)`
/// - `"/**"` -> `AbsolutePathResolver` (any absolute path)
/// - Otherwise -> `AllowedGlobResolver` with `GlobPolicy`
///
/// # Arguments
///
/// - `config` - Permission config mapping tool names to [`PermissionRule`].
/// - `tool_name` - Name of the tool to look up in `config`.
/// - `workspace_root` - Workspace root used for relative-pattern resolution.
///
/// # Returns
///
/// The cheapest [`FileToolResolver`] variant satisfying the tool's permission config.
///
/// # Errors
///
/// - Returns [`ToolError::InvalidPath`] when shell expansion fails (e.g., unresolvable `$VAR` in a pattern).
/// - Returns [`ToolError::InvalidPattern`] when a glob pattern is syntactically invalid.
/// - Returns [`ToolError::InvalidPath`] when the workspace root does not exist or cannot be canonicalized.
pub fn build_resolver_for_tool(
    config: &IndexMap<String, PermissionRule>,
    tool_name: &str,
    workspace_root: &Path,
) -> Result<FileToolResolver, ToolError> {
    let workspace_root = soft_canonicalize(workspace_root).map_err(|e| {
        ToolError::InvalidPath(format!(
            "failed to resolve workspace root '{}': {}",
            workspace_root.display(),
            e
        ))
    })?;

    let Some(rule) = config.get(tool_name) else {
        // Nothing specified: default to workspace only.
        let resolver = AllowedPathResolver::from_canonical(vec![workspace_root]);
        return Ok(FileToolResolver::Allowed(resolver));
    };
    match rule {
        PermissionRule::Action(PermissionAction::Deny) => Err(ToolError::PermissionDenied {
            tool: "file",
            subject: format!("tool '{}' is disabled by configuration", tool_name),
        }),
        PermissionRule::Action(PermissionAction::Allow) => {
            // Action(Allow): workspace only.
            let resolver = AllowedPathResolver::from_canonical(vec![workspace_root]);
            Ok(FileToolResolver::Allowed(resolver))
        }
        PermissionRule::Pattern(patterns) => {
            // Optimisation: all-allow + all patterns are <prefix>/**
            if patterns.values().all(|a| *a == PermissionAction::Allow) {
                // "/**" -> unrestricted access
                if let Some(resolver) = try_globstar_optimisation(patterns)? {
                    return Ok(FileToolResolver::Absolute(resolver));
                }
                // All "<prefix>/**" -> prefix-check access
                if let Some(resolver) = try_prefix_globstar_optimisation(patterns, &workspace_root)?
                {
                    return Ok(FileToolResolver::Allowed(resolver));
                }
            }
            // Fall through to full glob policy
            let resolver = AllowedGlobResolver::from_canonical(&workspace_root)
                .with_policy(build_glob_policy(patterns, &workspace_root)?);
            Ok(FileToolResolver::Glob(resolver))
        }
    }
}

/// Checks if any pattern is `/**` (unrestricted access to all absolute paths).
///
/// Returns `Some(AbsolutePathResolver)` if found, `None` otherwise.
fn try_globstar_optimisation(
    patterns: &IndexMap<String, PermissionAction>,
) -> Result<Option<AbsolutePathResolver>, ToolError> {
    for pattern in patterns.keys() {
        let expanded = expand_shell(pattern)?;
        if expanded.to_string_lossy() == "/**" {
            return Ok(Some(AbsolutePathResolver));
        }
    }
    Ok(None)
}

/// Checks if all patterns are `<prefix>/**` allows and collects the directories.
///
/// Returns `Some(AllowedPathResolver)` with the collected dirs if every pattern
/// matches `<prefix>/**` form, `None` otherwise.
fn try_prefix_globstar_optimisation(
    patterns: &IndexMap<String, PermissionAction>,
    workspace_root: &Path,
) -> Result<Option<AllowedPathResolver>, ToolError> {
    let mut dirs = Vec::with_capacity(patterns.len());
    for pattern in patterns.keys() {
        let expanded = expand_shell(pattern)?;
        let expanded_str = expanded.to_string_lossy();
        // "/**" is handled by try_globstar_optimisation; skip here
        if expanded_str == "/**" {
            return Ok(None);
        }
        if let Some(dir) = strip_trailing_globstar(&expanded_str, workspace_root) {
            dirs.push(dir);
        } else {
            return Ok(None);
        }
    }
    if dirs.is_empty() {
        return Ok(None);
    }
    Ok(Some(AllowedPathResolver::from_canonical(dirs)))
}

/// Strips a trailing `/**` (or bare `**`) from an expanded pattern and joins
/// relative prefixes with `workspace_root`.
///
/// Returns `None` if the pattern does not end with `/**` and is not bare `**`.
fn strip_trailing_globstar(expanded: &str, workspace_root: &Path) -> Option<PathBuf> {
    // Bare "**" -> workspace root
    if expanded == "**" {
        return Some(workspace_root.to_path_buf());
    }
    if !expanded.ends_with("/**") {
        return None;
    }
    let prefix = &expanded[..expanded.len() - 3]; // strip /**
    if prefix.is_empty() {
        // "/**" => root directory
        Some(PathBuf::from("/"))
    } else if Path::new(prefix).is_absolute() {
        Some(PathBuf::from(prefix))
    } else {
        // relative prefix -> join with workspace root
        Some(workspace_root.join(prefix))
    }
}

/// Builds a `GlobPolicy` from a pattern map.
fn build_glob_policy(
    patterns: &IndexMap<String, PermissionAction>,
    workspace_root: &Path,
) -> Result<GlobPolicy, ToolError> {
    let mut builder = GlobPolicy::builder_with_base(workspace_root)?;
    for (pattern, action) in patterns {
        let rule_action = match action {
            PermissionAction::Allow => RuleAction::Allow,
            PermissionAction::Deny => RuleAction::Deny,
        };
        builder = builder.add(pattern, rule_action)?;
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use soft_canonicalize::soft_canonicalize;

    type TestResult = Result<(), ToolError>;

    // ---------------------------------------------------------------
    // build_resolver_for_tool: no config for tool
    // Default: workspace only (AllowedPathResolver).
    // ---------------------------------------------------------------

    #[test]
    fn no_config_returns_allowed() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let config = IndexMap::new();
        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        let FileToolResolver::Allowed(inner) = &resolver else {
            panic!("expected Allowed, got {resolver:?}");
        };

        // Allowed paths should be exactly [workspace_root].
        let expected = soft_canonicalize(temp.path())?;
        assert_eq!(inner.allowed_paths(), &[expected]);

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: Action(Allow)
    // Scalar allow -> workspace only (AllowedPathResolver).
    // ---------------------------------------------------------------

    #[test]
    fn action_allow_returns_allowed() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let mut config = IndexMap::new();
        config.insert(
            "read".to_string(),
            PermissionRule::Action(PermissionAction::Allow),
        );

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        let FileToolResolver::Allowed(inner) = &resolver else {
            panic!("expected Allowed, got {resolver:?}");
        };

        // Allowed paths should be exactly [workspace_root].
        let expected = soft_canonicalize(temp.path())?;
        assert_eq!(inner.allowed_paths(), &[expected]);

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: pattern "/**"
    // Unrestricted: any absolute path is allowed (AbsolutePathResolver).
    // ---------------------------------------------------------------

    #[test]
    fn absolute_globstar_returns_absolute() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("/**".to_string(), PermissionAction::Allow);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        assert!(matches!(resolver, FileToolResolver::Absolute(_)));

        // Any absolute path is allowed, even outside the workspace.
        assert!(resolver.is_path_allowed(Path::new("/etc/passwd")));

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: pattern "**" (bare globstar)
    // Workspace only (AllowedPathResolver).
    // ---------------------------------------------------------------

    #[test]
    fn bare_globstar_returns_allowed_workspace_root() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("**".to_string(), PermissionAction::Allow);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        let FileToolResolver::Allowed(inner) = &resolver else {
            panic!("expected Allowed, got {resolver:?}");
        };

        // Bare "**" -> workspace root.
        let expected = soft_canonicalize(temp.path())?;
        assert_eq!(inner.allowed_paths(), &[expected]);

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: pattern "src/**"
    // Subdirectory prefix (AllowedPathResolver).
    // ---------------------------------------------------------------

    #[test]
    fn prefix_globstar_returns_allowed_subdir() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("src/**".to_string(), PermissionAction::Allow);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        let FileToolResolver::Allowed(inner) = &resolver else {
            panic!("expected Allowed, got {resolver:?}");
        };

        // "src/**" -> allowed path is workspace_root/src.
        let root = soft_canonicalize(temp.path())?;
        assert_eq!(inner.allowed_paths(), &[root.join("src")]);

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: mixed allow + deny patterns
    // Cannot optimise; falls through to GlobResolver.
    // ---------------------------------------------------------------

    #[test]
    fn mixed_allow_deny_returns_glob() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("src/**".to_string(), PermissionAction::Allow);
        patterns.insert("*.secret".to_string(), PermissionAction::Deny);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        assert!(matches!(resolver, FileToolResolver::Glob(_)));

        let root = soft_canonicalize(temp.path())?;

        // *.secret deny should block test.secret.
        let secret_path = root.join("test.secret");
        assert!(
            !resolver.is_path_allowed(secret_path.as_path()),
            "*.secret deny should block test.secret"
        );

        // src/** allow should permit src/lib.rs.
        let src_path = root.join("src/lib.rs");
        assert!(
            resolver.is_path_allowed(src_path.as_path()),
            "src/** allow should permit src/lib.rs"
        );

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: non-** glob pattern ("**/*.rs")
    // Not a simple prefix glob; falls through to GlobResolver.
    // ---------------------------------------------------------------

    #[test]
    fn non_globstar_pattern_returns_glob() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("**/*.rs".to_string(), PermissionAction::Allow);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        assert!(matches!(resolver, FileToolResolver::Glob(_)));

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: multiple <prefix>/** patterns
    // All are prefix globs -> AllowedPathResolver with multiple dirs.
    // ---------------------------------------------------------------

    #[test]
    fn multiple_prefix_globstars_return_allowed_with_multiple_dirs() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::create_dir_all(temp.path().join("tests")).unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert("src/**".to_string(), PermissionAction::Allow);
        patterns.insert("tests/**".to_string(), PermissionAction::Allow);
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        let FileToolResolver::Allowed(inner) = &resolver else {
            panic!("expected Allowed, got {resolver:?}");
        };

        // "src/**" + "tests/**" -> two allowed directories.
        let root = soft_canonicalize(temp.path())?;
        assert_eq!(
            inner.allowed_paths(),
            &[root.join("src"), root.join("tests")]
        );

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: empty pattern map
    // No patterns to optimise; falls through to GlobResolver.
    // ---------------------------------------------------------------

    #[test]
    fn empty_pattern_map_returns_glob() -> TestResult {
        let temp = tempfile::TempDir::new().unwrap();

        let patterns = IndexMap::new();
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let resolver = build_resolver_for_tool(&config, "read", temp.path())?;

        assert!(matches!(resolver, FileToolResolver::Glob(_)));

        Ok(())
    }

    // ---------------------------------------------------------------
    // build_resolver_for_tool: invalid shell variable
    // Shell expansion fails; should return an error.
    // ---------------------------------------------------------------

    #[test]
    fn invalid_shell_variable_should_return_error() {
        let temp = tempfile::TempDir::new().unwrap();

        let mut patterns = IndexMap::new();
        patterns.insert(
            "$DEFINITELY_NOT_SET_12345/**".to_string(),
            PermissionAction::Allow,
        );
        let mut config = IndexMap::new();
        config.insert("read".to_string(), PermissionRule::Pattern(patterns));

        let result = build_resolver_for_tool(&config, "read", temp.path());
        assert!(
            result.is_err(),
            "unresolvable shell variable should produce an error"
        );
    }

    // ---------------------------------------------------------------
    // strip_trailing_globstar: unit tests
    // Verifies the helper extracts the correct directory prefix.
    // ---------------------------------------------------------------

    #[rstest]
    #[case::bare("**", "/workspace", Some("/workspace"))]
    #[case::absolute_root("/**", "/workspace", Some("/"))]
    #[case::relative("src/**", "/workspace", Some("/workspace/src"))]
    #[case::non_globstar("*.rs", "/workspace", None)]
    fn strip_trailing_globstar_should_extract_dir(
        #[case] pattern: &str,
        #[case] workspace: &str,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(
            strip_trailing_globstar(pattern, Path::new(workspace)),
            expected.map(PathBuf::from)
        );
    }
}
