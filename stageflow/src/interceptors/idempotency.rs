//! Idempotency interceptor for WORK stages.

use super::Interceptor;
use crate::context::{ExecutionContext, StageContext};
use crate::core::StageOutput;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Entry in the idempotency store.
struct IdempotencyEntry {
    output: StageOutput,
    created_at: Instant,
}

/// Store for idempotency keys.
pub struct IdempotencyStore {
    entries: DashMap<String, IdempotencyEntry>,
    ttl: Duration,
}

impl IdempotencyStore {
    /// Creates a new store.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
        }
    }

    /// Gets a cached result.
    pub fn get(&self, key: &str) -> Option<StageOutput> {
        if let Some(entry) = self.entries.get(key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.output.clone());
            }
            // Expired
            drop(entry);
            self.entries.remove(key);
        }
        None
    }

    /// Stores a result.
    pub fn set(&self, key: String, output: StageOutput) {
        self.entries.insert(
            key,
            IdempotencyEntry {
                output,
                created_at: Instant::now(),
            },
        );
    }

    /// Clears all entries.
    pub fn clear(&self) {
        self.entries.clear();
    }
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600))
    }
}

/// Interceptor that enforces idempotent execution of WORK stages.
pub struct IdempotencyInterceptor {
    store: Arc<IdempotencyStore>,
}

impl IdempotencyInterceptor {
    /// Creates a new idempotency interceptor.
    #[must_use]
    pub fn new(store: Arc<IdempotencyStore>) -> Self {
        Self { store }
    }

    /// Generates an idempotency key for a stage execution.
    fn generate_key(&self, ctx: &StageContext) -> String {
        let pipeline_run_id = ctx
            .pipeline_run_id()
            .map(|id| id.to_string())
            .unwrap_or_default();

        format!("{}:{}", pipeline_run_id, ctx.stage_name())
    }
}

#[async_trait]
impl Interceptor for IdempotencyInterceptor {
    fn priority(&self) -> i32 {
        -100 // Run early
    }

    async fn before(&self, ctx: &StageContext) -> Option<StageOutput> {
        let key = self.generate_key(ctx);

        if let Some(cached) = self.store.get(&key) {
            ctx.try_emit_event(
                "stage.idempotency_hit",
                Some(serde_json::json!({
                    "stage": ctx.stage_name(),
                    "key": key,
                })),
            );
            return Some(cached);
        }

        None
    }

    async fn after(&self, ctx: &StageContext, output: StageOutput) -> StageOutput {
        if output.is_success() {
            let key = self.generate_key(ctx);
            self.store.set(key, output.clone());
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ContextSnapshot, PipelineContext, RunIdentity, StageInputs};

    fn test_stage_context() -> StageContext {
        let pipeline_ctx = Arc::new(PipelineContext::new(RunIdentity::new()));
        StageContext::new(
            pipeline_ctx,
            "test",
            StageInputs::default(),
            ContextSnapshot::new(),
        )
    }

    #[test]
    fn test_idempotency_store() {
        let store = IdempotencyStore::new(Duration::from_secs(60));

        assert!(store.get("key1").is_none());

        store.set("key1".to_string(), StageOutput::ok_empty());

        assert!(store.get("key1").is_some());
    }

    #[test]
    fn test_idempotency_store_expiry() {
        let store = IdempotencyStore::new(Duration::from_millis(1));

        store.set("key1".to_string(), StageOutput::ok_empty());

        std::thread::sleep(Duration::from_millis(10));

        assert!(store.get("key1").is_none());
    }

    #[tokio::test]
    async fn test_interceptor_caches_success() {
        let store = Arc::new(IdempotencyStore::new(Duration::from_secs(60)));
        let interceptor = IdempotencyInterceptor::new(store.clone());

        let ctx = test_stage_context();

        // First call - no cache
        let before_result = interceptor.before(&ctx).await;
        assert!(before_result.is_none());

        // After with success
        let output = StageOutput::ok_value("result", serde_json::json!(42));
        interceptor.after(&ctx, output.clone()).await;

        // Second call - should hit cache
        let before_result = interceptor.before(&ctx).await;
        assert!(before_result.is_some());
    }
}
