//! ActivityStorage trait implementation for ReconnectableStorage.

use super::storage::ReconnectableStorage;
use crate::domain::storage::{ActivityStorage, Observation, RawObservation, StorageError};
use async_trait::async_trait;

#[async_trait]
impl ActivityStorage for ReconnectableStorage {
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        self.current().store_observation(obs).await
    }
    async fn store_event(&self, event: &crate::domain::storage::Event) -> Result<(), StorageError> {
        self.current().store_event(event).await
    }
    async fn fleet_stats(
        &self,
        window_secs: u64,
        limit: u32,
    ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
        self.current().fleet_stats(window_secs, limit).await
    }
    async fn list_degraded_nodes(
        &self,
        window_secs: u64,
        fatal_threshold: f64,
    ) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
        self.current()
            .list_degraded_nodes(window_secs, fatal_threshold)
            .await
    }
    async fn list_events(
        &self,
        node: Option<&str>,
        tool: Option<&str>,
        limit: u32,
    ) -> Result<Vec<crate::domain::storage::RawEvent>, StorageError> {
        self.current().list_events(node, tool, limit).await
    }
    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current()
            .list_observations_raw(domain, agent_id, limit)
            .await
    }
    async fn list_observations_filtered(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        project: Option<&str>,
        workstream: Option<&str>,
        thread_id: Option<&str>,
        target: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current()
            .list_observations_filtered(
                domain, agent_id, project, workstream, thread_id, target, limit,
            )
            .await
    }
    async fn list_observations_by_target(
        &self,
        domain: &str,
        target: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current()
            .list_observations_by_target(domain, target, limit)
            .await
    }
    async fn list_observations_by_tag(
        &self,
        domain: &str,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current()
            .list_observations_by_tag(domain, tag, limit)
            .await
    }
    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
        self.current().last_observation_per_source().await
    }
    async fn count_observations(&self) -> Result<u64, StorageError> {
        self.current().count_observations().await
    }
    async fn store_session_summary(
        &self,
        summary: &crate::domain::ccm::SessionSummary,
    ) -> Result<(), StorageError> {
        self.current().store_session_summary(summary).await
    }
    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::ccm::SessionSummary>, StorageError> {
        self.current().list_session_summaries(limit).await
    }
    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        self.current()
            .get_unsummarized_sessions(min_observations, limit)
            .await
    }
    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        self.current().get_session_observations(session_id).await
    }
    async fn store_session_compliance(
        &self,
        compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        self.current().store_session_compliance(compliance).await
    }
    async fn list_session_compliance(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        self.current().list_session_compliance(limit).await
    }
    async fn zone_activity(
        &self,
        path_prefixes: &[String],
        exclude_agent: &str,
    ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
        self.current()
            .zone_activity(path_prefixes, exclude_agent)
            .await
    }
}
