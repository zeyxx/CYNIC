use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Helius,
    Alchemy,
    QuickNode,
    Chainstack,
    PublicRpc,
    LegacyCompatibility,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Commitment {
    Processed,
    Confirmed,
    Finalized,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecodeStatus {
    RawOnly,
    Decoded,
    PartiallyDecoded,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolanaObservation {
    pub cluster: String,
    pub signature: String,
    pub slot: Option<u64>,
    pub commitment: Commitment,
    pub provider: ProviderKind,
    pub retrieved_at_unix_ms: Option<u128>,
    pub raw_payload_digest: String,
    pub decoder_profile: Option<String>,
    pub decode_status: DecodeStatus,
}

impl SolanaObservation {
    pub fn identity(&self) -> (&str, &str, Option<u64>) {
        (&self.cluster, &self.signature, self.slot)
    }
}
