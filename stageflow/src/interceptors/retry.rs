//! Retry interceptor with backoff strategies.

use super::Interceptor;
use crate::context::{ExecutionContext, StageContext};
use crate::core::StageOutput;
use async_trait::async_trait;
use rand::Rng;
use std::time::Duration;

/// Backoff strategy for retries.
#[derive(Debug, Clone, Copy)]
pub enum BackoffStrategy {
    /// Constant delay between retries.
    Constant(Duration),
    /// Linear increase: delay * attempt.
    Linear(Duration),
    /// Exponential: delay * 2^attempt.
    Exponential(Duration),
}

impl BackoffStrategy {
    /// Calculates the delay for a given attempt.
    #[must_use]
    pub fn delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Constant(d) => *d,
            Self::Linear(d) => *d * attempt,
            Self::Exponential(d) => *d * 2u32.pow(attempt.saturating_sub(1)),
        }
    }
}

/// Jitter strategy for adding randomness to delays.
#[derive(Debug, Clone, Copy)]
pub enum JitterStrategy {
    /// No jitter.
    None,
    /// Full jitter: [0, delay].
    Full,
    /// Equal jitter: [delay/2, delay].
    Equal,
    /// Decorrelated jitter.
    Decorrelated,
}

impl JitterStrategy {
    /// Applies jitter to a delay.
    #[must_use]
    pub fn apply(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();

        match self {
            Self::None => delay,
            Self::Full => {
                let millis = delay.as_millis() as u64;
                Duration::from_millis(rng.gen_range(0..=millis))
            }
            Self::Equal => {
                let millis = delay.as_millis() as u64;
                let half = millis / 2;
                Duration::from_millis(half + rng.gen_range(0..=half))
            }
            Self::Decorrelated => {
                let millis = delay.as_millis() as u64;
                Duration::from_millis(rng.gen_range(millis..=millis * 3))
            }
        }
    }
}

/// Interceptor that retries failed stages.
pub struct RetryInterceptor {
    /// Maximum number of retry attempts.
    max_attempts: u32,
    /// Backoff strategy.
    backoff: BackoffStrategy,
    /// Jitter strategy.
    jitter: JitterStrategy,
}

impl RetryInterceptor {
    /// Creates a new retry interceptor.
    #[must_use]
    pub fn new(max_attempts: u32, backoff: BackoffStrategy, jitter: JitterStrategy) -> Self {
        Self {
            max_attempts,
            backoff,
            jitter,
        }
    }

    /// Creates a simple retry interceptor with constant delay.
    #[must_use]
    pub fn constant(max_attempts: u32, delay: Duration) -> Self {
        Self::new(max_attempts, BackoffStrategy::Constant(delay), JitterStrategy::None)
    }

    /// Creates an exponential backoff retry interceptor.
    #[must_use]
    pub fn exponential(max_attempts: u32, base_delay: Duration) -> Self {
        Self::new(
            max_attempts,
            BackoffStrategy::Exponential(base_delay),
            JitterStrategy::Full,
        )
    }

    /// Calculates delay for an attempt.
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.backoff.delay(attempt);
        self.jitter.apply(base_delay)
    }
}

#[async_trait]
impl Interceptor for RetryInterceptor {
    fn priority(&self) -> i32 {
        100 // Run after most other interceptors
    }

    async fn after(&self, ctx: &StageContext, output: StageOutput) -> StageOutput {
        if !output.is_retryable() {
            return output;
        }

        // In a real implementation, we'd track attempts and retry
        // For now, just emit the event and return
        ctx.try_emit_event(
            "stage.retry_scheduled",
            Some(serde_json::json!({
                "stage": ctx.stage_name(),
                "max_attempts": self.max_attempts,
            })),
        );

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_backoff() {
        let strategy = BackoffStrategy::Constant(Duration::from_secs(1));
        assert_eq!(strategy.delay(1), Duration::from_secs(1));
        assert_eq!(strategy.delay(5), Duration::from_secs(1));
    }

    #[test]
    fn test_linear_backoff() {
        let strategy = BackoffStrategy::Linear(Duration::from_secs(1));
        assert_eq!(strategy.delay(1), Duration::from_secs(1));
        assert_eq!(strategy.delay(3), Duration::from_secs(3));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = BackoffStrategy::Exponential(Duration::from_secs(1));
        assert_eq!(strategy.delay(1), Duration::from_secs(1));
        assert_eq!(strategy.delay(2), Duration::from_secs(2));
        assert_eq!(strategy.delay(3), Duration::from_secs(4));
        assert_eq!(strategy.delay(4), Duration::from_secs(8));
    }

    #[test]
    fn test_no_jitter() {
        let jitter = JitterStrategy::None;
        let delay = Duration::from_secs(10);
        assert_eq!(jitter.apply(delay), delay);
    }

    #[test]
    fn test_full_jitter_bounds() {
        let jitter = JitterStrategy::Full;
        let delay = Duration::from_secs(10);

        // Run multiple times to check bounds
        for _ in 0..100 {
            let result = jitter.apply(delay);
            assert!(result <= delay);
        }
    }

    #[test]
    fn test_retry_interceptor_creation() {
        let interceptor = RetryInterceptor::exponential(3, Duration::from_millis(100));
        assert_eq!(interceptor.max_attempts, 3);
    }
}
