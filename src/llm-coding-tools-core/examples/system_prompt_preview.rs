//! System prompt preview - demonstrates full system prompt generation.
//!
//! Shows how SystemPromptBuilder combines:
//! - Custom system prompts
//! - Environment section (working directory, allowed paths)
//! - Tool usage guidelines (from tracked tools)
//! - Supplemental context (git workflow, GitHub CLI)
//! - Provider-facing tool metadata that also consumes input tokens
//!
//! Run: cargo run --example system_prompt_preview -p llm-coding-tools-core

use llm_coding_tools_core::context::ToolContext;
use llm_coding_tools_core::{context, tool_metadata, AllowedPathResolver, SystemPromptBuilder};
use serde_json::{json, Map, Value};

fn main() {
    // Use from_canonical to avoid filesystem requirements for the example.
    // In real usage, AllowedPathResolver::new() canonicalizes and
    // validates paths.
    let resolver = AllowedPathResolver::from_canonical(["/home/user/project", "/tmp"]);

    // Build system prompt with all features demonstrated
    let mut pb = SystemPromptBuilder::new()
        .system_prompt(
            "# System Instructions\n\n\
             You are a helpful coding assistant. Follow best practices and \
             write clean, maintainable code.",
        )
        .working_directory("/home/user/project")
        .allowed_paths(&resolver)
        .add_context("Git Workflow", context::GIT_WORKFLOW)
        .add_context("GitHub CLI", context::GITHUB_CLI);

    // Track tools - in real usage this would be:
    //   .tool(pb.track(ReadTool::new()))
    // For the preview, we just register them without using the returned tool.
    let _ = pb.track(MockReadTool);
    let _ = pb.track(MockWriteTool);
    let _ = pb.track(MockEditTool);
    let _ = pb.track(MockBashTool);
    let _ = pb.track(MockGlobTool);
    let _ = pb.track(MockGrepTool);
    let _ = pb.track(MockWebFetchTool);
    let _ = pb.track(MockTodoWriteTool);
    let _ = pb.track(MockTodoReadTool);

    let system_prompt = pb.build();
    let tool_contexts = tracked_tool_contexts();
    let tool_definitions = tracked_tool_definitions();
    let tool_definition_payload = serde_json::to_string(&tool_definitions).unwrap();

    println!("{system_prompt}");

    println!("\n{}", "=".repeat(60));
    println!("Static request footprint:");
    println!(
        "  System prompt: {} chars, {} lines, ~{} tokens",
        system_prompt.len(),
        system_prompt.lines().count(),
        estimate_tokens(system_prompt.len())
    );
    println!(
        "  Tool definitions: {} chars, {} tools, ~{} tokens",
        tool_definition_payload.len(),
        tool_definitions.len(),
        estimate_tokens(tool_definition_payload.len())
    );

    let total_chars = system_prompt.len() + tool_definition_payload.len();
    println!(
        "  Combined static input: {} chars, ~{} tokens",
        total_chars,
        estimate_tokens(total_chars)
    );
    println!(
        "  Note: provider wrappers and user messages add extra overhead beyond these static counts."
    );

    let mut context_sizes: Vec<_> = tool_contexts
        .iter()
        .map(|(name, text)| (*name, text.len()))
        .collect();
    context_sizes
        .sort_unstable_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));

    println!("\nLargest tool context sections:");
    for (name, chars) in context_sizes {
        println!(
            "  {name}: {chars} chars (~{} tokens)",
            estimate_tokens(chars)
        );
    }

    let mut definition_sizes: Vec<_> = tool_definitions
        .iter()
        .map(|tool| {
            let name = tool["name"].as_str().unwrap_or("unknown");
            let chars = serde_json::to_string(tool).unwrap().len();
            (name.to_string(), chars)
        })
        .collect();
    definition_sizes
        .sort_unstable_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    println!("\nLargest tool definitions:");
    for (name, chars) in definition_sizes {
        println!(
            "  {name}: {chars} chars (~{} tokens)",
            estimate_tokens(chars)
        );
    }
}

fn estimate_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
}

