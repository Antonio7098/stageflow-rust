//! StagePorts - Injected capabilities for stages (callbacks, services, db).
//!
//! This module defines typed ports for different domains, following the
//! Interface Segregation Principle. Stages only receive the ports they need.

use std::sync::Arc;

/// Core capabilities needed by most stages.
#[derive(Clone, Default)]
pub struct CorePorts {
    /// Database connection or handle.
    pub db: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Call logger for tracking operations.
    pub call_logger: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl std::fmt::Debug for CorePorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CorePorts")
            .field("has_db", &self.db.is_some())
            .field("has_call_logger", &self.call_logger.is_some())
            .finish()
    }
}

impl CorePorts {
    /// Creates new empty core ports.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the database handle.
    #[must_use]
    pub fn with_db(mut self, db: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.db = Some(db);
        self
    }

    /// Returns true if a database is configured.
    #[must_use]
    pub fn has_db(&self) -> bool {
        self.db.is_some()
    }
}

/// Ports for LLM-powered stages.
#[derive(Clone, Default)]
pub struct LLMPorts {
    /// LLM provider for text generation.
    pub llm_provider: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Chat service for building context.
    pub chat_service: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl std::fmt::Debug for LLMPorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMPorts")
            .field("has_llm_provider", &self.llm_provider.is_some())
            .field("has_chat_service", &self.chat_service.is_some())
            .finish()
    }
}

impl LLMPorts {
    /// Creates new empty LLM ports.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the LLM provider.
    #[must_use]
    pub fn with_llm_provider(mut self, provider: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// Returns true if an LLM provider is configured.
    #[must_use]
    pub fn has_llm(&self) -> bool {
        self.llm_provider.is_some()
    }
}

/// Ports for audio processing stages.
#[derive(Clone, Default)]
pub struct AudioPorts {
    /// TTS provider for text-to-speech.
    pub tts_provider: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// STT provider for speech-to-text.
    pub stt_provider: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Audio data bytes.
    pub audio_data: Option<Vec<u8>>,
    /// Audio format string.
    pub audio_format: Option<String>,
}

impl std::fmt::Debug for AudioPorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioPorts")
            .field("has_tts", &self.tts_provider.is_some())
            .field("has_stt", &self.stt_provider.is_some())
            .field("has_audio", &self.audio_data.is_some())
            .field("audio_format", &self.audio_format)
            .finish()
    }
}

impl AudioPorts {
    /// Creates new empty audio ports.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the TTS provider.
    #[must_use]
    pub fn with_tts_provider(mut self, provider: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.tts_provider = Some(provider);
        self
    }

    /// Sets the STT provider.
    #[must_use]
    pub fn with_stt_provider(mut self, provider: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.stt_provider = Some(provider);
        self
    }

    /// Sets audio data.
    #[must_use]
    pub fn with_audio_data(mut self, data: Vec<u8>, format: impl Into<String>) -> Self {
        self.audio_data = Some(data);
        self.audio_format = Some(format.into());
        self
    }

    /// Returns true if audio data is present.
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.audio_data.is_some()
    }
}

/// Combined ports container for stages that need multiple port types.
#[derive(Clone, Default)]
pub struct StagePorts {
    /// Core ports.
    pub core: CorePorts,
    /// LLM ports.
    pub llm: LLMPorts,
    /// Audio ports.
    pub audio: AudioPorts,
}

impl std::fmt::Debug for StagePorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StagePorts")
            .field("core", &self.core)
            .field("llm", &self.llm)
            .field("audio", &self.audio)
            .finish()
    }
}

impl StagePorts {
    /// Creates new empty stage ports.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the core ports.
    #[must_use]
    pub fn with_core(mut self, core: CorePorts) -> Self {
        self.core = core;
        self
    }

    /// Sets the LLM ports.
    #[must_use]
    pub fn with_llm(mut self, llm: LLMPorts) -> Self {
        self.llm = llm;
        self
    }

    /// Sets the audio ports.
    #[must_use]
    pub fn with_audio(mut self, audio: AudioPorts) -> Self {
        self.audio = audio;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_ports_default() {
        let ports = CorePorts::new();
        assert!(!ports.has_db());
    }

    #[test]
    fn test_llm_ports_default() {
        let ports = LLMPorts::new();
        assert!(!ports.has_llm());
    }

    #[test]
    fn test_audio_ports_default() {
        let ports = AudioPorts::new();
        assert!(!ports.has_audio());
    }

    #[test]
    fn test_audio_ports_with_data() {
        let ports = AudioPorts::new()
            .with_audio_data(vec![1, 2, 3], "audio/wav");

        assert!(ports.has_audio());
        assert_eq!(ports.audio_format, Some("audio/wav".to_string()));
    }

    #[test]
    fn test_stage_ports_combined() {
        let ports = StagePorts::new()
            .with_core(CorePorts::new())
            .with_llm(LLMPorts::new())
            .with_audio(AudioPorts::new());

        assert!(!ports.core.has_db());
        assert!(!ports.llm.has_llm());
        assert!(!ports.audio.has_audio());
    }
}
