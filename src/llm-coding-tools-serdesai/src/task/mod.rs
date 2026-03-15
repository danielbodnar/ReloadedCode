//! Adapter-facing Task helpers for SerdesAI.
//!
//! # Public API
//! - [`task_tool_definition`] - Builds the Task definition and schema.
//! - [`render_task_targets`] - Renders callable targets for Task descriptions.
//! - [`build_delegated_message`] - Builds the stateless delegated prompt body.

mod definition;
mod message;

pub use definition::{render_task_targets, task_tool_definition};
pub use message::build_delegated_message;
