//! Guard retry strategy utilities for UnifiedStageGraph.

use super::StageSpec;
use crate::core::{StageKind, StageOutput};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

/// Policy describing how to retry when a guard stage fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRetryPolicy {
    /// The stage to re-execute when guard fails.
    pub retry_stage: String,
    /// Maximum retry attempts (default: 2).
    pub max_attempts: usize,
    /// Number of consecutive identical results before giving up (default: 2).
    pub stagnation_limit: usize,
    /// Optional fields to hash for stagnation detection.
    pub hash_fields: Option<Vec<String>>,
    /// Optional timeout in seconds.
    pub timeout_seconds: Option<f64>,
}

impl GuardRetryPolicy {
    /// Creates a new guard retry policy.
    pub fn new(retry_stage: impl Into<String>) -> Self {
        Self {
            retry_stage: retry_stage.into(),
            max_attempts: 2,
            stagnation_limit: 2,
            hash_fields: None,
            timeout_seconds: None,
        }
    }

    /// Sets the maximum attempts.
    #[must_use]
    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the stagnation limit.
    #[must_use]
    pub fn with_stagnation_limit(mut self, limit: usize) -> Self {
        self.stagnation_limit = limit;
        self
    }

    /// Sets the hash fields for stagnation detection.
    #[must_use]
    pub fn with_hash_fields(mut self, fields: Vec<String>) -> Self {
        self.hash_fields = Some(fields);
        self
    }

    /// Sets the timeout.
    #[must_use]
    pub fn with_timeout(mut self, seconds: f64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// Validates the policy configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts < 1 {
            return Err("max_attempts must be >= 1".to_string());
        }
        if self.stagnation_limit < 1 {
            return Err("stagnation_limit must be >= 1".to_string());
        }
        if let Some(timeout) = self.timeout_seconds {
            if timeout <= 0.0 {
                return Err("timeout_seconds must be positive when provided".to_string());
            }
        }
        Ok(())
    }
}

/// Collection of guard retry policies keyed by guard stage name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuardRetryStrategy {
    /// Policies keyed by guard stage name.
    pub policies: HashMap<String, GuardRetryPolicy>,
}

impl GuardRetryStrategy {
    /// Creates a new empty strategy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Adds a policy for a guard stage.
    #[must_use]
    pub fn with_policy(mut self, guard_stage: impl Into<String>, policy: GuardRetryPolicy) -> Self {
        self.policies.insert(guard_stage.into(), policy);
        self
    }

    /// Gets the policy for a guard stage.
    #[must_use]
    pub fn get_policy(&self, guard_stage: &str) -> Option<&GuardRetryPolicy> {
        self.policies.get(guard_stage)
    }

    /// Validates the strategy against the stage specifications.
    pub fn validate<S: StageSpecLike>(&self, stages: &HashMap<String, S>) -> Result<(), String> {
        for (guard_name, policy) in &self.policies {
            let guard_spec = stages.get(guard_name).ok_or_else(|| {
                format!(
                    "Guard retry policy references unknown guard stage '{}'",
                    guard_name
                )
            })?;

            if guard_spec.kind() != Some(StageKind::Guard) {
                return Err(format!(
                    "Guard retry policy requires '{}' to be a GUARD stage",
                    guard_name
                ));
            }

            if !stages.contains_key(&policy.retry_stage) {
                return Err(format!(
                    "Guard retry policy for '{}' references unknown retry stage '{}'",
                    guard_name, policy.retry_stage
                ));
            }

            if policy.retry_stage == *guard_name {
                return Err(format!(
                    "Guard retry policy for '{}' cannot target itself",
                    guard_name
                ));
            }

            let guard_deps = guard_spec.dependencies();
            if !guard_deps.contains(&policy.retry_stage) {
                return Err(format!(
                    "Guard '{}' must declare retry stage '{}' as a dependency to enable guard retries",
                    guard_name, policy.retry_stage
                ));
            }
        }
        Ok(())
    }
}

/// Trait for stage specifications that can be validated.
pub trait StageSpecLike {
    /// Returns the stage kind.
    fn kind(&self) -> Option<StageKind>;
    /// Returns the stage dependencies.
    fn dependencies(&self) -> Vec<String>;
}

/// Runtime state for guard retry tracking.
#[derive(Debug, Clone, Default)]
pub struct GuardRetryRuntimeState {
    /// Number of retry attempts made.
    pub attempts: usize,
    /// Number of consecutive stagnant results.
    pub stagnation_hits: usize,
    /// Hash of the last output for stagnation detection.
    pub last_hash: Option<String>,
    /// Timestamp when retrying started.
    pub started_at: Option<Instant>,
}

impl GuardRetryRuntimeState {
    /// Creates a new runtime state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StageSpecLike for StageSpec {
    fn kind(&self) -> Option<StageKind> {
        Some(self.kind)
    }

    fn dependencies(&self) -> Vec<String> {
        self.dependencies.iter().cloned().collect()
    }
}

/// Builds a stable hash for stagnation detection.
#[must_use]
pub fn hash_retry_payload(
    output: Option<&StageOutput>,
    fields: Option<&[String]>,
) -> Option<String> {
    let output = output?;
    let data = output.data.as_ref()?;

    let payload: serde_json::Value = if let Some(fields) = fields {
        let filtered: serde_json::Map<String, serde_json::Value> = fields
            .iter()
            .filter_map(|field| {
                data.get(field).map(|v| (field.clone(), v.clone()))
            })
            .collect();
        serde_json::Value::Object(filtered)
    } else {
        serde_json::Value::Object(
            data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
    };

    let serialized = serde_json::to_string(&payload).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    Some(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_retry_policy_defaults() {
        let policy = GuardRetryPolicy::new("retry_stage");
        assert_eq!(policy.max_attempts, 2);
        assert_eq!(policy.stagnation_limit, 2);
        assert!(policy.hash_fields.is_none());
        assert!(policy.timeout_seconds.is_none());
    }

    #[test]
    fn test_guard_retry_policy_validation() {
        let policy = GuardRetryPolicy::new("retry").with_max_attempts(0);
        assert!(policy.validate().is_err());

        let policy = GuardRetryPolicy::new("retry").with_stagnation_limit(0);
        assert!(policy.validate().is_err());

        let policy = GuardRetryPolicy::new("retry").with_timeout(-1.0);
        assert!(policy.validate().is_err());

        let policy = GuardRetryPolicy::new("retry");
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_hash_retry_payload() {
        let output = StageOutput::ok(
            [("key".to_string(), serde_json::json!("value"))]
                .into_iter()
                .collect(),
        );

        let hash1 = hash_retry_payload(Some(&output), None);
        let hash2 = hash_retry_payload(Some(&output), None);
        assert!(hash1.is_some());
        assert_eq!(hash1, hash2);

        // Different output should have different hash
        let output2 = StageOutput::ok(
            [("key".to_string(), serde_json::json!("other"))]
                .into_iter()
                .collect(),
        );
        let hash3 = hash_retry_payload(Some(&output2), None);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_with_fields() {
        let output = StageOutput::ok(
            [
                ("a".to_string(), serde_json::json!(1)),
                ("b".to_string(), serde_json::json!(2)),
            ]
            .into_iter()
            .collect(),
        );

        let hash_all = hash_retry_payload(Some(&output), None);
        let hash_a = hash_retry_payload(Some(&output), Some(&["a".to_string()]));

        assert!(hash_all.is_some());
        assert!(hash_a.is_some());
        assert_ne!(hash_all, hash_a);
    }
}
