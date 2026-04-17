//! AnchorProvider trait + AnchorId.

use async_trait::async_trait;
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

/// Opaque provider-specific anchor identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorId(pub String);

impl AnchorId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[async_trait]
pub trait AnchorProvider: Send + Sync {
    async fn create_recurring(
        &self,
        domain: &str,
        at: NaiveTime,
        description: &str,
    ) -> crate::Result<AnchorId>;

    async fn update_description(&self, id: AnchorId, new: &str) -> crate::Result<()>;
}

pub mod gcal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_id_roundtrip_via_serde() {
        let a = AnchorId::new("evt-abc");
        let json = serde_json::to_string(&a).unwrap();
        let parsed: AnchorId = serde_json::from_str(&json).unwrap();
        assert_eq!(a, parsed);
        assert_eq!(a.as_str(), "evt-abc");
    }
}
