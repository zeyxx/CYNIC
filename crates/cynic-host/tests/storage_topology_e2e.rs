use cynic_host::domain::storage::{
    InMemoryStorageTopologyAdapter, LocalStorageTopologyAdapter, RemoteStorageTopologyAdapter,
    StorageTopologyAction, StorageTopologyObservationPort, plan_storage_topology_apply,
    reconcile_storage_topology, storage_topology_report,
};
use cynic_host::update_lifecycle::{ArgvOutput, MapArgvRunner};
use cynic_protocol::storage_client_policy::{
    ClientPolicy, STORAGE_CLIENT_POLICY_SCHEMA, StorageClientPolicy,
};
use cynic_protocol::storage_topology::{
    STORAGE_TOPOLOGY_REPORT_SCHEMA, STORAGE_TOPOLOGY_SCHEMA, StorageBudget, StorageEngine,
    StorageTopologyDesired, StorageTopologyObserved, TargetTopology,
};
use std::sync::Arc;

fn desired_one() -> StorageTopologyDesired {
    StorageTopologyDesired {
        schema: STORAGE_TOPOLOGY_SCHEMA.to_string(),
        engine: StorageEngine {
            backend: "standalone_process".to_string(),
            cluster_mode: "none".to_string(),
        },
        target_topology: TargetTopology {
            replicas: 1,
            residual_risk: "single_endpoint_no_replication".to_string(),
        },
        budget: StorageBudget {
            wall_time_ms: 5000,
            cpu_ms: Some(5000),
        },
    }
}

fn observed_one(endpoint: &str) -> StorageTopologyObserved {
    StorageTopologyObserved {
        schema: "cynic.storage_topology_observed.v1".to_string(),
        observed_at_ms: 1000,
        endpoints: vec![endpoint.to_string()],
        leader: None,
        poll_cost_ms: Some(10),
        residual_unknowns: vec![],
    }
}

#[test]
fn in_memory_adapter_observe() {
    let observed = observed_one("http://127.0.0.1:8000");
    let adapter = InMemoryStorageTopologyAdapter {
        observed: observed.clone(),
    };
    let got = adapter.observe_topology();
    assert_eq!(got.endpoints, observed.endpoints);
    assert_eq!(got.poll_cost_ms, Some(10));
}

#[test]
fn local_adapter_observe_missing_file() {
    let tempdir = tempfile::tempdir().expect("create isolated topology fixture root");
    let absent = tempdir.path().join("storage_topology.json");
    let adapter = LocalStorageTopologyAdapter {
        topology_path: absent.to_string_lossy().to_string(),
    };
    let observed = adapter.observe_topology();
    assert!(
        observed
            .residual_unknowns
            .contains(&"topology_file_missing".to_string())
    );
}

#[test]
fn remote_adapter_reports_ssh_failure_as_residual_unknown() {
    let runner = Arc::new(MapArgvRunner {
        responses: vec![(
            vec!["ssh".to_string()],
            ArgvOutput {
                exit_code: Some(255),
                stdout: String::new(),
                stderr: "connection refused".to_string(),
                spawn_error: None,
            },
        )],
    });
    let adapter = RemoteStorageTopologyAdapter {
        ssh_target: "cynic@10.0.0.2".to_string(),
        topology_path: "/etc/cynic/storage_topology.json".to_string(),
        runner,
    };
    let observed = adapter.observe_topology();
    assert!(observed.endpoints.is_empty());
    assert!(
        observed
            .residual_unknowns
            .contains(&"ssh_topology_failed".to_string())
    );
}

#[test]
fn report_residual_single_endpoint() {
    let desired = StorageTopologyDesired {
        schema: STORAGE_TOPOLOGY_SCHEMA.to_string(),
        engine: StorageEngine {
            backend: "standalone_process".to_string(),
            cluster_mode: "none".to_string(),
        },
        target_topology: TargetTopology {
            replicas: 3,
            residual_risk: "single_endpoint_no_replication".to_string(),
        },
        budget: StorageBudget {
            wall_time_ms: 5000,
            cpu_ms: Some(5000),
        },
    };
    let observed = StorageTopologyObserved {
        schema: "cynic.storage_topology_observed.v1".to_string(),
        observed_at_ms: 1000,
        endpoints: vec!["http://127.0.0.1:8000".to_string()],
        leader: None,
        poll_cost_ms: Some(10),
        residual_unknowns: vec![],
    };
    let client_policy = StorageClientPolicy {
        schema: STORAGE_CLIENT_POLICY_SCHEMA.to_string(),
        client_policy: ClientPolicy {
            endpoints: vec!["http://127.0.0.1:8000".to_string()],
            failover_mode: "ordered_fallback".to_string(),
            write_quorum: 1,
            timeout_ms: 5000,
        },
    };
    let report = storage_topology_report(&desired, &observed, &client_policy);
    assert_eq!(report.schema, STORAGE_TOPOLOGY_REPORT_SCHEMA);
    assert!(
        report
            .residual_unknowns
            .contains(&"single_endpoint_no_replication".to_string())
    );
}

#[test]
fn reconcile_add_replica_and_backpressure() {
    let desired = StorageTopologyDesired {
        schema: STORAGE_TOPOLOGY_SCHEMA.to_string(),
        engine: StorageEngine {
            backend: "standalone_process".to_string(),
            cluster_mode: "none".to_string(),
        },
        target_topology: TargetTopology {
            replicas: 2,
            residual_risk: "single_endpoint_no_replication".to_string(),
        },
        budget: StorageBudget {
            wall_time_ms: 5000,
            cpu_ms: Some(5000),
        },
    };
    let observed = StorageTopologyObserved {
        schema: "cynic.storage_topology_observed.v1".to_string(),
        observed_at_ms: 1000,
        endpoints: vec!["http://127.0.0.1:8000".to_string()],
        leader: None,
        poll_cost_ms: Some(6000),
        residual_unknowns: vec![],
    };
    let client_policy = StorageClientPolicy::default();
    let plan = reconcile_storage_topology(&desired, &observed, &client_policy);
    assert!(plan.actions.contains(&StorageTopologyAction::AddReplica {
        desired: 2,
        observed: 1,
    }));
    assert!(plan.actions.contains(&StorageTopologyAction::Backpressure {
        poll_cost_ms: 6000,
        budget_ms: 5000,
    }));
    assert!(
        plan.residual_unknowns
            .contains(&"poll_cost_exceeded_budget".to_string())
    );
    let apply = plan_storage_topology_apply(&desired, &observed, &plan, &client_policy);
    assert!(apply.mutates);
    assert_eq!(apply.rollback, "rollback to 1 replicas");
    assert_eq!(apply.proof_ref, "storage_topology:1");
}
