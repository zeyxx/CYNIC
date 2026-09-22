use ed25519_dalek::VerifyingKey;
use serde;

/// Agent identity — derived from ed25519 public key (first 16 bytes of BLAKE3 hash).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentId(pub [u8; 16]);

impl AgentId {
    pub fn from_ed25519(key: &VerifyingKey) -> Self {
        let hash = blake3::hash(key.as_bytes());
        let mut id = [0u8; 16];
        id.copy_from_slice(&hash.as_bytes()[..16]);
        AgentId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    WaitingForAgent,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskKind {
    Solo,
    Chain,
    Fanout,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub created_by: AgentId,
    pub assigned_to: AgentId,
    pub kind: TaskKind,
    pub subject: String,
    pub body: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at_unix_s: u64,
    pub updated_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub budget: Budget,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Budget {
    pub actions: u32,
    pub wall_seconds: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    pub claim_id: String,
    pub task_id: String,
    pub agent: AgentId,
    pub status: ClaimStatus,
    pub budget: Budget,
    pub consumed_actions: u32,
    pub scope: Scope,
    pub acquired_at_unix_s: u64,
    pub ttl_seconds: u32,
    pub parent_claim_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClaimStatus {
    Active,
    Expired,
    Released,
    Preempted,
    Blocked,
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("claim {0} already exists")]
    AlreadyExists(String),
    #[error("claim {0} not found")]
    NotFound(String),
    #[error("scope {0} is locked by another task")]
    ScopeLocked(String),
    #[error("budget exhausted for claim {0}")]
    BudgetExhausted(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub kind: ScopeKind,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScopeKind {
    File,
    Module,
    Crate,
}

/// 32-byte hash (SHA-256 or BLAKE3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash32(pub [u8; 32]);

impl serde::Serialize for Hash32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("sha256:{}", hex::encode(self.0)))
    }
}

impl<'de> serde::Deserialize<'de> for Hash32 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        let s = s
            .strip_prefix("sha256:")
            .ok_or_else(|| serde::de::Error::custom("expected sha256: prefix"))?;
        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash32(arr))
    }
}


pub struct Proof {
    pub parent_task_id: String,
    pub child_task_id: String,
    pub scope: Scope,
    pub consumed_budget: Budget,
    pub output_hash: Hash32,
    pub timestamp_unix_s: u64,
}
