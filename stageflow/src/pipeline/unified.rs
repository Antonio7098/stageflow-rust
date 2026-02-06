//! Unified stage graph with enhanced execution features.

use super::StageGraph;
use crate::context::{ContextSnapshot, ExecutionContext, PipelineContext, StageContext, StageInputs};
use crate::core::{StageKind, StageOutput, StageStatus};
use crate::errors::StageflowError;
use crate::pipeline::{GuardRetryRuntimeState, GuardRetryStrategy, hash_retry_payload};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;

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
    guard_retry_strategy: Option<GuardRetryStrategy>,
}

impl UnifiedStageGraph {
    /// Creates a new unified stage graph.
    #[must_use]
    pub fn new(graph: StageGraph) -> Self {
        Self {
            inner: graph,
            guard_retry_strategy: None,
        }
    }

    /// Sets a guard-retry strategy.
    #[must_use]
    pub fn with_guard_retry_strategy(mut self, strategy: GuardRetryStrategy) -> Result<Self, StageflowError> {
        strategy
            .validate(self.inner.stage_specs())
            .map_err(StageflowError::Internal)?;
        self.guard_retry_strategy = Some(strategy);
        Ok(self)
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
        let specs = self.inner.stage_specs().clone();

        let completed: Arc<parking_lot::RwLock<HashMap<String, StageOutput>>> =
            Arc::new(parking_lot::RwLock::new(HashMap::new()));
        let mut guard_retry_state: HashMap<String, GuardRetryRuntimeState> = HashMap::new();
        let mut pending_guard_retries: HashMap<String, Vec<String>> = HashMap::new();
        let mut finalized: HashSet<String> = HashSet::new();
        let mut active_retry_targets: HashSet<String> = HashSet::new();

        let mut in_degree: HashMap<String, usize> = specs
            .iter()
            .map(|(name, spec)| (name.clone(), spec.dependencies.len()))
            .collect();

        let mut tasks: JoinSet<Result<(String, StageOutput), StageflowError>> = JoinSet::new();

        let schedule_stage = |tasks: &mut JoinSet<Result<(String, StageOutput), StageflowError>>,
                              stage_name: String,
                              ctx: Arc<PipelineContext>,
                              snapshot: ContextSnapshot,
                              completed: Arc<parking_lot::RwLock<HashMap<String, StageOutput>>>,
                              specs: HashMap<String, super::StageSpec>| {
            let spec = specs.get(&stage_name).cloned();
            if spec.is_none() {
                return;
            }
            let spec = spec.unwrap();
            tasks.spawn(async move {
                let prior_outputs: HashMap<String, StageOutput> = {
                    let lock = completed.read();
                    spec.dependencies
                        .iter()
                        .filter_map(|dep| lock.get(dep).cloned().map(|o| (dep.clone(), o)))
                        .collect()
                };

                let mut prior_data: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
                for (name, output) in &prior_outputs {
                    prior_data.insert(name.clone(), output.data.clone().unwrap_or_default());
                }

                let skip_reason = if spec.conditional {
                    find_skip_reason(&prior_data)
                } else {
                    None
                };

                if let Some(reason) = skip_reason {
                    ctx.try_emit_event(
                        "stage.skipped",
                        Some(serde_json::json!({
                            "stage": stage_name,
                            "reason": reason,
                        })),
                    );
                    return Ok((stage_name, StageOutput::skip(reason)));
                }

                let inputs = StageInputs::new(
                    prior_data,
                    spec.dependencies.clone(),
                    stage_name.clone(),
                    true,
                );

                let stage_ctx = StageContext::new(
                    ctx.clone(),
                    stage_name.clone(),
                    inputs,
                    snapshot,
                );

                ctx.try_emit_event(
                    "stage.started",
                    Some(serde_json::json!({
                        "stage": stage_name,
                    })),
                );

                let stage_start = Instant::now();
                let output = spec.runner.execute(&stage_ctx).await;
                let stage_duration_ms = stage_start.elapsed().as_secs_f64() * 1000.0;

                match output.status {
                    StageStatus::Ok => {
                        ctx.try_emit_event(
                            "stage.completed",
                            Some(serde_json::json!({
                                "stage": stage_name,
                                "duration_ms": stage_duration_ms,
                            })),
                        );
                    }
                    StageStatus::Skip => {
                        ctx.try_emit_event(
                            "stage.skipped",
                            Some(serde_json::json!({
                                "stage": stage_name,
                                "reason": output.skip_reason,
                            })),
                        );
                    }
                    StageStatus::Fail => {
                        ctx.try_emit_event(
                            "stage.failed",
                            Some(serde_json::json!({
                                "stage": stage_name,
                                "error": output.error,
                                "duration_ms": stage_duration_ms,
                            })),
                        );
                    }
                    StageStatus::Cancel => {
                        ctx.try_emit_event(
                            "stage.cancelled",
                            Some(serde_json::json!({
                                "stage": stage_name,
                                "reason": output.cancel_reason,
                            })),
                        );
                    }
                    _ => {}
                }

                Ok((stage_name, output))
            });
        };

        let ready_stages: Vec<String> = in_degree
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(name, _)| name.clone())
            .collect();

