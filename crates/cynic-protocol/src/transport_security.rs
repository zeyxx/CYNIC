use serde::{Deserialize, Serialize};

<<<<<<< HEAD
/// Shared proof-evidence vocabulary used across build, cli, host, node and vcs
/// surfaces. Kept in the protocol residue because it is cross-family vocabulary,
/// not transport-specific (40+ consumer sites across 5 crates).
=======
pub const TRANSPORT_SECURITY_REPORT_SCHEMA: &str = "cynic.transport_security.report.v1";
pub const NETWORK_PATH_EVIDENCE_SCHEMA: &str = "cynic.network_path.evidence.v1";

>>>>>>> github/main
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEvidenceState {
    Proven,
    Refuted,
    Declared,
    Absent,
    Unknown,
    NotApplicable,
}
<<<<<<< HEAD
=======

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportScope {
    LocalProcess,
    LocalPrivilegeBoundary,
    HypervisorGuest,
    RemoteNetwork,
    ExternalProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPathEvidence {
    pub schema: String,
    pub id: String,
    pub service_ref: String,
    pub name_resolution: SecurityEvidenceState,
    pub network_route: SecurityEvidenceState,
    pub transport_reachability: SecurityEvidenceState,
    pub confidentiality: SecurityEvidenceState,
    pub peer_identity: SecurityEvidenceState,
    pub application_identity: SecurityEvidenceState,
    #[serde(default)]
    pub resolved_addresses: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl NetworkPathEvidence {
    pub fn admits_authenticated_application(&self) -> bool {
        self.schema == NETWORK_PATH_EVIDENCE_SCHEMA
            && self.name_resolution == SecurityEvidenceState::Proven
            && self.network_route == SecurityEvidenceState::Proven
            && self.transport_reachability == SecurityEvidenceState::Proven
            && self.confidentiality == SecurityEvidenceState::Proven
            && self.peer_identity == SecurityEvidenceState::Proven
            && self.application_identity == SecurityEvidenceState::Proven
            && !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecuritySurface {
    pub id: String,
    pub scope: TransportScope,
    pub mutates: bool,
    pub confidentiality: SecurityEvidenceState,
    pub peer_identity: SecurityEvidenceState,
    pub mutual_authentication: SecurityEvidenceState,
    pub authorization_binding: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub residual_risks: Vec<String>,
}

impl TransportSecuritySurface {
    pub fn admits_remote_mutation(&self) -> bool {
        self.scope == TransportScope::RemoteNetwork
            && self.mutates
            && self.confidentiality == SecurityEvidenceState::Proven
            && self.peer_identity == SecurityEvidenceState::Proven
            && self.mutual_authentication == SecurityEvidenceState::Proven
            && self.authorization_binding == SecurityEvidenceState::Proven
            && !self.evidence_refs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportSecurityReport {
    pub schema: String,
    pub status: String,
    pub surfaces: Vec<TransportSecuritySurface>,
    pub remote_mutation_blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

impl TransportSecurityReport {
    pub fn from_surfaces(surfaces: Vec<TransportSecuritySurface>) -> Self {
        let mut remote_mutation_blockers = Vec::new();
        let mut residual_unknowns = Vec::new();
        for surface in &surfaces {
            if surface.scope == TransportScope::RemoteNetwork
                && surface.mutates
                && !surface.admits_remote_mutation()
            {
                remote_mutation_blockers
                    .push(format!("{}:remote_mutation_identity_unproven", surface.id));
            }
            for (field, state) in [
                ("confidentiality", surface.confidentiality),
                ("peer_identity", surface.peer_identity),
                ("mutual_authentication", surface.mutual_authentication),
                ("authorization_binding", surface.authorization_binding),
            ] {
                if state == SecurityEvidenceState::Unknown {
                    residual_unknowns.push(format!("{}:{field}", surface.id));
                }
            }
        }
        let status = if !remote_mutation_blockers.is_empty() {
            "blocked"
        } else if !residual_unknowns.is_empty() {
            "partial"
        } else {
            "proved"
        };
        Self {
            schema: TRANSPORT_SECURITY_REPORT_SCHEMA.to_string(),
            status: status.to_string(),
            surfaces,
            remote_mutation_blockers,
            residual_unknowns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_mutation_requires_mutual_identity_and_authorization_proof() {
        let report = TransportSecurityReport::from_surfaces(vec![TransportSecuritySurface {
            id: "fleet_node_exec".to_string(),
            scope: TransportScope::RemoteNetwork,
            mutates: true,
            confidentiality: SecurityEvidenceState::Declared,
            peer_identity: SecurityEvidenceState::Unknown,
            mutual_authentication: SecurityEvidenceState::Unknown,
            authorization_binding: SecurityEvidenceState::Declared,
            evidence_refs: Vec::new(),
            residual_risks: vec!["machine identity enrollment absent".to_string()],
        }]);

        assert_eq!(report.status, "blocked");
        assert_eq!(
            report.remote_mutation_blockers,
            vec!["fleet_node_exec:remote_mutation_identity_unproven"]
        );
        assert!(report.residual_unknowns.len() >= 2);
    }

    #[test]
    fn open_transport_is_not_authenticated_application_evidence() {
        let path = NetworkPathEvidence {
            schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
            id: "forgejo-admin".to_string(),
            service_ref: "forgejo".to_string(),
            name_resolution: SecurityEvidenceState::Proven,
            network_route: SecurityEvidenceState::Proven,
            transport_reachability: SecurityEvidenceState::Proven,
            confidentiality: SecurityEvidenceState::Absent,
            peer_identity: SecurityEvidenceState::Unknown,
            application_identity: SecurityEvidenceState::Unknown,
            resolved_addresses: vec!["10.10.10.101:3000".to_string()],
            evidence_refs: vec!["tcp-probe.json".to_string()],
        };
        assert!(!path.admits_authenticated_application());
    }
}
>>>>>>> github/main
