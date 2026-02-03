//! Subpipeline spawner with depth enforcement.

use super::{ChildRunInfo, ChildRunTracker, SubpipelineResult};
use crate::context::{ContextSnapshot, ExecutionContext, PipelineContext, RunIdentity};
use crate::errors::StageflowError;
use crate::pipeline::StageGraph;
use std::sync::Arc;
use uuid::Uuid;

/// Default maximum subpipeline depth.
pub const DEFAULT_MAX_DEPTH: u32 = 5;

/// Spawner for subpipelines with lifecycle event emission.
pub struct SubpipelineSpawner {
    /// Maximum allowed depth.
    max_depth: u32,
    /// Child run tracker.
    tracker: Arc<ChildRunTracker>,
}

impl SubpipelineSpawner {
    /// Creates a new spawner.
    #[must_use]
    pub fn new(tracker: Arc<ChildRunTracker>) -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            tracker,
        }
    }

    /// Sets the maximum depth.
    #[must_use]
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Spawns a subpipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if max depth is exceeded.
    pub async fn spawn(
        &self,
        parent_ctx: &Arc<PipelineContext>,
        graph: &StageGraph,
        snapshot: ContextSnapshot,
        current_depth: u32,
    ) -> Result<SubpipelineResult, StageflowError> {
        // Check depth
        if current_depth >= self.max_depth {
            return Err(StageflowError::Internal(format!(
                "Maximum subpipeline depth ({}) exceeded",
                self.max_depth
            )));
        }

        let child_run_id = RunIdentity::new();
        let child_pipeline_run_id = child_run_id.pipeline_run_id.unwrap_or_else(Uuid::new_v4);

        // Register child
        let info = ChildRunInfo {
            child_run_id: child_pipeline_run_id,
            parent_run_id: parent_ctx.run_id().pipeline_run_id.unwrap_or_default(),
            depth: current_depth + 1,
            spawned_at: crate::utils::iso_timestamp(),
        };
        self.tracker.register(info);

        // Emit spawned event
        parent_ctx.try_emit_event(
            "pipeline.spawned_child",
            Some(serde_json::json!({
                "child_run_id": child_pipeline_run_id.to_string(),
                "depth": current_depth + 1,
            })),
        );

        // Create child context
        let child_ctx = parent_ctx.fork_for_subpipeline(child_run_id);

        // Execute child pipeline
        let result = graph.execute(child_ctx.clone(), snapshot).await;

        // Unregister child
        self.tracker.unregister(child_pipeline_run_id);

        match result {
            Ok(exec_result) => {
                let subpipeline_result = if exec_result.success {
                    parent_ctx.try_emit_event(
                        "pipeline.child_completed",
                        Some(serde_json::json!({
                            "child_run_id": child_pipeline_run_id.to_string(),
                            "duration_ms": exec_result.duration_ms,
                        })),
                    );

                    SubpipelineResult::success(
                        child_pipeline_run_id,
                        exec_result.outputs,
                        exec_result.duration_ms,
                    )
                } else {
                    parent_ctx.try_emit_event(
                        "pipeline.child_failed",
                        Some(serde_json::json!({
                            "child_run_id": child_pipeline_run_id.to_string(),
                            "error": exec_result.error,
                        })),
                    );

                    SubpipelineResult::failure(
                        child_pipeline_run_id,
                        exec_result.error.unwrap_or_default(),
                        exec_result.outputs,
                        exec_result.duration_ms,
                    )
                };

                Ok(subpipeline_result)
            }
            Err(e) => {
                parent_ctx.try_emit_event(
                    "pipeline.child_failed",
                    Some(serde_json::json!({
                        "child_run_id": child_pipeline_run_id.to_string(),
                        "error": e.to_string(),
                    })),
                );

                Err(e)
            }
        }
    }

    /// Cancels all children of a parent.
    pub fn cancel_children(&self, parent_run_id: Uuid, parent_ctx: &PipelineContext) {
        let children = self.tracker.children_of(parent_run_id);

        for child in children {
            parent_ctx.try_emit_event(
                "pipeline.canceled",
                Some(serde_json::json!({
                    "child_run_id": child.child_run_id.to_string(),
                    "reason": "Parent cancelled",
                })),
            );

            self.tracker.unregister(child.child_run_id);
        }
    }
}

impl Default for SubpipelineSpawner {
    fn default() -> Self {
        Self::new(Arc::new(ChildRunTracker::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawner_creation() {
        let spawner = SubpipelineSpawner::default();
        assert_eq!(spawner.max_depth, DEFAULT_MAX_DEPTH);
    }

    #[test]
    fn test_spawner_with_max_depth() {
        let spawner = SubpipelineSpawner::default().with_max_depth(3);
        assert_eq!(spawner.max_depth, 3);
    }
}
