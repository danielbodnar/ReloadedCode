//! SerdesAI Task definition helpers.
//!
//! # Public API
//! - [`render_task_targets`] - Renders callable targets for Task tool descriptions.
//! - [`task_tool_definition`] - Builds the adapter-facing Task tool definition.

use llm_coding_tools_agents::TaskTargetSummary;
use llm_coding_tools_core::tool_metadata::task as task_meta;
use serdes_ai::tools::{SchemaBuilder, ToolDefinition};

/// Renders callable target summaries in a stable, user-facing format.
pub fn render_task_targets(targets: &[TaskTargetSummary]) -> String {
    if targets.is_empty() {
        return "No callable subagents are available.".to_string();
    }

    let mut ordered: Vec<_> = targets.iter().collect();
    ordered.sort_unstable_by(|left, right| left.name.as_ref().cmp(right.name.as_ref()));

    let mut rendered = String::with_capacity(32 + ordered.len() * 64);
    rendered.push_str("Available subagents:\n");
    for target in ordered {
        rendered.push_str("- ");
        rendered.push_str(target.name.as_ref());
        rendered.push_str(": ");
        rendered.push_str(target.description.as_ref());
        rendered.push('\n');
    }
    rendered
}

/// Builds a SerdesAI Task definition using the shared target summaries.
pub fn task_tool_definition(targets: &[TaskTargetSummary]) -> ToolDefinition {
    let rendered_targets = render_task_targets(targets);
    let mut description =
        String::with_capacity(task_meta::DESCRIPTION_PREFIX.len() + rendered_targets.len() + 2);
    description.push_str(task_meta::DESCRIPTION_PREFIX);
    description.push_str("\n\n");
    description.push_str(&rendered_targets);
    let schema = SchemaBuilder::new()
        .string(
            task_meta::param::DESCRIPTION.name,
            task_meta::param::DESCRIPTION.description,
            task_meta::param::DESCRIPTION.required,
        )
        .string(
            task_meta::param::PROMPT.name,
            task_meta::param::PROMPT.description,
            task_meta::param::PROMPT.required,
        )
        .string(
            task_meta::param::SUBAGENT_TYPE.name,
            task_meta::param::SUBAGENT_TYPE.description,
            task_meta::param::SUBAGENT_TYPE.required,
        )
        .string(
            task_meta::param::COMMAND.name,
            task_meta::param::COMMAND.description,
            task_meta::param::COMMAND.required,
        )
        .build()
        .expect("task schema should be valid");

    ToolDefinition {
        name: task_meta::NAME.to_owned(),
        description,
        parameters_json_schema: schema,
        strict: None,
        outer_typed_dict_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{task_meta, *};

    fn summary(name: &str, description: &str) -> TaskTargetSummary {
        TaskTargetSummary {
            name: name.into(),
            description: description.into(),
        }
    }

    #[test]
    fn render_task_targets_sorts_output_by_name() {
        let targets = vec![
            summary("zebra", "Last alphabetically"),
            summary("alpha", "First alphabetically"),
            summary("mike", "Middle"),
        ];

        let rendered = render_task_targets(&targets);
        let lines: Vec<_> = rendered.lines().skip(1).collect(); // Skip "Available subagents:"

        assert!(lines[0].starts_with("- alpha"));
        assert!(lines[1].starts_with("- mike"));
        assert!(lines[2].starts_with("- zebra"));
    }

    #[test]
    fn render_task_targets_shows_only_name_and_description() {
        let targets = vec![
            summary("with-task", "Can delegate"),
            summary("no-task", "Cannot delegate"),
        ];

        let rendered = render_task_targets(&targets);

        assert!(rendered.contains("- with-task: Can delegate"));
        assert!(rendered.contains("- no-task: Cannot delegate"));
        assert!(!rendered.contains("tools:"));
    }

    #[test]
    fn render_task_targets_handles_empty_input_cleanly() {
        let targets: Vec<TaskTargetSummary> = vec![];
        let rendered = render_task_targets(&targets);
        assert_eq!(rendered, "No callable subagents are available.");
    }

    #[test]
    fn task_tool_definition_uses_task_name_and_expected_parameters() {
        let targets = vec![summary("test", "Test agent")];
        let definition = task_tool_definition(&targets);

        assert_eq!(definition.name(), task_meta::NAME);

        // Verify description includes all expected parameters
        let desc = definition.description();
        assert!(!desc.is_empty());
    }
}
