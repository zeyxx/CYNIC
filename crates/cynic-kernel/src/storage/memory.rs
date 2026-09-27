//! InMemory Storage Adapter — deterministic, no external dependencies.
//!
//! Implements the same StoragePort contract as SurrealHttpStorage, including:
//! - Crystal state transitions at exact thresholds (21/233 obs, φ⁻¹/φ⁻² confidence)
//! - Quorum gate (voter_count < MIN_QUORUM → reject)
//! - Content evolves: updated when new observation has higher confidence (KC12 fix)
//! - Content sanitization via domain::sanitize
//! - Sorting by maturity then confidence DESC
//!
//! Used for: fast unit tests, CI without SurrealDB, proving StoragePort agnosticism.

use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::domain::ccm::{Crystal, CrystalState, SessionSummary};
use crate::domain::dog::{MIN_QUORUM, Verdict};
use cynic_eval::eval::dog_health::DogStats;
use crate::domain::sanitize::sanitize_crystal_content;
use crate::domain::storage::{
    ActivityStorage, AgentDispatch, AgentTask, CrystalStorage, DispatchStorage, Event, MeshNode,
    MeshRoute, MeshStorage, Observation, ProofOfHistory, ProofOfHistoryStorage, RawEvent,
    RawObservation, StateLogStorage, StorageError, StoragePort, SubmissionStorage, TaskStorage,
    UsageRow, VerdictStorage,
};
use crate::domain::usage::DogUsage;
use crate::domain::verdict_queue::QueuedVerdict;

/// In-memory storage for deterministic testing. Thread-safe via tokio::sync::Mutex.
/// Uses tokio Mutex (not std) to prevent latent .await-while-locked traps.
#[derive(Debug)]
pub struct InMemoryStorage {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    verdicts: Vec<Verdict>,
    crystals: HashMap<String, Crystal>,
    observations: Vec<RawObservation>,
    summaries: Vec<SessionSummary>,
    usage: HashMap<String, UsageRow>,
    tasks: HashMap<String, AgentTask>,
    events: Vec<Event>,
    dog_stats: HashMap<String, DogStats>,
    mesh_nodes: HashMap<String, MeshNode>,
    mesh_routes: Vec<MeshRoute>,
}

// State computation uses domain function (ccm::classify) — single source of truth.

/// Sort crystals by maturity (Canonical > Crystallized > Forming > Decaying > Dissolved)
/// then by confidence DESC within same state.
fn state_rank(s: &CrystalState) -> u8 {
    match s {
        CrystalState::Canonical => 0,
        CrystalState::Crystallized => 1,
        CrystalState::Forming => 2,
        CrystalState::Decaying => 3,
        CrystalState::Dissolved => 4,
    }
}

fn get_coordination_workstream(row: &crate::domain::storage::RawObservation) -> Option<String> {
    row.value.as_ref().and_then(|value| {
        value
            .get("workstream")
            .and_then(|value| value.as_str().map(ToString::to_string))
            .or_else(|| extract_workstream_from_coordination_target(&row.target))
    })
}

fn get_coordination_thread_id(row: &crate::domain::storage::RawObservation) -> Option<String> {
    row.value.as_ref().and_then(|value| {
        value
            .get("thread_id")
            .and_then(|value| value.as_str().map(ToString::to_string))
            .or_else(|| extract_thread_from_coordination_target(&row.target))
    })
}

fn extract_thread_from_coordination_target(target: &str) -> Option<String> {
    target.split_once('#').map(|(_, thread)| thread.to_string())
}

fn extract_workstream_from_coordination_target(target: &str) -> Option<String> {
    let after_prefix = target.strip_prefix("coord://")?;
    let without_thread = after_prefix.split('#').next()?;
    let mut parts = without_thread.split('/');
    parts.next()?;
    let workstream = parts.next()?;
    if workstream.is_empty() {
        None
    } else {
        Some(workstream.to_string())
    }
}

include!("memory/verdicts.rs");
include!("memory/crystals.rs");
include!("memory/activity.rs");
include!("memory/agent_tasks.rs");
include!("memory/mesh.rs");
include!("memory/dispatch.rs");
include!("memory/state_log.rs");
include!("memory/submission.rs");
include!("memory/core.rs");
include!("memory/proof_of_history.rs");

#[cfg(test)]
mod crystal_hypha_tests {
    use super::*;

    #[tokio::test]
    async fn hypha_observe_creates_forming_crystal() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "hermes-skill",
            0.5,
            1_778_803_200_000,
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Forming);
        assert_eq!(c.observations, 1);
        assert_eq!(c.source_diversity, 1);
    }

    #[tokio::test]
    async fn hypha_observe_feeds_existing_crystal() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "hermes-skill",
            0.5,
            1_778_803_200_000,
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "hermes-skill",
            0.6,
            1_778_806_800_000,
            "hermes-agent",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.observations, 2);
        assert!((c.confidence - 0.55).abs() < 0.01);
    }

    #[tokio::test]
    async fn hypha_observe_tracks_multiple_sources() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            1_778_803_200_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.6,
            1_778_806_800_000,
            "claude",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.55,
            1_778_810_400_000,
            "gemini",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.source_diversity, 3);
    }

    #[tokio::test]
    async fn hypha_observe_rejected_on_dissolved() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            1_778_803_200_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.shatter_crystal("test-1", "rug", "admin", 1_778_806_800_000)
            .await
            .unwrap();
        let result = s
            .observe_crystal_hypha(
                "test-1",
                "",
                "skill",
                0.6,
                1_778_810_400_000,
                "hermes",
                None,
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn shatter_transitions_to_dissolved() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "insight",
            "skill",
            0.5,
            1_778_803_200_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.shatter_crystal("test-1", "rug", "watch", 1_778_806_800_000)
            .await
            .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.state, CrystalState::Dissolved);
    }

    #[tokio::test]
    async fn sentiment_maps_to_polarity() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "good",
            "skill",
            0.7,
            1_778_803_200_000,
            "hermes",
            Some("positive"),
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.3,
            1_778_806_800_000,
            "hermes",
            Some("negative"),
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.wag_count, 1);
        assert_eq!(c.growl_count, 1);
    }

    #[tokio::test]
    async fn content_updates_on_higher_score() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "v1",
            "skill",
            0.5,
            1_778_803_200_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "v2",
            "skill",
            0.8,
            1_778_806_800_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.content, "v2");
    }

    #[tokio::test]
    async fn content_does_not_update_on_lower_score() {
        let s = InMemoryStorage::new();
        s.observe_crystal_hypha(
            "test-1",
            "v1",
            "skill",
            0.8,
            1_778_803_200_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        s.observe_crystal_hypha(
            "test-1",
            "",
            "skill",
            0.3,
            1_778_806_800_000,
            "hermes",
            None,
        )
        .await
        .unwrap();
        let c = s.get_crystal("test-1").await.unwrap().unwrap();
        assert_eq!(c.content, "v1");
    }

    #[tokio::test]
    async fn shatter_nonexistent_crystal_errors() {
        let s = InMemoryStorage::new();
        let result = s.shatter_crystal("no", "no", "no", 0).await;
        assert!(result.is_err());
    }
}