fn tracked_tool_contexts() -> [(&'static str, &'static str); 9] {
    [
        (tool_metadata::read::NAME, context::READ_ALLOWED),
        (tool_metadata::write::NAME, context::WRITE_ALLOWED),
        (tool_metadata::edit::NAME, context::EDIT_ALLOWED),
        (tool_metadata::bash::NAME, context::BASH),
        (tool_metadata::glob::NAME, context::GLOB_ALLOWED),
        (tool_metadata::grep::NAME, context::GREP_ALLOWED),
        (tool_metadata::webfetch::NAME, context::WEBFETCH),
        (tool_metadata::todo_write::NAME, context::TODO_WRITE),
        (tool_metadata::todo_read::NAME, context::TODO_READ),
    ]
}

fn tracked_tool_definitions() -> Vec<Value> {
    vec![
        tool_definition(
            tool_metadata::read::NAME,
            tool_metadata::read::description::allowed(true),
            object_schema(
                vec![
                    (
                        tool_metadata::read::param::FILE_PATH_ALLOWED.name,
                        string_schema(tool_metadata::read::param::FILE_PATH_ALLOWED.description),
                    ),
                    (
                        tool_metadata::read::param::OFFSET.name,
                        integer_schema(
                            tool_metadata::read::param::OFFSET.description,
                            Some(1),
                            None,
                        ),
                    ),
                    (
                        tool_metadata::read::param::LIMIT.name,
                        integer_schema(
                            tool_metadata::read::param::LIMIT.description,
                            Some(1),
                            None,
                        ),
                    ),
                ],
                &[tool_metadata::read::param::FILE_PATH_ALLOWED.name],
            ),
        ),
        tool_definition(
            tool_metadata::write::NAME,
            tool_metadata::write::description::ALLOWED,
            object_schema(
                vec![
                    (
                        tool_metadata::write::param::FILE_PATH_ALLOWED.name,
                        string_schema(tool_metadata::write::param::FILE_PATH_ALLOWED.description),
                    ),
                    (
                        tool_metadata::write::param::CONTENT.name,
                        string_schema(tool_metadata::write::param::CONTENT.description),
                    ),
                ],
                &[
                    tool_metadata::write::param::FILE_PATH_ALLOWED.name,
                    tool_metadata::write::param::CONTENT.name,
                ],
            ),
        ),
        tool_definition(
            tool_metadata::edit::NAME,
            tool_metadata::edit::description::ALLOWED,
            object_schema(
                vec![
                    (
                        tool_metadata::edit::param::FILE_PATH_ALLOWED.name,
                        string_schema(tool_metadata::edit::param::FILE_PATH_ALLOWED.description),
                    ),
                    (
                        tool_metadata::edit::param::OLD_STRING.name,
                        string_schema(tool_metadata::edit::param::OLD_STRING.description),
                    ),
                    (
                        tool_metadata::edit::param::NEW_STRING.name,
                        string_schema(tool_metadata::edit::param::NEW_STRING.description),
                    ),
                    (
                        tool_metadata::edit::param::REPLACE_ALL.name,
                        boolean_schema(tool_metadata::edit::param::REPLACE_ALL.description),
                    ),
                ],
                &[
                    tool_metadata::edit::param::FILE_PATH_ALLOWED.name,
                    tool_metadata::edit::param::OLD_STRING.name,
                    tool_metadata::edit::param::NEW_STRING.name,
                ],
            ),
        ),
        tool_definition(
            tool_metadata::bash::NAME,
            tool_metadata::bash::DESCRIPTION,
            object_schema(
                vec![
                    (
                        tool_metadata::bash::param::COMMAND.name,
                        string_schema_constrained(
                            tool_metadata::bash::param::COMMAND.description,
                            Some(1),
                            None,
                        ),
                    ),
                    (
                        tool_metadata::bash::param::WORKDIR.name,
                        string_schema(tool_metadata::bash::param::WORKDIR.description),
                    ),
                    (
                        tool_metadata::bash::param::TIMEOUT_MS.name,
                        integer_schema(
                            tool_metadata::bash::param::TIMEOUT_MS.description,
                            Some(1),
                            Some(tool_metadata::bash::MAX_TIMEOUT_MS as i64),
                        ),
                    ),
                ],
                &[tool_metadata::bash::param::COMMAND.name],
            ),
        ),
        tool_definition(
            tool_metadata::glob::NAME,
            tool_metadata::glob::description::ALLOWED,
            object_schema(
                vec![
                    (
                        tool_metadata::glob::param::PATTERN.name,
                        string_schema(tool_metadata::glob::param::PATTERN.description),
                    ),
                    (
                        tool_metadata::glob::param::PATH_ALLOWED.name,
                        string_schema(tool_metadata::glob::param::PATH_ALLOWED.description),
                    ),
                ],
                &[
                    tool_metadata::glob::param::PATTERN.name,
                    tool_metadata::glob::param::PATH_ALLOWED.name,
                ],
            ),
        ),
        tool_definition(
            tool_metadata::grep::NAME,
            tool_metadata::grep::description::allowed(true),
            object_schema(
                vec![
                    (
                        tool_metadata::grep::param::PATTERN.name,
                        string_schema(tool_metadata::grep::param::PATTERN.description),
                    ),
                    (
                        tool_metadata::grep::param::PATH_ALLOWED.name,
                        string_schema(tool_metadata::grep::param::PATH_ALLOWED.description),
                    ),
                    (
                        tool_metadata::grep::param::INCLUDE.name,
                        string_schema(tool_metadata::grep::param::INCLUDE.description),
                    ),
                    (
                        tool_metadata::grep::param::LIMIT.name,
                        integer_schema(
                            tool_metadata::grep::param::LIMIT.description,
                            Some(1),
                            Some(tool_metadata::grep::MAX_LIMIT as i64),
                        ),
                    ),
                ],
                &[
                    tool_metadata::grep::param::PATTERN.name,
                    tool_metadata::grep::param::PATH_ALLOWED.name,
                ],
            ),
        ),
        tool_definition(
            tool_metadata::webfetch::NAME,
            tool_metadata::webfetch::DESCRIPTION,
            object_schema(
                vec![
                    (
                        tool_metadata::webfetch::param::URL.name,
                        string_schema(tool_metadata::webfetch::param::URL.description),
                    ),
                    (
                        tool_metadata::webfetch::param::TIMEOUT_MS.name,
                        integer_schema(
                            tool_metadata::webfetch::param::TIMEOUT_MS.description,
                            Some(1),
                            Some(tool_metadata::webfetch::MAX_TIMEOUT_MS as i64),
                        ),
                    ),
                ],
                &[tool_metadata::webfetch::param::URL.name],
            ),
        ),
        tool_definition(
            tool_metadata::todo_write::NAME,
            tool_metadata::todo_write::DESCRIPTION,
            object_schema(
                vec![(
                    tool_metadata::todo_write::param::TODOS.name,
                    json!({
                        "type": "array",
                        "description": tool_metadata::todo_write::param::TODOS.description,
                        "items": {
                            "type": "object",
                            "properties": {
                                tool_metadata::todo_write::param::ID.name: string_schema(
                                    tool_metadata::todo_write::param::ID.description,
                                ),
                                tool_metadata::todo_write::param::CONTENT.name: string_schema(
                                    tool_metadata::todo_write::param::CONTENT.description,
                                ),
                                tool_metadata::todo_write::param::STATUS.name: enum_schema(
                                    tool_metadata::todo_write::param::STATUS.description,
                                    &["pending", "in_progress", "completed", "cancelled"],
                                ),
                                tool_metadata::todo_write::param::PRIORITY.name: enum_schema(
                                    tool_metadata::todo_write::param::PRIORITY.description,
                                    &["high", "medium", "low"],
                                ),
                            },
                            "required": [
                                tool_metadata::todo_write::param::ID.name,
                                tool_metadata::todo_write::param::CONTENT.name,
                                tool_metadata::todo_write::param::STATUS.name,
                                tool_metadata::todo_write::param::PRIORITY.name,
                            ],
                        },
                    }),
                )],
                &[tool_metadata::todo_write::param::TODOS.name],
            ),
        ),
        tool_definition(
            tool_metadata::todo_read::NAME,
            tool_metadata::todo_read::DESCRIPTION,
            object_schema(Vec::new(), &[]),
        ),
    ]
}

