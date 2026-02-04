//! Validation utilities for pipeline configuration.
//!
//! These utilities help validate stage configurations, dependencies,
//! and detect common issues like cycles.

use std::collections::{HashMap, HashSet};

/// Validates that dependencies form a valid DAG (no cycles).
pub fn validate_dag<S: AsRef<str>>(
    stages: &HashMap<String, Vec<S>>,
) -> Result<Vec<String>, CycleError> {
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();
    let mut order = Vec::new();
    let mut path = Vec::new();

    fn dfs<S: AsRef<str>>(
        node: &str,
        stages: &HashMap<String, Vec<S>>,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        order: &mut Vec<String>,
        path: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        if in_stack.contains(node) {
            // Found a cycle - build the cycle path
            let start_idx = path.iter().position(|n| n == node).unwrap_or(0);
            let mut cycle_path: Vec<String> = path[start_idx..].to_vec();
            cycle_path.push(node.to_string());
            return Err(cycle_path);
        }

        if visited.contains(node) {
            return Ok(());
        }

        visited.insert(node.to_string());
        in_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(deps) = stages.get(node) {
            for dep in deps {
                dfs(dep.as_ref(), stages, visited, in_stack, order, path)?;
            }
        }

        in_stack.remove(node);
        path.pop();
        order.push(node.to_string());
        Ok(())
    }

    for node in stages.keys() {
        dfs(node, stages, &mut visited, &mut in_stack, &mut order, &mut path)
            .map_err(|cycle_path| CycleError { cycle_path })?;
    }

    order.reverse();
    Ok(order)
}

/// Error indicating a cycle was detected in the DAG.
#[derive(Debug, Clone)]
pub struct CycleError {
    /// The path that forms the cycle.
    pub cycle_path: Vec<String>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cycle detected: {}", self.cycle_path.join(" -> "))
    }
}

impl std::error::Error for CycleError {}

