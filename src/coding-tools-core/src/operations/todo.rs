//! Todo list management operation.

use crate::error::{ToolError, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Task status.
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

/// Writes/replaces the todo list with new items.
///
/// Validates that all todos have non-empty id and content.
pub async fn write_todos(state: &TodoState, todos: Vec<Todo>) -> ToolResult<String> {
    for todo in &todos {
        if todo.id.trim().is_empty() {
            return Err(ToolError::Validation("todo id cannot be empty".into()));
        }
        if todo.content.trim().is_empty() {
            return Err(ToolError::Validation("todo content cannot be empty".into()));
        }
    }

    let count = todos.len();
    *state.todos.write().await = todos;
    Ok(format!("Updated todo list with {count} task(s)."))
}

/// Reads and formats the current todo list.
pub async fn read_todos(state: &TodoState) -> String {
    let todos = state.todos.read().await;

    if todos.is_empty() {
        return "No tasks.".to_string();
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

    output.truncate(output.trim_end().len());
    output
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

        let todos = vec![
            make_todo("1", TodoStatus::Completed),
            make_todo("2", TodoStatus::InProgress),
            make_todo("3", TodoStatus::Pending),
        ];

        let result = write_todos(&state, todos).await.unwrap();
        assert!(result.contains("3 task(s)"));

        let output = read_todos(&state).await;
        assert!(output.contains("[x]"));
        assert!(output.contains("[>]"));
        assert!(output.contains("[ ]"));
    }

    #[tokio::test]
    async fn read_empty_list() {
        let state = TodoState::new();
        let output = read_todos(&state).await;
        assert_eq!(output, "No tasks.");
    }

    #[tokio::test]
    async fn write_replaces_existing() {
        let state = TodoState::new();

        write_todos(&state, vec![make_todo("a", TodoStatus::Pending)])
            .await
            .unwrap();
        write_todos(&state, vec![make_todo("b", TodoStatus::Completed)])
            .await
            .unwrap();

        let output = read_todos(&state).await;
        assert!(!output.contains("Task a"));
        assert!(output.contains("Task b"));
    }

    #[tokio::test]
    async fn write_validates_empty_id() {
        let state = TodoState::new();
        let todo = Todo {
            id: "".to_string(),
            content: "Task".to_string(),
            status: TodoStatus::Pending,
            priority: TodoPriority::Low,
        };
        let result = write_todos(&state, vec![todo]).await;
        assert!(matches!(result, Err(ToolError::Validation(_))));
    }

    #[tokio::test]
    async fn write_validates_empty_content() {
        let state = TodoState::new();
        let todo = Todo {
            id: "1".to_string(),
            content: "  ".to_string(),
            status: TodoStatus::Pending,
            priority: TodoPriority::Low,
        };
        let result = write_todos(&state, vec![todo]).await;
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
}
