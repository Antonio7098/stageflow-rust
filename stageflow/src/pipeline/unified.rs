//! Unified stage graph with enhanced execution features.

use super::StageGraph;
use crate::context::{ContextSnapshot, ExecutionContext, PipelineContext, StageContext, StageInputs};
use crate::core::{StageOutput, StageStatus};
use crate::errors::StageflowError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Cancellation error for unified pipeline.
#[derive(Debug)]
pub struct UnifiedPipelineCancelled {
    /// The reason for cancellation.
    pub reason: String,
    /// The stage that triggered cancellation.
    pub stage: Option<String>,
}

impl std::fmt::Display for UnifiedPipelineCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pipeline cancelled: {}", self.reason)
    }
}

impl std::error::Error for UnifiedPipelineCancelled {}

/// Result of unified graph execution.
#[derive(Debug)]
pub struct UnifiedExecutionResult {
    /// Per-stage outputs keyed by stage name.
    pub outputs: HashMap<String, StageOutput>,
    /// Total execution time in milliseconds.
    pub duration_ms: f64,
    /// Whether execution completed successfully.
    pub success: bool,
    /// Error if execution failed.
    pub error: Option<String>,
    /// Whether execution was cancelled.
    pub cancelled: bool,
    /// Cancellation reason if cancelled.
    pub cancel_reason: Option<String>,
}

/// Enhanced stage graph with conditional execution and cancellation.
pub struct UnifiedStageGraph {
    /// The underlying stage graph.
    inner: StageGraph,
}

impl UnifiedStageGraph {
    /// Creates a new unified stage graph.
    #[must_use]
    pub fn new(graph: StageGraph) -> Self {
        Self { inner: graph }
    }

    /// Returns the pipeline name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns the number of stages.
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.inner.stage_count()
    }

    /// Executes the unified stage graph.
    ///
    /// Supports:
    /// - Conditional stage execution (skip if inputs contain skip_reason)
    /// - Cancellation on StageStatus::Cancel
    pub async fn execute(
        &self,
        ctx: Arc<PipelineContext>,
        snapshot: ContextSnapshot,
    ) -> Result<UnifiedExecutionResult, StageflowError> {
        let start = Instant::now();
        let mut outputs: HashMap<String, StageOutput> = HashMap::new();
        let mut completed_outputs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

        let execution_order = self.inner.execution_order().to_vec();

        for stage_name in &execution_order {
            if (*ctx).is_cancelled() {
                (*ctx).try_emit_event(
                    "pipeline.cancelled",
                    Some(serde_json::json!({
                        "reason": ctx.cancel_reason(),
                    })),
                );

                return Ok(UnifiedExecutionResult {
                    outputs,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    success: false,
                    error: None,
                    cancelled: true,
                    cancel_reason: (*ctx).cancel_reason(),
                });
            }

            // Check for conditional skip based on inputs
            let should_skip = check_skip_condition(&completed_outputs, stage_name);

            if should_skip {
                (*ctx).try_emit_event(
                    "stage.skipped",
                    Some(serde_json::json!({
                        "stage": stage_name,
                        "reason": "Conditional skip",
                    })),
                );

                let skip_output = StageOutput::skip("Conditional skip based on inputs");
                outputs.insert(stage_name.clone(), skip_output);
                completed_outputs.insert(stage_name.clone(), HashMap::new());
                continue;
            }

            // Build inputs
            let inputs = StageInputs::permissive(completed_outputs.clone(), stage_name);

            // Create stage context
            let stage_ctx = StageContext::new(
                ctx.clone(),
                stage_name,
                inputs,
                snapshot.clone(),
            );

            (*ctx).try_emit_event(
                "stage.started",
                Some(serde_json::json!({
                    "stage": stage_name,
                })),
            );

            let stage_start = Instant::now();

            // Get the stage spec and execute
            // Note: In a real implementation, we'd access the inner graph's stages
            // For now, use the basic execution from the inner graph
            let output = execute_stage(&self.inner, stage_name, &stage_ctx).await?;
            let stage_duration_ms = stage_start.elapsed().as_secs_f64() * 1000.0;

            match output.status {
                StageStatus::Cancel => {
                    let reason = output.cancel_reason.clone().unwrap_or_else(|| "Stage requested cancellation".to_string());

                    (*ctx).mark_cancelled_with_reason(&reason);

                    (*ctx).try_emit_event(
                        "pipeline.cancelled",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "reason": &reason,
                        })),
                    );

                    outputs.insert(stage_name.clone(), output);

                    return Ok(UnifiedExecutionResult {
                        outputs,
                        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                        success: false,
                        error: None,
                        cancelled: true,
                        cancel_reason: Some(reason),
                    });
                }
                StageStatus::Fail => {
                    (*ctx).try_emit_event(
                        "stage.failed",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "error": output.error,
                            "duration_ms": stage_duration_ms,
                        })),
                    );

                    outputs.insert(stage_name.clone(), output);

                    return Ok(UnifiedExecutionResult {
                        outputs,
                        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                        success: false,
                        error: Some(format!("Stage '{}' failed", stage_name)),
                        cancelled: false,
                        cancel_reason: None,
                    });
                }
                StageStatus::Skip => {
                    (*ctx).try_emit_event(
                        "stage.skipped",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "reason": output.skip_reason,
                        })),
                    );
                }
                StageStatus::Ok => {
                    (*ctx).try_emit_event(
                        "stage.completed",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "duration_ms": stage_duration_ms,
                        })),
                    );
                }
                _ => {}
            }

            // Store output
            if let Some(data) = output.data.clone() {
                completed_outputs.insert(stage_name.clone(), data);
            } else {
                completed_outputs.insert(stage_name.clone(), HashMap::new());
            }

            outputs.insert(stage_name.clone(), output);
        }

        Ok(UnifiedExecutionResult {
            outputs,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            success: true,
            error: None,
            cancelled: false,
            cancel_reason: None,
        })
    }
}

fn check_skip_condition(
    outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
    _stage_name: &str,
) -> bool {
    // Check if any dependency output contains a skip_reason
    for (_name, output) in outputs {
        if output.contains_key("skip_reason") {
            return true;
        }
    }
    false
}

async fn execute_stage(
    _graph: &StageGraph,
    _stage_name: &str,
    _ctx: &StageContext,
) -> Result<StageOutput, StageflowError> {
    // This is a simplified version - in the full implementation,
    // we'd need to expose the stages from StageGraph
    // For now, return ok
    Ok(StageOutput::ok_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RunIdentity;
    use crate::pipeline::PipelineBuilder;
    use crate::stages::NoOpStage;

    fn noop(name: &str) -> Arc<dyn crate::stages::Stage> {
        Arc::new(NoOpStage::new(name))
    }

    #[tokio::test]
    async fn test_unified_graph_creation() {
        let graph = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[])
            .unwrap()
            .build()
            .unwrap();

        let unified = UnifiedStageGraph::new(graph);
        assert_eq!(unified.name(), "test");
    }

    #[tokio::test]
    async fn test_unified_execution() {
        let graph = PipelineBuilder::new("test")
            .stage("stage1", noop("stage1"), &[])
            .unwrap()
            .build()
            .unwrap();

        let unified = UnifiedStageGraph::new(graph);
        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let snapshot = ContextSnapshot::new();

        let result = unified.execute(ctx, snapshot).await.unwrap();
        assert!(result.success);
        assert!(!result.cancelled);
    }
}
