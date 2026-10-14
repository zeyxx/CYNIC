use cynic_node::usecases::remote_runner_enrollment_uc::{
    REMOTE_RUNNER_ENROLLMENT_REPORT_SCHEMA, execute_remote_runner_enrollment,
};
use cynic_protocol::compute::RemoteRunnerSpec;
use cynic_types::fleet_mesh::{
    FleetMeshPort, MeshError, MeshNode, MeshRoute, PreAuthKey, PreAuthKeySpec,
};

struct MockMeshPort {
    fail: bool,
}

impl FleetMeshPort for MockMeshPort {
    fn list_nodes(&self) -> Result<Vec<MeshNode>, MeshError> {
        Ok(vec![])
    }

    fn enroll_node(&self, _node: &MeshNode) -> Result<(), MeshError> {
        Ok(())
    }

    fn remove_node(&self, _node_id: &str) -> Result<(), MeshError> {
        Ok(())
    }

    fn list_routes(&self) -> Result<Vec<MeshRoute>, MeshError> {
        Ok(vec![])
    }

    fn enable_route(&self, _route_id: &str) -> Result<(), MeshError> {
        Ok(())
    }

    fn disable_route(&self, _route_id: &str) -> Result<(), MeshError> {
        Ok(())
    }

    fn create_preauth_key(&self, spec: &PreAuthKeySpec) -> Result<PreAuthKey, MeshError> {
        if self.fail {
            return Err(MeshError::ApiError("simulated_api_down".to_string()));
        }
        Ok(PreAuthKey {
            id: "fake-key-id".to_string(),
            key: "hskey-fake-mesh-token-123456789".to_string(),
            user: spec.user.clone(),
            expiration: Some("2026-10-12T18:00:00Z".to_string()),
            reusable: spec.reusable,
            ephemeral: spec.ephemeral,
            used: false,
        })
    }

    fn expire_preauth_key(&self, _user: &str, _key: &str) -> Result<(), MeshError> {
        Ok(())
    }
}

#[test]
fn test_remote_runner_enrollment_happy_path() {
    let mock = MockMeshPort { fail: false };
    let spec = RemoteRunnerSpec {
        node_name: "runpod-gpu-node-01".to_string(),
        vcpus: 8,
        memory_bytes: 32 * 1024 * 1024 * 1024,
        disk_bytes: 100 * 1024 * 1024 * 1024,
        has_gpu: true,
        gpu_model: Some("NVIDIA-RTX-4090".to_string()),
        wireguard_pubkey: "abcd1234abcd1234abcd1234abcd1234abcd1234==".to_string(),
    };

    let report = execute_remote_runner_enrollment(&spec, &mock).expect("enrollment should succeed");

    assert_eq!(report.schema, REMOTE_RUNNER_ENROLLMENT_REPORT_SCHEMA);
    assert_eq!(report.node_name, "runpod-gpu-node-01");
    assert_eq!(report.preauth_key, "hskey-fake-mesh-token-123456789");
    assert_eq!(report.acl_tags, vec!["tag:runner".to_string()]);
    assert_eq!(report.status, "enrolled_pending_connect");
}

#[test]
fn test_remote_runner_enrollment_validation_failures() {
    let mock = MockMeshPort { fail: false };

    // Empty node name
    let spec_bad_name = RemoteRunnerSpec {
        node_name: "   ".to_string(),
        vcpus: 8,
        memory_bytes: 1024 * 1024 * 1024,
        disk_bytes: 10 * 1024 * 1024 * 1024,
        has_gpu: false,
        gpu_model: None,
        wireguard_pubkey: "abcd1234abcd1234abcd1234abcd1234abcd1234==".to_string(),
    };
    let err = execute_remote_runner_enrollment(&spec_bad_name, &mock).unwrap_err();
    assert!(err.contains("invalid_node_name"));

    // 0 vCPUs
    let spec_zero_vcpu = RemoteRunnerSpec {
        node_name: "valid-name".to_string(),
        vcpus: 0,
        memory_bytes: 1024 * 1024 * 1024,
        disk_bytes: 10 * 1024 * 1024 * 1024,
        has_gpu: false,
        gpu_model: None,
        wireguard_pubkey: "abcd1234abcd1234abcd1234abcd1234abcd1234==".to_string(),
    };
    let err = execute_remote_runner_enrollment(&spec_zero_vcpu, &mock).unwrap_err();
    assert!(err.contains("invalid_capacity"));

    // Key too short
    let spec_short_key = RemoteRunnerSpec {
        node_name: "valid-name".to_string(),
        vcpus: 4,
        memory_bytes: 1024 * 1024 * 1024,
        disk_bytes: 10 * 1024 * 1024 * 1024,
        has_gpu: false,
        gpu_model: None,
        wireguard_pubkey: "short_key".to_string(),
    };
    let err = execute_remote_runner_enrollment(&spec_short_key, &mock).unwrap_err();
    assert!(err.contains("invalid_wireguard_pubkey"));
}
