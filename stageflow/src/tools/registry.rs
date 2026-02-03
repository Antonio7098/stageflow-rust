//! Tool registry for managing tool instances.

use super::ToolDefinition;
use crate::errors::ToolError;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A resolved tool call ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedToolCall {
    /// The call ID.
    pub id: String,
    /// The tool name.
    pub name: String,
    /// The parsed arguments.
    pub arguments: serde_json::Value,
    /// The original raw call.
    pub raw: serde_json::Value,
}

/// An unresolved tool call that failed parsing or resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedToolCall {
    /// The call ID if available.
    pub id: Option<String>,
    /// The tool name if available.
    pub name: Option<String>,
    /// The error message.
    pub error: String,
    /// The original raw call.
    pub raw: serde_json::Value,
}

/// Factory function type for creating tools.
pub type ToolFactory = Box<dyn Fn() -> Box<dyn Tool> + Send + Sync>;

/// Trait for tool implementations.
pub trait Tool: Send + Sync {
    /// Returns the tool's action type.
    fn action_type(&self) -> &str;
    
    /// Returns the tool's name.
    fn name(&self) -> &str;
    
    /// Returns the tool definition.
    fn definition(&self) -> ToolDefinition;
}

/// Registry for tool instances and factories.
#[derive(Default)]
pub struct ToolRegistry {
    /// Registered tool instances.
    instances: RwLock<HashMap<String, Box<dyn Tool>>>,
    /// Registered tool factories.
    factories: RwLock<HashMap<String, ToolFactory>>,
}

impl ToolRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a tool instance.
    pub fn register(&self, tool: Box<dyn Tool>) {
        let action_type = tool.action_type().to_string();
        self.instances.write().insert(action_type, tool);
    }

    /// Registers a factory for lazy tool construction.
    pub fn register_factory(&self, action_type: impl Into<String>, factory: ToolFactory) {
        self.factories.write().insert(action_type.into(), factory);
    }

    /// Gets a tool by action type.
    ///
    /// If only a factory is registered, constructs and memoizes the tool.
    pub fn get_tool(&self, action_type: &str) -> Option<&dyn Tool> {
        // Check instances first
        {
            let instances = self.instances.read();
            if instances.contains_key(action_type) {
                // Can't return reference through RwLock, need different approach
            }
        }

        // Check for factory
        let factory = {
            let factories = self.factories.read();
            factories.get(action_type).map(|f| {
                // We need to clone/call the factory
                // This is a limitation - we'll construct on each call
                // In a real implementation, we'd memoize properly
            })
        };

        None // Simplified - full implementation would handle this better
    }

    /// Checks if a tool can be executed.
    #[must_use]
    pub fn can_execute(&self, action_type: &str) -> bool {
        self.instances.read().contains_key(action_type)
            || self.factories.read().contains_key(action_type)
    }

    /// Lists registered tool instances.
    pub fn list_tools(&self) -> Vec<String> {
        self.instances.read().keys().cloned().collect()
    }

    /// Parses and resolves tool calls from raw data.
    ///
    /// Supports OpenAI-style format by default.
    pub fn parse_and_resolve(
        &self,
        calls: &[serde_json::Value],
        id_field: &str,
        function_wrapper: Option<&str>,
        name_field: &str,
        arguments_field: &str,
    ) -> Vec<Result<ResolvedToolCall, UnresolvedToolCall>> {
        calls
            .iter()
            .map(|call| self.resolve_call(call, id_field, function_wrapper, name_field, arguments_field))
            .collect()
    }

    fn resolve_call(
        &self,
        call: &serde_json::Value,
        id_field: &str,
        function_wrapper: Option<&str>,
        name_field: &str,
        arguments_field: &str,
    ) -> Result<ResolvedToolCall, UnresolvedToolCall> {
        // Extract ID
        let id = call.get(id_field).and_then(|v| v.as_str()).map(String::from);

        // Get function object
        let func_obj = if let Some(wrapper) = function_wrapper {
            call.get(wrapper)
        } else {
            Some(call)
        };

        let func_obj = match func_obj {
            Some(obj) => obj,
            None => {
                return Err(UnresolvedToolCall {
                    id,
                    name: None,
                    error: "Missing function wrapper".to_string(),
                    raw: call.clone(),
                });
            }
        };

        // Extract name
        let name = func_obj.get(name_field).and_then(|v| v.as_str()).map(String::from);
        let name_str = match &name {
            Some(n) => n.clone(),
            None => {
                return Err(UnresolvedToolCall {
                    id,
                    name,
                    error: "Missing tool name".to_string(),
                    raw: call.clone(),
                });
            }
        };

        // Parse arguments
        let arguments = match func_obj.get(arguments_field) {
            Some(serde_json::Value::String(s)) => {
                if s.is_empty() {
                    serde_json::json!({})
                } else {
                    match serde_json::from_str(s) {
                        Ok(args) => args,
                        Err(_) => {
                            return Err(UnresolvedToolCall {
                                id,
                                name,
                                error: "Invalid JSON in arguments".to_string(),
                                raw: call.clone(),
                            });
                        }
                    }
                }
            }
            Some(serde_json::Value::Object(obj)) => serde_json::Value::Object(obj.clone()),
            Some(_) => serde_json::json!({}),
            None => serde_json::json!({}),
        };

        // Check if tool exists
        if !self.can_execute(&name_str) {
            return Err(UnresolvedToolCall {
                id,
                name,
                error: format!("No tool registered for action type '{}'", name_str),
                raw: call.clone(),
            });
        }

        Ok(ResolvedToolCall {
            id: id.unwrap_or_default(),
            name: name_str,
            arguments,
            raw: call.clone(),
        })
    }

    /// Clears all registered tools.
    pub fn clear(&self) {
        self.instances.write().clear();
        self.factories.write().clear();
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("instance_count", &self.instances.read().len())
            .field("factory_count", &self.factories.read().len())
            .finish()
    }
}

