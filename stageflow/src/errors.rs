//! Error types for the stageflow framework.
//!
//! This module provides a comprehensive error taxonomy matching the Python
//! implementation's error types and behaviors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// The main error type for stageflow operations.
#[derive(Debug, Error)]
pub enum StageflowError {
    /// A pipeline validation error occurred.
    #[error("{0}")]
    Validation(#[from] PipelineValidationError),

    /// A data conflict occurred in a context bag.
    #[error("{0}")]
    DataConflict(#[from] DataConflictError),

    /// An output conflict occurred in an output bag.
    #[error("{0}")]
    OutputConflict(#[from] OutputConflictError),

    /// An undeclared dependency was accessed.
    #[error("{0}")]
    UndeclaredDependency(#[from] UndeclaredDependencyError),

    /// A cycle was detected in the pipeline.
    #[error("{0}")]
    CycleDetected(#[from] CycleDetectedError),

    /// A stage execution error.
    #[error("Stage execution error: {0}")]
    StageExecution(String),

    /// A cancellation occurred.
    #[error("Pipeline cancelled: {0}")]
    Cancelled(String),

    /// A tool-related error.
    #[error("{0}")]
    Tool(#[from] ToolError),

    /// A generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Metadata about a contract error for better diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractErrorInfo {
    /// Error code (e.g., "CONTRACT-004-CYCLE").
    pub code: String,
    /// Short summary of the error.
    pub summary: String,
    /// Hint for fixing the error.
    pub fix_hint: Option<String>,
    /// URL to documentation.
    pub doc_url: Option<String>,
    /// Additional context key-value pairs.
    #[serde(default)]
    pub context: HashMap<String, String>,
}

impl ContractErrorInfo {
    /// Creates a new contract error info.
    #[must_use]
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
            fix_hint: None,
            doc_url: None,
            context: HashMap::new(),
        }
    }

    /// Sets the fix hint.
    #[must_use]
    pub fn with_fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }

    /// Sets the documentation URL.
    #[must_use]
    pub fn with_doc_url(mut self, url: impl Into<String>) -> Self {
        self.doc_url = Some(url.into());
        self
    }

    /// Adds context key-value pairs.
    #[must_use]
    pub fn with_context(mut self, context: HashMap<String, String>) -> Self {
        self.context.extend(context);
        self
    }

    /// Adds a single context entry.
    #[must_use]
    pub fn with_context_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("code".to_string(), serde_json::Value::String(self.code.clone()));
        map.insert("summary".to_string(), serde_json::Value::String(self.summary.clone()));
        
        if let Some(ref hint) = self.fix_hint {
            map.insert("fix_hint".to_string(), serde_json::Value::String(hint.clone()));
        }
        if let Some(ref url) = self.doc_url {
            map.insert("doc_url".to_string(), serde_json::Value::String(url.clone()));
        }
        if !self.context.is_empty() {
            let context_map: serde_json::Map<String, serde_json::Value> = self
                .context
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            map.insert("context".to_string(), serde_json::Value::Object(context_map));
        }
        
        map
    }
}

/// Error raised when pipeline validation fails.
#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct PipelineValidationError {
    /// The error message.
    pub message: String,
    /// The stages involved in the error.
    pub stages: Vec<String>,
    /// Optional contract error info.
    pub error_info: Option<ContractErrorInfo>,
}

impl PipelineValidationError {
    /// Creates a new pipeline validation error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            stages: Vec::new(),
            error_info: None,
        }
    }

    /// Sets the stages involved.
    #[must_use]
    pub fn with_stages(mut self, stages: Vec<String>) -> Self {
        self.stages = stages;
        self
    }

    /// Sets the contract error info.
    #[must_use]
    pub fn with_error_info(mut self, info: ContractErrorInfo) -> Self {
        self.error_info = Some(info);
        self
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("message".to_string(), serde_json::Value::String(self.message.clone()));
        map.insert(
            "stages".to_string(),
            serde_json::Value::Array(
                self.stages
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
        if let Some(ref info) = self.error_info {
            let info_map: serde_json::Map<String, serde_json::Value> =
                info.to_dict().into_iter().collect();
            map.insert("error_info".to_string(), serde_json::Value::Object(info_map));
        }
        map
    }
}

/// Error raised when a cycle is detected in the pipeline graph.
#[derive(Debug, Clone, Error)]
#[error("Cycle detected in pipeline: {}", cycle_path.join(" -> "))]
pub struct CycleDetectedError {
    /// The path of stages forming the cycle.
    pub cycle_path: Vec<String>,
    /// Contract error info.
    pub error_info: ContractErrorInfo,
}

impl CycleDetectedError {
    /// Creates a new cycle detected error.
    #[must_use]
    pub fn new(cycle_path: Vec<String>) -> Self {
        let info = ContractErrorInfo::new(
            "CONTRACT-004-CYCLE",
            format!("Pipeline contains a dependency cycle: {}", cycle_path.join(" -> ")),
        )
        .with_fix_hint("Remove one of the dependencies in the cycle to break it.");

        Self {
            cycle_path,
            error_info: info,
        }
    }
}

impl From<CycleDetectedError> for PipelineValidationError {
    fn from(err: CycleDetectedError) -> Self {
        PipelineValidationError {
            message: err.to_string(),
            stages: err.cycle_path.clone(),
            error_info: Some(err.error_info),
        }
    }
}

/// Error raised when writing to an existing key in a context bag.
#[derive(Debug, Clone, Error)]
#[error("Data conflict: key '{key}' already exists")]
pub struct DataConflictError {
    /// The conflicting key.
    pub key: String,
}

impl DataConflictError {
    /// Creates a new data conflict error.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// Error raised when writing to an existing output in an output bag.
#[derive(Debug, Clone, Error)]
#[error("Output conflict for stage '{stage}': {message}")]
pub struct OutputConflictError {
    /// The stage name.
    pub stage: String,
    /// Additional message.
    pub message: String,
}

impl OutputConflictError {
    /// Creates a new output conflict error.
    #[must_use]
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
        }
    }
}

