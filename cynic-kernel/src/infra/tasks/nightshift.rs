//! Nightshift — autonomous dev judgment loop.
//! Every 4h: git log --since=24h → judge each commit in domain="dev" → observe patterns.
//! Sovereign: runs inside the kernel process, no bash scripts.

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::constants;
use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
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

/// Judge a session observation and observe the result as a session crystal.
async fn judge_session_observation(
    obs: &crate::domain::storage::RawObservation,
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
) -> Result<(), String> {
    let stimulus = Stimulus {
        content: format!("[{}] {}: {}", obs.agent_id, obs.tool, obs.context),
        context: Some(format!("tags: {:?}", obs.tags)),
        domain: Some("session".to_string()),
        request_id: None,
    };

    let metrics = Metrics::new();
    let verdict = judge
        .evaluate(&stimulus, None, &metrics)
        .await
        .map_err(|e| format!("judge failed for session obs {}: {e}", obs.id))?;

    let timestamp = chrono::Utc::now().to_rfc3339();
    let verdict_kind = format!("{:?}", verdict.kind).to_lowercase();

    storage
        .observe_crystal(
            &format!("nightshift-session-{}", &obs.id),
            &stimulus.content,
            "session",
            verdict.q_score.total,
            &timestamp,
            verdict.voter_count,
            &verdict.id,
            &verdict_kind,
        )
        .await
        .map_err(|e| format!("observe_crystal failed for session obs {}: {e}", obs.id))?;

    Ok(())
}

/// Judge a single commit and observe the result as a dev crystal.
async fn judge_commit(
    commit: &GitCommit,
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
) -> Result<(), String> {
    let stimulus = Stimulus {
        content: format!("{}: {}", commit.hash, commit.message),
        context: None,
        domain: Some("dev".to_string()),
        request_id: None,
    };

    let metrics = Metrics::new();
    let verdict = judge
        .evaluate(&stimulus, None, &metrics)
        .await
        .map_err(|e| format!("judge failed for {}: {e}", commit.hash))?;

    let timestamp = chrono::Utc::now().to_rfc3339();
    let verdict_kind = format!("{:?}", verdict.kind).to_lowercase();

    storage
        .observe_crystal(
            &format!("nightshift-{}", commit.hash),
            &stimulus.content,
            "dev",
            verdict.q_score.total,
            &timestamp,
            verdict.voter_count,
            &verdict.id,
            &verdict_kind,
        )
        .await
        .map_err(|e| format!("observe_crystal failed for {}: {e}", commit.hash))?;

    Ok(())
}

/// Spawn the nightshift loop.
/// - 60s warmup sleep (respects shutdown)
/// - Ticks every `NIGHTSHIFT_INTERVAL` (4h)
/// - Each tick: git log --since=24h → judge each commit → observe dev crystal
/// - Each commit judgment is bounded by `NIGHTSHIFT_COMMIT_TIMEOUT` (5min)
pub fn spawn_nightshift_loop(
    judge: Arc<crate::judge::Judge>,
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    repo_path: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Warmup: wait 60s before first cycle so the rest of the kernel is fully booted.
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
        }

        let mut interval = tokio::time::interval(constants::NIGHTSHIFT_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick — consistent with all other loops

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
                    let mut judged = 0usize;
                    let mut errors = 0usize;

                    for commit in &commits {
                        match tokio::time::timeout(
                            constants::NIGHTSHIFT_COMMIT_TIMEOUT,
                            judge_commit(commit, &judge, &storage),
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

                    // Phase 2: judge recent session observations (inter-agent learning)
                    match storage.list_observations_raw(Some("session"), None, 10).await {
                        Ok(session_obs) if !session_obs.is_empty() => {
                            klog!("[Nightshift] {} session observation(s) to review", session_obs.len());
                            let mut s_judged = 0usize;
                            let mut s_errors = 0usize;
                            for obs in &session_obs {
                                match tokio::time::timeout(
                                    constants::NIGHTSHIFT_COMMIT_TIMEOUT,
                                    judge_session_observation(obs, &judge, &storage),
                                ).await {
                                    Ok(Ok(())) => s_judged += 1,
                                    Ok(Err(e)) => {
                                        s_errors += 1;
                                        tracing::warn!(id = %obs.id, error = %e, "[Nightshift] session obs judgment failed");
                                    }
                                    Err(_) => {
                                        s_errors += 1;
                                        tracing::warn!(id = %obs.id, "[Nightshift] session obs judgment timed out");
                                    }
                                }
                            }
                            klog!("[Nightshift] Sessions: {} judged, {} errors", s_judged, s_errors);
                        }
                        Ok(_) => klog!("[Nightshift] No session observations to review"),
                        Err(e) => tracing::warn!(error = %e, "[Nightshift] Failed to fetch session observations"),
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

    #[tokio::test]
    async fn nightshift_respects_shutdown() {
        let judge = Arc::new(crate::judge::Judge::new(vec![], vec![]));
        let storage: Arc<dyn StoragePort> = Arc::new(crate::domain::storage::NullStorage);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_nightshift_loop(
            judge,
            storage,
            task_health,
            shutdown.clone(),
            "/tmp".to_string(),
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(3), handle)
            .await
            .expect("nightshift should stop within 3s")
            .expect("task should not panic");
    }
}
