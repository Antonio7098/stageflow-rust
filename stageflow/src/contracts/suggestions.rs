//! Contract suggestion registry mapping error codes to remediation hints.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Structured remediation info for a contract violation.
#[derive(Debug, Clone)]
pub struct ContractSuggestion {
    /// Error code this suggestion applies to.
    pub code: String,
    /// Short title for the error.
    pub title: String,
    /// Detailed summary of the issue.
    pub summary: String,
    /// Steps to fix the issue.
    pub fix_steps: Vec<String>,
    /// Optional documentation URL.
    pub doc_url: Option<String>,
}

impl ContractSuggestion {
    /// Creates a new contract suggestion.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        fix_steps: Vec<String>,
    ) -> Self {
        Self {
            code: code.into(),
            title: title.into(),
            summary: summary.into(),
            fix_steps,
            doc_url: None,
        }
    }

    /// Adds a documentation URL.
    #[must_use]
    pub fn with_doc_url(mut self, url: impl Into<String>) -> Self {
        self.doc_url = Some(url.into());
        self
    }
}

static SUGGESTIONS: LazyLock<RwLock<HashMap<String, ContractSuggestion>>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // Preload default suggestions for common pipeline contract violations
    map.insert(
        "CONTRACT-004-CYCLE".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-CYCLE",
            "Dependency Cycle Detected",
            "Stages depend on each other in a loop, preventing execution order.",
            vec![
                "Review the reported cycle path".to_string(),
                "Remove at least one dependency edge to break the loop".to_string(),
                "Re-run pipeline validation or the contracts CLI".to_string(),
            ],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#dependency-cycles",
        ),
    );

    map.insert(
        "CONTRACT-004-MISSING_DEP".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-MISSING_DEP",
            "Missing Stage Dependency",
            "A stage declares a dependency on a stage that is not in the pipeline graph.",
            vec![
                "Ensure the referenced stage is added to the pipeline".to_string(),
                "Or remove/rename the dependency if it is not needed".to_string(),
            ],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#missing-stage-dependencies",
        ),
    );

    map.insert(
        "CONTRACT-004-SELF_DEP".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-SELF_DEP",
            "Stage Depends on Itself",
            "A stage lists itself in its dependency tuple, which creates an impossible prerequisite.",
            vec!["Remove the self-reference from the dependency list".to_string()],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#self-dependencies",
        ),
    );

    map.insert(
        "CONTRACT-004-CONFLICT".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-CONFLICT",
            "Conflicting Stage Definition",
            "The same stage name is defined multiple times with incompatible specs when composing pipelines.",
            vec![
                "Ensure composed pipelines define the stage with the same runner and dependency set".to_string(),
                "Rename one of the stages if they represent different logic".to_string(),
            ],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#conflicting-stage-definitions",
        ),
    );

    map.insert(
        "CONTRACT-004-ORPHAN".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-ORPHAN",
            "Isolated Stage Warning",
            "A stage is neither depended on nor depends on any other stage, which usually indicates misconfiguration.",
            vec![
                "Add dependencies so the stage participates in the pipeline".to_string(),
                "Or remove the stage if it should not run".to_string(),
            ],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#isolated-stages",
        ),
    );

    map.insert(
        "CONTRACT-004-EMPTY".to_string(),
        ContractSuggestion::new(
            "CONTRACT-004-EMPTY",
            "Empty Pipeline",
            "Attempted to build or execute a pipeline without any stages.",
            vec!["Add at least one stage before invoking Pipeline.build()".to_string()],
        )
        .with_doc_url(
            "https://github.com/stageflow/stageflow/blob/main/docs/advanced/error-messages.md#empty-pipelines",
        ),
    );

    RwLock::new(map)
});

/// Register a suggestion for a contract code.
pub fn register_suggestion(suggestion: ContractSuggestion) {
    SUGGESTIONS.write().insert(suggestion.code.clone(), suggestion);
}

/// Return suggestion metadata for an error code if registered.
#[must_use]
pub fn get_contract_suggestion(code: &str) -> Option<ContractSuggestion> {
    SUGGESTIONS.read().get(code).cloned()
}

/// Returns all registered suggestions.
#[must_use]
pub fn list_suggestions() -> Vec<ContractSuggestion> {
    SUGGESTIONS.read().values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cycle_suggestion() {
        let suggestion = get_contract_suggestion("CONTRACT-004-CYCLE");
        assert!(suggestion.is_some());
        let s = suggestion.unwrap();
        assert_eq!(s.title, "Dependency Cycle Detected");
        assert!(!s.fix_steps.is_empty());
    }

    #[test]
    fn test_get_missing_dep_suggestion() {
        let suggestion = get_contract_suggestion("CONTRACT-004-MISSING_DEP");
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_get_unknown_suggestion() {
        let suggestion = get_contract_suggestion("UNKNOWN-CODE");
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_register_custom_suggestion() {
        let custom = ContractSuggestion::new(
            "CUSTOM-001",
            "Custom Error",
            "A custom error for testing",
            vec!["Fix step 1".to_string()],
        );
        register_suggestion(custom);

        let suggestion = get_contract_suggestion("CUSTOM-001");
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap().title, "Custom Error");
    }

    #[test]
    fn test_list_suggestions() {
        let suggestions = list_suggestions();
        assert!(!suggestions.is_empty());
    }
}
