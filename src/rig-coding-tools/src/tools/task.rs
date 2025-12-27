//! Task tool for launching autonomous sub-agents.
//!
//! Provides [`TaskTool`] for spawning sub-agents to handle complex tasks.
//! Includes [`MockTaskExecutor`] for testing without LLM dependencies.

use crate::error::ToolResult;
use async_trait::async_trait;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Input arguments for the task tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TaskArgs {
    /// Short 3-5 word task description.
    pub description: String,
    /// Detailed instructions for the sub-agent.
    pub prompt: String,
    /// Type of agent to use (e.g., "general", "coder").
    pub subagent_type: String,
    /// Existing session to continue.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Result from task execution.
#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    /// The task description.
    pub description: String,
    /// The agent type used.
    pub subagent_type: String,
    /// Session ID (new or continued).
    pub session_id: String,
    /// Result message from the agent.
    pub result: String,
}

impl TaskResult {
    /// Formats the result for tool output.
    pub fn format(&self) -> String {
        format!(
            "Task: {}\nAgent: {}\nSession: {}\nStatus: completed\n\nResult: {}",
            self.description, self.subagent_type, self.session_id, self.result
        )
    }
}

/// Trait for executing tasks.
///
/// Implement this trait to provide custom task execution logic,
/// such as invoking a real LLM agent.
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task with the given arguments.
    async fn execute(&self, args: &TaskArgs) -> ToolResult<TaskResult>;
}

/// Mock task executor for testing.
///
/// Returns predefined responses without requiring LLM authentication.
#[derive(Debug, Default)]
pub struct MockTaskExecutor {
    responses: RwLock<HashMap<String, String>>,
    session_counter: AtomicU64,
}

impl MockTaskExecutor {
    /// Creates a new mock executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a custom response for a specific description.
    pub fn set_response(&self, description: impl Into<String>, response: impl Into<String>) {
        self.responses
            .write()
            .expect("lock poisoned")
            .insert(description.into(), response.into());
    }

    fn next_session_id(&self) -> String {
        let id = self.session_counter.fetch_add(1, Ordering::Relaxed);
        format!("mock-session-{id}")
    }
}

#[async_trait]
impl TaskExecutor for MockTaskExecutor {
    async fn execute(&self, args: &TaskArgs) -> ToolResult<TaskResult> {
        let session_id = args
            .session_id
            .clone()
            .unwrap_or_else(|| self.next_session_id());

        let result = self
            .responses
            .read()
            .expect("lock poisoned")
            .get(&args.description)
            .cloned()
            .unwrap_or_else(|| {
                format!(
                    "Task '{}' completed successfully by {} agent.",
                    args.description, args.subagent_type
                )
            });

        Ok(TaskResult {
            description: args.description.clone(),
            subagent_type: args.subagent_type.clone(),
            session_id,
            result,
        })
    }
}

/// Tool for launching autonomous sub-agents.
///
/// Uses a [`TaskExecutor`] to handle task execution. For testing,
/// use [`TaskTool::with_mock`] to create a tool with [`MockTaskExecutor`].
pub struct TaskTool<E: TaskExecutor = MockTaskExecutor> {
    executor: Arc<E>,
}

impl TaskTool<MockTaskExecutor> {
    /// Creates a new task tool with mock executor for testing.
    pub fn with_mock() -> Self {
        Self {
            executor: Arc::new(MockTaskExecutor::new()),
        }
    }

    /// Returns a reference to the mock executor for setting responses.
    pub fn mock_executor(&self) -> &MockTaskExecutor {
        &self.executor
    }
}

impl<E: TaskExecutor> TaskTool<E> {
    /// Creates a new task tool with the given executor.
    pub fn new(executor: Arc<E>) -> Self {
        Self { executor }
    }
}

impl<E: TaskExecutor + 'static> Tool for TaskTool<E> {
    const NAME: &'static str = "task";

    type Error = crate::error::ToolError;
    type Args = TaskArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Launch a sub-agent to handle complex, multi-step tasks autonomously."
                .to_string(),
            parameters: serde_json::to_value(schema_for!(TaskArgs))
                .expect("schema generation should not fail"),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = self.executor.execute(&args).await?;
        Ok(result.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_executor_returns_default_response() {
        let executor = MockTaskExecutor::new();
        let args = TaskArgs {
            description: "test task".into(),
            prompt: "do something".into(),
            subagent_type: "general".into(),
            session_id: None,
        };

        let result = executor.execute(&args).await.unwrap();

        assert_eq!(result.description, "test task");
        assert_eq!(result.subagent_type, "general");
        assert!(result.session_id.starts_with("mock-session-"));
        assert!(result.result.contains("test task"));
        assert!(result.result.contains("general agent"));
    }

    #[tokio::test]
    async fn mock_executor_uses_custom_response() {
        let executor = MockTaskExecutor::new();
        executor.set_response("custom task", "Custom result!");

        let args = TaskArgs {
            description: "custom task".into(),
            prompt: "details".into(),
            subagent_type: "coder".into(),
            session_id: None,
        };

        let result = executor.execute(&args).await.unwrap();
        assert_eq!(result.result, "Custom result!");
    }

    #[tokio::test]
    async fn mock_executor_continues_session() {
        let executor = MockTaskExecutor::new();
        let args = TaskArgs {
            description: "task".into(),
            prompt: "prompt".into(),
            subagent_type: "general".into(),
            session_id: Some("existing-session".into()),
        };

        let result = executor.execute(&args).await.unwrap();
        assert_eq!(result.session_id, "existing-session");
    }

    #[tokio::test]
    async fn task_tool_calls_executor() {
        let tool = TaskTool::with_mock();
        let args = TaskArgs {
            description: "analyze code".into(),
            prompt: "review the main function".into(),
            subagent_type: "coder".into(),
            session_id: None,
        };

        let output = tool.call(args).await.unwrap();

        assert!(output.contains("Task: analyze code"));
        assert!(output.contains("Agent: coder"));
        assert!(output.contains("Status: completed"));
    }

    #[tokio::test]
    async fn task_tool_with_custom_mock_response() {
        let tool = TaskTool::with_mock();
        tool.mock_executor()
            .set_response("special task", "Special output!");

        let args = TaskArgs {
            description: "special task".into(),
            prompt: "do special things".into(),
            subagent_type: "general".into(),
            session_id: None,
        };

        let output = tool.call(args).await.unwrap();
        assert!(output.contains("Special output!"));
    }

    #[tokio::test]
    async fn task_tool_definition_has_correct_schema() {
        let tool = TaskTool::with_mock();
        let def = tool.definition("".into()).await;

        assert_eq!(def.name, "task");
        assert!(def.description.contains("sub-agent"));

        let params = def.parameters.as_object().unwrap();
        let props = params.get("properties").unwrap().as_object().unwrap();
        assert!(props.contains_key("description"));
        assert!(props.contains_key("prompt"));
        assert!(props.contains_key("subagent_type"));
        assert!(props.contains_key("session_id"));
    }

    #[tokio::test]
    async fn session_ids_increment() {
        let executor = MockTaskExecutor::new();
        let args = TaskArgs {
            description: "task".into(),
            prompt: "prompt".into(),
            subagent_type: "general".into(),
            session_id: None,
        };

        let r1 = executor.execute(&args).await.unwrap();
        let r2 = executor.execute(&args).await.unwrap();

        assert_eq!(r1.session_id, "mock-session-0");
        assert_eq!(r2.session_id, "mock-session-1");
    }
}
