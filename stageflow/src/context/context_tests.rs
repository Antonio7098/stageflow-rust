//! Comprehensive tests for context module.

#[cfg(test)]
mod tests {
    use crate::context::{
        ContextSnapshot, ExecutionContext, PipelineContext, RunIdentity, StageContext, StageInputs,
    };
    use std::sync::Arc;

    #[test]
    fn test_run_identity_creation() {
        let identity = RunIdentity::new();
        assert!(identity.pipeline_run_id.is_some());
    }

    #[test]
    fn test_run_identity_with_user_id() {
        let user_id = uuid::Uuid::new_v4();
        let identity = RunIdentity::new().with_user_id(user_id);
        assert_eq!(identity.user_id, Some(user_id));
    }

    #[test]
    fn test_run_identity_with_session_id() {
        let session_id = uuid::Uuid::new_v4();
        let identity = RunIdentity::new().with_session_id(session_id);
        assert_eq!(identity.session_id, Some(session_id));
    }

    #[test]
    fn test_run_identity_with_org_id() {
        let org_id = uuid::Uuid::new_v4();
        let identity = RunIdentity::new().with_org_id(org_id);
        assert_eq!(identity.org_id, Some(org_id));
    }

    #[test]
    fn test_run_identity_to_dict() {
        let identity = RunIdentity::new();
        let dict = identity.to_dict();
        assert!(dict.contains_key("pipeline_run_id"));
    }

    #[test]
    fn test_context_snapshot_default() {
        let snapshot = ContextSnapshot::new();
        assert!(snapshot.run_id.pipeline_run_id.is_some());
    }

    #[test]
    fn test_context_snapshot_with_run_id() {
        let identity = RunIdentity::new();
        let snapshot = ContextSnapshot::new().with_run_id(identity.clone());
        assert_eq!(snapshot.run_id.pipeline_run_id, identity.pipeline_run_id);
    }

    #[test]
    fn test_pipeline_context_creation() {
        let ctx = PipelineContext::new(RunIdentity::new());
        assert!(!ctx.is_cancelled());
    }

    #[test]
    fn test_pipeline_context_with_topology() {
        let ctx = PipelineContext::new(RunIdentity::new())
            .with_topology("test-pipeline");
        assert_eq!(ctx.topology(), Some("test-pipeline"));
    }

    #[test]
    fn test_pipeline_context_with_execution_mode() {
        let ctx = PipelineContext::new(RunIdentity::new())
            .with_execution_mode("development");
        assert_eq!(ctx.execution_mode(), "development");
    }

    #[test]
    fn test_pipeline_context_with_service() {
        let ctx = PipelineContext::new(RunIdentity::new())
            .with_service("my-service");
        assert_eq!(ctx.service(), Some("my-service"));
    }

    #[test]
    fn test_pipeline_context_cancellation() {
        let ctx = PipelineContext::new(RunIdentity::new());
        assert!(!ctx.is_cancelled());

        ctx.mark_cancelled_with_reason("Test cancellation");
        assert!(ctx.is_cancelled());
        assert_eq!(ctx.cancel_reason(), Some("Test cancellation".to_string()));
    }

    #[test]
    fn test_stage_context_creation() {
        let pipeline_ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let inputs = StageInputs::default();
        let snapshot = ContextSnapshot::new();

        let stage_ctx = StageContext::new(pipeline_ctx, "test_stage", inputs, snapshot);
        assert_eq!(stage_ctx.stage_name(), "test_stage");
    }

    #[test]
    fn test_stage_inputs_default() {
        let inputs = StageInputs::default();
        assert!(inputs.stages().is_empty());
    }

    #[test]
    fn test_stage_inputs_permissive() {
        let mut outputs = std::collections::HashMap::new();
        let mut stage_output = std::collections::HashMap::new();
        stage_output.insert("key".to_string(), serde_json::json!("value"));
        outputs.insert("dep_stage".to_string(), stage_output);

        let inputs = StageInputs::permissive(outputs, "test_stage");
        assert!(!inputs.stages().is_empty());
        assert!(inputs.contains("dep_stage"));
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
    }

    #[test]
    fn test_run_identity_serialization() {
        let identity = RunIdentity::new();
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: RunIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity.pipeline_run_id, deserialized.pipeline_run_id);
    }
}
