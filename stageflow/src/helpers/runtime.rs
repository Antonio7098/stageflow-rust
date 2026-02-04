//! Runtime helpers for pipeline execution.
//!
//! These helpers provide utilities for running pipelines with proper
//! error handling, timeouts, and cleanup.

use std::time::Duration;
use tokio::time::timeout;

/// Timeout configuration for pipeline execution.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Overall pipeline timeout.
    pub pipeline_timeout: Option<Duration>,
    /// Per-stage timeout.
    pub stage_timeout: Option<Duration>,
    /// Cleanup timeout.
    pub cleanup_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            pipeline_timeout: None,
            stage_timeout: None,
            cleanup_timeout: Duration::from_secs(10),
        }
    }
}

impl TimeoutConfig {
    /// Creates a new timeout configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the pipeline timeout.
    #[must_use]
    pub fn with_pipeline_timeout(mut self, timeout: Duration) -> Self {
        self.pipeline_timeout = Some(timeout);
        self
    }

    /// Sets the stage timeout.
    #[must_use]
    pub fn with_stage_timeout(mut self, timeout: Duration) -> Self {
        self.stage_timeout = Some(timeout);
        self
    }

    /// Sets the cleanup timeout.
    #[must_use]
    pub fn with_cleanup_timeout(mut self, timeout: Duration) -> Self {
        self.cleanup_timeout = timeout;
        self
    }
}

/// Result of a timed operation.
#[derive(Debug)]
pub enum TimedResult<T, E> {
    /// Operation completed successfully.
    Ok(T),
    /// Operation failed with an error.
    Err(E),
    /// Operation timed out.
    Timeout,
}

impl<T, E> TimedResult<T, E> {
    /// Returns true if the operation succeeded.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, TimedResult::Ok(_))
    }

    /// Returns true if the operation failed.
    #[must_use]
    pub fn is_err(&self) -> bool {
        matches!(self, TimedResult::Err(_))
    }

    /// Returns true if the operation timed out.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self, TimedResult::Timeout)
    }

    /// Converts to a standard Result, treating timeout as an error.
    pub fn into_result(self, timeout_error: E) -> Result<T, E> {
        match self {
            TimedResult::Ok(v) => Ok(v),
            TimedResult::Err(e) => Err(e),
            TimedResult::Timeout => Err(timeout_error),
        }
    }
}

/// Runs a future with a timeout.
pub async fn run_with_timeout<T, E, F>(
    duration: Duration,
    future: F,
) -> TimedResult<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    match timeout(duration, future).await {
        Ok(Ok(value)) => TimedResult::Ok(value),
        Ok(Err(error)) => TimedResult::Err(error),
        Err(_) => TimedResult::Timeout,
    }
}

/// Runs a cleanup function with a timeout, suppressing errors.
pub async fn run_cleanup_with_timeout<F, Fut>(
    duration: Duration,
    cleanup: F,
) -> bool
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    match timeout(duration, cleanup()).await {
        Ok(()) => true,
        Err(_) => {
            tracing::warn!("Cleanup timed out after {:?}", duration);
            false
        }
    }
}

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of attempts.
    pub max_attempts: usize,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
    /// Whether to add jitter.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum attempts.
    #[must_use]
    pub fn with_max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Sets the initial delay.
    #[must_use]
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Calculates the delay for a given attempt.
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let base_delay = self.initial_delay.as_secs_f64() 
            * self.backoff_multiplier.powi(attempt as i32);
        let capped = base_delay.min(self.max_delay.as_secs_f64());
        
        let final_delay = if self.jitter {
            // Add up to 25% jitter
            let jitter = capped * 0.25 * rand::random::<f64>();
            capped + jitter
        } else {
            capped
        };
        
        Duration::from_secs_f64(final_delay)
    }
}

/// Runs a future with retries.
pub async fn run_with_retry<T, E, F, Fut>(
    policy: &RetryPolicy,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error: Option<E> = None;

    for attempt in 0..policy.max_attempts {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt + 1 < policy.max_attempts {
                    let delay = policy.delay_for_attempt(attempt);
                    tracing::debug!(
                        "Attempt {} failed: {}. Retrying in {:?}",
                        attempt + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                last_error = Some(e);
            }
        }
    }

    Err(last_error.expect("At least one attempt should have been made"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert!(config.pipeline_timeout.is_none());
        assert!(config.stage_timeout.is_none());
        assert_eq!(config.cleanup_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_timeout_config_builder() {
        let config = TimeoutConfig::new()
            .with_pipeline_timeout(Duration::from_secs(60))
            .with_stage_timeout(Duration::from_secs(10));

        assert_eq!(config.pipeline_timeout, Some(Duration::from_secs(60)));
        assert_eq!(config.stage_timeout, Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_timed_result() {
        let ok: TimedResult<i32, &str> = TimedResult::Ok(42);
        assert!(ok.is_ok());
        assert!(!ok.is_err());
        assert!(!ok.is_timeout());

        let err: TimedResult<i32, &str> = TimedResult::Err("error");
        assert!(!err.is_ok());
        assert!(err.is_err());

        let timeout: TimedResult<i32, &str> = TimedResult::Timeout;
        assert!(timeout.is_timeout());
    }

    #[test]
    fn test_timed_result_into_result() {
        let ok: TimedResult<i32, &str> = TimedResult::Ok(42);
        assert_eq!(ok.into_result("timeout"), Ok(42));

        let err: TimedResult<i32, &str> = TimedResult::Err("error");
        assert_eq!(err.into_result("timeout"), Err("error"));

        let timeout: TimedResult<i32, &str> = TimedResult::Timeout;
        assert_eq!(timeout.into_result("timeout"), Err("timeout"));
    }

    #[tokio::test]
    async fn test_run_with_timeout_success() {
        let result: TimedResult<i32, &str> = run_with_timeout(
            Duration::from_secs(1),
            async { Ok(42) },
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_with_timeout_error() {
        let result: TimedResult<i32, &str> = run_with_timeout(
            Duration::from_secs(1),
            async { Err("error") },
        ).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_with_timeout_timeout() {
        let result: TimedResult<i32, &str> = run_with_timeout(
            Duration::from_millis(10),
            async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(42)
            },
        ).await;

        assert!(result.is_timeout());
    }

    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_retry_policy_delay() {
        let policy = RetryPolicy::new()
            .with_initial_delay(Duration::from_secs(1));
        
        // Without jitter for predictable testing
        let policy = RetryPolicy {
            jitter: false,
            ..policy
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
    }

    #[tokio::test]
    async fn test_run_with_retry_success_first() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let policy = RetryPolicy::new();
        let result: Result<i32, &str> = run_with_retry(&policy, || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(42)
            }
        }).await;

        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_run_with_retry_success_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_multiplier: 1.0,
            jitter: false,
        };

        let result: Result<i32, String> = run_with_retry(&policy, || {
            let c = counter_clone.clone();
            async move {
                let count = c.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(format!("attempt {}", count))
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_run_with_retry_all_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_multiplier: 1.0,
            jitter: false,
        };

        let result: Result<i32, String> = run_with_retry(&policy, || {
            let c = counter_clone.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err("always fails".to_string())
            }
        }).await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