/// Error raised when accessing an undeclared dependency.
#[derive(Debug, Clone, Error)]
#[error("Undeclared dependency: stage '{stage}' attempted to access '{key}' which was not declared as a dependency")]
pub struct UndeclaredDependencyError {
    /// The stage attempting access.
    pub stage: String,
    /// The undeclared key.
    pub key: String,
}

impl UndeclaredDependencyError {
    /// Creates a new undeclared dependency error.
    #[must_use]
    pub fn new(stage: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            key: key.into(),
        }
    }
}

/// Errors related to tool execution.
#[derive(Debug, Clone, Error)]
pub enum ToolError {
    /// Tool was not found in the registry.
    #[error("Tool not found: {name}")]
    NotFound {
        /// The tool name.
        name: String,
    },

    /// Tool execution was denied due to behavior gating.
    #[error("Tool denied: {name} - {reason}")]
    Denied {
        /// The tool name.
        name: String,
        /// The reason for denial.
        reason: String,
    },

    /// Tool approval was denied by the user.
    #[error("Approval denied for tool: {name}")]
    ApprovalDenied {
        /// The tool name.
        name: String,
    },

    /// Tool approval timed out.
    #[error("Approval timeout for tool: {name} (request_id: {request_id}, timeout: {timeout_seconds}s)")]
    ApprovalTimeout {
        /// The tool name.
        name: String,
        /// The approval request ID.
        request_id: String,
        /// The timeout in seconds.
        timeout_seconds: f64,
    },

    /// Tool undo failed.
    #[error("Undo failed for tool: {name} - {reason}")]
    UndoFailed {
        /// The tool name.
        name: String,
        /// The reason for failure.
        reason: String,
    },

    /// Tool execution failed.
    #[error("Tool execution failed: {name} - {reason}")]
    ExecutionFailed {
        /// The tool name.
        name: String,
        /// The reason for failure.
        reason: String,
    },
}

impl ToolError {
    /// Creates a tool not found error.
    #[must_use]
    pub fn not_found(name: impl Into<String>) -> Self {
        Self::NotFound { name: name.into() }
    }