fn tool_definition(name: &'static str, description: &'static str, parameters: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "parameters": parameters,
    })
}

fn object_schema(properties: Vec<(&'static str, Value)>, required: &[&'static str]) -> Value {
    let mut props = Map::with_capacity(properties.len());
    for (name, schema) in properties {
        props.insert(name.to_string(), schema);
    }

    let mut object = Map::with_capacity(3);
    object.insert("type".to_string(), Value::String("object".to_string()));
    object.insert("properties".to_string(), Value::Object(props));
    if !required.is_empty() {
        object.insert(
            "required".to_string(),
            Value::Array(
                required
                    .iter()
                    .map(|name| Value::String((*name).to_string()))
                    .collect(),
            ),
        );
    }

    Value::Object(object)
}

fn string_schema(description: &'static str) -> Value {
    json!({
        "type": "string",
        "description": description,
    })
}

fn string_schema_constrained(
    description: &'static str,
    min_length: Option<usize>,
    max_length: Option<usize>,
) -> Value {
    let mut schema = Map::with_capacity(4);
    schema.insert("type".to_string(), Value::String("string".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    if let Some(min) = min_length {
        schema.insert("minLength".to_string(), Value::from(min));
    }
    if let Some(max) = max_length {
        schema.insert("maxLength".to_string(), Value::from(max));
    }
    Value::Object(schema)
}

fn integer_schema(description: &'static str, minimum: Option<i64>, maximum: Option<i64>) -> Value {
    let mut schema = Map::with_capacity(4);
    schema.insert("type".to_string(), Value::String("integer".to_string()));
    schema.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    if let Some(min) = minimum {
        schema.insert("minimum".to_string(), Value::from(min));
    }
    if let Some(max) = maximum {
        schema.insert("maximum".to_string(), Value::from(max));
    }
    Value::Object(schema)
}

fn boolean_schema(description: &'static str) -> Value {
    json!({
        "type": "boolean",
        "description": description,
    })
}

fn enum_schema(description: &'static str, values: &[&'static str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values,
    })
}