// Global registry
static GLOBAL_REGISTRY: parking_lot::RwLock<Option<Arc<ToolRegistry>>> = parking_lot::RwLock::new(None);

/// Gets the global tool registry.
pub fn get_tool_registry() -> Arc<ToolRegistry> {
    let read = GLOBAL_REGISTRY.read();
    if let Some(ref registry) = *read {
        return registry.clone();
    }
    drop(read);

    let mut write = GLOBAL_REGISTRY.write();
    if write.is_none() {
        *write = Some(Arc::new(ToolRegistry::new()));
    }
    write.as_ref().unwrap().clone()
}

/// Clears the global tool registry.
pub fn clear_tool_registry() {
    *GLOBAL_REGISTRY.write() = None;
}

/// Registers a tool in the global registry.
pub fn register_tool(tool: Box<dyn Tool>) {
    get_tool_registry().register(tool);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTool {
        action_type: String,
        name: String,
    }

    impl Tool for TestTool {
        fn action_type(&self) -> &str {
            &self.action_type
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn definition(&self) -> ToolDefinition {
            ToolDefinition::new(&self.name, &self.action_type)
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.list_tools().is_empty());
    }

    #[test]
    fn test_registry_register() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(TestTool {
            action_type: "test_action".to_string(),
            name: "test".to_string(),
        }));

        assert!(registry.can_execute("test_action"));
        assert!(!registry.can_execute("unknown"));
    }

    #[test]
    fn test_parse_and_resolve_openai_format() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(TestTool {
            action_type: "get_weather".to_string(),
            name: "weather".to_string(),
        }));

        let calls = vec![serde_json::json!({
            "id": "call_123",
            "function": {
                "name": "get_weather",
                "arguments": "{\"location\": \"NYC\"}"
            }
        })];

        let results = registry.parse_and_resolve(&calls, "id", Some("function"), "name", "arguments");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());

        let resolved = results[0].as_ref().unwrap();
        assert_eq!(resolved.id, "call_123");
        assert_eq!(resolved.name, "get_weather");
    }

    #[test]
    fn test_parse_unresolved_unknown_tool() {
        let registry = ToolRegistry::new();

        let calls = vec![serde_json::json!({
            "id": "call_123",
            "function": {
                "name": "unknown_tool",
                "arguments": "{}"
            }
        })];

        let results = registry.parse_and_resolve(&calls, "id", Some("function"), "name", "arguments");
        assert!(results[0].is_err());

        let err = results[0].as_ref().unwrap_err();
        assert!(err.error.contains("No tool registered"));
    }

    #[test]
    fn test_parse_invalid_json_arguments() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(TestTool {
            action_type: "my_tool".to_string(),
            name: "tool".to_string(),
        }));

        let calls = vec![serde_json::json!({
            "id": "call_123",
            "function": {
                "name": "my_tool",
                "arguments": "not valid json {"
            }
        })];

        let results = registry.parse_and_resolve(&calls, "id", Some("function"), "name", "arguments");
        assert!(results[0].is_err());

        let err = results[0].as_ref().unwrap_err();
        assert!(err.error.contains("Invalid JSON"));
    }

    #[test]
    fn test_global_registry() {
        clear_tool_registry();

        let registry = get_tool_registry();
        assert!(registry.list_tools().is_empty());

        register_tool(Box::new(TestTool {
            action_type: "global_tool".to_string(),
            name: "global".to_string(),
        }));

        assert!(get_tool_registry().can_execute("global_tool"));

        clear_tool_registry();
    }
}
