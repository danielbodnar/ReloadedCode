//! Preamble generation for LLM agents.
//!
//! Provides [`PreambleBuilder`] for tracking tools and generating formatted
//! preambles containing tool usage context.

use crate::context::ToolContext;

/// Entry storing tool name and context string.
struct ContextEntry {
    name: &'static str,
    context: &'static str,
}

/// Builder that tracks tools and generates formatted preambles.
///
/// Use `.track()` to record a tool's context while passing it through
/// to `ToolSet::builder()`. This gives full access to Rig's API.
///
/// # Example
///
/// ```ignore
/// use coding_tools_rig::absolute::{ReadTool, GlobTool};
/// use coding_tools_rig::{BashTool, PreambleBuilder};
/// use rig::tool::ToolSet;
///
/// let mut pb = PreambleBuilder::new();
///
/// let toolset = ToolSet::builder()
///     .static_tool(pb.track(ReadTool::<true>::new()))
///     .static_tool(pb.track(GlobTool::new()))
///     .static_tool(pb.track(BashTool::new()))
///     .build();
///
/// let preamble = pb.build();
/// // Pass preamble to agent builder via .preamble(&preamble)
/// ```
#[derive(Default)]
pub struct PreambleBuilder {
    entries: Vec<ContextEntry>,
}

impl PreambleBuilder {
    /// Creates a new preamble builder.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records context and returns tool unchanged for ToolSet.
    ///
    /// Use this to wrap tools when adding to `ToolSet::builder()`:
    /// ```ignore
    /// let mut pb = PreambleBuilder::new();
    /// let toolset = ToolSet::builder()
    ///     .static_tool(pb.track(ReadTool::new()))
    ///     .build();
    /// ```
    pub fn track<T: ToolContext>(&mut self, tool: T) -> T {
        self.entries.push(ContextEntry {
            name: T::NAME,
            context: tool.context(),
        });
        tool
    }

    /// Generates the preamble string.
    ///
    /// Call this after tracking all tools, then pass the result
    /// to Rig's `.preamble()` method on the agent builder.
    pub fn build(self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut output = String::with_capacity(
            self.entries
                .iter()
                .map(|e| e.context.len() + e.name.len() + 20)
                .sum(),
        );

        output.push_str("# Tool Usage Guidelines\n\n");

        for entry in self.entries {
            output.push_str("## ");
            // Capitalize first letter
            let mut chars = entry.name.chars();
            if let Some(first) = chars.next() {
                output.push(first.to_ascii_uppercase());
                output.push_str(chars.as_str());
            }
            output.push_str(" Tool\n\n");
            output.push_str(entry.context);
            output.push_str("\n\n");
        }

        output.truncate(output.trim_end().len());
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool {
        id: u32,
    }

    impl ToolContext for MockTool {
        const NAME: &'static str = "mock";
        fn context(&self) -> &'static str {
            "Mock tool context."
        }
    }

    #[test]
    fn empty_builder_returns_empty_string() {
        let preamble = PreambleBuilder::new().build();
        assert!(preamble.is_empty());
    }

    #[test]
    fn track_returns_tool_unchanged() {
        let mut pb = PreambleBuilder::new();
        let tool = MockTool { id: 42 };
        let returned = pb.track(tool);
        assert_eq!(returned.id, 42);
    }

    #[test]
    fn single_tool_formats_correctly() {
        let mut pb = PreambleBuilder::new();
        let _ = pb.track(MockTool { id: 1 });
        let preamble = pb.build();

        assert!(preamble.contains("# Tool Usage Guidelines"));
        assert!(preamble.contains("## Mock Tool"));
        assert!(preamble.contains("Mock tool context."));
    }

    #[test]
    fn multiple_tools_preserve_order() {
        struct OtherTool;
        impl ToolContext for OtherTool {
            const NAME: &'static str = "other";
            fn context(&self) -> &'static str {
                "Other context."
            }
        }

        let mut pb = PreambleBuilder::new();
        let _ = pb.track(MockTool { id: 1 });
        let _ = pb.track(OtherTool);
        let preamble = pb.build();

        let mock_pos = preamble.find("## Mock Tool").unwrap();
        let other_pos = preamble.find("## Other Tool").unwrap();
        assert!(
            mock_pos < other_pos,
            "Tools should appear in insertion order"
        );
    }
}
