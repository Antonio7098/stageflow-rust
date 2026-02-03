//! Run identity for tracking pipeline executions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Identifies a pipeline run with various correlation IDs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunIdentity {
    /// The unique ID for this pipeline run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_run_id: Option<Uuid>,

    /// The request ID (for request-scoped tracking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,

    /// The session ID (for session-scoped tracking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,

    /// The user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,

    /// The organization ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<Uuid>,

    /// The interaction ID (for multi-turn conversations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_id: Option<Uuid>,
}

impl RunIdentity {
    /// Creates a new run identity with a generated pipeline run ID.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pipeline_run_id: Some(Uuid::new_v4()),
            ..Default::default()
        }
    }

    /// Creates a run identity with a specific pipeline run ID.
    #[must_use]
    pub fn with_pipeline_run_id(pipeline_run_id: Uuid) -> Self {
        Self {
            pipeline_run_id: Some(pipeline_run_id),
            ..Default::default()
        }
    }

    /// Sets the request ID.
    #[must_use]
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Sets the session ID.
    #[must_use]
    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Sets the user ID.
    #[must_use]
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Sets the organization ID.
    #[must_use]
    pub fn with_org_id(mut self, org_id: Uuid) -> Self {
        self.org_id = Some(org_id);
        self
    }

    /// Sets the interaction ID.
    #[must_use]
    pub fn with_interaction_id(mut self, interaction_id: Uuid) -> Self {
        self.interaction_id = Some(interaction_id);
        self
    }

    /// Converts to a dictionary with string values (or null).
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();

        map.insert(
            "pipeline_run_id".to_string(),
            self.pipeline_run_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );
        map.insert(
            "request_id".to_string(),
            self.request_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );
        map.insert(
            "session_id".to_string(),
            self.session_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );
        map.insert(
            "user_id".to_string(),
            self.user_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );
        map.insert(
            "org_id".to_string(),
            self.org_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );
        map.insert(
            "interaction_id".to_string(),
            self.interaction_id
                .map_or(serde_json::Value::Null, |id| serde_json::json!(id.to_string())),
        );

        map
    }

    /// Returns the pipeline run ID as a string, or None.
    #[must_use]
    pub fn pipeline_run_id_str(&self) -> Option<String> {
        self.pipeline_run_id.map(|id| id.to_string())
    }

    /// Returns the request ID as a string, or None.
    #[must_use]
    pub fn request_id_str(&self) -> Option<String> {
        self.request_id.map(|id| id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_identity_new() {
        let identity = RunIdentity::new();
        assert!(identity.pipeline_run_id.is_some());
        assert!(identity.request_id.is_none());
    }

    #[test]
    fn test_run_identity_builder() {
        let user_id = Uuid::new_v4();
        let identity = RunIdentity::new()
            .with_user_id(user_id)
            .with_session_id(Uuid::new_v4());

        assert_eq!(identity.user_id, Some(user_id));
        assert!(identity.session_id.is_some());
    }

    #[test]
    fn test_run_identity_to_dict() {
        let identity = RunIdentity::new();
        let dict = identity.to_dict();

        assert!(dict.contains_key("pipeline_run_id"));
        assert!(dict.contains_key("request_id"));
        assert!(!dict["pipeline_run_id"].is_null());
        assert!(dict["request_id"].is_null());
    }

    #[test]
    fn test_run_identity_serialization() {
        let identity = RunIdentity::new().with_user_id(Uuid::new_v4());
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: RunIdentity = serde_json::from_str(&json).unwrap();

        assert_eq!(identity.pipeline_run_id, deserialized.pipeline_run_id);
        assert_eq!(identity.user_id, deserialized.user_id);
    }
}
