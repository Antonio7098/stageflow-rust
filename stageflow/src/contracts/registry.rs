//! Schema registry utilities for stage contract management.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Registered metadata for a stage contract version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    /// Stage name.
    pub stage: String,
    /// Contract version.
    pub version: String,
    /// JSON schema for the contract.
    pub schema: serde_json::Value,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// When the contract was registered.
    pub created_at: DateTime<Utc>,
}

impl ContractMetadata {
    /// Creates new contract metadata.
    #[must_use]
    pub fn new(
        stage: impl Into<String>,
        version: impl Into<String>,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            stage: stage.into(),
            version: version.into(),
            schema,
            description: None,
            created_at: Utc::now(),
        }
    }

    /// Adds a description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Simple compatibility diff between two contract versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCompatibilityReport {
    /// Stage name.
    pub stage: String,
    /// Source version.
    pub from_version: String,
    /// Target version.
    pub to_version: String,
    /// Breaking changes detected.
    pub breaking_changes: Vec<String>,
    /// Non-breaking warnings.
    pub warnings: Vec<String>,
}

impl ContractCompatibilityReport {
    /// True when no breaking changes were detected.
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        self.breaking_changes.is_empty()
    }

    /// Human readable summary string.
    #[must_use]
    pub fn summary(&self) -> String {
        let status = if self.is_compatible() {
            "compatible"
        } else {
            "breaking"
        };
        format!(
            "Contract diff for {} {}->{}: {} (breaking={}, warnings={})",
            self.stage,
            self.from_version,
            self.to_version,
            status,
            self.breaking_changes.len(),
            self.warnings.len()
        )
    }
}

/// In-memory registry of stage contract schemas.
#[derive(Debug, Default)]
pub struct ContractRegistry {
    entries: RwLock<HashMap<(String, String), ContractMetadata>>,
}

impl ContractRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove all registered entries (primarily for tests).
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Register or update metadata for a stage/version pair.
    pub fn register(
        &self,
        stage: impl Into<String>,
        version: impl Into<String>,
        schema: serde_json::Value,
        description: Option<String>,
    ) -> Result<ContractMetadata, String> {
        let stage = stage.into();
        let version = version.into();
        let key = (stage.clone(), version.clone());

        let mut entries = self.entries.write();
        if let Some(existing) = entries.get(&key) {
            if existing.schema == schema {
                return Ok(existing.clone());
            }
            return Err(format!(
                "Contract {}@{} already registered with a different schema",
                stage, version
            ));
        }

        let mut metadata = ContractMetadata::new(&stage, &version, schema);
        if let Some(desc) = description {
            metadata = metadata.with_description(desc);
        }
        entries.insert(key, metadata.clone());
        Ok(metadata)
    }

    /// Fetch metadata for a given stage/version.
    #[must_use]
    pub fn get(&self, stage: &str, version: &str) -> Option<ContractMetadata> {
        self.entries
            .read()
            .get(&(stage.to_string(), version.to_string()))
            .cloned()
    }

    /// Return all registrations, optionally filtered by stage.
    #[must_use]
    pub fn list(&self, stage: Option<&str>) -> Vec<ContractMetadata> {
        let entries = self.entries.read();
        let mut result: Vec<_> = entries
            .values()
            .filter(|m| stage.is_none() || stage == Some(m.stage.as_str()))
            .cloned()
            .collect();
        result.sort_by(|a, b| (&a.stage, &a.version).cmp(&(&b.stage, &b.version)));
        result
    }

    /// Compute compatibility between two versions of a stage contract.
    pub fn diff(
        &self,
        stage: &str,
        from_version: &str,
        to_version: &str,
    ) -> Result<ContractCompatibilityReport, String> {
        let left = self.get(stage, from_version).ok_or_else(|| {
            format!("Contract {}@{} not registered", stage, from_version)
        })?;
        let right = self.get(stage, to_version).ok_or_else(|| {
            format!("Contract {}@{} not registered", stage, to_version)
        })?;

        let mut breaking = Vec::new();
        let mut warnings = Vec::new();

        let left_fields = field_map(&left.schema);
        let right_fields = field_map(&right.schema);

        // Removed fields -> breaking
        for field in left_fields.keys() {
            if !right_fields.contains_key(field) {
                breaking.push(format!("Field '{}' removed", field));
            }
        }

        // Added fields -> warning unless required
        for (field, meta) in &right_fields {
            if !left_fields.contains_key(field) {
                if meta.required {
                    breaking.push(format!("Required field '{}' added", field));
                } else {
                    warnings.push(format!("Optional field '{}' added", field));
                }
                continue;
            }

            let left_meta = &left_fields[field];
            if !types_compatible(&left_meta.types, &meta.types) {
                breaking.push(format!(
                    "Field '{}' changed types {:?} -> {:?}",
                    field, left_meta.types, meta.types
                ));
            }
        }

        Ok(ContractCompatibilityReport {
            stage: stage.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            breaking_changes: breaking,
            warnings,
        })
    }

    /// Returns the number of registered contracts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Returns true if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

