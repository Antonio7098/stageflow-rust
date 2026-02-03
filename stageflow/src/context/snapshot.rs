//! Immutable context snapshots for pipeline execution.

use super::RunIdentity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role (e.g., "user", "assistant", "system").
    pub role: String,
    /// The message content.
    pub content: String,
    /// Optional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Message {
    /// Creates a new message.
    #[must_use]
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Creates an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Creates a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }
}

/// Conversation history with routing decision.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conversation {
    /// The message history.
    #[serde(default)]
    pub messages: Vec<Message>,
    /// Optional routing decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_decision: Option<String>,
}

impl Conversation {
    /// Creates a new empty conversation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a conversation with messages.
    #[must_use]
    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self {
            messages,
            routing_decision: None,
        }
    }

    /// Adds a message to the conversation.
    #[must_use]
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets the routing decision.
    #[must_use]
    pub fn with_routing_decision(mut self, decision: impl Into<String>) -> Self {
        self.routing_decision = Some(decision.into());
        self
    }

    /// Returns the last user message content, if any.
    #[must_use]
    pub fn last_user_message(&self) -> Option<&str> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
    }
}

/// Enrichment data groups.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Enrichments {
    /// User profile data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<serde_json::Value>,
    /// Memory/context data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<serde_json::Value>,
    /// Retrieved documents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<serde_json::Value>,
    /// Web search results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub web_results: Vec<serde_json::Value>,
    /// Custom enrichment data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Enrichments {
    /// Creates new empty enrichments.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the profile data.
    #[must_use]
    pub fn with_profile(mut self, profile: serde_json::Value) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Sets the memory data.
    #[must_use]
    pub fn with_memory(mut self, memory: serde_json::Value) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Adds documents.
    #[must_use]
    pub fn with_documents(mut self, documents: Vec<serde_json::Value>) -> Self {
        self.documents = documents;
        self
    }

    /// Adds web results.
    #[must_use]
    pub fn with_web_results(mut self, results: Vec<serde_json::Value>) -> Self {
        self.web_results = results;
        self
    }

    /// Adds a custom enrichment.
    #[must_use]
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// A bundle of typed extensions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionBundle {
    /// Extension data keyed by type name.
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ExtensionBundle {
    /// Creates a new empty extension bundle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an extension.
    pub fn register(&mut self, type_name: impl Into<String>, data: serde_json::Value) {
        self.extensions.insert(type_name.into(), data);
    }

    /// Gets an extension by type name.
    #[must_use]
    pub fn get(&self, type_name: &str) -> Option<&serde_json::Value> {
        self.extensions.get(type_name)
    }

    /// Checks if an extension is registered.
    #[must_use]
    pub fn contains(&self, type_name: &str) -> bool {
        self.extensions.contains_key(type_name)
    }
}

/// An immutable snapshot of the execution context.
///
/// Snapshots capture the state at a point in time and are used
/// for serialization, caching, and passing to stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Run identity with correlation IDs.
    pub run_id: RunIdentity,

    /// Conversation history.
    #[serde(default)]
    pub conversation: Conversation,

    /// Enrichment data.
    #[serde(default)]
    pub enrichments: Enrichments,

    /// Extension bundle.
    #[serde(default)]
    pub extensions: ExtensionBundle,

    /// The input text (convenience field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_text: Option<String>,

    /// Additional metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ContextSnapshot {
    fn default() -> Self {
        Self {
            run_id: RunIdentity::new(),
            conversation: Conversation::default(),
            enrichments: Enrichments::default(),
            extensions: ExtensionBundle::default(),
            input_text: None,
            metadata: HashMap::new(),
        }
    }
}

impl ContextSnapshot {
    /// Creates a new context snapshot.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a snapshot with a specific run identity.
    #[must_use]
    pub fn with_run_id(mut self, run_id: RunIdentity) -> Self {
        self.run_id = run_id;
        self
    }

    /// Sets the conversation.
    #[must_use]
    pub fn with_conversation(mut self, conversation: Conversation) -> Self {
        self.conversation = conversation;
        self
    }

    /// Sets the enrichments.
    #[must_use]
    pub fn with_enrichments(mut self, enrichments: Enrichments) -> Self {
        self.enrichments = enrichments;
        self
    }

