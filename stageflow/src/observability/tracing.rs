//! Tracing and OpenTelemetry integration for stageflow pipelines.
//!
//! This module provides structured tracing capabilities that integrate with
//! the broader Rust tracing ecosystem and OpenTelemetry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Span attributes for pipeline execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineSpanAttributes {
    /// Pipeline name.
    pub pipeline_name: Option<String>,
    /// Pipeline run ID.
    pub pipeline_run_id: Option<String>,
    /// Request ID.
    pub request_id: Option<String>,
    /// Session ID.
    pub session_id: Option<String>,
    /// User ID.
    pub user_id: Option<String>,
    /// Organization ID.
    pub org_id: Option<String>,
    /// Execution mode.
    pub execution_mode: Option<String>,
    /// Service name.
    pub service: Option<String>,
    /// Topology name.
    pub topology: Option<String>,
}

impl PipelineSpanAttributes {
    /// Creates new pipeline span attributes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pipeline name.
    #[must_use]
    pub fn with_pipeline_name(mut self, name: impl Into<String>) -> Self {
        self.pipeline_name = Some(name.into());
        self
    }

    /// Sets the pipeline run ID.
    #[must_use]
    pub fn with_pipeline_run_id(mut self, id: impl Into<String>) -> Self {
        self.pipeline_run_id = Some(id.into());
        self
    }

    /// Converts to OpenTelemetry attributes.
    #[must_use]
    pub fn to_otel_attributes(&self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        
        if let Some(ref v) = self.pipeline_name {
            attrs.insert("pipeline.name".to_string(), v.clone());
        }
        if let Some(ref v) = self.pipeline_run_id {
            attrs.insert("pipeline.run_id".to_string(), v.clone());
        }
        if let Some(ref v) = self.request_id {
            attrs.insert("pipeline.request_id".to_string(), v.clone());
        }
        if let Some(ref v) = self.session_id {
            attrs.insert("pipeline.session_id".to_string(), v.clone());
        }
        if let Some(ref v) = self.user_id {
            attrs.insert("pipeline.user_id".to_string(), v.clone());
        }
        if let Some(ref v) = self.org_id {
            attrs.insert("pipeline.org_id".to_string(), v.clone());
        }
        if let Some(ref v) = self.execution_mode {
            attrs.insert("pipeline.execution_mode".to_string(), v.clone());
        }
        if let Some(ref v) = self.service {
            attrs.insert("service.name".to_string(), v.clone());
        }
        if let Some(ref v) = self.topology {
            attrs.insert("pipeline.topology".to_string(), v.clone());
        }
        
        attrs
    }
}

/// Span attributes for stage execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageSpanAttributes {
    /// Stage name.
    pub stage_name: String,
    /// Stage kind.
    pub stage_kind: Option<String>,
    /// Stage status.
    pub status: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: Option<f64>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Data keys produced.
    pub data_keys: Vec<String>,
}

impl StageSpanAttributes {
    /// Creates new stage span attributes.
    #[must_use]
    pub fn new(stage_name: impl Into<String>) -> Self {
        Self {
            stage_name: stage_name.into(),
            ..Default::default()
        }
    }

    /// Sets the stage status.
    #[must_use]
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Sets the duration.
    #[must_use]
    pub fn with_duration_ms(mut self, duration_ms: f64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Sets the error.
    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Converts to OpenTelemetry attributes.
    #[must_use]
    pub fn to_otel_attributes(&self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        
        attrs.insert("stage.name".to_string(), self.stage_name.clone());
        
        if let Some(ref v) = self.stage_kind {
            attrs.insert("stage.kind".to_string(), v.clone());
        }
        if let Some(ref v) = self.status {
            attrs.insert("stage.status".to_string(), v.clone());
        }
        if let Some(v) = self.duration_ms {
            attrs.insert("stage.duration_ms".to_string(), v.to_string());
        }
        if let Some(ref v) = self.error {
            attrs.insert("stage.error".to_string(), v.clone());
        }
        if !self.data_keys.is_empty() {
            attrs.insert("stage.data_keys".to_string(), self.data_keys.join(","));
        }
        
        attrs
    }
}

/// Simple span timing helper.
#[derive(Debug)]
pub struct SpanTimer {
    start: Instant,
    name: String,
}

impl SpanTimer {
    /// Starts a new span timer.
    #[must_use]
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Returns the elapsed time in milliseconds.
    #[must_use]
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    /// Returns the span name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Finishes the span and returns the duration.
    #[must_use]
    pub fn finish(self) -> f64 {
        self.elapsed_ms()
    }
}

/// Trait for types that can emit tracing events.
pub trait TracingEmitter: Send + Sync {
    /// Emits a span start event.
    fn span_start(&self, name: &str, attributes: &HashMap<String, String>);
    
    /// Emits a span end event.
    fn span_end(&self, name: &str, duration_ms: f64, attributes: &HashMap<String, String>);
    
    /// Emits an error event.
    fn span_error(&self, name: &str, error: &str, attributes: &HashMap<String, String>);
}

/// No-op tracing emitter.
#[derive(Debug, Clone, Default)]
pub struct NoOpTracingEmitter;

impl TracingEmitter for NoOpTracingEmitter {
    fn span_start(&self, _name: &str, _attributes: &HashMap<String, String>) {}
    fn span_end(&self, _name: &str, _duration_ms: f64, _attributes: &HashMap<String, String>) {}
    fn span_error(&self, _name: &str, _error: &str, _attributes: &HashMap<String, String>) {}
}

/// Logging-based tracing emitter.
#[derive(Debug, Clone, Default)]
pub struct LoggingTracingEmitter;

impl TracingEmitter for LoggingTracingEmitter {
    fn span_start(&self, name: &str, attributes: &HashMap<String, String>) {
        tracing::info!(
            span_name = name,
            ?attributes,
            "Span started"
        );
    }

    fn span_end(&self, name: &str, duration_ms: f64, attributes: &HashMap<String, String>) {
        tracing::info!(
            span_name = name,
            duration_ms,
            ?attributes,
            "Span ended"
        );
    }

    fn span_error(&self, name: &str, error: &str, attributes: &HashMap<String, String>) {
        tracing::error!(
            span_name = name,
            error,
            ?attributes,
            "Span error"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_span_attributes() {
        let attrs = PipelineSpanAttributes::new()
            .with_pipeline_name("test-pipeline")
            .with_pipeline_run_id("run-123");

        let otel = attrs.to_otel_attributes();
        assert_eq!(otel.get("pipeline.name"), Some(&"test-pipeline".to_string()));
        assert_eq!(otel.get("pipeline.run_id"), Some(&"run-123".to_string()));
    }

    #[test]
    fn test_stage_span_attributes() {
        let attrs = StageSpanAttributes::new("my_stage")
            .with_status("completed")
            .with_duration_ms(123.45);

        let otel = attrs.to_otel_attributes();
        assert_eq!(otel.get("stage.name"), Some(&"my_stage".to_string()));
        assert_eq!(otel.get("stage.status"), Some(&"completed".to_string()));
        assert_eq!(otel.get("stage.duration_ms"), Some(&"123.45".to_string()));
    }

    #[test]
    fn test_span_timer() {
        let timer = SpanTimer::start("test_span");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish();
        assert!(duration >= 10.0);
    }

    #[test]
    fn test_noop_emitter() {
        let emitter = NoOpTracingEmitter;
        emitter.span_start("test", &HashMap::new());
        emitter.span_end("test", 100.0, &HashMap::new());
        emitter.span_error("test", "error", &HashMap::new());
        // Should not panic
    }
}
