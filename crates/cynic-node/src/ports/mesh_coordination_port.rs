//! Port: mesh **membership** coordination — who may join, who is revoked.
//!
//! This is deliberately separate from `MeshAdapter` (packet path, interfaces,
//! routes) and from `ProviderControlPort` (machine power on a hypervisor).
//! Carrying traffic, powering a machine, and deciding membership are three
//! different authorities; collapsing them is how a coordination provider ends
//! up treated as network truth.
//!
//! It is also distinct from `cynic_protocol::machine_enrollment::MachineEnrollment`,
//! which is the signed identity of a CYNIC *node*. A node can be enrolled in
//! CYNIC and absent from the mesh, or present on the mesh without a CYNIC
//! identity. Membership here means only: may this key participate in the
//! overlay.
//!
//! Sovereignty is enforced one layer up by
//! `cynic_protocol::network::admit_mesh_provider`: an implementor whose control
//! plane is SaaS is refused before it reaches this port, so nothing here needs
//! to re-check it. Today the only admitted implementor is self-hosted Headscale
//! (LXC 100, `10.10.10.40:8080` — `/health` 200, `/api/v1/node` 401, i.e. alive
//! and authenticating).

use cynic_protocol::network::{MeshRole, MeshSovereignty};

/// One member as the coordination provider sees it.
///
/// This is the provider's *claim*, not observed truth: a member may be listed
/// here and unreachable, which is exactly the divergence class worth detecting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshMember {
    /// Provider-local identifier, needed to address a revocation.
    pub member_id: String,
    /// Name the provider publishes for this member.
    pub name: String,
    /// Overlay addresses the provider believes this member holds.
    pub addresses: Vec<String>,
    /// The provider's own liveness claim. `None` when it does not say.
    pub provider_reports_online: Option<bool>,
}

/// A short-lived credential authorising one node to join the overlay.
///
/// The secret is carried as an opaque string and must never be logged or
/// persisted into evidence; only its presence and expiry are reportable.
#[derive(Debug, Clone)]
pub struct MeshJoinGrant {
    pub secret: String,
    pub expires_at_unix_s: Option<u64>,
    /// The provider that issued it, for attribution in proofs.
    pub issued_by_provider: String,
}

impl MeshJoinGrant {
    /// Reportable projection: proves a grant exists without disclosing it.
    pub fn redacted_summary(&self) -> String {
        format!(
            "grant from {} (len={}, expires={})",
            self.issued_by_provider,
            self.secret.len(),
            self.expires_at_unix_s
                .map(|t| t.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )
    }
}

/// Why a coordination request could not be answered.
///
/// `Unobservable` is distinct from `Failed`: a control plane that cannot be
/// reached has not refused anything, and reporting it as a failure would invent
/// a verdict the provider never gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshCoordinationError {
    /// The control plane could not be reached or did not answer.
    Unobservable(String),
    /// The control plane answered and refused.
    Refused(String),
    /// The control plane answered with something this port cannot interpret.
    Malformed(String),
}

impl std::fmt::Display for MeshCoordinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unobservable(m) => write!(f, "unobservable: {m}"),
            Self::Refused(m) => write!(f, "refused: {m}"),
            Self::Malformed(m) => write!(f, "malformed: {m}"),
        }
    }
}

impl std::error::Error for MeshCoordinationError {}

/// Membership coordination for the sovereign overlay.
///
/// Implementors own enrolment, discovery, and revocation. They must not own
/// interfaces, routes, or topology truth — that belongs to `MeshAdapter` and to
/// the registry respectively.
pub trait MeshCoordinationPort {
    /// Provider name, matching a `provider` in
    /// `cynic_protocol::network::mesh_provider_bindings`.
    fn provider(&self) -> String;

    /// Always `MeshRole::Coordination` for this port. Stated so a misfiled
    /// transport implementor is a visible contradiction rather than silent.
    fn role(&self) -> MeshRole {
        MeshRole::Coordination
    }

