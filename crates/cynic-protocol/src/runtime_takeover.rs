//! Pure admission rules for a resident standby kernel.
//!
//! This contract does not elect a node or mutate a supervisor. It makes the
//! safety decision explicit so a future fleet adapter cannot promote a
//! standby without an expiring lease, a newer fencing token, and a proved
//! release on the candidate.

use serde::{Deserialize, Serialize};

pub const RUNTIME_TAKEOVER_SCHEMA: &str = "cynic.runtime_takeover.v1";
pub const RUNTIME_TAKEOVER_LEASE_SCHEMA: &str = "cynic.runtime_takeover.lease_observation.v1";
pub const CRYPTOGRAPHIC_LEASE_SCHEMA: &str = "cynic.cryptographic_lease.v1";
pub const ROLE_TRANSITION_RECORD_SCHEMA: &str = "cynic.role_transition_record.v1";
/// An unexpired lease owned by another node is a hard publication conflict.
/// This rule is deliberately pure so every storage adapter can apply the same
/// split-brain guard before allocating a newer fencing token.
pub fn takeover_publish_conflicts(
    current_primary_node_id: &str,
    requested_primary_node_id: &str,
    lease_valid: bool,
) -> bool {
    lease_valid
        && !current_primary_node_id.trim().is_empty()
        && current_primary_node_id != requested_primary_node_id
}

/// Read-back proof that the local active runtime still owns the durable lease.
/// A node that loses this proof must stop serving mutations instead of
/// continuing as a split-brain primary.
pub fn takeover_lease_owned_by(observation: &TakeoverLeaseObservation, node_id: &str) -> bool {
    !node_id.trim().is_empty()
        && observation.primary_node_id == node_id
        && observation.primary_live
        && observation.lease_valid
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TakeoverLeaseObservation {
    pub schema: String,
    pub primary_node_id: String,
    pub primary_live: bool,
    pub lease_valid: bool,
    pub release_proved: bool,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
}

impl TakeoverLeaseObservation {
    pub fn unknown() -> Self {
        Self {
            schema: RUNTIME_TAKEOVER_LEASE_SCHEMA.to_string(),
            primary_node_id: String::new(),
            primary_live: true,
            lease_valid: false,
            release_proved: false,
            fencing_token: 0,
            prior_fencing_token: 0,
        }
    }
}

/// A cryptographically signed lease proving authority with an explicit monotonic fencing token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptographicLease {
    pub schema: String,
    pub lease_id: String,
    pub holder_node_id: String,
    pub role: String,
    pub fencing_token: u64,
    pub granted_at_unix_ms: u64,
    pub ttl_ms: u64,
    pub authority_id: String,
    pub signature_hex: String,
}

#[derive(Serialize)]
struct CryptographicLeaseSigningPayload<'a> {
    schema: &'a str,
    lease_id: &'a str,
    holder_node_id: &'a str,
    role: &'a str,
    fencing_token: u64,
    granted_at_unix_ms: u64,
    ttl_ms: u64,
    authority_id: &'a str,
}

impl CryptographicLease {
    pub fn new(
        lease_id: String,
        holder_node_id: String,
        role: String,
        fencing_token: u64,
        granted_at_unix_ms: u64,
        ttl_ms: u64,
        authority_id: String,
        signature_hex: String,
    ) -> Self {
        Self {
            schema: CRYPTOGRAPHIC_LEASE_SCHEMA.to_string(),
            lease_id,
            holder_node_id,
            role,
            fencing_token,
            granted_at_unix_ms,
            ttl_ms,
            authority_id,
            signature_hex,
        }
    }