        for stage_name in ready_stages {
            schedule_stage(
                &mut tasks,
                stage_name,
                ctx.clone(),
                snapshot.clone(),
                completed.clone(),
                specs.clone(),
            );
        }

        while finalized.len() < specs.len() {
            if (*ctx).is_cancelled() {
                let reason = ctx.cancel_reason().unwrap_or_else(|| "Pipeline cancelled".to_string());
                ctx.try_emit_event(
                    "pipeline_cancelled",
                    Some(serde_json::json!({
                        "reason": &reason,
                    })),
                );
                tasks.abort_all();
                let outputs = completed.read().clone();
                return Ok(UnifiedExecutionResult {
                    outputs,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    success: false,
                    error: None,
                    cancelled: true,
                    cancel_reason: Some(reason),
                });
            }

            if tasks.len() == 0 {
                let pending: Vec<_> = specs
                    .keys()
                    .filter(|name| !finalized.contains(*name))
                    .cloned()
                    .collect();
                return Err(StageflowError::Internal(format!(
                    "Deadlocked stage graph; remaining stages: {:?}",
                    pending
                )));
            }

            let next = tasks.join_next().await;
            let result = match next {
                Some(res) => res,
                None => continue,
            };

            let (stage_name, stage_output) = match result {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => {
                    tasks.abort_all();
                    return Err(e);
                }
                Err(e) => {
                    tasks.abort_all();
                    return Err(StageflowError::Internal(format!("Task join error: {e}")));
                }
            };

            {
                completed.write().insert(stage_name.clone(), stage_output.clone());
            }

            let spec = match specs.get(&stage_name) {
                Some(s) => s,
                None => continue,
            };

            let mut policy = None;
            if self.guard_retry_strategy.is_some() && spec.kind == StageKind::Guard {
                policy = self
                    .guard_retry_strategy
                    .as_ref()
                    .and_then(|s| s.get_policy(&stage_name));
            }

            if let (Some(policy), StageStatus::Fail) = (policy, stage_output.status) {
                let state = guard_retry_state
                    .entry(stage_name.clone())
                    .or_insert_with(GuardRetryRuntimeState::new);

                if state.started_at.is_none() {
                    state.started_at = Some(Instant::now());
                }

                state.attempts += 1;

                let retry_hash = hash_retry_payload(
                    Some(&stage_output),
                    policy.hash_fields.as_deref(),
                );
                if retry_hash.is_some() && retry_hash == state.last_hash {
                    state.stagnation_hits += 1;
                } else {
                    state.stagnation_hits = 0;
                }
                state.last_hash = retry_hash;

                ctx.try_emit_event(
                    "guard_retry.attempt",
                    Some(serde_json::json!({
                        "guard": stage_name,
                        "attempt": state.attempts,
                        "retry_stage": policy.retry_stage,
                        "max_attempts": policy.max_attempts,
                        "stagnation_hits": state.stagnation_hits,
                        "timeout_seconds": policy.timeout_seconds,
                    })),
                );

                let exceeded_attempts = state.attempts >= policy.max_attempts;
                let exceeded_stagnation = state.stagnation_hits >= policy.stagnation_limit;
                let exceeded_timeout = policy
                    .timeout_seconds
                    .and_then(|timeout| state.started_at.map(|t| t.elapsed().as_secs_f64() >= timeout))
                    .unwrap_or(false);

                if exceeded_attempts || exceeded_stagnation || exceeded_timeout {
                    ctx.try_emit_event(
                        "guard_retry.exhausted",
                        Some(serde_json::json!({
                            "guard": stage_name,
                            "attempts": state.attempts,
                            "stagnation_hits": state.stagnation_hits,
                            "retry_stage": policy.retry_stage,
                            "timeout_seconds": policy.timeout_seconds,
                            "reason": if exceeded_timeout { "timeout" } else if exceeded_stagnation { "stagnation" } else { "max_attempts" },
                        })),
                    );
                } else {
                    ctx.try_emit_event(
                        "guard_retry.scheduled",
                        Some(serde_json::json!({
                            "guard": stage_name,
                            "attempt": state.attempts,
                            "retry_stage": policy.retry_stage,
                            "stagnation_hits": state.stagnation_hits,
                            "timeout_seconds": policy.timeout_seconds,
                        })),
                    );

                    pending_guard_retries
                        .entry(policy.retry_stage.clone())
                        .or_default()
                        .push(stage_name.clone());

                    if !active_retry_targets.contains(&policy.retry_stage) {
                        active_retry_targets.insert(policy.retry_stage.clone());
                        schedule_stage(
                            &mut tasks,
                            policy.retry_stage.clone(),
                            ctx.clone(),
                            snapshot.clone(),
                            completed.clone(),
                            specs.clone(),
                        );
                    }

                    continue;
                }
            }

            if stage_output.status == StageStatus::Cancel {
                let reason = stage_output
                    .cancel_reason
                    .clone()
                    .unwrap_or_else(|| "Pipeline cancelled".to_string());

                (*ctx).mark_cancelled_with_reason(&reason);
                ctx.try_emit_event(
                    "pipeline_cancelled",
                    Some(serde_json::json!({
                        "stage": stage_name,
                        "reason": &reason,
                    })),
                );
                tasks.abort_all();
                let outputs = completed.read().clone();
                return Ok(UnifiedExecutionResult {
                    outputs,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    success: false,
                    error: None,
                    cancelled: true,
                    cancel_reason: Some(reason),
                });
            }

            if stage_output.status == StageStatus::Fail {
                tasks.abort_all();
                let outputs = completed.read().clone();
                return Ok(UnifiedExecutionResult {
                    outputs,
                    duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                    success: false,
                    error: Some(format!("Stage '{}' failed", stage_name)),
                    cancelled: false,
                    cancel_reason: None,
                });
            }

            if guard_retry_state.contains_key(&stage_name) && stage_output.status != StageStatus::Fail {
                if let Some(state) = guard_retry_state.remove(&stage_name) {
                    if state.attempts > 0 {
                        ctx.try_emit_event(
                            "guard_retry.recovered",
                            Some(serde_json::json!({
                                "guard": stage_name,
                                "attempts": state.attempts,
                            })),
                        );
                    }
                }
            }

            let pending_guards = pending_guard_retries.remove(&stage_name).unwrap_or_default();
            if active_retry_targets.contains(&stage_name) {
                active_retry_targets.remove(&stage_name);
            }
            for guard_name in pending_guards {
                schedule_stage(
                    &mut tasks,
                    guard_name,
                    ctx.clone(),
                    snapshot.clone(),
                    completed.clone(),
                    specs.clone(),
                );
            }

            if !finalized.contains(&stage_name) {
                finalized.insert(stage_name.clone());
                for (child_name, child_spec) in &specs {
                    if child_spec.dependencies.contains(&stage_name) {
                        if let Some(count) = in_degree.get_mut(child_name) {
                            *count = count.saturating_sub(1);
                            if *count == 0 && !finalized.contains(child_name) {
                                schedule_stage(
                                    &mut tasks,
                                    child_name.clone(),
                                    ctx.clone(),
                                    snapshot.clone(),
                                    completed.clone(),
                                    specs.clone(),
                                );
                            }
                        }
                    }
                }
            }
        }

