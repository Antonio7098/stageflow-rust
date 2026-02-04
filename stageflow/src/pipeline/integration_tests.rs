//! Comprehensive integration tests for pipeline execution.

#[cfg(test)]
mod tests {
    use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageContext, StageInputs};
    use crate::core::StageOutput;
    use crate::pipeline::{
        BackoffStrategy, FailureMode, JitterStrategy, PipelineBuilder, RetryConfig, StageSpec,
    };
    use crate::stages::{NoOpStage, Stage};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug)]
    struct CountingStage {
        name: String,
        counter: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Stage for CountingStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn execute(&self, _ctx: &StageContext) -> StageOutput {
            self.counter.fetch_add(1, Ordering::SeqCst);
            StageOutput::ok_empty()
        }
    }

    #[derive(Debug)]
    struct FailingStage {
        name: String,
        retryable: bool,
    }

    #[async_trait]
    impl Stage for FailingStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn execute(&self, _ctx: &StageContext) -> StageOutput {
            if self.retryable {
                StageOutput::fail_retryable("Transient error")
            } else {
                StageOutput::fail("Permanent failure")
            }
        }
    }

    #[derive(Debug)]
    struct DataProducerStage {
        name: String,
        key: String,
        value: serde_json::Value,
    }

    #[async_trait]
    impl Stage for DataProducerStage {
        fn name(&self) -> &str {
            &self.name
        }

        async fn execute(&self, _ctx: &StageContext) -> StageOutput {
            StageOutput::ok_value(&self.key, self.value.clone())
        }
    }

    fn test_context() -> Arc<PipelineContext> {
        Arc::new(PipelineContext::new(RunIdentity::new()))
    }

    #[test]
    fn test_pipeline_builder_single_stage() {
        let stage = Arc::new(NoOpStage::new("single"));
        let mut builder = PipelineBuilder::new("test_pipeline");
        builder.add_stage_spec(StageSpec::new("single", stage)).unwrap();

        assert_eq!(builder.name(), "test_pipeline");
        assert_eq!(builder.stage_count(), 1);
    }

    #[test]
    fn test_pipeline_builder_with_dependencies() {
        let stage1 = Arc::new(NoOpStage::new("first"));
        let stage2 = Arc::new(NoOpStage::new("second"));

        let mut builder = PipelineBuilder::new("dep_test");
        builder.add_stage_spec(StageSpec::new("first", stage1)).unwrap();
        builder.add_stage_spec(
            StageSpec::new("second", stage2)
                .with_dependency("first"),
        ).unwrap();

        assert_eq!(builder.stage_count(), 2);
    }

    #[test]
    fn test_pipeline_builder_name() {
        let builder = PipelineBuilder::new("my_pipeline");
        assert_eq!(builder.name(), "my_pipeline");
    }

    #[test]
    fn test_retry_config_exponential() {
        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_base_delay_ms(100)
            .with_max_delay_ms(5000)
            .with_backoff(BackoffStrategy::Exponential);

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay_ms, 100);
    }

    #[test]
    fn test_retry_config_linear() {
        let config = RetryConfig::new()
            .with_backoff(BackoffStrategy::Linear);

        assert!(matches!(config.backoff_strategy, BackoffStrategy::Linear));
    }

    #[test]
    fn test_retry_config_with_jitter() {
        let config = RetryConfig::new()
            .with_jitter(JitterStrategy::Full);

        assert!(matches!(config.jitter_strategy, JitterStrategy::Full));
    }

    #[test]
    fn test_failure_mode_variants() {
        let fail_fast = FailureMode::FailFast;
        let continue_on = FailureMode::ContinueOnFailure;
        let best_effort = FailureMode::BestEffort;

        assert!(matches!(fail_fast, FailureMode::FailFast));
        assert!(matches!(continue_on, FailureMode::ContinueOnFailure));
        assert!(matches!(best_effort, FailureMode::BestEffort));
    }

    #[test]
    fn test_stage_spec_validation() {
        let stage = Arc::new(NoOpStage::new("test"));
        let spec = StageSpec::new("test", stage);

        // Should not depend on itself
        let result = spec.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stage_spec_self_dependency_error() {
        let stage = Arc::new(NoOpStage::new("self_dep"));
        let spec = StageSpec::new("self_dep", stage)
            .with_dependency("self_dep");

        let result = spec.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_run_identity_complete() {
        let user_id = uuid::Uuid::new_v4();
        let session_id = uuid::Uuid::new_v4();
        let org_id = uuid::Uuid::new_v4();

        let identity = RunIdentity::new()
            .with_user_id(user_id)
            .with_session_id(session_id)
            .with_org_id(org_id);

        assert_eq!(identity.user_id, Some(user_id));
        assert_eq!(identity.session_id, Some(session_id));
        assert_eq!(identity.org_id, Some(org_id));
    }

    #[test]
    fn test_context_snapshot_serialization() {
        let snapshot = ContextSnapshot::new();
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ContextSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(
            snapshot.run_id.pipeline_run_id,
            deserialized.run_id.pipeline_run_id
        );
    }

    #[test]
    fn test_stage_output_chaining() {
        let output = StageOutput::ok_empty()
            .add_metadata("version", serde_json::json!("1.0"))
            .add_metadata("author", serde_json::json!("test"));

        assert_eq!(output.metadata.len(), 2);
    }

    #[test]
    fn test_counting_stage() {
        let counter = Arc::new(AtomicUsize::new(0));
        let stage = CountingStage {
            name: "counter".to_string(),
            counter: counter.clone(),
        };

        assert_eq!(stage.name(), "counter");
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_failing_stage_variants() {
        let retryable = FailingStage {
            name: "retry".to_string(),
            retryable: true,
        };
        let permanent = FailingStage {
            name: "perm".to_string(),
            retryable: false,
        };

        assert_eq!(retryable.name(), "retry");
        assert_eq!(permanent.name(), "perm");
    }

    #[test]
    fn test_data_producer_stage() {
        let stage = DataProducerStage {
            name: "producer".to_string(),
            key: "result".to_string(),
            value: serde_json::json!({"count": 42}),
        };

        assert_eq!(stage.name(), "producer");
    }

    #[tokio::test]
    async fn test_noop_stage_execution() {
        let stage = NoOpStage::new("noop");
        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let stage_ctx = StageContext::new(
            ctx,
            "noop",
            StageInputs::default(),
            ContextSnapshot::new(),
        );

        let output = stage.execute(&stage_ctx).await;
        assert!(output.is_success());
    }

    #[tokio::test]
    async fn test_counting_stage_execution() {
        let counter = Arc::new(AtomicUsize::new(0));
        let stage = CountingStage {
            name: "counter".to_string(),
            counter: counter.clone(),
        };

        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let stage_ctx = StageContext::new(
            ctx,
            "counter",
            StageInputs::default(),
            ContextSnapshot::new(),
        );

        let output = stage.execute(&stage_ctx).await;
        assert!(output.is_success());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_failing_stage_execution() {
        let stage = FailingStage {
            name: "fail".to_string(),
            retryable: false,
        };

        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let stage_ctx = StageContext::new(
            ctx,
            "fail",
            StageInputs::default(),
            ContextSnapshot::new(),
        );

        let output = stage.execute(&stage_ctx).await;
        assert!(output.is_failure());
        assert!(!output.is_retryable());
    }

    #[tokio::test]
    async fn test_retryable_stage_execution() {
        let stage = FailingStage {
            name: "retry".to_string(),
            retryable: true,
        };

        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let stage_ctx = StageContext::new(
            ctx,
            "retry",
            StageInputs::default(),
            ContextSnapshot::new(),
        );

        let output = stage.execute(&stage_ctx).await;
        assert!(output.is_failure());
        assert!(output.is_retryable());
    }

    #[tokio::test]
    async fn test_data_producer_execution() {
        let stage = DataProducerStage {
            name: "producer".to_string(),
            key: "answer".to_string(),
            value: serde_json::json!(42),
        };

        let ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        let stage_ctx = StageContext::new(
            ctx,
            "producer",
            StageInputs::default(),
            ContextSnapshot::new(),
        );

        let output = stage.execute(&stage_ctx).await;
        assert!(output.is_success());
        assert_eq!(output.get("answer"), Some(&serde_json::json!(42)));
    }
}
