use cynic_host::runtime_seed::{SshRuntimeSeedSignatureVerifier, observe_runtime_seed};
use cynic_protocol::{SecurityEvidenceState};
use cynic_node_protocol::{NODE_CONTROL_OBSERVATION_SCHEMA, NodeControlConnectionState, NodeControlObservation, NodeDesiredState};

use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::ports::node_control_plane_port::NodeControlObservationPort;

#[derive(Debug, Clone)]
pub struct LocalNodeControlObserver {
    node_id: String,
    enrollment_reader: LocalNodeEnrollmentReader,
    observed_capabilities: Vec<String>,
    observed_services: Vec<String>,
}

impl LocalNodeControlObserver {
    pub fn new(
        node_id: String,
        enrollment_reader: LocalNodeEnrollmentReader,
        observed_capabilities: Vec<String>,
        observed_services: Vec<String>,
    ) -> Self {
        Self {
            node_id,
            enrollment_reader,
            observed_capabilities,
            observed_services,
        }
    }
}

impl NodeControlObservationPort for LocalNodeControlObserver {
    async fn observe_node_control(
        &self,
        desired: &NodeDesiredState,
    ) -> Result<NodeControlObservation, String> {
        let enrollment = self.enrollment_reader.read_back(&self.node_id);
        let release = observe_runtime_seed(&desired.release, &SshRuntimeSeedSignatureVerifier);
        let mut unknowns = release.unknowns.clone();
        unknowns.extend(enrollment.unknowns.clone());
        if enrollment.state == SecurityEvidenceState::Unknown {
            unknowns.push("node_enrollment_grant_unobserved".to_string());
        }
        unknowns.sort();
        unknowns.dedup();
        Ok(NodeControlObservation {
            schema: NODE_CONTROL_OBSERVATION_SCHEMA.to_string(),
            node_id: release.node_id.clone(),
            observed_at_unix_ms: release.observed_at_unix_ms,
            observed_generation: None,
            enrollment: enrollment.state,
            connection: NodeControlConnectionState::Connected,
            release,
            observed_capabilities: self.observed_capabilities.clone(),
            authorized_capabilities: enrollment.granted_capabilities,
            observed_services: self.observed_services.clone(),
            evidence_refs: enrollment
                .evidence_refs
                .iter()
                .cloned()
                .chain(["runtime-seed:local-read-back".to_string()])
                .collect(),
            unknowns,
        })
    }
}
