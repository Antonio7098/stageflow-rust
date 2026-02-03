//! Wide event emitter for comprehensive observability.

use crate::context::ExecutionContext;
use crate::core::StageStatus;
use std::collections::HashMap;

/// Emitter for wide events (comprehensive event payloads).
pub struct WideEventEmitter {
    /// Default event type for stage events.
    pub stage_event_type: String,
    /// Default event type for pipeline events.
    pub pipeline_event_type: String,
}

impl Default for WideEventEmitter {
    fn default() -> Self {
        Self {
            stage_event_type: "stage.wide".to_string(),
            pipeline_event_type: "pipeline.wide".to_string(),
        }
    }
}

impl WideEventEmitter {
    /// Creates a new wide event emitter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a stage payload.
    #[must_use]
    pub fn build_stage_payload<C: ExecutionContext>(
        ctx: &C,
        stage_name: &str,
        status: StageStatus,
        started_at: &str,
        ended_at: &str,
        duration_ms: f64,
        error: Option<&str>,
        data_keys: &[String],
        extra: Option<HashMap<String, serde_json::Value>>,
    ) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "pipeline_run_id": ctx.pipeline_run_id().map(|id| id.to_string()),
            "request_id": ctx.request_id().map(|id| id.to_string()),
            "execution_mode": ctx.execution_mode(),
            "topology": ctx.topology(),
            "stage": stage_name,
            "status": status.to_string(),
            "started_at": started_at,
            "ended_at": ended_at,
            "duration_ms": duration_ms,
        });

        if let Some(err) = error {
            payload["error"] = serde_json::json!(err);
        }

        let mut sorted_keys = data_keys.to_vec();
        sorted_keys.sort();
        payload["data_keys"] = serde_json::json!(sorted_keys);

        if let Some(extra_data) = extra {
            if let serde_json::Value::Object(ref mut map) = payload {
                map.insert("extra".to_string(), serde_json::json!(extra_data));
            }
        }

        payload
    }

    /// Builds a pipeline payload.
    #[must_use]
    pub fn build_pipeline_payload<C: ExecutionContext>(
        ctx: &C,
        pipeline_name: Option<&str>,
        stage_statuses: &[(String, StageStatus)],
        stage_details: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        let name = pipeline_name
            .or(ctx.topology())
            .unwrap_or("pipeline")
            .to_string();

        // Determine overall status
        let has_failure = stage_statuses.iter().any(|(_, s)| *s == StageStatus::Fail);
        let status = if has_failure { "failed" } else { "completed" };

        // Count statuses
        let mut stage_counts: HashMap<String, u32> = HashMap::new();
        for (_, s) in stage_statuses {
            *stage_counts.entry(s.to_string()).or_insert(0) += 1;
        }

        serde_json::json!({
            "pipeline_run_id": ctx.pipeline_run_id().map(|id| id.to_string()),
            "request_id": ctx.request_id().map(|id| id.to_string()),
            "execution_mode": ctx.execution_mode(),
            "pipeline_name": name,
            "status": status,
            "stage_counts": stage_counts,
            "stage_details": stage_details,
        })
    }

    /// Emits a stage wide event.
    pub fn emit_stage_event<C: ExecutionContext>(
        &self,
        ctx: &C,
        stage_name: &str,
        status: StageStatus,
        started_at: &str,
        ended_at: &str,
        duration_ms: f64,
        error: Option<&str>,
        data_keys: &[String],
        extra: Option<HashMap<String, serde_json::Value>>,
    ) {
        let payload = Self::build_stage_payload(
            ctx,
            stage_name,
            status,
            started_at,
            ended_at,
            duration_ms,
            error,
            data_keys,
            extra,
        );

        ctx.try_emit_event(&self.stage_event_type, Some(payload));
    }

    /// Emits a pipeline wide event.
    pub fn emit_pipeline_event<C: ExecutionContext>(
        &self,
        ctx: &C,
        pipeline_name: Option<&str>,
        stage_statuses: &[(String, StageStatus)],
        stage_details: Vec<serde_json::Value>,
    ) {
        let payload = Self::build_pipeline_payload(ctx, pipeline_name, stage_statuses, stage_details);
        ctx.try_emit_event(&self.pipeline_event_type, Some(payload));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::DictContextAdapter;
    use uuid::Uuid;

    #[test]
    fn test_emitter_creation() {
        let emitter = WideEventEmitter::new();
        assert_eq!(emitter.stage_event_type, "stage.wide");
        assert_eq!(emitter.pipeline_event_type, "pipeline.wide");
    }

    #[test]
    fn test_build_stage_payload() {
        let mut data = std::collections::HashMap::new();
        data.insert("pipeline_run_id".to_string(), serde_json::json!(Uuid::new_v4().to_string()));

        let ctx = DictContextAdapter::new(data);

        let payload = WideEventEmitter::build_stage_payload(
            &ctx,
            "my_stage",
            StageStatus::Ok,
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:01Z",
            1000.0,
            None,
            &["key1".to_string(), "key2".to_string()],
            None,
        );

        assert_eq!(payload["stage"], "my_stage");
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["duration_ms"], 1000.0);
        // data_keys should be sorted
        assert_eq!(payload["data_keys"], serde_json::json!(["key1", "key2"]));
    }

    #[test]
    fn test_build_pipeline_payload() {
        let ctx = DictContextAdapter::new(std::collections::HashMap::new());

        let statuses = vec![
            ("stage1".to_string(), StageStatus::Ok),
            ("stage2".to_string(), StageStatus::Ok),
        ];

        let payload = WideEventEmitter::build_pipeline_payload(
            &ctx,
            Some("my_pipeline"),
            &statuses,
            vec![],
        );

        assert_eq!(payload["pipeline_name"], "my_pipeline");
        assert_eq!(payload["status"], "completed");
    }

    #[test]
    fn test_pipeline_failed_status() {
        let ctx = DictContextAdapter::new(std::collections::HashMap::new());

        let statuses = vec![
            ("stage1".to_string(), StageStatus::Ok),
            ("stage2".to_string(), StageStatus::Fail),
        ];

        let payload = WideEventEmitter::build_pipeline_payload(&ctx, None, &statuses, vec![]);

        assert_eq!(payload["status"], "failed");
    }
}
