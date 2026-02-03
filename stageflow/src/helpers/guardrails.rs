//! Guardrails SDK for content safety.

use serde::{Deserialize, Serialize};

/// Violation type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    PiiDetected,
    Profanity,
    Toxicity,
    ContentTooLong,
    RateLimited,
    BlockedTopic,
    InjectionAttempt,
    Custom,
}

/// A policy violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    #[serde(rename = "type")]
    pub violation_type: ViolationType,
    pub message: String,
    pub severity: f64,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub location: Option<(usize, usize)>,
}

/// Result of a guardrail check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    pub passed: bool,
    pub violations: Vec<PolicyViolation>,
    pub transformed_content: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl GuardrailResult {
    /// Creates a passing result.
    #[must_use]
    pub fn pass() -> Self {
        Self { passed: true, violations: Vec::new(), transformed_content: None, metadata: std::collections::HashMap::new() }
    }
}

/// PII detector.
pub struct PIIDetector {
    detect_types: Vec<String>,
    redact: bool,
}

impl PIIDetector {
    /// Creates a new PII detector.
    #[must_use]
    pub fn new(detect_types: Vec<String>, redact: bool) -> Self {
        Self { detect_types, redact }
    }
}

/// Content filter for profanity and blocked topics.
pub struct ContentFilter {
    profanity_words: Vec<String>,
    blocked_patterns: Vec<String>,
}

impl ContentFilter {
    /// Creates a new content filter.
    #[must_use]
    pub fn new() -> Self {
        Self { profanity_words: Vec::new(), blocked_patterns: Vec::new() }
    }
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Injection attempt detector.
pub struct InjectionDetector {
    additional_patterns: Vec<String>,
}

impl InjectionDetector {
    /// Creates a new injection detector.
    #[must_use]
    pub fn new() -> Self {
        Self { additional_patterns: Vec::new() }
    }
}

impl Default for InjectionDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Guardrail stage for pipeline integration.
pub struct GuardrailStage {
    content_key: Option<String>,
    fail_on_violation: bool,
}

impl GuardrailStage {
    /// Creates a new guardrail stage.
    #[must_use]
    pub fn new() -> Self {
        Self { content_key: None, fail_on_violation: true }
    }
}

impl Default for GuardrailStage {
    fn default() -> Self {
        Self::new()
    }
}