    /// Sets the extensions.
    #[must_use]
    pub fn with_extensions(mut self, extensions: ExtensionBundle) -> Self {
        self.extensions = extensions;
        self
    }

    /// Sets the input text.
    #[must_use]
    pub fn with_input_text(mut self, text: impl Into<String>) -> Self {
        self.input_text = Some(text.into());
        self
    }

    /// Adds metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Returns the pipeline run ID.
    #[must_use]
    pub fn pipeline_run_id(&self) -> Option<Uuid> {
        self.run_id.pipeline_run_id
    }

    /// Returns the request ID.
    #[must_use]
    pub fn request_id(&self) -> Option<Uuid> {
        self.run_id.request_id
    }

    /// Returns the session ID.
    #[must_use]
    pub fn session_id(&self) -> Option<Uuid> {
        self.run_id.session_id
    }

    /// Returns the user ID.
    #[must_use]
    pub fn user_id(&self) -> Option<Uuid> {
        self.run_id.user_id
    }

    /// Converts to a dictionary representation.
    ///
    /// Includes both composed keys and legacy flattened keys for compatibility.
    #[must_use]
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();

        // Composed keys
        map.insert(
            "run_id".to_string(),
            serde_json::to_value(&self.run_id).unwrap_or_default(),
        );
        map.insert(
            "conversation".to_string(),
            serde_json::to_value(&self.conversation).unwrap_or_default(),
        );
        map.insert(
            "enrichments".to_string(),
            serde_json::to_value(&self.enrichments).unwrap_or_default(),
        );
        map.insert(
            "extensions".to_string(),
            serde_json::to_value(&self.extensions).unwrap_or_default(),
        );

        // Legacy flattened keys for compatibility
        let run_dict = self.run_id.to_dict();
        for (k, v) in run_dict {
            map.insert(k, v);
        }

        if let Some(ref text) = self.input_text {
            map.insert("input_text".to_string(), serde_json::json!(text));
        }

        if !self.metadata.is_empty() {
            let meta_map: serde_json::Map<String, serde_json::Value> =
                self.metadata.clone().into_iter().collect();
            map.insert("metadata".to_string(), serde_json::Value::Object(meta_map));
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_conversation_builder() {
        let conv = Conversation::new()
            .add_message(Message::user("Hi"))
            .add_message(Message::assistant("Hello!"))
            .with_routing_decision("general");

        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.routing_decision, Some("general".to_string()));
    }

    #[test]
    fn test_conversation_last_user_message() {
        let conv = Conversation::new()
            .add_message(Message::user("First"))
            .add_message(Message::assistant("Response"))
            .add_message(Message::user("Second"));

        assert_eq!(conv.last_user_message(), Some("Second"));
    }

    #[test]
    fn test_enrichments_builder() {
        let enrichments = Enrichments::new()
            .with_profile(serde_json::json!({"name": "Alice"}))
            .with_custom("key", serde_json::json!("value"));

        assert!(enrichments.profile.is_some());
        assert!(enrichments.custom.contains_key("key"));
    }

    #[test]
    fn test_extension_bundle() {
        let mut bundle = ExtensionBundle::new();
        bundle.register("my_extension", serde_json::json!({"data": 42}));

        assert!(bundle.contains("my_extension"));
        assert!(!bundle.contains("other"));
    }

    #[test]
    fn test_context_snapshot_creation() {
        let snapshot = ContextSnapshot::new()
            .with_input_text("Hello world")
            .with_metadata("channel", serde_json::json!("web"));

        assert!(snapshot.pipeline_run_id().is_some());
        assert_eq!(snapshot.input_text, Some("Hello world".to_string()));
    }

    #[test]
    fn test_context_snapshot_to_dict() {
        let snapshot = ContextSnapshot::new();
        let dict = snapshot.to_dict();

        // Should have both composed and flattened keys
        assert!(dict.contains_key("run_id"));
        assert!(dict.contains_key("pipeline_run_id"));
        assert!(dict.contains_key("conversation"));
    }

    #[test]
    fn test_context_snapshot_serialization() {
        let snapshot = ContextSnapshot::new().with_input_text("test");
        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ContextSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(snapshot.input_text, deserialized.input_text);
    }
}