/// Validates that all dependencies exist.
pub fn validate_dependencies_exist<S: AsRef<str>>(
    stages: &HashMap<String, Vec<S>>,
) -> Result<(), MissingDependencyError> {
    let all_stages: HashSet<&String> = stages.keys().collect();

    for (stage_name, deps) in stages {
        for dep in deps {
            let dep_ref = dep.as_ref();
            if !all_stages.contains(&dep_ref.to_string()) {
                return Err(MissingDependencyError {
                    stage: stage_name.clone(),
                    missing_dependency: dep_ref.to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Error indicating a missing dependency.
#[derive(Debug, Clone)]
pub struct MissingDependencyError {
    /// The stage that has the missing dependency.
    pub stage: String,
    /// The name of the missing dependency.
    pub missing_dependency: String,
}

impl std::fmt::Display for MissingDependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stage '{}' depends on non-existent stage '{}'",
            self.stage, self.missing_dependency
        )
    }
}

impl std::error::Error for MissingDependencyError {}

/// Validates that no stage depends on itself.
pub fn validate_no_self_dependencies<S: AsRef<str>>(
    stages: &HashMap<String, Vec<S>>,
) -> Result<(), SelfDependencyError> {
    for (stage_name, deps) in stages {
        for dep in deps {
            if dep.as_ref() == stage_name {
                return Err(SelfDependencyError {
                    stage: stage_name.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Error indicating a stage depends on itself.
#[derive(Debug, Clone)]
pub struct SelfDependencyError {
    /// The stage that depends on itself.
    pub stage: String,
}

impl std::fmt::Display for SelfDependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stage '{}' cannot depend on itself", self.stage)
    }
}

impl std::error::Error for SelfDependencyError {}

/// Validates a stage name is not empty or whitespace-only.
pub fn validate_stage_name(name: &str) -> Result<(), InvalidNameError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(InvalidNameError {
            reason: "Stage name cannot be empty or whitespace-only".to_string(),
        });
    }
    Ok(())
}

/// Error indicating an invalid name.
#[derive(Debug, Clone)]
pub struct InvalidNameError {
    /// The reason the name is invalid.
    pub reason: String,
}

impl std::fmt::Display for InvalidNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid name: {}", self.reason)
    }
}

impl std::error::Error for InvalidNameError {}

/// Combined validation result.
#[derive(Debug)]
pub enum ValidationError {
    Cycle(CycleError),
    MissingDependency(MissingDependencyError),
    SelfDependency(SelfDependencyError),
    InvalidName(InvalidNameError),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Cycle(e) => write!(f, "{}", e),
            ValidationError::MissingDependency(e) => write!(f, "{}", e),
            ValidationError::SelfDependency(e) => write!(f, "{}", e),
            ValidationError::InvalidName(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<CycleError> for ValidationError {
    fn from(e: CycleError) -> Self {
        ValidationError::Cycle(e)
    }
}

impl From<MissingDependencyError> for ValidationError {
    fn from(e: MissingDependencyError) -> Self {
        ValidationError::MissingDependency(e)
    }
}

impl From<SelfDependencyError> for ValidationError {
    fn from(e: SelfDependencyError) -> Self {
        ValidationError::SelfDependency(e)
    }
}

impl From<InvalidNameError> for ValidationError {
    fn from(e: InvalidNameError) -> Self {
        ValidationError::InvalidName(e)
    }
}

/// Performs all validations on a stage dependency graph.
pub fn validate_all<S: AsRef<str>>(
    stages: &HashMap<String, Vec<S>>,
) -> Result<Vec<String>, ValidationError> {
    // Validate names
    for name in stages.keys() {
        validate_stage_name(name)?;
    }

    // Validate no self-dependencies
    validate_no_self_dependencies(stages)?;

    // Validate all dependencies exist
    validate_dependencies_exist(stages)?;

    // Validate no cycles (returns topological order)
    let order = validate_dag(stages)?;

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_dag_simple() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["a".to_string()]);
        stages.insert("c".to_string(), vec!["b".to_string()]);

        let result = validate_dag(&stages);
        assert!(result.is_ok());
        let order = result.unwrap();
        
        // All three stages should be present
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"a".to_string()));
        assert!(order.contains(&"b".to_string()));
        assert!(order.contains(&"c".to_string()));
    }

    #[test]
    fn test_validate_dag_cycle() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec!["c".to_string()]);
        stages.insert("b".to_string(), vec!["a".to_string()]);
        stages.insert("c".to_string(), vec!["b".to_string()]);

        let result = validate_dag(&stages);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dag_diamond() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["a".to_string()]);
        stages.insert("c".to_string(), vec!["a".to_string()]);
        stages.insert("d".to_string(), vec!["b".to_string(), "c".to_string()]);

        let result = validate_dag(&stages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dependencies_exist_ok() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["a".to_string()]);

        let result = validate_dependencies_exist(&stages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dependencies_exist_missing() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["nonexistent".to_string()]);

        let result = validate_dependencies_exist(&stages);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.stage, "b");
        assert_eq!(err.missing_dependency, "nonexistent");
    }

    #[test]
    fn test_validate_no_self_dependencies_ok() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["a".to_string()]);

        let result = validate_no_self_dependencies(&stages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_no_self_dependencies_fail() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec!["a".to_string()]);

        let result = validate_no_self_dependencies(&stages);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.stage, "a");
    }

    #[test]
    fn test_validate_stage_name_ok() {
        assert!(validate_stage_name("valid_name").is_ok());
        assert!(validate_stage_name("name123").is_ok());
        assert!(validate_stage_name("  trimmed  ").is_ok());
    }

    #[test]
    fn test_validate_stage_name_empty() {
        assert!(validate_stage_name("").is_err());
        assert!(validate_stage_name("   ").is_err());
    }

    #[test]
    fn test_validate_all_success() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec![]);
        stages.insert("b".to_string(), vec!["a".to_string()]);
        stages.insert("c".to_string(), vec!["a".to_string(), "b".to_string()]);

        let result = validate_all(&stages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_all_cycle() {
        let mut stages: HashMap<String, Vec<String>> = HashMap::new();
        stages.insert("a".to_string(), vec!["b".to_string()]);
        stages.insert("b".to_string(), vec!["a".to_string()]);

        let result = validate_all(&stages);
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_error_display() {
        let err = CycleError {
            cycle_path: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert_eq!(err.to_string(), "Cycle detected: a -> b -> a");
    }

    #[test]
    fn test_missing_dependency_error_display() {
        let err = MissingDependencyError {
            stage: "b".to_string(),
            missing_dependency: "x".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Stage 'b' depends on non-existent stage 'x'"
        );
    }
}
