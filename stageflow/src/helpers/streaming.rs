//! Streaming primitives for audio processing.

use serde::{Deserialize, Serialize};

/// Audio format enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Pcm16,
    Pcm32,
    Float32,
}

/// An audio chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioChunk {
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u8,
    pub format: AudioFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<f64>,
    pub sequence: u32,
    pub is_final: bool,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl AudioChunk {
    /// Returns duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> f64 {
        let bytes_per_sample = match self.format {
            AudioFormat::Pcm16 => 2,
            AudioFormat::Pcm32 | AudioFormat::Float32 => 4,
        };
        let samples = self.data.len() as f64 / (bytes_per_sample as f64 * self.channels as f64);
        (samples / self.sample_rate as f64) * 1000.0
    }
}

mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

/// Backpressure statistics.
#[derive(Debug, Clone, Default)]
pub struct BackpressureStats {
    pub total_items: u64,
    pub dropped_items: u64,
    pub max_queue_size: usize,
    pub fill_percentage: f64,
    pub blocked_puts: u64,
    pub total_blocked_ms: f64,
}

impl BackpressureStats {
    /// Returns the drop rate.
    #[must_use]
    pub fn drop_rate(&self) -> f64 {
        if self.total_items == 0 { 0.0 } else { self.dropped_items as f64 / self.total_items as f64 }
    }
}

/// Backpressure monitor.
#[derive(Default)]
pub struct BackpressureMonitor {
    stats: parking_lot::RwLock<BackpressureStats>,
    high_water_mark: f64,
    low_water_mark: f64,
    is_throttling: std::sync::atomic::AtomicBool,
}

impl BackpressureMonitor {
    /// Creates a new monitor.
    #[must_use]
    pub fn new(high_water_mark: f64, low_water_mark: f64) -> Self {
        Self {
            stats: parking_lot::RwLock::new(BackpressureStats::default()),
            high_water_mark,
            low_water_mark,
            is_throttling: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Records a put operation.
    pub fn record_put(&self, queue_size: usize, max_size: usize) {
        let mut stats = self.stats.write();
        stats.total_items += 1;
        stats.max_queue_size = stats.max_queue_size.max(queue_size);
        stats.fill_percentage = (queue_size as f64 / max_size as f64) * 100.0;
    }

    /// Records a drop.
    pub fn record_drop(&self) {
        self.stats.write().dropped_items += 1;
    }

    /// Returns current stats.
    #[must_use]
    pub fn stats(&self) -> BackpressureStats {
        self.stats.read().clone()
    }
}

/// Bounded async chunk queue.
pub struct ChunkQueue {
    max_size: usize,
    drop_on_overflow: bool,
}

impl ChunkQueue {
    /// Creates a new queue.
    #[must_use]
    pub fn new(max_size: usize, drop_on_overflow: bool) -> Self {
        Self { max_size, drop_on_overflow }
    }
}

/// Streaming buffer for audio.
pub struct StreamingBuffer {
    max_duration_ms: f64,
    sample_rate: u32,
}

impl StreamingBuffer {
    /// Creates a new buffer.
    #[must_use]
    pub fn new(max_duration_ms: f64, sample_rate: u32) -> Self {
        Self { max_duration_ms, sample_rate }
    }
}
