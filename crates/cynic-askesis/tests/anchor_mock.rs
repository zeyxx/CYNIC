#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use chrono::NaiveTime;
use cynic_askesis::anchor::{AnchorId, AnchorProvider};

struct MockAnchor;

#[async_trait]
impl AnchorProvider for MockAnchor {
    async fn create_recurring(
        &self,
        domain: &str,
        _at: NaiveTime,
        _desc: &str,
    ) -> cynic_askesis::Result<AnchorId> {
        Ok(AnchorId::new(format!("mock-{domain}")))
    }
    async fn update_description(&self, _id: AnchorId, _new: &str) -> cynic_askesis::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn mock_anchor_creates_and_updates() {
    let anchor = MockAnchor;
    let id = anchor
        .create_recurring("body", NaiveTime::from_hms_opt(19, 0, 0).unwrap(), "test")
        .await
        .unwrap();
    assert_eq!(id.as_str(), "mock-body");
    anchor.update_description(id, "updated").await.unwrap();
}
