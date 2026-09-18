use super::types::*;

pub fn reconcile(observation: &SyncObservation) -> SyncDecision {
    let mut actions = Vec::new();
    let mut needs_review = false;
    let mut unknowns = Vec::new();

    for layer in [&observation.forgejo, &observation.github] {
        if !layer.reachable || layer.remote_url.is_none() {
            needs_review = true;
            unknowns.extend(
                layer
                    .blockers
                    .iter()
                    .map(|blocker| format!("{}: {blocker}", layer.layer)),
            );
            if layer.blockers.is_empty() {
                unknowns.push(format!("{} remote identity is unresolved", layer.layer));
            }
        }
    }

    if !unknowns.is_empty() {
        return SyncDecision {
            schema: "cynic.host.source_sync_decision.v1",
            observation: observation.clone(),
            actions,
            needs_review,
            unknowns,
        };
    }

    for layer in [
        &observation.cortex,
        &observation.forgejo,
        &observation.github,
    ] {
        for blocker in &layer.blockers {
            if blocker.contains("unresolved merge conflicts") {
                needs_review = true;
                unknowns.push(format!("{}: {}", layer.layer, blocker));
            }
        }
    }

    if let Some(ahead) = observation.cortex.commits_ahead {
        if ahead > 0 && observation.cortex.dirty_files.is_empty() {
            actions.push(SyncAction::PushCortexToForgejo { commits: ahead });
        } else if ahead > 0 && !observation.cortex.dirty_files.is_empty() {
            unknowns.push(format!(
                "cortex has {} unpushed commits but {} dirty files",
                ahead,
                observation.cortex.dirty_files.len()
            ));
            needs_review = true;
        }
    }

    if let Some(ahead) = observation.forgejo.commits_ahead {
        if ahead > 0 {
            if ahead > 50 {
                actions.push(SyncAction::ArchiveBranch {
                    branch: "origin/main".to_string(),
                });
                actions.push(SyncAction::PushForgejoToGithub { commits: ahead });
                needs_review = true;
                unknowns.push(format!(
                    "large gap ({} commits) — archive + force-push required",
                    ahead
                ));
            } else {
                actions.push(SyncAction::PushForgejoToGithub { commits: ahead });
            }
        }
    }

    if let (Some(cortex_behind), Some(forgejo_behind)) = (
        observation.cortex.commits_behind,
        observation.forgejo.commits_behind,
    ) {
        if cortex_behind > 0 && forgejo_behind > 0 {
            unknowns.push(format!(
                "divergence: cortex behind forgejo by {}, forgejo behind github by {}",
                cortex_behind, forgejo_behind
            ));
            needs_review = true;
        }
    }

    if actions.is_empty() && unknowns.is_empty() {
        actions.push(SyncAction::NoOp {
            reason: "all layers converged".to_string(),
        });
    }

    SyncDecision {
        schema: "cynic.host.source_sync_decision.v1",
        observation: observation.clone(),
        actions,
        needs_review,
        unknowns,
    }
}

