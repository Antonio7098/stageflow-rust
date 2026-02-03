//! Legacy StageGraph DAG execution engine.

use super::StageSpec;
use crate::context::{ContextSnapshot, ExecutionContext, PipelineContext, StageContext, StageInputs};
use crate::core::{StageOutput, StageStatus};
use crate::errors::StageflowError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

/// Result of executing a stage graph.
#[derive(Debug)]
pub struct GraphExecutionResult {
    /// Per-stage outputs.
    pub outputs: HashMap<String, StageOutput>,
    /// Total execution time in milliseconds.
    pub duration_ms: f64,
    /// Whether execution completed successfully.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// A directed acyclic graph of stages for execution.
#[derive(Debug)]
pub struct StageGraph {
    /// The pipeline name.
    name: String,
    /// Stage specifications.
    stages: HashMap<String, StageSpec>,
    /// Execution order (topologically sorted).
    execution_order: Vec<String>,
}

impl StageGraph {
    /// Creates a new stage graph.
    #[must_use]
    pub fn new(
        name: String,
        stages: HashMap<String, StageSpec>,
        stage_order: Vec<String>,
    ) -> Self {
        // Compute topological order
        let execution_order = topological_sort(&stages, &stage_order);

        Self {
            name,
            stages,
            execution_order,
        }
    }

    /// Returns the pipeline name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of stages.
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Returns the execution order.
    #[must_use]
    pub fn execution_order(&self) -> &[String] {
        &self.execution_order
    }

    /// Executes the stage graph.
    pub async fn execute(
        &self,
        ctx: Arc<PipelineContext>,
        snapshot: ContextSnapshot,
    ) -> Result<GraphExecutionResult, StageflowError> {
        let start = Instant::now();
        let mut outputs: HashMap<String, StageOutput> = HashMap::new();
        let mut completed_outputs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

        for stage_name in &self.execution_order {
            if (*ctx).is_cancelled() {
                return Ok(GraphExecutionResult {
                    outputs,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    success: false,
                    error: Some("Pipeline cancelled".to_string()),
                });
            }

            let spec = self.stages.get(stage_name).ok_or_else(|| {
                StageflowError::Internal(format!("Stage '{}' not found", stage_name))
            })?;

            // Build inputs from completed outputs
            let inputs = StageInputs::new(
                completed_outputs.clone(),
                spec.dependencies.clone(),
                stage_name,
                true,
            );

            // Create stage context
            let stage_ctx = StageContext::new(
                ctx.clone(),
                stage_name,
                inputs,
                snapshot.clone(),
            );

            // Emit stage.started
            (*ctx).try_emit_event(
                "stage.started",
                Some(serde_json::json!({
                    "stage": stage_name,
                })),
            );

            let stage_start = Instant::now();

            // Execute stage
            let output = spec.runner.execute(&stage_ctx).await;
            let stage_duration_ms = stage_start.elapsed().as_secs_f64() * 1000.0;

            // Emit appropriate event based on status
            match output.status {
                StageStatus::Ok => {
                    (*ctx).try_emit_event(
                        "stage.completed",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "duration_ms": stage_duration_ms,
                        })),
                    );
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

                    return Ok(GraphExecutionResult {
                        outputs,
                        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                        success: false,
                        error: Some(format!("Stage '{}' failed", stage_name)),
                    });
                }
                StageStatus::Cancel => {
                    (*ctx).try_emit_event(
                        "stage.cancelled",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "reason": output.cancel_reason,
                        })),
                    );

                    outputs.insert(stage_name.clone(), output);

                    return Ok(GraphExecutionResult {
                        outputs,
                        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                        success: false,
                        error: Some(format!("Stage '{}' cancelled", stage_name)),
                    });
                }
                _ => {}
            }

            // Store output for downstream stages
            if let Some(data) = output.data.clone() {
                completed_outputs.insert(stage_name.clone(), data);
            } else {
                completed_outputs.insert(stage_name.clone(), HashMap::new());
            }

            outputs.insert(stage_name.clone(), output);
        }

        Ok(GraphExecutionResult {
            outputs,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            success: true,
            error: None,
        })
    }
}

/// Performs topological sort on the stage graph.
fn topological_sort(
    stages: &HashMap<String, StageSpec>,
    stage_order: &[String],
) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_visited = HashSet::new();

    fn visit(
        node: &str,
        stages: &HashMap<String, StageSpec>,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        if visited.contains(node) {
            return;
        }
        if temp_visited.contains(node) {
            return; // Cycle, but we've already validated
        }

        temp_visited.insert(node.to_string());

        if let Some(spec) = stages.get(node) {
            for dep in &spec.dependencies {
                visit(dep, stages, visited, temp_visited, result);
            }
        }

        temp_visited.remove(node);
        visited.insert(node.to_string());
        result.push(node.to_string());
    }

    // Visit in insertion order for determinism
    for name in stage_order {
        visit(name, stages, &mut visited, &mut temp_visited, &mut result);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RunIdentity;
    use crate::stages::NoOpStage;

    fn noop(name: &str) -> Arc<dyn crate::stages::Stage> {
        Arc::new(NoOpStage::new(name))
    }

    fn build_simple_graph() -> StageGraph {
        let mut stages = HashMap::new();
        let mut order = Vec::new();

        let spec1 = StageSpec::new("stage1", noop("stage1"));
        stages.insert("stage1".to_string(), spec1);
        order.push("stage1".to_string());

        let spec2 = StageSpec::new("stage2", noop("stage2"))
            .with_dependency("stage1");
        stages.insert("stage2".to_string(), spec2);
        order.push("stage2".to_string());

        StageGraph::new("test".to_string(), stages, order)
    }

    #[test]
    fn test_graph_creation() {
        let graph = build_simple_graph();
        assert_eq!(graph.name(), "test");
        assert_eq!(graph.stage_count(), 2);
    }

    #[test]
    fn test_topological_order() {
        let graph = build_simple_graph();
        let order = graph.execution_order();

        // stage1 must come before stage2
        let pos1 = order.iter().position(|n| n == "stage1").unwrap();
        let pos2 = order.iter().position(|n| n == "stage2").unwrap();
        assert!(pos1 < pos2);
    }

    #[tokio::test]
    async fn test_graph_execution() {
        let graph = build_simple_graph();
        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let snapshot = ContextSnapshot::new();

        let result = graph.execute(ctx, snapshot).await.unwrap();

        assert!(result.success);
        assert_eq!(result.outputs.len(), 2);
    }
}
