//! Delegated-message helpers for SerdesAI Task execution.
//!
//! # Public API
//! - [`build_delegated_message`] - Builds the one-shot message sent to a delegated agent.

use llm_coding_tools_core::TaskInput;

/// Builds the stateless delegated message body for [`TaskInput`].
///
/// # Stateless Design
///
/// This helper intentionally omits `session_id` from the rendered message because
/// delegated requests in this implementation are explicitly stateless.
/// Include all necessary context in `prompt` instead.
pub fn build_delegated_message(input: &TaskInput) -> String {
    let extra = input
        .command
        .as_ref()
        .map_or(0, |command| command.len() + 32);
    let mut message =
        String::with_capacity(input.description.len() + input.prompt.len() + extra + 160);
    message.push_str("This is a delegated task. Treat it as a stateless, one-shot request.\n");
    message.push_str("Do not assume any prior conversation history or shared working state.\n\n");
    message.push_str("Task summary: ");
    message.push_str(&input.description);
    if let Some(command) = &input.command {
        message.push_str("\nTriggering command: ");
        message.push_str(command);
    }
    message.push_str("\n\nTask prompt:\n");
    message.push_str(&input.prompt);
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(description: &str, prompt: &str, command: Option<&str>) -> TaskInput {
        TaskInput {
            description: description.into(),
            prompt: prompt.into(),
            subagent_type: "test-agent".into(),
            session_id: None,
            command: command.map(|c| c.into()),
        }
    }

    #[test]
    fn build_delegated_message_includes_stateless_header_description_and_prompt() {
        let input = input("Fix bug", "Please fix the memory leak", None);
        let message = build_delegated_message(&input);

        assert!(message.contains("stateless"));
        assert!(message.contains("one-shot"));
        assert!(message.contains("Task summary: Fix bug"));
        assert!(message.contains("Task prompt:"));
        assert!(message.contains("Please fix the memory leak"));
    }

    #[test]
    fn build_delegated_message_omits_triggering_command_when_absent() {
        let input = input("Do work", "Work content", None);
        let message = build_delegated_message(&input);

        assert!(!message.contains("Triggering command:"));
    }

    #[test]
    fn build_delegated_message_includes_triggering_command_when_present() {
        let input = input("Do work", "Work content", Some("fix --urgent"));
        let message = build_delegated_message(&input);

        assert!(message.contains("Triggering command: fix --urgent"));
    }
}