pub fn plan(decision: &SyncDecision) -> SyncPlan {
    let mut steps = Vec::new();
    let mut rollback = RollbackIntent::None;

    for action in &decision.actions {
        match action {
            SyncAction::PushCortexToForgejo { commits: _ } => {
                if let Some(name) = decision.observation.forgejo.remote_name.as_deref() {
                    rollback = RollbackIntent::ResetCortex {
                        to_sha: decision
                            .observation
                            .cortex
                            .head_sha
                            .clone()
                            .unwrap_or_default(),
                    };
                    steps.push(SyncStep::GitCommand {
                        argv: vec![
                            "git".to_string(),
                            "push".to_string(),
                            name.to_string(),
                            "main".to_string(),
                        ],
                        layer: SourceLayer::Cortex,
                    });
                }
            }
            SyncAction::PushForgejoToGithub { commits: _ } => {
                if let Some(name) = decision.observation.github.remote_name.as_deref() {
                    steps.push(SyncStep::GitCommand {
                        argv: vec![
                            "git".to_string(),
                            "push".to_string(),
                            "--force".to_string(),
                            name.to_string(),
                            "main".to_string(),
                        ],
                        layer: SourceLayer::Forgejo,
                    });
                }
            }
            SyncAction::MergeBranchIntoMain { branch, strategy } => match strategy {
                MergeStrategy::Merge => {
                    steps.push(SyncStep::GitCommand {
                        argv: vec![
                            "git".to_string(),
                            "merge".to_string(),
                            "--no-ff".to_string(),
                            branch.clone(),
                        ],
                        layer: SourceLayer::Cortex,
                    });
                }
                MergeStrategy::Squash => {
                    steps.push(SyncStep::GitCommand {
                        argv: vec![
                            "git".to_string(),
                            "merge".to_string(),
                            "--squash".to_string(),
                            branch.clone(),
                        ],
                        layer: SourceLayer::Cortex,
                    });
                }
                MergeStrategy::Rebase => {
                    steps.push(SyncStep::GitCommand {
                        argv: vec!["git".to_string(), "rebase".to_string(), "main".to_string()],
                        layer: SourceLayer::Cortex,
                    });
                }
            },
            SyncAction::ArchiveBranch { branch } => {
                let archive_ref = format!("archive/{}", branch.replace('/', "-"));
                steps.push(SyncStep::Archive {
                    branch: branch.clone(),
                    archive_ref,
                });
            }
            SyncAction::NoOp { reason: _ } => {}
        }
    }

    SyncPlan {
        schema: "cynic.host.source_sync_plan.v1",
        decision: decision.clone(),
        steps,
        rollback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_observation() -> SyncObservation {
        SyncObservation {
            schema: "test",
            cortex: SourceState {
                layer: SourceLayer::Cortex,
                remote_name: None,
                remote_url: None,
                head_sha: Some("abc123".to_string()),
                branch: Some("main".to_string()),
                commits_ahead: Some(0),
                commits_behind: Some(0),
                dirty_files: vec![],
                reachable: true,
                blockers: vec![],
            },
            forgejo: SourceState {
                layer: SourceLayer::Forgejo,
                remote_name: Some("origin".to_string()),
                remote_url: Some("ssh://forgejo@...".to_string()),
                head_sha: Some("abc123".to_string()),
                branch: Some("main".to_string()),
                commits_ahead: Some(0),
                commits_behind: Some(0),
                dirty_files: vec![],
                reachable: true,
                blockers: vec![],
            },
            github: SourceState {
                layer: SourceLayer::GitHub,
                remote_name: Some("github".to_string()),
                remote_url: Some("https://github.com/...".to_string()),
                head_sha: Some("abc123".to_string()),
                branch: Some("main".to_string()),
                commits_ahead: None,
                commits_behind: None,
                dirty_files: vec![],
                reachable: true,
                blockers: vec![],
            },
            timestamp_unix_s: 0,
        }
    }

    #[test]
    fn converged_returns_noop() {
        let obs = mock_observation();
        let decision = reconcile(&obs);
        assert_eq!(decision.actions.len(), 1);
        assert!(matches!(&decision.actions[0], SyncAction::NoOp { .. }));
    }

    #[test]
    fn cortex_ahead_returns_push() {
        let mut obs = mock_observation();
        obs.cortex.commits_ahead = Some(5);
        let decision = reconcile(&obs);
        assert!(
            decision
                .actions
                .iter()
                .any(|a| matches!(a, SyncAction::PushCortexToForgejo { commits: 5 }))
        );
    }

    #[test]
    fn forgejo_ahead_returns_push_to_github() {
        let mut obs = mock_observation();
        obs.forgejo.commits_ahead = Some(10);
        let decision = reconcile(&obs);
        assert!(
            decision
                .actions
                .iter()
                .any(|a| matches!(a, SyncAction::PushForgejoToGithub { commits: 10 }))
        );
    }

    #[test]
    fn large_gap_needs_review() {
        let mut obs = mock_observation();
        obs.forgejo.commits_ahead = Some(500);
        let decision = reconcile(&obs);
        assert!(decision.needs_review);
    }

    #[test]
    fn dirty_cortex_blocks_push() {
        let mut obs = mock_observation();
        obs.cortex.commits_ahead = Some(5);
        obs.cortex.dirty_files = vec!["file.rs".to_string()];
        let decision = reconcile(&obs);
        assert!(decision.needs_review);
        assert!(decision.actions.is_empty());
    }

    #[test]
    fn unreachable_remote_cannot_report_convergence() {
        let mut obs = mock_observation();
        obs.forgejo.reachable = false;
        obs.forgejo.remote_url = None;
        obs.forgejo
            .blockers
            .push("forgejo remote is not reachable".to_string());

        let decision = reconcile(&obs);

        assert!(decision.needs_review);
        assert!(decision.actions.is_empty());
        assert!(
            decision
                .unknowns
                .iter()
                .any(|unknown| unknown.contains("forgejo remote is not reachable"))
        );
    }
}
