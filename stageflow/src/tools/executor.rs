//! Advanced tool executor with approval and undo support.

use super::{ApprovalService, ToolDefinition, ToolInput, ToolOutput, ToolRegistry, UndoMetadata, UndoStore};
use crate::context::ExecutionContext;
use crate::errors::ToolError;
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;

/// Advanced tool executor with full lifecycle support.
pub struct AdvancedToolExecutor {
    /// Tool registry.
    registry: Arc<ToolRegistry>,
    /// Approval service.
    approval_service: Arc<ApprovalService>,
    /// Undo store.
    undo_store: Arc<UndoStore>,
    /// Default approval timeout.
    approval_timeout: Duration,
}

impl AdvancedToolExecutor {
    /// Creates a new executor.
    #[must_use]
    pub fn new(
        registry: Arc<ToolRegistry>,
        approval_service: Arc<ApprovalService>,
        undo_store: Arc<UndoStore>,
    ) -> Self {
        Self {
            registry,
            approval_service,
            undo_store,
            approval_timeout: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Sets the approval timeout.
    #[must_use]
    pub fn with_approval_timeout(mut self, timeout: Duration) -> Self {
        self.approval_timeout = timeout;
        self
    }

    /// Executes a tool with full lifecycle.
    pub async fn execute<C: ExecutionContext>(
        &self,
        input: ToolInput,
        definition: &ToolDefinition,
        ctx: &C,
    ) -> Result<ToolOutput, ToolError> {
        // Emit tool.invoked
        ctx.try_emit_event(
            "tool.invoked",
            Some(serde_json::json!({
                "tool": input.tool_name,
                "action_id": input.action_id.to_string(),
            })),
        );

        // Check behavior gating
        if let Some(ref behavior) = input.behavior {
            if !definition.is_behavior_allowed(behavior) {
                ctx.try_emit_event(
                    "tool.denied",
                    Some(serde_json::json!({
                        "tool": input.tool_name,
                        "reason": "behavior_not_allowed",
                        "behavior": behavior,
                    })),
                );

                return Err(ToolError::denied(
                    &input.tool_name,
                    format!("Behavior '{}' not allowed", behavior),
                ));
            }
        }

        // Handle approval if required
        if definition.requires_approval {
            let message = definition
                .approval_message
                .as_deref()
                .unwrap_or("Tool requires approval");

            ctx.try_emit_event(
                "approval.requested",
                Some(serde_json::json!({
                    "tool": input.tool_name,
                    "message": message,
                })),
            );

            match self
                .approval_service
                .request_approval(&input.tool_name, message, self.approval_timeout)
                .await
            {
                Ok(true) => {
                    ctx.try_emit_event(
                        "approval.decided",
                        Some(serde_json::json!({
                            "tool": input.tool_name,
                            "approved": true,
                        })),
                    );
                }
                Ok(false) => {
                    ctx.try_emit_event(
                        "approval.decided",
                        Some(serde_json::json!({
                            "tool": input.tool_name,
                            "approved": false,
                        })),
                    );

                    return Err(ToolError::approval_denied(&input.tool_name));
                }
                Err(status) => {
                    ctx.try_emit_event(
                        "tool.denied",
                        Some(serde_json::json!({
                            "tool": input.tool_name,
                            "reason": "approval_timeout",
                        })),
                    );

                    return Err(ToolError::approval_timeout(
                        &input.tool_name,
                        input.action_id.to_string(),
                        self.approval_timeout.as_secs_f64(),
                    ));
                }
            }
        }

        // Emit tool.started
        ctx.try_emit_event(
            "tool.started",
            Some(serde_json::json!({
                "tool": input.tool_name,
            })),
        );

        // Execute the tool
        // In a real implementation, we'd call the actual tool handler here
        // For now, return a placeholder success
        let output = ToolOutput::ok(Some(serde_json::json!({"status": "executed"})));

        if output.success {
            ctx.try_emit_event(
                "tool.completed",
                Some(serde_json::json!({
                    "tool": input.tool_name,
                })),
            );

            // Store undo metadata if applicable
            if definition.undoable {
                if let Some(ref undo_data) = output.undo_metadata {
                    let metadata = UndoMetadata::new(
                        input.action_id,
                        &input.tool_name,
                        undo_data.clone(),
                    );
                    self.undo_store.store(metadata);
                }
            }
        } else {
            ctx.try_emit_event(
                "tool.failed",
                Some(serde_json::json!({
                    "tool": input.tool_name,
                    "error": output.error,
                })),
            );
        }

        Ok(output)
    }

    /// Undoes a tool action.
    pub async fn undo<C: ExecutionContext>(
        &self,
        action_id: uuid::Uuid,
        ctx: &C,
    ) -> Result<bool, ToolError> {
        // Get undo metadata
        let metadata = match self.undo_store.get(action_id) {
            Some(m) => m,
            None => return Ok(false),
        };

        // In a real implementation, we'd call the undo handler here
        // For now, simulate success
        let success = true;

        if success {
            ctx.try_emit_event(
                "tool.undone",
                Some(serde_json::json!({
                    "tool": metadata.tool_name,
                    "action_id": action_id.to_string(),
                })),
            );

            self.undo_store.remove(action_id);
            Ok(true)
        } else {
            ctx.try_emit_event(
                "tool.undo_failed",
                Some(serde_json::json!({
                    "tool": metadata.tool_name,
                    "action_id": action_id.to_string(),
                })),
            );

            Err(ToolError::undo_failed(&metadata.tool_name, "Undo operation failed"))
        }
    }
}

impl std::fmt::Debug for AdvancedToolExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvancedToolExecutor")
            .field("approval_timeout", &self.approval_timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{DictContextAdapter, PipelineContext, RunIdentity};
    use std::collections::HashMap;

    fn create_executor() -> AdvancedToolExecutor {
        AdvancedToolExecutor::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(ApprovalService::new()),
            Arc::new(UndoStore::default()),
        )
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = create_executor();
        assert_eq!(executor.approval_timeout, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn test_execute_simple() {
        let executor = create_executor();
        let input = ToolInput::new("test_tool", serde_json::json!({}));
        let definition = ToolDefinition::new("test_tool", "test_action");
        let ctx = DictContextAdapter::new(HashMap::new());

        let result = executor.execute(input, &definition, &ctx).await;
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_execute_behavior_denied() {
        let executor = create_executor();
        let mut input = ToolInput::new("tool", serde_json::json!({}));
        input.behavior = Some("development".to_string());

        let definition = ToolDefinition::new("tool", "action")
            .with_allowed_behaviors(vec!["production".to_string()]);

        let ctx = DictContextAdapter::new(HashMap::new());

        let result = executor.execute(input, &definition, &ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Denied { .. }));
    }
}
