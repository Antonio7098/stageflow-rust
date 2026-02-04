//! Shared contract error metadata types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured metadata for surfaced contract violations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractErrorInfo {
    /// Stable identifier that maps to a runbook or tracker entry.
    pub code: String,
    /// Human-readable description of the issue.
    pub summary: String,
    /// Optional remediation guidance that can be surfaced to users.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
    /// Optional documentation link for deeper troubleshooting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_url: Option<String>,
    /// Arbitrary structured data that helps downstream tooling render rich errors.
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

impl ContractErrorInfo {
    /// Creates a new contract error info.
    #[must_use]
    pub fn new(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            summary: summary.into(),
            fix_hint: None,
            doc_url: None,
            context: HashMap::new(),
        }
    }

    /// Adds a fix hint.
    #[must_use]
    pub fn with_fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }

    /// Adds a documentation URL.
    #[must_use]
    pub fn with_doc_url(mut self, url: impl Into<String>) -> Self {
        self.doc_url = Some(url.into());
        self
    }

    /// Adds context data.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Returns a copy with additional context merged in.
    #[must_use]
    pub fn merge_context(&self, extra: HashMap<String, serde_json::Value>) -> Self {
        let mut merged = self.context.clone();
        merged.extend(extra);
        Self {
            code: self.code.clone(),
            summary: self.summary.clone(),
            fix_hint: self.fix_hint.clone(),
            doc_url: self.doc_url.clone(),
            context: merged,
        }
    }

    /// Serialize the metadata for logging or API responses.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut dict = HashMap::new();
        dict.insert("code".to_string(), serde_json::json!(self.code));
        dict.insert("summary".to_string(), serde_json::json!(self.summary));
        dict.insert("fix_hint".to_string(), serde_json::json!(self.fix_hint));
        dict.insert("doc_url".to_string(), serde_json::json!(self.doc_url));
        dict.insert("context".to_string(), serde_json::json!(self.context));
        dict
    }
}

/// Common contract error codes.
pub mod codes {
    /// Missing dependency error.
    pub const MISSING_DEP: &str = "CONTRACT-004-MISSING_DEP";
    /// Cycle detected error.
    pub const CYCLE: &str = "CONTRACT-004-CYCLE";
    /// Conflict error.
    pub const CONFLICT: &str = "CONTRACT-004-CONFLICT";
    /// Empty pipeline error.
    pub const EMPTY: &str = "CONTRACT-004-EMPTY";
    /// Validation error.
    pub const VALIDATION: &str = "CONTRACT-001-VALIDATION";
    /// Schema mismatch error.
    pub const SCHEMA_MISMATCH: &str = "CONTRACT-002-SCHEMA";
    /// Version mismatch error.
    pub const VERSION_MISMATCH: &str = "CONTRACT-003-VERSION";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_error_info_new() {
        let info = ContractErrorInfo::new("TEST-001", "Test error");
        assert_eq!(info.code, "TEST-001");
        assert_eq!(info.summary, "Test error");
        assert!(info.fix_hint.is_none());
        assert!(info.doc_url.is_none());
        assert!(info.context.is_empty());
    }

    #[test]
    fn test_contract_error_info_builder() {
        let info = ContractErrorInfo::new("TEST-001", "Test error")
            .with_fix_hint("Try this fix")
            .with_doc_url("https://docs.example.com")
            .with_context("stage", serde_json::json!("fetch"));

        assert_eq!(info.fix_hint, Some("Try this fix".to_string()));
        assert_eq!(info.doc_url, Some("https://docs.example.com".to_string()));
        assert_eq!(info.context.get("stage"), Some(&serde_json::json!("fetch")));
    }

    #[test]
    fn test_contract_error_info_merge_context() {
        let info = ContractErrorInfo::new("TEST-001", "Test error")
            .with_context("key1", serde_json::json!("value1"));

        let mut extra = HashMap::new();
        extra.insert("key2".to_string(), serde_json::json!("value2"));

        let merged = info.merge_context(extra);
        assert_eq!(merged.context.get("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(merged.context.get("key2"), Some(&serde_json::json!("value2")));
    }

    #[test]
    fn test_contract_error_info_to_dict() {
        let info = ContractErrorInfo::new("TEST-001", "Test error");
        let dict = info.to_dict();

        assert_eq!(dict.get("code"), Some(&serde_json::json!("TEST-001")));
        assert_eq!(dict.get("summary"), Some(&serde_json::json!("Test error")));
    }

    #[test]
    fn test_contract_error_info_serialization() {
        let info = ContractErrorInfo::new("TEST-001", "Test error")
            .with_fix_hint("Fix it");

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ContractErrorInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, deserialized);
    }
}