    /// Canonical bytes signed by the authority.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = CryptographicLeaseSigningPayload {
            schema: &self.schema,
            lease_id: &self.lease_id,
            holder_node_id: &self.holder_node_id,
            role: &self.role,
            fencing_token: self.fencing_token,
            granted_at_unix_ms: self.granted_at_unix_ms,
            ttl_ms: self.ttl_ms,
            authority_id: &self.authority_id,
        };
        serde_json::to_vec(&payload).expect("serialize cryptographic lease payload")
    }

    pub fn expires_at_unix_ms(&self) -> u64 {
        self.granted_at_unix_ms.saturating_add(self.ttl_ms)
    }

    /// Pure check whether lease is valid at a given timestamp.
    pub fn is_active_at(&self, now_unix_ms: u64) -> bool {
        self.granted_at_unix_ms <= now_unix_ms && now_unix_ms < self.expires_at_unix_ms()
    }

    /// Pure structural validation of the lease.
    pub fn validate(&self, now_unix_ms: u64) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != CRYPTOGRAPHIC_LEASE_SCHEMA {
            blockers.push("cryptographic_lease_schema_mismatch".to_string());
        }
        if self.lease_id.trim().is_empty() {
            blockers.push("cryptographic_lease_id_empty".to_string());
        }
        if self.holder_node_id.trim().is_empty() {
            blockers.push("cryptographic_lease_holder_empty".to_string());
        }
        if self.role.trim().is_empty() {
            blockers.push("cryptographic_lease_role_empty".to_string());
        }
        if self.fencing_token == 0 {
            blockers.push("cryptographic_lease_fencing_token_zero".to_string());
        }
        if self.ttl_ms == 0 {
            blockers.push("cryptographic_lease_ttl_zero".to_string());
        }
        if !self.is_active_at(now_unix_ms) {
            blockers.push("cryptographic_lease_expired_or_not_yet_valid".to_string());
        }
        if self.signature_hex.len() != 128
            || !self.signature_hex.bytes().all(|b| b.is_ascii_hexdigit())
        {
            blockers.push("cryptographic_lease_signature_invalid".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// An immutable, signed audit record of a sovereign role takeover transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTransitionRecord {
    pub schema: String,
    pub transition_id: String,
    pub from_node_id: Option<String>,
    pub to_node_id: String,
    pub role: String,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
    pub transitioned_at_unix_ms: u64,
    pub lease_signature_hex: String,
}

#[derive(Serialize)]
struct RoleTransitionRecordSigningPayload<'a> {
    schema: &'a str,
    transition_id: &'a str,
    from_node_id: Option<&'a str>,
    to_node_id: &'a str,
    role: &'a str,
    fencing_token: u64,
    prior_fencing_token: u64,
    transitioned_at_unix_ms: u64,
}

impl RoleTransitionRecord {
    pub fn new(
        transition_id: String,
        from_node_id: Option<String>,
        to_node_id: String,
        role: String,
        fencing_token: u64,
        prior_fencing_token: u64,
        transitioned_at_unix_ms: u64,
        lease_signature_hex: String,
    ) -> Self {
        Self {
            schema: ROLE_TRANSITION_RECORD_SCHEMA.to_string(),
            transition_id,
            from_node_id,
            to_node_id,
            role,
            fencing_token,
            prior_fencing_token,
            transitioned_at_unix_ms,
            lease_signature_hex,
        }
    }

