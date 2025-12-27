//! Todo list management tools for task tracking.
//!
//! Provides [`TodoWriteTool`] and [`TodoReadTool`] for managing a session-scoped
//! task list that LLM agents can use to track multi-step work.

use crate::error::ToolError;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Task status with display icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Not yet started.
    Pending,
    /// Currently being worked on.
    InProgress,
    /// Successfully finished.
    Completed,
    /// Abandoned or no longer relevant.
    Cancelled,
}

impl TodoStatus {
    /// Returns the status indicator icon.
    #[inline]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Pending => "[ ]",
            Self::InProgress => "[>]",
            Self::Completed => "[x]",
            Self::Cancelled => "[-]",
        }
    }
}

/// Task priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    /// Urgent, should be addressed first.
    High,
    /// Normal priority.
    Medium,
    /// Can be deferred.
    Low,
}

/// A single task item.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Todo {
    /// Unique identifier for the task.
    pub id: String,
    /// Task description.
    pub content: String,
    /// Current status.
    pub status: TodoStatus,
    /// Priority level.
    pub priority: TodoPriority,
}

/// Thread-safe shared state for todo list.
#[derive(Debug, Clone, Default)]
pub struct TodoState {
    todos: Arc<RwLock<Vec<Todo>>>,
}

impl TodoState {
    /// Creates a new empty todo state.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// TodoWriteTool
// ============================================================================

/// Arguments for [`TodoWriteTool`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoWriteArgs {
    /// The complete updated todo list.
    pub todos: Vec<Todo>,
}

/// Tool for writing/replacing the todo list.
#[derive(Debug, Clone)]
pub struct TodoWriteTool {
    state: TodoState,
}

impl TodoWriteTool {
    /// Creates a new write tool with the given shared state.
    #[inline]
    pub fn new(state: TodoState) -> Self {
        Self { state }
    }
}

impl Tool for TodoWriteTool {
    const NAME: &'static str = "todowrite";

    type Error = ToolError;
    type Args = TodoWriteArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let schema = schema_for!(TodoWriteArgs);
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Replace the entire todo list with a new list of tasks.".to_string(),
            parameters: serde_json::to_value(schema).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate all todos have non-empty id and content
        for todo in &args.todos {
            if todo.id.trim().is_empty() {
                return Err(ToolError::Validation("todo id cannot be empty".into()));
            }
            if todo.content.trim().is_empty() {
                return Err(ToolError::Validation("todo content cannot be empty".into()));
            }
        }

        let count = args.todos.len();
        *self.state.todos.write().await = args.todos;
        Ok(format!("Updated todo list with {count} task(s)."))
    }
}

// ============================================================================
// TodoReadTool
// ============================================================================

/// Arguments for [`TodoReadTool`] (empty).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TodoReadArgs {}

/// Tool for reading the current todo list.
#[derive(Debug, Clone)]
pub struct TodoReadTool {
    state: TodoState,
}

impl TodoReadTool {
    /// Creates a new read tool with the given shared state.
    #[inline]
    pub fn new(state: TodoState) -> Self {
        Self { state }
    }
}

impl Tool for TodoReadTool {
    const NAME: &'static str = "todoread";

    type Error = ToolError;
    type Args = TodoReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let schema = schema_for!(TodoReadArgs);
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Read the current todo list.".to_string(),
            parameters: serde_json::to_value(schema).unwrap_or_default(),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let todos = self.state.todos.read().await;

        if todos.is_empty() {
            return Ok("No tasks.".to_string());
        }

        let mut output = format!("Tasks ({} total):\n", todos.len());
        for todo in todos.iter() {
            let _ = writeln!(
                output,
                "{} ({:?}) {}: {}",
                todo.status.icon(),
                todo.priority,
                todo.id,
                todo.content
            );
        }

        // Remove trailing newline
        output.truncate(output.trim_end().len());
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_todo(id: &str, status: TodoStatus) -> Todo {
        Todo {
            id: id.to_string(),
            content: format!("Task {id}"),
            status,
            priority: TodoPriority::Medium,
        }
    }

    #[tokio::test]
    async fn write_and_read_todos() {
        let state = TodoState::new();
        let write_tool = TodoWriteTool::new(state.clone());
        let read_tool = TodoReadTool::new(state);

        let todos = vec![
            make_todo("1", TodoStatus::Completed),
            make_todo("2", TodoStatus::InProgress),
            make_todo("3", TodoStatus::Pending),
        ];

        let result = write_tool.call(TodoWriteArgs { todos }).await.unwrap();
        assert!(result.contains("3 task(s)"));

        let output = read_tool.call(TodoReadArgs {}).await.unwrap();
        assert!(output.contains("[x]")); // completed
        assert!(output.contains("[>]")); // in_progress
        assert!(output.contains("[ ]")); // pending
    }

    #[tokio::test]
    async fn read_empty_list() {
        let state = TodoState::new();
        let read_tool = TodoReadTool::new(state);
        let output = read_tool.call(TodoReadArgs {}).await.unwrap();
        assert_eq!(output, "No tasks.");
    }

    #[tokio::test]
    async fn write_replaces_existing() {
        let state = TodoState::new();
        let write_tool = TodoWriteTool::new(state.clone());
        let read_tool = TodoReadTool::new(state);

        // First write
        write_tool
            .call(TodoWriteArgs {
                todos: vec![make_todo("a", TodoStatus::Pending)],
            })
            .await
            .unwrap();

        // Second write replaces
        write_tool
            .call(TodoWriteArgs {
                todos: vec![make_todo("b", TodoStatus::Completed)],
            })
            .await
            .unwrap();

        let output = read_tool.call(TodoReadArgs {}).await.unwrap();
        assert!(!output.contains("Task a")); // Check that todo "a" is not present
        assert!(output.contains("Task b")); // Check that todo "b" is present
    }

    #[tokio::test]
    async fn write_validates_empty_id() {
        let state = TodoState::new();
        let write_tool = TodoWriteTool::new(state);
        let todo = Todo {
            id: "".to_string(),
            content: "Task".to_string(),
            status: TodoStatus::Pending,
            priority: TodoPriority::Low,
        };
        let result = write_tool.call(TodoWriteArgs { todos: vec![todo] }).await;
        assert!(matches!(result, Err(ToolError::Validation(_))));
    }

    #[tokio::test]
    async fn write_validates_empty_content() {
        let state = TodoState::new();
        let write_tool = TodoWriteTool::new(state);
        let todo = Todo {
            id: "1".to_string(),
            content: "  ".to_string(),
            status: TodoStatus::Pending,
            priority: TodoPriority::Low,
        };
        let result = write_tool.call(TodoWriteArgs { todos: vec![todo] }).await;
        assert!(matches!(result, Err(ToolError::Validation(_))));
    }

    #[test]
    fn status_icons_are_correct() {
        assert_eq!(TodoStatus::Pending.icon(), "[ ]");
        assert_eq!(TodoStatus::InProgress.icon(), "[>]");
        assert_eq!(TodoStatus::Completed.icon(), "[x]");
        assert_eq!(TodoStatus::Cancelled.icon(), "[-]");
    }

    #[test]
    fn status_serde_roundtrip() {
        let json = serde_json::to_string(&TodoStatus::InProgress).unwrap();
        assert_eq!(json, "\"in_progress\"");
        let parsed: TodoStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, TodoStatus::InProgress);
    }

    #[test]
    fn priority_serde_roundtrip() {
        let json = serde_json::to_string(&TodoPriority::High).unwrap();
        assert_eq!(json, "\"high\"");
        let parsed: TodoPriority = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, TodoPriority::High);
    }
}
