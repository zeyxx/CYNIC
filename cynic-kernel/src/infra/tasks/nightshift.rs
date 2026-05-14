//! Nightshift — autonomous dev judgment loop.
//! Every 4h: git log --since=24h → judge each commit in domain="dev" → observe patterns.
//! Sovereign: runs inside the kernel process, no bash scripts.

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::constants;
use crate::domain::dog::{MIN_QUORUM, Stimulus};
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;

/// A parsed git commit.
struct GitCommit {
    hash: String,
    message: String,
}

/// Parse output of `git log --format="%h %s"` into a list of commits.
/// Pure function — no I/O.
fn parse_git_log(raw: &str) -> Vec<GitCommit> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            line.split_once(' ').map(|(hash, message)| GitCommit {
                hash: hash.to_string(),
                message: message.to_string(),
            })
        })
        .collect()
}

/// Run `git log --since=<lookback>` in `repo_path` and return parsed commits.
/// Bounded: 10s timeout on the git subprocess.
async fn git_commits_since(lookback: &str, repo_path: &str) -> Result<Vec<GitCommit>, String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("git")
            .args([
                "-C",
                repo_path,
                "log",
                &format!("--since={lookback}"),
                "--format=%h %s",
            ])
            .output(),
    )
    .await
    .map_err(|_elapsed| "git log timed out (10s)".to_string())?
    .map_err(|e| format!("git log failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_git_log(&stdout))
}

/// Map raw observation domains to stable CCM compounding domains.
/// Keeps temporal accumulation coherent instead of fragmenting by file extension.
fn ccm_domain_for_observation(raw_domain: &str) -> &'static str {
    match raw_domain {
        "session" | "session-metrics" | "temporal-meta" => "session",
        "token" | "token-analysis" => "token",
        "chess" => "chess",
        "rust" | "typescript" | "javascript" | "python" | "docs" | "config" => "dev",
        "kernel" | "kernel-lifecycle" | "infra" | "ops" | "git" | "harness" => "ops",
        "D1" | "D2" | "D3" | "D4" | "D5" | "D6" | "twitter" => "twitter",
        "hermes-cycle" | "organ-health" => "hermes",
        _ => "general",
    }
}

/// Judge an observation and observe the result as a CCM crystal.
/// Uses the same crystal observation path as the pipeline (KNN merge, epistemic
/// gate, Q-score normalization) — unifying CHAOS→MATRIX for all crystal sources.
async fn judge_observation(
    obs: &crate::domain::storage::RawObservation,
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
    embedding: &dyn crate::domain::embedding::EmbeddingPort,
    metrics: &Metrics,
) -> Result<(), String> {
    let domain = ccm_domain_for_observation(&obs.domain);
    let stimulus = Stimulus {
        content: format!(
            "[{}] {} {}: {}",
            obs.agent_id, obs.tool, obs.target, obs.context
        ),
        context: Some(format!("tags: {:?}", obs.tags)),
        domain: Some(domain.to_string()),
        request_id: None,
    };

    let verdict = judge
        .evaluate(&stimulus, None, metrics, SlotPriority::Nightshift)
        .await
        .map_err(|e| format!("judge failed for observation {}: {e}", obs.id))?;

    // Embed stimulus for KNN crystal merge (Phase 2: closes the slug fragmentation gap).
    // If embedding fails, falls back to semantic_slug (same as before).
    // R2-exempt: intentional fallback to semantic_slug on embedding failure
    let stimulus_embedding = embedding.embed(&stimulus.content).await.ok();

    // Unified crystal path: same gates (quorum, epistemic, KNN merge) as pipeline.
    crate::pipeline::observe_crystal_for_verdict_core(
        &verdict,
        &stimulus_embedding,
        domain,
        storage.as_ref(),
        metrics,
        None, // no event_tx in nightshift
    )
    .await;

    Ok(())
}

/// Judge a single commit and observe the result as a dev crystal.
/// Uses unified crystal path (same as pipeline: KNN merge, epistemic gate).
async fn judge_commit(
    commit: &GitCommit,
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
    embedding: &dyn crate::domain::embedding::EmbeddingPort,
    metrics: &Metrics,
) -> Result<(), String> {
    let stimulus_content = format!("{}: {}", commit.hash, commit.message);
    let stimulus = Stimulus {
        content: stimulus_content.clone(),
        context: None,
        domain: Some("dev".to_string()),
        request_id: None,
    };

    let verdict = judge
        .evaluate(&stimulus, None, metrics, SlotPriority::Nightshift)
        .await
        .map_err(|e| format!("judge failed for {}: {e}", commit.hash))?;

    // R2-exempt: intentional fallback to semantic_slug on embedding failure
    let stimulus_embedding = embedding.embed(&stimulus_content).await.ok();

    crate::pipeline::observe_crystal_for_verdict_core(
        &verdict,
        &stimulus_embedding,
        "dev",
        storage.as_ref(),
        metrics,
        None,
    )
    .await;

    Ok(())
}

