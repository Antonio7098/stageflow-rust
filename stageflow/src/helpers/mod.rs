//! Helper modules for analytics, streaming, mocks, memory, guardrails, and runtime.

// Stub modules - will be fully implemented in Phase 9
pub mod analytics;
pub mod guardrails;
pub mod memory;
pub mod mocks;
pub mod providers;
pub mod streaming;
pub mod timestamps;

pub use analytics::{AnalyticsEvent, AnalyticsSink, BufferedExporter, ConsoleExporter, JSONFileExporter};
pub use guardrails::{ContentFilter, GuardrailResult, GuardrailStage, InjectionDetector, PIIDetector, PolicyViolation};
pub use memory::{InMemoryStore, MemoryConfig, MemoryEntry, MemoryFetchStage};
pub use mocks::{MockAuthProvider, MockLLMProvider, MockSTTProvider, MockToolExecutor, MockTTSProvider};
pub use providers::{LLMResponse, STTResponse, TTSResponse};
pub use streaming::{AudioChunk, BackpressureMonitor, ChunkQueue, StreamingBuffer};
pub use timestamps::{detect_unix_precision, normalize_to_utc, parse_timestamp as parse_ts};
