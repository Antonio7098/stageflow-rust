//! Mutable execution contexts for pipeline and stage execution.

use super::{ContextBag, ContextSnapshot, OutputBag, RunIdentity, StageInputs};
use crate::events::{get_event_sink, EventSink};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Trait unifying pipeline and stage context behaviors.
#[async_trait]
pub trait ExecutionContext: Send + Sync {
    /// Returns the pipeline run ID.
    fn pipeline_run_id(&self) -> Option<Uuid>;

    /// Returns the request ID.
    fn request_id(&self) -> Option<Uuid>;

    /// Returns the execution mode.
    fn execution_mode(&self) -> &str;

    /// Returns the topology name.
    fn topology(&self) -> Option<&str>;

    /// Tries to emit an event.
    fn try_emit_event(&self, event_type: &str, data: Option<serde_json::Value>);

    /// Checks if the context is cancelled.
    fn is_cancelled(&self) -> bool;
}

/// The mutable context for a pipeline execution.
pub struct PipelineContext {
    /// Run identity.
    run_id: RunIdentity,
    /// Topology name.
    topology: Option<String>,
    /// Execution mode (e.g., "production", "development").
    execution_mode: String,
    /// Mutable context data.
    pub data: ContextBag,
    /// Enrichments.
    pub enrichments: RwLock<serde_json::Value>,
    /// Stage outputs.
    pub outputs: OutputBag,
    /// Event sink for emitting events.
    event_sink: Arc<dyn EventSink>,
    /// Cancellation flag.
    cancelled: AtomicBool,
    /// Cancel reason.
    cancel_reason: RwLock<Option<String>>,
    /// Service name.
    service: Option<String>,
    /// Parent context (for subpipelines).
    parent: Option<Arc<PipelineContext>>,
}

impl PipelineContext {
    /// Creates a new pipeline context.
    #[must_use]
    pub fn new(run_id: RunIdentity) -> Self {
        Self {
            run_id,
            topology: None,
            execution_mode: "production".to_string(),
            data: ContextBag::new(),
            enrichments: RwLock::new(serde_json::json!({})),
            outputs: OutputBag::new(),
            event_sink: get_event_sink(),
            cancelled: AtomicBool::new(false),
            cancel_reason: RwLock::new(None),
            service: None,
            parent: None,
        }
    }

    /// Creates a pipeline context from a snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: &ContextSnapshot) -> Self {
        Self {
            run_id: snapshot.run_id.clone(),
            topology: None,
            execution_mode: "production".to_string(),
            data: ContextBag::new(),
            enrichments: RwLock::new(serde_json::to_value(&snapshot.enrichments).unwrap_or_default()),
            outputs: OutputBag::new(),
            event_sink: get_event_sink(),
            cancelled: AtomicBool::new(false),
            cancel_reason: RwLock::new(None),
            service: None,
            parent: None,
        }
    }

    /// Sets the topology name.
    #[must_use]
    pub fn with_topology(mut self, topology: impl Into<String>) -> Self {
        self.topology = Some(topology.into());
        self
    }

    /// Sets the execution mode.
    #[must_use]
    pub fn with_execution_mode(mut self, mode: impl Into<String>) -> Self {
        self.execution_mode = mode.into();
        self
    }

    /// Sets the event sink.
    #[must_use]
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = sink;
        self
    }

    /// Sets the service name.
    #[must_use]
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Marks the context as cancelled.
    pub fn mark_cancelled(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Marks the context as cancelled with a reason.
    pub fn mark_cancelled_with_reason(&self, reason: impl Into<String>) {
        self.cancelled.store(true, Ordering::SeqCst);
        *self.cancel_reason.write() = Some(reason.into());
    }

    /// Returns the cancel reason, if any.
    #[must_use]
    pub fn cancel_reason(&self) -> Option<String> {
        self.cancel_reason.read().clone()
    }

    /// Creates a child context for a subpipeline.
    #[must_use]
    pub fn fork_for_subpipeline(self: &Arc<Self>, child_run_id: RunIdentity) -> Arc<Self> {
        Arc::new(Self {
            run_id: child_run_id,
            topology: self.topology.clone(),
            execution_mode: self.execution_mode.clone(),
            data: ContextBag::new(),
            enrichments: RwLock::new(self.enrichments.read().clone()),
            outputs: OutputBag::new(),
            event_sink: self.event_sink.clone(),
            cancelled: AtomicBool::new(false),
            cancel_reason: RwLock::new(None),
            service: self.service.clone(),
            parent: Some(self.clone()),
        })
    }

    /// Returns the run identity.
    #[must_use]
    pub fn run_id(&self) -> &RunIdentity {
        &self.run_id
    }

    /// Returns the event sink.
    #[must_use]
    pub fn event_sink(&self) -> &Arc<dyn EventSink> {
        &self.event_sink
    }

    /// Returns the service name.
    #[must_use]
    pub fn service(&self) -> Option<&str> {
        self.service.as_deref()
    }

    /// Returns the parent context, if any.
    #[must_use]
    pub fn parent(&self) -> Option<&Arc<PipelineContext>> {
        self.parent.as_ref()
    }
}