// Mock tools implementing ToolContext for demonstration.
// In real usage, these would be actual tool structs from
// llm-coding-tools-serdesai.

struct MockReadTool;
impl ToolContext for MockReadTool {
    const NAME: &'static str = tool_metadata::read::NAME;
    fn context(&self) -> &'static str {
        context::READ_ALLOWED
    }
}

struct MockWriteTool;
impl ToolContext for MockWriteTool {
    const NAME: &'static str = tool_metadata::write::NAME;
    fn context(&self) -> &'static str {
        context::WRITE_ALLOWED
    }
}

struct MockEditTool;
impl ToolContext for MockEditTool {
    const NAME: &'static str = tool_metadata::edit::NAME;
    fn context(&self) -> &'static str {
        context::EDIT_ALLOWED
    }
}

struct MockBashTool;
impl ToolContext for MockBashTool {
    const NAME: &'static str = tool_metadata::bash::NAME;
    fn context(&self) -> &'static str {
        context::BASH
    }
}

struct MockGlobTool;
impl ToolContext for MockGlobTool {
    const NAME: &'static str = tool_metadata::glob::NAME;
    fn context(&self) -> &'static str {
        context::GLOB_ALLOWED
    }
}

struct MockGrepTool;
impl ToolContext for MockGrepTool {
    const NAME: &'static str = tool_metadata::grep::NAME;
    fn context(&self) -> &'static str {
        context::GREP_ALLOWED
    }
}

struct MockWebFetchTool;
impl ToolContext for MockWebFetchTool {
    const NAME: &'static str = tool_metadata::webfetch::NAME;
    fn context(&self) -> &'static str {
        context::WEBFETCH
    }
}

struct MockTodoWriteTool;
impl ToolContext for MockTodoWriteTool {
    const NAME: &'static str = tool_metadata::todo_write::NAME;
    fn context(&self) -> &'static str {
        context::TODO_WRITE
    }
}

struct MockTodoReadTool;
impl ToolContext for MockTodoReadTool {
    const NAME: &'static str = tool_metadata::todo_read::NAME;
    fn context(&self) -> &'static str {
        context::TODO_READ
    }
}