    /// Creates a tool denied error.
    #[must_use]
    pub fn denied(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Denied {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Creates an approval denied error.
    #[must_use]
    pub fn approval_denied(name: impl Into<String>) -> Self {
        Self::ApprovalDenied { name: name.into() }
    }

    /// Creates an approval timeout error.
    #[must_use]
    pub fn approval_timeout(
        name: impl Into<String>,
        request_id: impl Into<String>,
        timeout_seconds: f64,
    ) -> Self {
        Self::ApprovalTimeout {
            name: name.into(),
            request_id: request_id.into(),
            timeout_seconds,
        }
    }

    /// Creates an undo failed error.
    #[must_use]
    pub fn undo_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::UndoFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Creates an execution failed error.
    #[must_use]
    pub fn execution_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Converts to a dictionary representation.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        
        match self {
            Self::NotFound { name } => {
                map.insert("type".to_string(), serde_json::json!("ToolNotFound"));
                map.insert("name".to_string(), serde_json::json!(name));
            }
            Self::Denied { name, reason } => {
                map.insert("type".to_string(), serde_json::json!("ToolDenied"));
                map.insert("name".to_string(), serde_json::json!(name));
                map.insert("reason".to_string(), serde_json::json!(reason));
            }
            Self::ApprovalDenied { name } => {
                map.insert("type".to_string(), serde_json::json!("ToolApprovalDenied"));
                map.insert("name".to_string(), serde_json::json!(name));
            }
            Self::ApprovalTimeout { name, request_id, timeout_seconds } => {
                map.insert("type".to_string(), serde_json::json!("ToolApprovalTimeout"));
                map.insert("name".to_string(), serde_json::json!(name));
                map.insert("request_id".to_string(), serde_json::json!(request_id));
                map.insert("timeout_seconds".to_string(), serde_json::json!(timeout_seconds));
            }
            Self::UndoFailed { name, reason } => {
                map.insert("type".to_string(), serde_json::json!("ToolUndoError"));
                map.insert("name".to_string(), serde_json::json!(name));
                map.insert("reason".to_string(), serde_json::json!(reason));
            }
            Self::ExecutionFailed { name, reason } => {
                map.insert("type".to_string(), serde_json::json!("ToolExecutionError"));
                map.insert("name".to_string(), serde_json::json!(name));
                map.insert("reason".to_string(), serde_json::json!(reason));
            }
        }
        
        map.insert("message".to_string(), serde_json::json!(self.to_string()));
        map
    }
}

/// Provides default suggestions for common contract error codes.
pub struct ContractSuggestions;

impl ContractSuggestions {
    /// Gets a suggestion for a given error code.
    #[must_use]
    pub fn get(code: &str) -> Option<&'static str> {
        match code {
            "CONTRACT-004-CYCLE" => Some(
                "Check your stage dependencies for circular references. \
                 Use a linear chain or fan-out pattern instead.",
            ),
            "CONTRACT-004-MISSING_DEP" => Some(
                "Ensure all dependencies reference stages that exist in the pipeline. \
                 Check for typos in stage names.",
            ),
            "CONTRACT-004-CONFLICT" => Some(
                "Two pipelines being composed have conflicting stage definitions. \
                 Either rename stages or ensure they have identical configurations.",
            ),
            "CONTRACT-004-EMPTY" => Some(
                "Add at least one stage to the pipeline before building.",
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_info_creation() {
        let info = ContractErrorInfo::new("TEST-001", "Test error")
            .with_fix_hint("Fix this by doing that")
            .with_context_entry("stage", "my_stage");

        assert_eq!(info.code, "TEST-001");
        assert_eq!(info.summary, "Test error");
        assert_eq!(info.fix_hint, Some("Fix this by doing that".to_string()));
        assert_eq!(info.context.get("stage"), Some(&"my_stage".to_string()));
    }

    #[test]
    fn test_pipeline_validation_error_to_dict() {
        let err = PipelineValidationError::new("Test error")
            .with_stages(vec!["stage1".to_string(), "stage2".to_string()]);

        let dict = err.to_dict();
        assert_eq!(dict.get("message").unwrap(), "Test error");
    }

    #[test]
    fn test_cycle_detected_error() {
        let err = CycleDetectedError::new(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "a".to_string(),
        ]);

        assert!(err.to_string().contains("a -> b -> c -> a"));
        assert_eq!(err.error_info.code, "CONTRACT-004-CYCLE");
    }

    #[test]
    fn test_tool_error_to_dict() {
        let err = ToolError::not_found("my_tool");
        let dict = err.to_dict();
        
        assert_eq!(dict.get("type").unwrap(), "ToolNotFound");
        assert_eq!(dict.get("name").unwrap(), "my_tool");
    }

    #[test]
    fn test_contract_suggestions() {
        assert!(ContractSuggestions::get("CONTRACT-004-CYCLE").is_some());
        assert!(ContractSuggestions::get("UNKNOWN").is_none());
    }
}
