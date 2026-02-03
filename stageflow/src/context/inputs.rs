//! Stage inputs with strictness enforcement.

use crate::errors::UndeclaredDependencyError;
use std::collections::{HashMap, HashSet};

/// Provides an immutable view of prior stage outputs.
///
/// In strict mode, accessing undeclared dependencies raises an error.
#[derive(Debug, Clone)]
pub struct StageInputs {
    /// The available outputs from prior stages.
    outputs: HashMap<String, HashMap<String, serde_json::Value>>,
    /// The declared dependencies for this stage.
    declared_dependencies: HashSet<String>,
    /// The name of the current stage (for error messages).
    stage_name: String,
    /// Whether strict mode is enabled.
    strict: bool,
}

impl StageInputs {
    /// Creates new stage inputs.
    #[must_use]
    pub fn new(
        outputs: HashMap<String, HashMap<String, serde_json::Value>>,
        declared_dependencies: HashSet<String>,
        stage_name: impl Into<String>,
        strict: bool,
    ) -> Self {
        Self {
            outputs,
            declared_dependencies,
            stage_name: stage_name.into(),
            strict,
        }
    }

    /// Creates permissive stage inputs (no strictness).
    #[must_use]
    pub fn permissive(
        outputs: HashMap<String, HashMap<String, serde_json::Value>>,
        stage_name: impl Into<String>,
    ) -> Self {
        Self {
            declared_dependencies: outputs.keys().cloned().collect(),
            outputs,
            stage_name: stage_name.into(),
            strict: false,
        }
    }

    /// Gets output from a specific stage.
    ///
    /// # Errors
    ///
    /// Returns `UndeclaredDependencyError` in strict mode if the stage
    /// is not a declared dependency.
    pub fn get(&self, stage: &str) -> Result<Option<&HashMap<String, serde_json::Value>>, UndeclaredDependencyError> {
        if self.strict && !self.declared_dependencies.contains(stage) {
            return Err(UndeclaredDependencyError::new(&self.stage_name, stage));
        }
        Ok(self.outputs.get(stage))
    }

    /// Gets a specific value from a stage's output.
    ///
    /// # Errors
    ///
    /// Returns `UndeclaredDependencyError` in strict mode if the stage
    /// is not a declared dependency.
    pub fn get_value(&self, stage: &str, key: &str) -> Result<Option<&serde_json::Value>, UndeclaredDependencyError> {
        if self.strict && !self.declared_dependencies.contains(stage) {
            return Err(UndeclaredDependencyError::new(&self.stage_name, stage));
        }
        Ok(self.outputs.get(stage).and_then(|o| o.get(key)))
    }

    /// Gets output from a stage without strictness check.
    #[must_use]
    pub fn get_unchecked(&self, stage: &str) -> Option<&HashMap<String, serde_json::Value>> {
        self.outputs.get(stage)
    }

    /// Checks if output exists for a stage.
    #[must_use]
    pub fn contains(&self, stage: &str) -> bool {
        self.outputs.contains_key(stage)
    }

    /// Returns all available stage names.
    #[must_use]
    pub fn stages(&self) -> Vec<&String> {
        self.outputs.keys().collect()
    }

    /// Returns the declared dependencies.
    #[must_use]
    pub fn declared_dependencies(&self) -> &HashSet<String> {
        &self.declared_dependencies
    }

    /// Returns whether strict mode is enabled.
    #[must_use]
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// Converts all outputs to a flat dictionary.
    #[must_use]
    pub fn to_flat_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        for (stage, outputs) in &self.outputs {
            for (key, value) in outputs {
                result.insert(format!("{stage}.{key}"), value.clone());
            }
        }
        result
    }
}

impl Default for StageInputs {
    fn default() -> Self {
        Self {
            outputs: HashMap::new(),
            declared_dependencies: HashSet::new(),
            stage_name: String::new(),
            strict: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_outputs() -> HashMap<String, HashMap<String, serde_json::Value>> {
        let mut outputs = HashMap::new();
        
        let mut stage1_output = HashMap::new();
        stage1_output.insert("result".to_string(), serde_json::json!("ok"));
        outputs.insert("stage1".to_string(), stage1_output);

        let mut stage2_output = HashMap::new();
        stage2_output.insert("value".to_string(), serde_json::json!(42));
        outputs.insert("stage2".to_string(), stage2_output);

        outputs
    }

    #[test]
    fn test_permissive_access() {
        let inputs = StageInputs::permissive(sample_outputs(), "current");

        assert!(inputs.get("stage1").unwrap().is_some());
        assert!(inputs.get("stage2").unwrap().is_some());
        assert!(inputs.get("stage3").unwrap().is_none());
    }

    #[test]
    fn test_strict_declared_dependency() {
        let mut deps = HashSet::new();
        deps.insert("stage1".to_string());

        let inputs = StageInputs::new(sample_outputs(), deps, "current", true);

        // Declared dependency works
        assert!(inputs.get("stage1").is_ok());

        // Undeclared dependency fails
        let result = inputs.get("stage2");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_value() {
        let inputs = StageInputs::permissive(sample_outputs(), "current");

        let value = inputs.get_value("stage2", "value").unwrap();
        assert_eq!(value, Some(&serde_json::json!(42)));

        let missing = inputs.get_value("stage1", "missing").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_strict_get_value() {
        let mut deps = HashSet::new();
        deps.insert("stage1".to_string());

        let inputs = StageInputs::new(sample_outputs(), deps, "current", true);

        // Declared works
        assert!(inputs.get_value("stage1", "result").is_ok());

        // Undeclared fails
        assert!(inputs.get_value("stage2", "value").is_err());
    }

    #[test]
    fn test_contains() {
        let inputs = StageInputs::permissive(sample_outputs(), "current");

        assert!(inputs.contains("stage1"));
        assert!(!inputs.contains("stage3"));
    }

    #[test]
    fn test_to_flat_dict() {
        let inputs = StageInputs::permissive(sample_outputs(), "current");
        let flat = inputs.to_flat_dict();

        assert!(flat.contains_key("stage1.result"));
        assert!(flat.contains_key("stage2.value"));
    }

    #[test]
    fn test_get_unchecked_bypasses_strict() {
        let inputs = StageInputs::new(sample_outputs(), HashSet::new(), "current", true);

        // Even with no declared deps, unchecked works
        assert!(inputs.get_unchecked("stage1").is_some());
    }
}
