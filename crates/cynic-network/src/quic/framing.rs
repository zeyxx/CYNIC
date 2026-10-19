//! Pure binary framing for CYNIC QUIC multiplexed agent communication.
//!
//! Designed for zero-copy deserialization and zero Head-of-Line blocking (RFC 9000).

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};

/// Magic byte prefix identifying CYNIC QUIC stream frames ('C' = 0x43).
pub const CYNIC_FRAME_MAGIC: u8 = 0x43;

/// Protocol frame wire version.
pub const CYNIC_FRAME_VERSION: u8 = 1;

/// High-level frame type tag on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Control = 0x01,
    Telemetry = 0x02,
    ArtifactChunk = 0x03,
}

impl TryFrom<u8> for FrameType {
    type Error = FramingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Control),
            0x02 => Ok(Self::Telemetry),
            0x03 => Ok(Self::ArtifactChunk),
            other => Err(FramingError::UnknownFrameType(other)),
        }
    }
}

/// Errors occurring during framing encode/decode.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FramingError {
    #[error("buffer too short: need {expected} bytes, got {actual}")]
    BufferTooShort { expected: usize, actual: usize },
    #[error("invalid magic byte: expected 0x{:02x}, got 0x{:02x}", CYNIC_FRAME_MAGIC, .0)]
    InvalidMagic(u8),
    #[error("unsupported frame version: {0}")]
    UnsupportedVersion(u8),
    #[error("unknown frame type: 0x{0:02x}")]
    UnknownFrameType(u8),
    #[error("payload serialization error: {0}")]
    Serialization(String),
    #[error("artifact chunk hash mismatch: declared {declared}, computed {computed}")]
    ChunkHashMismatch { declared: String, computed: String },
}

/// Control payload transported over bidirectional Control stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMessage {
    TaskDispatch {
        task_id: String,
        assignee: String,
        lease_ms: u64,
        payload_json: String,
    },
    TaskAck {
        task_id: String,
        accepted: bool,
        reason: Option<String>,
    },
    Heartbeat {
        agent_id: String,
        epoch: u64,
        status: String,
    },
}

/// High-frequency telemetry payload transported over unidirectional Telemetry stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryMessage {
    pub timestamp_ms: u64,
    pub cpu_util_pct: f32,
    pub mem_util_pct: f32,
    pub open_streams_count: u32,
}

/// Binary chunk payload transported over unidirectional Artifact stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactChunkMessage {
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub blake3_hash: [u8; 32],
    pub data: Bytes,
}

impl ArtifactChunkMessage {
    /// Creates a verified chunk message with computed BLAKE3 hash.
    #[must_use]
    pub fn new(chunk_index: u32, total_chunks: u32, data: Bytes) -> Self {
        let hash = *blake3::hash(&data).as_bytes();
        Self {
            chunk_index,
            total_chunks,
            blake3_hash: hash,
            data,
        }
    }

    /// Verifies that data matches the declared BLAKE3 hash.
    pub fn verify_integrity(&self) -> Result<(), FramingError> {
        let computed = *blake3::hash(&self.data).as_bytes();
        if self.blake3_hash != computed {
            return Err(FramingError::ChunkHashMismatch {
                declared: hex::encode(self.blake3_hash),
                computed: hex::encode(computed),
            });
        }
        Ok(())
    }
}

/// Top-level decoded frame envelope.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentFrame {
    Control(ControlMessage),
    Telemetry(TelemetryMessage),
    Artifact(ArtifactChunkMessage),
}

/// Pure encoding of an AgentFrame into a binary buffer.
pub fn encode_frame(frame: &AgentFrame) -> Result<Bytes, FramingError> {
    let mut buf = BytesMut::with_capacity(128);
    buf.put_u8(CYNIC_FRAME_MAGIC);
    buf.put_u8(CYNIC_FRAME_VERSION);

    match frame {
        AgentFrame::Control(msg) => {
            buf.put_u8(FrameType::Control as u8);
            let payload =
                serde_json::to_vec(msg).map_err(|e| FramingError::Serialization(e.to_string()))?;
            buf.put_u32(payload.len() as u32);
            buf.extend_from_slice(&payload);
        }
        AgentFrame::Telemetry(msg) => {
            buf.put_u8(FrameType::Telemetry as u8);
            let payload =
                serde_json::to_vec(msg).map_err(|e| FramingError::Serialization(e.to_string()))?;
            buf.put_u32(payload.len() as u32);
            buf.extend_from_slice(&payload);
        }
        AgentFrame::Artifact(chunk) => {
            buf.put_u8(FrameType::ArtifactChunk as u8);
            let total_len = 4 + 4 + 32 + chunk.data.len();
            buf.put_u32(total_len as u32);
            buf.put_u32(chunk.chunk_index);
            buf.put_u32(chunk.total_chunks);
            buf.extend_from_slice(&chunk.blake3_hash);
            buf.extend_from_slice(&chunk.data);
        }
    }

    Ok(buf.freeze())
}

/// Pure decoding of an AgentFrame from a binary buffer.
pub fn decode_frame(mut src: &[u8]) -> Result<AgentFrame, FramingError> {
    if src.len() < 7 {
        return Err(FramingError::BufferTooShort {
            expected: 7,
            actual: src.len(),
        });
    }

    let magic = src.get_u8();
    if magic != CYNIC_FRAME_MAGIC {
        return Err(FramingError::InvalidMagic(magic));
    }

    let version = src.get_u8();
    if version != CYNIC_FRAME_VERSION {
        return Err(FramingError::UnsupportedVersion(version));
    }

    let frame_type = FrameType::try_from(src.get_u8())?;
    let payload_len = src.get_u32() as usize;

    if src.len() < payload_len {
        return Err(FramingError::BufferTooShort {
            expected: payload_len,
            actual: src.len(),
        });
    }

    let payload = &src[..payload_len];

    match frame_type {
        FrameType::Control => {
            let msg: ControlMessage = serde_json::from_slice(payload)
                .map_err(|e| FramingError::Serialization(e.to_string()))?;
            Ok(AgentFrame::Control(msg))
        }
        FrameType::Telemetry => {
            let msg: TelemetryMessage = serde_json::from_slice(payload)
                .map_err(|e| FramingError::Serialization(e.to_string()))?;
            Ok(AgentFrame::Telemetry(msg))
        }
        FrameType::ArtifactChunk => {
            if payload_len < 40 {
                return Err(FramingError::BufferTooShort {
                    expected: 40,
                    actual: payload_len,
                });
            }
            let mut chunk_reader = payload;
            let chunk_index = chunk_reader.get_u32();
            let total_chunks = chunk_reader.get_u32();
            let mut blake3_hash = [0u8; 32];
            chunk_reader.copy_to_slice(&mut blake3_hash);
            let data = Bytes::copy_from_slice(chunk_reader);

            let chunk = ArtifactChunkMessage {
                chunk_index,
                total_chunks,
                blake3_hash,
                data,
            };
            chunk.verify_integrity()?;
            Ok(AgentFrame::Artifact(chunk))
        }
    }
}
