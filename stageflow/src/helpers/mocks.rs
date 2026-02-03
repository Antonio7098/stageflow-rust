//! Mock providers for testing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Mock LLM provider.
pub struct MockLLMProvider {
    responses: Vec<String>,
    patterns: HashMap<String, String>,
    echo_mode: bool,
    latency_ms: u64,
    fail_rate: f64,
    call_count: AtomicUsize,
}

impl MockLLMProvider {
    /// Creates a new mock provider.
    #[must_use]
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            patterns: HashMap::new(),
            echo_mode: false,
            latency_ms: 0,
            fail_rate: 0.0,
            call_count: AtomicUsize::new(0),
        }
    }

    /// Returns the call count.
    #[must_use]
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Resets the mock.
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
    }
}

/// Mock STT provider.
pub struct MockSTTProvider {
    transcriptions: Vec<String>,
    fail_rate: f64,
    call_count: AtomicUsize,
}

impl MockSTTProvider {
    /// Creates a new mock provider.
    #[must_use]
    pub fn new(transcriptions: Vec<String>) -> Self {
        Self { transcriptions, fail_rate: 0.0, call_count: AtomicUsize::new(0) }
    }

    /// Returns the call count.
    #[must_use]
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

/// Mock TTS provider.
pub struct MockTTSProvider {
    sample_rate: u32,
    fail_rate: f64,
    call_count: AtomicUsize,
}

impl MockTTSProvider {
    /// Creates a new mock provider.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate, fail_rate: 0.0, call_count: AtomicUsize::new(0) }
    }
}

/// Mock auth provider.
pub struct MockAuthProvider {
    valid_tokens: HashMap<String, serde_json::Value>,
    accept_any: bool,
    fail_rate: f64,
}

impl MockAuthProvider {
    /// Creates a new mock provider.
    #[must_use]
    pub fn new(accept_any: bool) -> Self {
        Self { valid_tokens: HashMap::new(), accept_any, fail_rate: 0.0 }
    }
}

/// Mock tool executor.
pub struct MockToolExecutor {
    tools: HashMap<String, Box<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
    execution_count: AtomicUsize,
    fail_rate: f64,
    latency_ms: u64,
}

impl MockToolExecutor {
    /// Creates a new mock executor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            execution_count: AtomicUsize::new(0),
            fail_rate: 0.0,
            latency_ms: 0,
        }
    }

    /// Returns the execution count.
    #[must_use]
    pub fn execution_count(&self) -> usize {
        self.execution_count.load(Ordering::SeqCst)
    }
}

impl Default for MockToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}