#[async_trait]
impl ExecutionContext for PipelineContext {
    fn pipeline_run_id(&self) -> Option<Uuid> {
        self.run_id.pipeline_run_id
    }

    fn request_id(&self) -> Option<Uuid> {
        self.run_id.request_id
    }

    fn execution_mode(&self) -> &str {
        &self.execution_mode
    }

    fn topology(&self) -> Option<&str> {
        self.topology.as_deref()
    }

    fn try_emit_event(&self, event_type: &str, data: Option<serde_json::Value>) {
        let mut enriched = data.unwrap_or(serde_json::json!({}));
        
        if let serde_json::Value::Object(ref mut map) = enriched {
            if let Some(id) = self.run_id.pipeline_run_id {
                map.insert("pipeline_run_id".to_string(), serde_json::json!(id.to_string()));
            }
            if let Some(id) = self.run_id.request_id {
                map.insert("request_id".to_string(), serde_json::json!(id.to_string()));
            }
            map.insert("execution_mode".to_string(), serde_json::json!(&self.execution_mode));
            if let Some(ref topology) = self.topology {
                map.insert("topology".to_string(), serde_json::json!(topology));
            }
        }

        self.event_sink.try_emit(event_type, Some(enriched));
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// The context for a single stage execution.
pub struct StageContext {
    /// The pipeline context.
    pipeline_ctx: Arc<PipelineContext>,
    /// The stage name.
    stage_name: String,
    /// The stage inputs.
    inputs: StageInputs,
    /// The context snapshot.
    snapshot: ContextSnapshot,
}

impl StageContext {
    /// Creates a new stage context.
    #[must_use]
    pub fn new(
        pipeline_ctx: Arc<PipelineContext>,
        stage_name: impl Into<String>,
        inputs: StageInputs,
        snapshot: ContextSnapshot,
    ) -> Self {
        Self {
            pipeline_ctx,
            stage_name: stage_name.into(),
            inputs,
            snapshot,
        }
    }

    /// Returns the stage name.
    #[must_use]
    pub fn stage_name(&self) -> &str {
        &self.stage_name
    }

    /// Returns the stage inputs.
    #[must_use]
    pub fn inputs(&self) -> &StageInputs {
        &self.inputs
    }

    /// Returns the context snapshot.
    #[must_use]
    pub fn snapshot(&self) -> &ContextSnapshot {
        &self.snapshot
    }

    /// Returns the pipeline context.
    #[must_use]
    pub fn pipeline_ctx(&self) -> &Arc<PipelineContext> {
        &self.pipeline_ctx
    }

    /// Returns the context data bag.
    #[must_use]
    pub fn data(&self) -> &ContextBag {
        &self.pipeline_ctx.data
    }
}

#[async_trait]
impl ExecutionContext for StageContext {
    fn pipeline_run_id(&self) -> Option<Uuid> {
        self.pipeline_ctx.pipeline_run_id()
    }

    fn request_id(&self) -> Option<Uuid> {
        self.pipeline_ctx.request_id()
    }

    fn execution_mode(&self) -> &str {
        self.pipeline_ctx.execution_mode()
    }

    fn topology(&self) -> Option<&str> {
        self.pipeline_ctx.topology()
    }

    fn try_emit_event(&self, event_type: &str, data: Option<serde_json::Value>) {
        let mut enriched = data.unwrap_or(serde_json::json!({}));
        
        if let serde_json::Value::Object(ref mut map) = enriched {
            if let Some(id) = self.pipeline_run_id() {
                map.insert("pipeline_run_id".to_string(), serde_json::json!(id.to_string()));
            }
            if let Some(id) = self.request_id() {
                map.insert("request_id".to_string(), serde_json::json!(id.to_string()));
            }
            map.insert("execution_mode".to_string(), serde_json::json!(self.execution_mode()));
            map.insert("stage".to_string(), serde_json::json!(&self.stage_name));
        }

        self.pipeline_ctx.event_sink.try_emit(event_type, Some(enriched));
    }

    fn is_cancelled(&self) -> bool {
        self.pipeline_ctx.is_cancelled()
    }
}

/// Adapts a plain dictionary into an execution context.
pub struct DictContextAdapter {
    /// The data dictionary.
    data: HashMap<String, serde_json::Value>,
    /// Execution mode.
    execution_mode: String,
}

impl DictContextAdapter {
    /// Creates a new adapter from a dictionary.
    #[must_use]
    pub fn new(data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            data,
            execution_mode: "production".to_string(),
        }
    }

    /// Sets the execution mode.
    #[must_use]
    pub fn with_execution_mode(mut self, mode: impl Into<String>) -> Self {
        self.execution_mode = mode.into();
        self
    }

    /// Gets a value from the data.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Gets a string value from the data.
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    /// Gets a UUID value from the data.
    #[must_use]
    pub fn get_uuid(&self, key: &str) -> Option<Uuid> {
        self.get_string(key).and_then(|s| Uuid::parse_str(&s).ok())
    }
}

#[async_trait]
impl ExecutionContext for DictContextAdapter {
    fn pipeline_run_id(&self) -> Option<Uuid> {
        self.get_uuid("pipeline_run_id")
    }

