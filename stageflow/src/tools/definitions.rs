//! Tool definitions and I/O types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Definition of a tool that can be executed.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// The tool name.
    pub name: String,
    /// The action type for matching.
    pub action_type: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON Schema for input validation.
    pub input_schema: serde_json::Value,
    /// Allowed behaviors (empty = all allowed).
    pub allowed_behaviors: Vec<String>,
    /// Whether approval is required.
    pub requires_approval: bool,
    /// Custom approval message.
    pub approval_message: Option<String>,
    /// Whether the tool supports undo.
    pub undoable: bool,
    /// Artifact type produced by the tool.
    pub artifact_type: Option<String>,
}

impl ToolDefinition {
    /// Creates a new tool definition.
    #[must_use]
    pub fn new(name: impl Into<String>, action_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            action_type: action_type.into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
            allowed_behaviors: Vec::new(),
            requires_approval: false,
            approval_message: None,
            undoable: false,
            artifact_type: None,
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Sets the input schema.
    #[must_use]
    pub fn with_input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Sets allowed behaviors.
    #[must_use]
    pub fn with_allowed_behaviors(mut self, behaviors: Vec<String>) -> Self {
        self.allowed_behaviors = behaviors;
        self
    }

    /// Sets approval requirement.
    #[must_use]
    pub fn requires_approval_with_message(mut self, message: impl Into<String>) -> Self {
        self.requires_approval = true;
        self.approval_message = Some(message.into());
        self
    }

    /// Marks the tool as undoable.
    #[must_use]
    pub fn undoable(mut self) -> Self {
        self.undoable = true;
        self
    }

    /// Checks if a behavior is allowed.
    #[must_use]
    pub fn is_behavior_allowed(&self, behavior: &str) -> bool {
        self.allowed_behaviors.is_empty() || self.allowed_behaviors.contains(&behavior.to_string())
    }
}

/// Input to a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// The action ID.
    pub action_id: Uuid,
    /// The tool name.
    pub tool_name: String,
    /// The input payload.
    pub payload: serde_json::Value,
    /// The execution behavior/mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<String>,
    /// The pipeline run ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_run_id: Option<Uuid>,
    /// The request ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
}

impl ToolInput {
    /// Creates a new tool input.
    #[must_use]
    pub fn new(tool_name: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            action_id: Uuid::new_v4(),
            tool_name: tool_name.into(),
            payload,
            behavior: None,
            pipeline_run_id: None,
            request_id: None,
        }
    }

    /// Creates input from an action with context.
    #[must_use]
    pub fn from_action(
        action_id: Uuid,
        tool_name: impl Into<String>,
        payload: serde_json::Value,
        execution_mode: Option<String>,
        pipeline_run_id: Option<Uuid>,
        request_id: Option<Uuid>,
    ) -> Self {
        Self {
            action_id,
            tool_name: tool_name.into(),
            payload,
            behavior: execution_mode,
            pipeline_run_id,
            request_id,
        }
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("action_id".to_string(), serde_json::json!(self.action_id.to_string()));
        map.insert("tool_name".to_string(), serde_json::json!(self.tool_name));
        map.insert("payload".to_string(), self.payload.clone());

        if let Some(ref behavior) = self.behavior {
            map.insert("behavior".to_string(), serde_json::json!(behavior));
        }
        if let Some(id) = self.pipeline_run_id {
            map.insert("pipeline_run_id".to_string(), serde_json::json!(id.to_string()));
        }
        if let Some(id) = self.request_id {
            map.insert("request_id".to_string(), serde_json::json!(id.to_string()));
        }

        map
    }
}

/// Output from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Whether the execution succeeded.
    pub success: bool,
    /// The output data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Artifacts produced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<serde_json::Value>,
    /// Undo metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_metadata: Option<serde_json::Value>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolOutput {
    /// Creates a successful output.
    #[must_use]
    pub fn ok(data: Option<serde_json::Value>) -> Self {
        Self {
            success: true,
            data,
            artifacts: Vec::new(),
            undo_metadata: None,
            error: None,
        }
    }

    /// Creates a successful output with artifacts.
    #[must_use]
    pub fn ok_with_artifacts(data: Option<serde_json::Value>, artifacts: Vec<serde_json::Value>) -> Self {
        Self {
            success: true,
            data,
            artifacts,
            undo_metadata: None,
            error: None,
        }
    }

    /// Creates a successful output with undo metadata.
    #[must_use]
    pub fn ok_with_undo(data: Option<serde_json::Value>, undo_metadata: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            artifacts: Vec::new(),
            undo_metadata: Some(undo_metadata),
            error: None,
        }
    }

    /// Creates a failure output.
    #[must_use]
    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            artifacts: Vec::new(),
            undo_metadata: None,
            error: Some(error.into()),
        }
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("success".to_string(), serde_json::json!(self.success));

        if let Some(ref data) = self.data {
            map.insert("data".to_string(), data.clone());
        }
        if !self.artifacts.is_empty() {
            map.insert("artifacts".to_string(), serde_json::json!(self.artifacts));
        }
        if let Some(ref undo) = self.undo_metadata {
            map.insert("undo_metadata".to_string(), undo.clone());
        }
        if let Some(ref error) = self.error {
            map.insert("error".to_string(), serde_json::json!(error));
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_creation() {
        let def = ToolDefinition::new("my_tool", "my_action")
            .with_description("Does things");

        assert_eq!(def.name, "my_tool");
        assert_eq!(def.action_type, "my_action");
        assert!(!def.requires_approval);
    }

    #[test]
    fn test_behavior_allowed_empty() {
        let def = ToolDefinition::new("tool", "action");
        // Empty means all allowed
        assert!(def.is_behavior_allowed("anything"));
    }

    #[test]
    fn test_behavior_allowed_restricted() {
        let def = ToolDefinition::new("tool", "action")
            .with_allowed_behaviors(vec!["production".to_string()]);

        assert!(def.is_behavior_allowed("production"));
        assert!(!def.is_behavior_allowed("development"));
    }

    #[test]
    fn test_tool_input_creation() {
        let input = ToolInput::new("my_tool", serde_json::json!({"arg": "value"}));

        assert_eq!(input.tool_name, "my_tool");
        assert!(input.behavior.is_none());
    }

    #[test]
    fn test_tool_input_to_dict() {
        let input = ToolInput::from_action(
            Uuid::new_v4(),
            "tool",
            serde_json::json!({}),
            Some("production".to_string()),
            Some(Uuid::new_v4()),
            None,
        );

        let dict = input.to_dict();
        assert!(dict.contains_key("action_id"));
        assert!(dict.contains_key("behavior"));
        assert!(dict.contains_key("pipeline_run_id"));
        assert!(!dict.contains_key("request_id")); // None values excluded
    }

    #[test]
    fn test_tool_output_ok() {
        let output = ToolOutput::ok(Some(serde_json::json!({"result": 42})));

        assert!(output.success);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_tool_output_fail() {
        let output = ToolOutput::fail("Something went wrong");

        assert!(!output.success);
        assert_eq!(output.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_tool_output_to_dict() {
        let output = ToolOutput::ok(Some(serde_json::json!({"x": 1})));
        let dict = output.to_dict();

        assert_eq!(dict.get("success"), Some(&serde_json::json!(true)));
        assert!(dict.contains_key("data"));
        assert!(!dict.contains_key("error"));
    }
}
