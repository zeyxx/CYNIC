//! CrystalStorage trait implementation for ReconnectableStorage.

use async_trait::async_trait;
use crate::domain::storage::{CrystalStorage, StorageError};
use super::storage::ReconnectableStorage;

#[async_trait]
impl CrystalStorage for ReconnectableStorage {
    async fn store_crystal(
        &self,
        crystal: &crate::domain::ccm::Crystal,
    ) -> Result<(), StorageError> {
        self.current().store_crystal(crystal).await
    }
    async fn get_crystal(
        &self,
        id: &str,
    ) -> Result<Option<crate::domain::ccm::Crystal>, StorageError> {
        self.current().get_crystal(id).await
    }
    async fn list_crystals(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals(limit).await
    }
    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current()
            .list_crystals_filtered(limit, domain, state)
            .await
    }
    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        self.current().delete_crystal(id).await
    }

    async fn update_crystal_relations(
        &self,
        id: &str,
        relations: std::collections::BTreeMap<String, String>,
    ) -> Result<(), StorageError> {
        self.current().update_crystal_relations(id, relations).await
    }

    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals_for_domain(domain, limit).await
    }
    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        self.current().count_crystal_observations().await
    }
    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        voter_count: usize,
        verdict_id: &str,
        verdict_kind: &str,
    ) -> Result<(), StorageError> {
        self.current()
            .observe_crystal(
                id,
                content,
                domain,
                score,
                timestamp,
                voter_count,
                verdict_id,
                verdict_kind,
            )
            .await
    }
    async fn observe_crystal_hypha(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        source: &str,
        sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        self.current()
            .observe_crystal_hypha(id, content, domain, score, timestamp, source, sentiment)
            .await
    }
    async fn shatter_crystal(
        &self,
        id: &str,
        reason: &str,
        source: &str,
        timestamp: i64,
    ) -> Result<(), StorageError> {
        self.current()
            .shatter_crystal(id, reason, source, timestamp)
            .await
    }
    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        self.current().store_crystal_embedding(id, embedding).await
    }
    async fn search_crystals_semantic(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current()
            .search_crystals_semantic(query_embedding, limit)
            .await
    }
    async fn find_similar_crystal(
        &self,
        embedding: &[f32],
        domain: &str,
        threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        self.current()
            .find_similar_crystal(embedding, domain, threshold)
            .await
    }
    async fn list_crystals_missing_embedding(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::Crystal>, StorageError> {
        self.current().list_crystals_missing_embedding(limit).await
    }
    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        self.current().consolidate_duplicate_crystals().await
    }
}