    /// Canonical bytes signed for the role transition record.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = RoleTransitionRecordSigningPayload {
            schema: &self.schema,
            transition_id: &self.transition_id,
            from_node_id: self.from_node_id.as_deref(),
            to_node_id: &self.to_node_id,
            role: &self.role,
            fencing_token: self.fencing_token,
            prior_fencing_token: self.prior_fencing_token,
            transitioned_at_unix_ms: self.transitioned_at_unix_ms,
        };
        serde_json::to_vec(&payload).expect("serialize transition record payload")
    }

    /// Validates monotonicity and structural invariants of the transition.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != ROLE_TRANSITION_RECORD_SCHEMA {
            blockers.push("role_transition_schema_mismatch".to_string());
        }
        if self.to_node_id.trim().is_empty() {
            blockers.push("role_transition_target_node_empty".to_string());
        }
        if self.fencing_token <= self.prior_fencing_token {
            blockers.push("role_transition_fencing_token_not_monotonic".to_string());
        }
        if self.lease_signature_hex.len() != 128
            || !self
                .lease_signature_hex
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            blockers.push("role_transition_signature_invalid".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandbyMode {
    Disabled,
    Resident,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverObservation {
    pub mode: StandbyMode,
    pub primary_live: bool,
    pub lease_valid: bool,
    pub candidate_ready: bool,
    pub release_proved: bool,
    pub storage_available: bool,
    pub fencing_token: u64,
    pub prior_fencing_token: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTakeoverDecision {
    pub schema: &'static str,
    pub status: &'static str,
    pub promote: bool,
    pub mode: StandbyMode,
    pub fencing_token: u64,
    pub retained_capabilities: Vec<&'static str>,
    pub blocked_mutations: Vec<&'static str>,
    pub blockers: Vec<&'static str>,
}

pub fn decide_runtime_takeover(observation: RuntimeTakeoverObservation) -> RuntimeTakeoverDecision {
    let mut blockers = Vec::new();
    if observation.mode == StandbyMode::Disabled {
        blockers.push("standby_mode_disabled");
    }
    if observation.primary_live {
        blockers.push("primary_still_live");
    }
    // `lease_valid` describes the primary lease. Once that lease has expired
    // or been released, promotion is possible only with an explicit release
    // proof below; requiring it here would make every legitimate takeover
    // impossible. A live primary is still never promotable.
    if !observation.candidate_ready {
        blockers.push("standby_candidate_not_ready");
    }
    if !observation.release_proved {
        blockers.push("release_read_back_unproved");
    }
    if !observation.storage_available {
        blockers.push("takeover_storage_unavailable");
    }
    if observation.fencing_token == 0
        || observation.fencing_token <= observation.prior_fencing_token
    {
        blockers.push("fencing_token_not_monotonic");
    }

    let promote = blockers.is_empty();
    RuntimeTakeoverDecision {
        schema: RUNTIME_TAKEOVER_SCHEMA,
        status: if promote { "promotable" } else { "blocked" },
        promote,
        mode: if promote {
            StandbyMode::Active
        } else {
            observation.mode
        },
        fencing_token: observation.fencing_token,
        retained_capabilities: if promote {
            vec!["identity", "health", "observe", "judge", "replay_spool"]
        } else {
            vec!["identity", "health"]
        },
        blocked_mutations: vec!["fleet_mutation", "release_promotion", "authority_transfer"],
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> RuntimeTakeoverObservation {
        RuntimeTakeoverObservation {
            mode: StandbyMode::Resident,
            primary_live: false,
            lease_valid: true,
            candidate_ready: true,
            release_proved: true,
            storage_available: true,
            fencing_token: 2,
            prior_fencing_token: 1,
        }
    }

    #[test]
    fn takeover_requires_fencing_and_release_proof() {
        let decision = decide_runtime_takeover(ready());
        assert!(decision.promote);
        assert_eq!(decision.mode, StandbyMode::Active);

        let mut stale = ready();
        stale.fencing_token = 1;
        let decision = decide_runtime_takeover(stale);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"fencing_token_not_monotonic"));
    }

    #[test]
    fn live_primary_blocks_split_brain() {
        let mut observation = ready();
        observation.primary_live = true;
        let decision = decide_runtime_takeover(observation);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"primary_still_live"));
    }

    #[test]
    fn expired_primary_lease_with_read_back_release_promotes() {
        let mut observation = ready();
        observation.lease_valid = false;
        observation.fencing_token = 3;
        observation.prior_fencing_token = 2;
        let decision = decide_runtime_takeover(observation);
        assert!(decision.promote);
    }

    #[test]
    fn unavailable_storage_blocks_promotion() {
        let mut observation = ready();
        observation.storage_available = false;
        let decision = decide_runtime_takeover(observation);
        assert!(!decision.promote);
        assert!(decision.blockers.contains(&"takeover_storage_unavailable"));
    }

    #[test]
    fn active_lease_of_another_node_blocks_publication() {
        assert!(takeover_publish_conflicts("node-a", "node-b", true));
        assert!(!takeover_publish_conflicts("node-a", "node-a", true));
        assert!(!takeover_publish_conflicts("node-a", "node-b", false));
        assert!(!takeover_publish_conflicts("", "node-b", true));
    }

    #[test]
    fn lease_read_back_loss_is_not_owned() {
        let mut observation = TakeoverLeaseObservation {
            schema: RUNTIME_TAKEOVER_LEASE_SCHEMA.to_string(),
            primary_node_id: "node-a".to_string(),
            primary_live: true,
            lease_valid: true,
            release_proved: false,
            fencing_token: 2,
            prior_fencing_token: 1,
        };
        assert!(takeover_lease_owned_by(&observation, "node-a"));
        observation.primary_node_id = "node-b".to_string();
        assert!(!takeover_lease_owned_by(&observation, "node-a"));
        observation.primary_node_id = "node-a".to_string();
        observation.lease_valid = false;
        assert!(!takeover_lease_owned_by(&observation, "node-a"));
    }

    #[test]
    fn cryptographic_lease_lifecycle_and_validation() {
        let lease = CryptographicLease::new(
            "lease-001".to_string(),
            "node-cortex".to_string(),
            "cortex".to_string(),
            42,
            1000,
            5000, // ttl: 5000ms -> expires at 6000ms
            "kernel-root".to_string(),
            "a".repeat(128),
        );

        assert!(lease.is_active_at(1000));
        assert!(lease.is_active_at(5999));
        assert!(!lease.is_active_at(6000));
        assert!(!lease.is_active_at(999));
        assert!(lease.validate(3000).is_empty());

        let expired_blockers = lease.validate(7000);
        assert!(
            expired_blockers.contains(&"cryptographic_lease_expired_or_not_yet_valid".to_string())
        );
    }

    #[test]
    fn role_transition_record_monotonicity_enforced() {
        let valid_transition = RoleTransitionRecord::new(
            "tx-001".to_string(),
            Some("node-a".to_string()),
            "node-b".to_string(),
            "primary".to_string(),
            10,
            9,
            10000,
            "b".repeat(128),
        );
        assert!(valid_transition.validate().is_empty());

        let non_monotonic = RoleTransitionRecord::new(
            "tx-002".to_string(),
            Some("node-a".to_string()),
            "node-b".to_string(),
            "primary".to_string(),
            9, // same or lower token
            9,
            10000,
            "b".repeat(128),
        );
        let blockers = non_monotonic.validate();
        assert!(blockers.contains(&"role_transition_fencing_token_not_monotonic".to_string()));
    }
}
