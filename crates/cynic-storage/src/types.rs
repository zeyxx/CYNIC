//! Storage domain types — immutable records, cryptographic witnesses, and envelopes.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Monotonically increasing sequence number in the WAL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SequenceNumber(pub u64);

impl SequenceNumber {
    pub const GENESIS: Self = Self(0);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Proof-of-History tick index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoHTick(pub u64);

/// Fixed-size SHA3-512 cryptographic digest (64 raw bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Hash512 {
    pub const GENESIS: Self = Self([0u8; 64]);

    /// Canonical prefixed hex representation per Foundation §11.
    pub fn to_prefixed_hex(&self) -> String {
        format!("sha3-512:{}", hex::encode(self.0))
    }

    /// Parse from a canonical prefixed hex string.
    pub fn from_prefixed_hex(s: &str) -> Result<Self, String> {
        let hex_str = s
            .strip_prefix("sha3-512:")
            .ok_or_else(|| format!("missing sha3-512: prefix in '{s}'"))?;
        let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))?;
        if bytes.len() != 64 {
            return Err(format!(
                "expected 64 bytes for SHA3-512, got {}",
                bytes.len()
            ));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Serialize for Hash512 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_prefixed_hex())
    }
}

impl<'de> Deserialize<'de> for Hash512 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_prefixed_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// Cryptographic witness asserting execution mandate, signer pubkey, and proof anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalWitness {
    #[serde(with = "hex")]
    pub principal_pubkey: [u8; 32],
    #[serde(with = "hex")]
    pub signature: [u8; 64],
    pub mandate_id: Option<String>,
    pub proof_anchor: Option<Hash512>,
}

/// Fully decoded entry read from the append-only WAL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalEntry {
    pub sequence: SequenceNumber,
    pub poh_tick: PoHTick,
    pub previous_hash: Hash512,
    pub commit_hash: Hash512,
    pub entry_hash: Hash512,
    pub schema: String,
    pub payload: Bytes,
    pub witness: CausalWitness,
}

/// Input payload submitted for appending to the WAL.
#[derive(Debug, Clone)]
pub struct WalAppendInput {
    pub schema: String,
    pub payload: Bytes,
    pub witness: CausalWitness,
}

/// Durable receipt returned upon successful fdatasync write to the WAL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalReceipt {
    pub sequence: SequenceNumber,
    pub entry_hash: Hash512,
    pub poh_tick: PoHTick,
}

/// Cryptographic verification report of the entire WAL chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub verified_up_to_seq: SequenceNumber,
    pub root_hash: Hash512,
    pub total_entries: u64,
    pub is_valid: bool,
}

/// Lossless extensibility envelope for unformalized matter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpaqueEnvelope {
    pub envelope_id: String,
    pub schema: String,
    pub source_principal: String,
    pub recorded_at_unix_ms: u64,
    pub indexed_tags: BTreeMap<String, String>,
    pub parent_envelope_id: Option<String>,
    pub payload_json: String,
    pub payload_hash: String,
    pub residual_unknowns: Vec<String>,
}

/// Extensible, unbounded metadata tensor attached to any storage envelope (Fiber Bundle model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataTensor {
    /// Invariant base manifold schema identifier
    pub schema_uri: String,
    /// Cryptographic causal anchor
    pub causal_anchor: Hash512,
    /// Continuous scalar degrees of freedom (energy, probability, duration, market score)
    pub continuous_axes: BTreeMap<String, f64>,
    /// Discrete structural degrees of freedom (spatial scope, agent phase, tenant, role)
    pub discrete_axes: BTreeMap<String, String>,
    /// Explicit residual unknowns for unobserved dimensions
    pub residual_unknowns: Vec<String>,
}

impl MetadataTensor {
    pub fn new(schema_uri: impl Into<String>, causal_anchor: Hash512) -> Self {
        Self {
            schema_uri: schema_uri.into(),
            causal_anchor,
            continuous_axes: BTreeMap::new(),
            discrete_axes: BTreeMap::new(),
            residual_unknowns: Vec::new(),
        }
    }
}

/// 2-byte interned token representing a known metadata axis dimension.
pub type AxisToken = u16;

/// High-throughput zero-heap-allocation metadata tensor.
/// Inlines up to 8 continuous axes, 6 discrete string values, and 4 unknowns on the stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactMetadataTensor {
    pub schema_token: AxisToken,
    pub causal_anchor: Hash512,
    pub continuous_keys: smallvec::SmallVec<[AxisToken; 8]>,
    pub continuous_vals: smallvec::SmallVec<[f64; 8]>,
    pub discrete_keys: smallvec::SmallVec<[AxisToken; 6]>,
    pub discrete_vals: smallvec::SmallVec<[String; 6]>,
    pub residual_unknowns: smallvec::SmallVec<[AxisToken; 4]>,
}

impl CompactMetadataTensor {
    pub fn new(schema_token: AxisToken, causal_anchor: Hash512) -> Self {
        Self {
            schema_token,
            causal_anchor,
            continuous_keys: smallvec::SmallVec::new(),
            continuous_vals: smallvec::SmallVec::new(),
            discrete_keys: smallvec::SmallVec::new(),
            discrete_vals: smallvec::SmallVec::new(),
            residual_unknowns: smallvec::SmallVec::new(),
        }
    }

    pub fn insert_continuous(&mut self, token: AxisToken, val: f64) {
        if let Some(pos) = self.continuous_keys.iter().position(|&k| k == token) {
            self.continuous_vals[pos] = val;
        } else {
            self.continuous_keys.push(token);
            self.continuous_vals.push(val);
        }
    }
}

/// Periodic snapshot checkpoint binding state root to WAL sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCheckpoint {
    pub sequence: SequenceNumber,
    pub root_hash: Hash512,
    pub created_at_unix_ms: u64,
}

/// Distance metric for vector search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    Cosine,
    DotProduct,
    Euclidean,
}

/// Dynamic-dimensional vector embedding representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub model_id: String,
    pub dimension: usize,
    pub metric: DistanceMetric,
    pub components: Vec<f32>,
}

impl VectorEmbedding {
    pub fn new(model_id: impl Into<String>, metric: DistanceMetric, components: Vec<f32>) -> Self {
        let dimension = components.len();
        Self {
            model_id: model_id.into(),
            dimension,
            metric,
            components,
        }
    }
}