    /// Where this implementor's control plane runs.
    fn sovereignty(&self) -> MeshSovereignty;

    /// Read-only: members the provider currently claims. No mutation.
    fn list_members(&self) -> Result<Vec<MeshMember>, MeshCoordinationError>;

    /// Mutation: issue a join credential for `user`.
    fn issue_join_grant(&self, user: &str) -> Result<MeshJoinGrant, MeshCoordinationError>;

    /// Mutation: revoke a member's participation.
    fn revoke_member(&self, member_id: &str) -> Result<(), MeshCoordinationError>;

    /// The provider's liveness claim for one member.
    ///
    /// This is a *claim*, never proof: confirming reachability requires probing
    /// the address, which is the transport layer's job.
    fn provider_liveness_claim(&self, name: &str) -> Result<Option<bool>, MeshCoordinationError> {
        Ok(self
            .list_members()?
            .into_iter()
            .find(|m| m.name == name)
            .and_then(|m| m.provider_reports_online))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeCoordinator {
        members: Vec<MeshMember>,
    }

    impl MeshCoordinationPort for FakeCoordinator {
        fn provider(&self) -> String {
            "headscale".to_string()
        }
        fn sovereignty(&self) -> MeshSovereignty {
            MeshSovereignty::Sovereign
        }
        fn list_members(&self) -> Result<Vec<MeshMember>, MeshCoordinationError> {
            Ok(self.members.clone())
        }
        fn issue_join_grant(&self, user: &str) -> Result<MeshJoinGrant, MeshCoordinationError> {
            Ok(MeshJoinGrant {
                secret: format!("key-for-{user}"),
                expires_at_unix_s: Some(1_800_000_000),
                issued_by_provider: self.provider(),
            })
        }
        fn revoke_member(&self, _member_id: &str) -> Result<(), MeshCoordinationError> {
            Ok(())
        }
    }

    fn fake() -> FakeCoordinator {
        FakeCoordinator {
            members: vec![
                MeshMember {
                    member_id: "1".to_string(),
                    name: "cortex".to_string(),
                    addresses: vec!["10.20.0.3".to_string()],
                    provider_reports_online: Some(true),
                },
                MeshMember {
                    member_id: "2".to_string(),
                    name: "silent".to_string(),
                    addresses: vec!["10.20.0.9".to_string()],
                    provider_reports_online: None,
                },
            ],
        }
    }

    #[test]
    fn the_port_is_coordination_never_transport() {
        assert_eq!(fake().role(), MeshRole::Coordination);
    }

    #[test]
    fn a_grant_is_reportable_without_disclosing_the_secret() {
        let grant = fake().issue_join_grant("cynic").expect("grant");
        let summary = grant.redacted_summary();
        assert!(!summary.contains(&grant.secret), "secret leaked: {summary}");
        assert!(summary.contains("headscale"), "{summary}");
    }

    /// A provider that says nothing about a member must yield `None`, not
    /// `false`: silence is not a claim of being offline.
    #[test]
    fn provider_silence_is_not_an_offline_claim() {
        assert_eq!(fake().provider_liveness_claim("silent").expect("ok"), None);
        assert_eq!(
            fake().provider_liveness_claim("cortex").expect("ok"),
            Some(true)
        );
    }

    /// An unknown name is absence, not an error: the provider answered fine.
    #[test]
    fn an_unknown_member_is_absence_not_failure() {
        assert_eq!(fake().provider_liveness_claim("ghost").expect("ok"), None);
    }

    /// Unreachable and refused must stay distinguishable, so an unreachable
    /// control plane is never reported as a refusal it never made.
    #[test]
    fn unobservable_is_not_refused() {
        let unobservable = MeshCoordinationError::Unobservable("timeout".to_string());
        let refused = MeshCoordinationError::Refused("no such user".to_string());
        assert_ne!(unobservable, refused);
        assert!(unobservable.to_string().starts_with("unobservable"));
        assert!(refused.to_string().starts_with("refused"));
    }
}