        let outputs = completed.read().clone();
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

fn find_skip_reason(
    outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> Option<String> {
    for output in outputs.values() {
        if let Some(value) = output.get("skip_reason") {
            if let Some(s) = value.as_str() {
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RunIdentity;
    use crate::pipeline::PipelineBuilder;
    use crate::stages::{FnStage, NoOpStage};

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

    #[tokio::test]
    async fn test_unified_conditional_skip() {
        let producer = Arc::new(FnStage::new("producer", |_ctx| {
            StageOutput::ok(
                [("skip_reason".to_string(), serde_json::json!("skip"))]
                    .into_iter()
                    .collect(),
            )
        }));
        let consumer = Arc::new(NoOpStage::new("consumer"));

        let mut builder = PipelineBuilder::new("test");
        builder
            .add_stage_spec(super::super::StageSpec::new("producer", producer))
            .unwrap();
        builder
            .add_stage_spec(
                super::super::StageSpec::new("consumer", consumer)
                    .with_dependency("producer")
                    .conditional(),
            )
            .unwrap();

        let graph = builder.build().unwrap();

        let unified = UnifiedStageGraph::new(graph);
        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let snapshot = ContextSnapshot::new();

        let result = unified.execute(ctx, snapshot).await.unwrap();
        assert!(result.outputs.contains_key("consumer"));
        assert_eq!(result.outputs["consumer"].status, StageStatus::Skip);
    }

    #[tokio::test]
    async fn test_unified_guard_retry_schedules_retry_stage() {
        let retry = Arc::new(FnStage::new("retry", |_ctx| {
            StageOutput::ok_empty()
        }));
        let guard = Arc::new(FnStage::new("guard", |_ctx| {
            StageOutput::fail("no")
        }));

        let mut builder = PipelineBuilder::new("test");
        builder
            .add_stage_spec(super::super::StageSpec::new("retry", retry))
            .unwrap();
        builder
            .add_stage_spec(
                super::super::StageSpec::new("guard", guard)
                    .with_dependency("retry")
                    .with_kind(StageKind::Guard),
            )
            .unwrap();

        let graph = builder.build().unwrap();

        let strategy = GuardRetryStrategy::new().with_policy(
            "guard",
            crate::pipeline::GuardRetryPolicy::new("retry").with_max_attempts(2),
        );

        let unified = UnifiedStageGraph::new(graph)
            .with_guard_retry_strategy(strategy)
            .unwrap();

        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let snapshot = ContextSnapshot::new();

        let result = unified.execute(ctx, snapshot).await.unwrap();
        assert!(!result.success);
        assert!(result.outputs.contains_key("retry"));
        assert!(result.outputs.contains_key("guard"));
    }
}
