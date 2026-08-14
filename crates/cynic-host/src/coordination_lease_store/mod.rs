use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use cynic_protocol::{
    BatchClaimResult, LeaseAction, LeaseConflict, LeaseGrant, LeaseGuard, LeaseMutationOutcome,
    TargetClaimOutcome,
};

#[derive(Debug, Clone)]
pub struct ActiveLease {
    pub lease_id: String,
    pub target: String,
    pub agent_id: String,
    pub fencing_token: u64,
    pub claimed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct CoordinationLeaseStore {
    leases: Arc<RwLock<BTreeMap<String, ActiveLease>>>,
    token_counter: Arc<RwLock<u64>>,
}

impl CoordinationLeaseStore {
    pub fn new() -> Self {
        Self {
            leases: Arc::new(RwLock::new(BTreeMap::new())),
            token_counter: Arc::new(RwLock::new(1000)),
        }
    }

    fn next_fencing_token(&self) -> u64 {
        let mut counter = self.token_counter.write().expect("token_counter write");
        *counter += 1;
        *counter
    }

    pub fn claim(
        &self,
        agent_id: &str,
        target: &str,
        ttl_seconds: i64,
    ) -> TargetClaimOutcome {
        let mut leases = self.leases.write().expect("leases write");
        let now = Utc::now();

        // Prune expired lease for this target if any
        if let Some(existing) = leases.get(target) {
            if existing.expires_at > now && existing.agent_id != agent_id {
                return TargetClaimOutcome::Conflict {
                    target: target.to_string(),
                    conflicts: vec![LeaseConflict {
                        agent_id: existing.agent_id.clone(),
                        claimed_at: existing.claimed_at.to_rfc3339(),
                    }],
                };
            }
        }

        let fencing_token = self.next_fencing_token();
        let lease_id = format!("lease-{}-{}", target.replace('/', "_"), fencing_token);
        let expires_at = now + Duration::seconds(ttl_seconds);

        let lease = ActiveLease {
            lease_id: lease_id.clone(),
            target: target.to_string(),
            agent_id: agent_id.to_string(),
            fencing_token,
            claimed_at: now,
            expires_at,
        };

        leases.insert(target.to_string(), lease);

        TargetClaimOutcome::Claimed {
            target: target.to_string(),
            grant: LeaseGrant {
                lease_id,
                fencing_token,
                expires_at: expires_at.to_rfc3339(),
            },
        }
    }

    pub fn claim_batch(
        &self,
        agent_id: &str,
        targets: &[String],
        ttl_seconds: i64,
    ) -> BatchClaimResult {
        let mut outcomes = Vec::new();
        for target in targets {
            outcomes.push(self.claim(agent_id, target, ttl_seconds));
        }
        BatchClaimResult { outcomes }
    }

    pub fn mutate(
        &self,
        guard: &LeaseGuard,
        action: LeaseAction,
        extend_ttl_seconds: Option<i64>,
        proof_ref: Option<String>,
    ) -> LeaseMutationOutcome {
        let mut leases = self.leases.write().expect("leases write");
        let now = Utc::now();

        let Some(active) = leases.get_mut(&guard.target) else {
            return LeaseMutationOutcome::rejected(action, &guard.target, guard.fencing_token);
        };

        if active.lease_id != guard.lease_id
            || active.agent_id != guard.agent_id
            || active.fencing_token != guard.fencing_token
            || active.expires_at <= now
        {
            return LeaseMutationOutcome::rejected(action, &guard.target, guard.fencing_token);
        }

        match action {
            LeaseAction::Renew => {
                let ttl = extend_ttl_seconds.unwrap_or(3600);
                active.expires_at = now + Duration::seconds(ttl);
                LeaseMutationOutcome::applied(
                    LeaseAction::Renew,
                    &guard.target,
                    guard.fencing_token,
                    Some(active.expires_at.to_rfc3339()),
                    None,
                )
            }
            LeaseAction::Release => {
                leases.remove(&guard.target);
                LeaseMutationOutcome::applied(
                    LeaseAction::Release,
                    &guard.target,
                    guard.fencing_token,
                    None,
                    None,
                )
            }
            LeaseAction::Finalize => {
                leases.remove(&guard.target);
                LeaseMutationOutcome::applied(
                    LeaseAction::Finalize,
                    &guard.target,
                    guard.fencing_token,
                    None,
                    proof_ref,
                )
            }
        }
    }

    pub fn who(&self) -> Vec<ActiveLease> {
        let leases = self.leases.read().expect("leases read");
        let now = Utc::now();
        leases
            .values()
            .filter(|l| l.expires_at > now)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_claim_and_conflict() {
        let store = CoordinationLeaseStore::new();

        let outcome1 = store.claim("agent-codex", "crates/cynic-inference", 3600);
        assert!(matches!(outcome1, TargetClaimOutcome::Claimed { .. }));

        let outcome2 = store.claim("agent-claude", "crates/cynic-inference", 3600);
        assert!(matches!(outcome2, TargetClaimOutcome::Conflict { .. }));

        if let TargetClaimOutcome::Claimed { grant, .. } = outcome1 {
            let guard = LeaseGuard {
                agent_id: "agent-codex".to_string(),
                target: "crates/cynic-inference".to_string(),
                lease_id: grant.lease_id,
                fencing_token: grant.fencing_token,
            };

            let release = store.mutate(&guard, LeaseAction::Release, None, None);
            assert!(release.is_applied());

            let outcome3 = store.claim("agent-claude", "crates/cynic-inference", 3600);
            assert!(matches!(outcome3, TargetClaimOutcome::Claimed { .. }));
        }
    }
}
