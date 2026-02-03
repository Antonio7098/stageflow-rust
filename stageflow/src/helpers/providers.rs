//! Provider response types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

impl LLMResponse {
    /// Returns total tokens.
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens.unwrap_or(0) + self.output_tokens.unwrap_or(0)
    }

    /// Converts to OTel attributes.
    #[must_use]
    pub fn to_otel_attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("llm.model".to_string(), serde_json::json!(self.model));
        map.insert("llm.provider".to_string(), serde_json::json!(self.provider));
        if let Some(t) = self.input_tokens { map.insert("llm.input_tokens".to_string(), serde_json::json!(t)); }
        if let Some(t) = self.output_tokens { map.insert("llm.output_tokens".to_string(), serde_json::json!(t)); }
        map.insert("llm.total_tokens".to_string(), serde_json::json!(self.total_tokens()));
        if let Some(l) = self.latency_ms { map.insert("llm.latency_ms".to_string(), serde_json::json!(l)); }
        map
    }
}

/// STT response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTResponse {
    pub text: String,
    pub confidence: f64,
    pub language: String,
    pub is_final: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
}

impl Default for STTResponse {
    fn default() -> Self {
        Self {
            text: String::new(),
            confidence: 1.0,
            language: "en".to_string(),
            is_final: true,
            duration_ms: None,
            provider: None,
            model: None,
            latency_ms: None,
        }
    }
}

/// TTS response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSResponse {
    #[serde(skip)]
    pub audio: Vec<u8>,
    pub duration_ms: f64,
    pub sample_rate: u32,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    pub channels: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters_processed: Option<usize>,
}

impl TTSResponse {
    /// Returns the byte count.
    #[must_use]
    pub fn byte_count(&self) -> usize {
        self.audio.len()
    }
}