#[derive(Debug, Clone)]
struct FieldInfo {
    types: HashSet<String>,
    required: bool,
}

fn field_map(schema: &serde_json::Value) -> HashMap<String, FieldInfo> {
    let mut field_info = HashMap::new();

    let props = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .cloned()
        .unwrap_or_default();

    let required: HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    for (name, meta) in props {
        let types = match meta.get("type") {
            Some(serde_json::Value::String(s)) => {
                let mut set = HashSet::new();
                set.insert(s.clone());
                set
            }
            Some(serde_json::Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect(),
            _ => {
                if meta.get("properties").is_some() {
                    let mut set = HashSet::new();
                    set.insert("object".to_string());
                    set
                } else {
                    HashSet::new()
                }
            }
        };

        field_info.insert(
            name.clone(),
            FieldInfo {
                types,
                required: required.contains(&name),
            },
        );
    }

    field_info
}

fn types_compatible(old: &HashSet<String>, new: &HashSet<String>) -> bool {
    if old.is_empty() {
        return true;
    }
    if new.is_superset(old) {
        return true;
    }
    // Allow widening to include "null"
    let mut old_with_null = old.clone();
    old_with_null.insert("null".to_string());
    old == &old_with_null.intersection(new).cloned().collect::<HashSet<_>>()
        || new.is_superset(&old_with_null)
}

/// Global contract registry.
pub static REGISTRY: std::sync::LazyLock<Arc<ContractRegistry>> =
    std::sync::LazyLock::new(|| Arc::new(ContractRegistry::new()));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_registry_register() {
        let registry = ContractRegistry::new();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let result = registry.register("test_stage", "1.0.0", schema.clone(), None);
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.stage, "test_stage");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_contract_registry_duplicate() {
        let registry = ContractRegistry::new();
        let schema1 = serde_json::json!({"type": "object"});
        let schema2 = serde_json::json!({"type": "string"});

        registry.register("stage", "1.0", schema1, None).unwrap();
        let result = registry.register("stage", "1.0", schema2, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_contract_registry_get() {
        let registry = ContractRegistry::new();
        let schema = serde_json::json!({"type": "object"});

        registry.register("stage", "1.0", schema, None).unwrap();

        let metadata = registry.get("stage", "1.0");
        assert!(metadata.is_some());

        let missing = registry.get("stage", "2.0");
        assert!(missing.is_none());
    }

    #[test]
    fn test_contract_registry_list() {
        let registry = ContractRegistry::new();
        let schema = serde_json::json!({"type": "object"});

        registry
            .register("stage_a", "1.0", schema.clone(), None)
            .unwrap();
        registry
            .register("stage_b", "1.0", schema.clone(), None)
            .unwrap();
        registry.register("stage_a", "2.0", schema, None).unwrap();

        let all = registry.list(None);
        assert_eq!(all.len(), 3);

        let stage_a = registry.list(Some("stage_a"));
        assert_eq!(stage_a.len(), 2);
    }

    #[test]
    fn test_contract_registry_diff_compatible() {
        let registry = ContractRegistry::new();
        let schema1 = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let schema2 = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        registry
            .register("user", "1.0", schema1, None)
            .unwrap();
        registry
            .register("user", "2.0", schema2, None)
            .unwrap();

        let report = registry.diff("user", "1.0", "2.0").unwrap();
        assert!(report.is_compatible());
        assert_eq!(report.warnings.len(), 1); // Optional field added
    }

    #[test]
    fn test_contract_registry_diff_breaking() {
        let registry = ContractRegistry::new();
        let schema1 = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        });
        let schema2 = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        registry
            .register("user", "1.0", schema1, None)
            .unwrap();
        registry
            .register("user", "2.0", schema2, None)
            .unwrap();

        let report = registry.diff("user", "1.0", "2.0").unwrap();
        assert!(!report.is_compatible());
        assert!(report.breaking_changes.iter().any(|c| c.contains("email")));
    }

    #[test]
    fn test_compatibility_report_summary() {
        let report = ContractCompatibilityReport {
            stage: "test".to_string(),
            from_version: "1.0".to_string(),
            to_version: "2.0".to_string(),
            breaking_changes: vec!["Field removed".to_string()],
            warnings: vec![],
        };

        assert!(report.summary().contains("breaking"));
        assert!(!report.is_compatible());
    }
}