/// Spawn the nightshift loop.
/// - 60s warmup sleep (respects shutdown)
/// - Ticks every `NIGHTSHIFT_INTERVAL` (4h)
/// - Each tick: git log --since=24h → judge each commit → observe dev crystal
/// - Each commit judgment is bounded by `NIGHTSHIFT_COMMIT_TIMEOUT` (5min)
/// - Slot coordination is handled inside Judge::evaluate via SlotSemaphore (Nightshift priority)
pub fn spawn_nightshift_loop(
    judge: Arc<crate::judge::Judge>,
    storage: Arc<dyn StoragePort>,
    embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    repo_path: String,
    metrics: Arc<Metrics>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Warmup: wait 60s before first cycle so the rest of the kernel is fully booted.
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
        }

        let mut interval = tokio::time::interval(constants::NIGHTSHIFT_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // NOTE: Do NOT consume the first tick here. Other loops skip it because
        // their intervals are short (seconds/minutes). Nightshift is 4h — skipping
        // the first tick means the kernel needs 4h+ continuous uptime before the
        // first cycle ever runs. With frequent redeploys, nightshift never fires.
        // The 60s warmup above is sufficient boot protection.

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Nightshift loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    klog!("[Nightshift] Starting cycle — git log --since={}", constants::NIGHTSHIFT_GIT_LOOKBACK);

                    let commits = match git_commits_since(constants::NIGHTSHIFT_GIT_LOOKBACK, &repo_path).await {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::warn!(error = %e, "[Nightshift] git log failed — skipping cycle");
                            task_health.touch_nightshift();
                            continue;
                        }
                    };

                    if commits.is_empty() {
                        klog!("[Nightshift] No commits in the last {} — idle cycle", constants::NIGHTSHIFT_GIT_LOOKBACK);
                        task_health.touch_nightshift();
                        continue;
                    }

                    klog!("[Nightshift] {} commit(s) to judge", commits.len());

                    let cycle_metrics = Metrics::new();

                    // Quorum guard: check that at least MIN_QUORUM Dogs respond
                    // before burning cycles on commits that will all fail at storage level.
                    let probe = Stimulus {
                        content: "nightshift quorum probe".to_string(),
                        context: None,
                        domain: Some("dev".to_string()),
                        request_id: None,
                    };
                    match judge.evaluate(&probe, None, &cycle_metrics, SlotPriority::Nightshift).await {
                        Ok(v) if v.voter_count < MIN_QUORUM => {
                            tracing::warn!(
                                voter_count = v.voter_count,
                                min_quorum = MIN_QUORUM,
                                "[Nightshift] quorum insufficient — skipping cycle (Dogs may be down)"
                            );
                            task_health.touch_nightshift();
                            continue;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "[Nightshift] quorum probe failed — skipping cycle");
                            task_health.touch_nightshift();
                            continue;
                        }
                        Ok(_) => {} // quorum OK, proceed
                    }

                    let mut judged = 0usize;
                    let mut errors = 0usize;

                    for commit in &commits {
                        match tokio::time::timeout(
                            constants::NIGHTSHIFT_COMMIT_TIMEOUT,
                            judge_commit(commit, &judge, &storage, embedding.as_ref(), &cycle_metrics),
                        )
                        .await
                        {
                            Ok(Ok(())) => {
                                judged += 1;
                                tracing::debug!(hash = %commit.hash, "[Nightshift] commit judged");
                            }
                            Ok(Err(e)) => {
                                errors += 1;
                                tracing::warn!(hash = %commit.hash, error = %e, "[Nightshift] commit judgment failed");
                            }
                            Err(_) => {
                                errors += 1;
                                tracing::warn!(hash = %commit.hash, "[Nightshift] commit judgment timed out (5min)");
                            }
                        }
                    }

                    klog!("[Nightshift] Commits: {} judged, {} errors", judged, errors);

                    // Phase 2: judge recent observations across domains (temporal compounding).
                    // Skip verdict-feedback observations (tagged "compound-loop") — they are
                    // metadata about verdicts, not content to judge. Re-judging them wastes
                    // sovereign Dog slots and produces degenerate scores (K20 format mismatch).
                    match storage.list_observations_raw(None, None, 500).await {
                        Ok(observations) if !observations.is_empty() => {
                            let judgeable: Vec<_> = observations.iter()
                                .filter(|o| !o.tags.contains(&"compound-loop".to_string()))
                                .collect();
                            let skipped = observations.len() - judgeable.len();
                            if skipped > 0 {
                                klog!("[Nightshift] {} observation(s) to review ({} compound-loop skipped)", judgeable.len(), skipped);
                            } else {
                                klog!("[Nightshift] {} observation(s) to review", judgeable.len());
                            }
                            let mut s_judged = 0usize;
                            let mut s_errors = 0usize;
                            for obs in &judgeable {
                                match tokio::time::timeout(
                                    constants::NIGHTSHIFT_COMMIT_TIMEOUT,
                                    judge_observation(obs, &judge, &storage, embedding.as_ref(), &cycle_metrics),
                                ).await {
                                    Ok(Ok(())) => s_judged += 1,
                                    Ok(Err(e)) => {
                                        s_errors += 1;
                                        tracing::warn!(id = %obs.id, error = %e, "[Nightshift] observation judgment failed");
                                    }
                                    Err(_) => {
                                        s_errors += 1;
                                        tracing::warn!(id = %obs.id, "[Nightshift] observation judgment timed out");
                                    }
                                }
                            }
                            klog!("[Nightshift] Observations: {} judged, {} errors", s_judged, s_errors);
                            // Persist digestion counters — the organism remembers how much it digested
                            metrics.add_nightshift_digested(s_judged as u64);
                            metrics.add_nightshift_errors(s_errors as u64);
                        }
                        Ok(_) => klog!("[Nightshift] No observations to review"),
                        Err(e) => tracing::warn!(error = %e, "[Nightshift] Failed to fetch observations"),
                    }

                    klog!("[Nightshift] Cycle complete");
                    task_health.touch_nightshift();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_git_log_output() {
        let raw = "abc1234 feat: add new feature\ndef5678 fix: handle edge case\n";
        let commits = parse_git_log(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].message, "feat: add new feature");
    }

    #[test]
    fn parse_git_log_empty() {
        assert!(parse_git_log("").is_empty());
    }

    #[test]
    fn parse_git_log_single_word_message() {
        let commits = parse_git_log("a1b2c3d refactor\n");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "refactor");
    }

    #[test]
    fn maps_observation_domains_to_stable_ccm_domains() {
        assert_eq!(ccm_domain_for_observation("rust"), "dev");
        assert_eq!(ccm_domain_for_observation("session"), "session");
        assert_eq!(ccm_domain_for_observation("session-metrics"), "session");
        assert_eq!(ccm_domain_for_observation("token"), "token");
        assert_eq!(ccm_domain_for_observation("token-analysis"), "token");
        assert_eq!(ccm_domain_for_observation("kernel"), "ops");
        assert_eq!(ccm_domain_for_observation("kernel-lifecycle"), "ops");
        assert_eq!(ccm_domain_for_observation("infra"), "ops");
        assert_eq!(ccm_domain_for_observation("D1"), "twitter");
        assert_eq!(ccm_domain_for_observation("D6"), "twitter");
        assert_eq!(ccm_domain_for_observation("hermes-cycle"), "hermes");
        assert_eq!(ccm_domain_for_observation("unknown"), "general");
    }

    #[tokio::test]
    async fn nightshift_respects_shutdown() {
        let judge = Arc::new(crate::judge::Judge::new(vec![], vec![]));
        let storage: Arc<dyn StoragePort> = Arc::new(crate::domain::storage::NullStorage);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let metrics = Arc::new(Metrics::new());
        let embedding: Arc<dyn crate::domain::embedding::EmbeddingPort> =
            Arc::new(crate::domain::embedding::NullEmbedding);
        let handle = spawn_nightshift_loop(
            judge,
            storage,
            embedding,
            task_health,
            shutdown.clone(),
            "/tmp".to_string(),
            metrics,
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(3), handle)
            .await
            .expect("nightshift should stop within 3s")
            .expect("task should not panic");
    }
}