    fn request_id(&self) -> Option<Uuid> {
        self.get_uuid("request_id")
    }

    fn execution_mode(&self) -> &str {
        &self.execution_mode
    }

    fn topology(&self) -> Option<&str> {
        self.data.get("topology").and_then(|v| v.as_str())
    }

    fn try_emit_event(&self, event_type: &str, data: Option<serde_json::Value>) {
        let mut enriched = data.unwrap_or(serde_json::json!({}));
        
        if let serde_json::Value::Object(ref mut map) = enriched {
            if let Some(id) = self.pipeline_run_id() {
                map.insert("pipeline_run_id".to_string(), serde_json::json!(id.to_string()));
            }
            if let Some(id) = self.request_id() {
                map.insert("request_id".to_string(), serde_json::json!(id.to_string()));
            }
            map.insert("execution_mode".to_string(), serde_json::json!(&self.execution_mode));
        }

        tracing::debug!(
            event_type = %event_type,
            data = ?enriched,
            "DictContextAdapter event"
        );
    }

    fn is_cancelled(&self) -> bool {
        self.data
            .get("cancelled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_context_creation() {
        let ctx = PipelineContext::new(RunIdentity::new())
            .with_topology("test-pipeline")
            .with_execution_mode("development");

        assert!(ctx.pipeline_run_id().is_some());
        assert_eq!(ctx.execution_mode(), "development");
        assert_eq!(ctx.topology(), Some("test-pipeline"));
    }

    #[test]
    fn test_pipeline_context_cancellation() {
        let ctx = PipelineContext::new(RunIdentity::new());
        assert!(!ctx.is_cancelled());

        ctx.mark_cancelled_with_reason("User cancelled");
        assert!(ctx.is_cancelled());
        assert_eq!(ctx.cancel_reason(), Some("User cancelled".to_string()));
    }

    #[test]
    fn test_pipeline_context_fork() {
        let parent = Arc::new(
            PipelineContext::new(RunIdentity::new())
                .with_topology("parent")
                .with_service("test-service"),
        );

        let child = parent.fork_for_subpipeline(RunIdentity::new());

        assert_eq!(child.topology(), Some("parent"));
        assert_eq!(child.service(), Some("test-service"));
        assert!(child.parent().is_some());
        assert_ne!(child.pipeline_run_id(), parent.pipeline_run_id());
    }

    #[test]
    fn test_stage_context() {
        let pipeline_ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let inputs = StageInputs::default();
        let snapshot = ContextSnapshot::new();

        let stage_ctx = StageContext::new(
            pipeline_ctx.clone(),
            "my_stage",
            inputs,
            snapshot,
        );

        assert_eq!(stage_ctx.stage_name(), "my_stage");
        assert_eq!(stage_ctx.pipeline_run_id(), pipeline_ctx.pipeline_run_id());
    }

    #[test]
    fn test_dict_context_adapter() {
        let mut data = HashMap::new();
        data.insert("pipeline_run_id".to_string(), serde_json::json!(Uuid::new_v4().to_string()));
        data.insert("topology".to_string(), serde_json::json!("test"));

        let adapter = DictContextAdapter::new(data).with_execution_mode("dev");

        assert!(adapter.pipeline_run_id().is_some());
        assert_eq!(adapter.topology(), Some("test"));
        assert_eq!(adapter.execution_mode(), "dev");
    }
}
