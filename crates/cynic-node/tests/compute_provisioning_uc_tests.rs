//! Verification of compute provisioning use case in cynic-node.

use cynic_node::usecases::compute_provisioning_uc::{
    COMPUTE_PROVISIONING_REPORT_SCHEMA, ComputeProvisioningIntent, execute_compute_provisioning,
};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    CommandExecutionResult, ComputeRequirements, ComputeSubstratePort, RunnerAllocation,
    RunnerStatus, SubstrateKind, SubstrateObservation,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct MockSubstratePort {
    allocated: Arc<AtomicBool>,
    executed: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
}

#[derive(Debug, thiserror::Error)]
#[allow(
    dead_code,
    reason = "mock error variants exist for fault-injection scenarios"
)]
enum MockError {
    #[error("mock error: {0}")]
    General(String),
}

impl ComputeSubstratePort for MockSubstratePort {
    type Error = MockError;

    async fn allocate_runner(
        &self,
        workload_id: &str,
        _requirements: &ComputeRequirements,
    ) -> Result<RunnerAllocation, Self::Error> {
        self.allocated.store(true, Ordering::SeqCst);
        Ok(RunnerAllocation {
            allocation_id: format!("alloc-{workload_id}"),
            workload_id: workload_id.to_string(),
            substrate: SubstrateKind::Mock,
            strategy: SandboxStrategy::BwrapEphemeral,
            status: RunnerStatus::Allocated,
            network_endpoint: None,
        })
    }

    async fn execute_command(
        &self,
        _allocation_id: &str,
        _argv: &[String],
        _timeout_ms: u64,
    ) -> Result<CommandExecutionResult, Self::Error> {
        self.executed.store(true, Ordering::SeqCst);
        Ok(CommandExecutionResult {
            exit_code: 0,
            stdout: "execution ok\n".to_string(),
            stderr: String::new(),
            duration_ms: 12,
        })
    }

    async fn release_runner(&self, _allocation_id: &str) -> Result<(), Self::Error> {
        self.released.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn compute_provisioning_happy_path() {
    let allocated = Arc::new(AtomicBool::new(false));
    let executed = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));

    let port = MockSubstratePort {
        allocated: Arc::clone(&allocated),
        executed: Arc::clone(&executed),
        released: Arc::clone(&released),
    };

    let intent = ComputeProvisioningIntent {
        workload_id: "eval-42".to_string(),
        requirements: ComputeRequirements::default(),
        command: vec!["cargo".to_string(), "check".to_string()],
        timeout_ms: 30000,
    };

    let fleet = vec![SubstrateObservation {
        name: "local-node".to_string(),
        kind: SubstrateKind::LocalBwrap,
        available_vcpus: 8,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_disk_bytes: 100 * 1024 * 1024 * 1024,
        has_gpu: false,
        supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
    }];

    let report = execute_compute_provisioning(&intent, &fleet, &port)
        .await
        .expect("use case should succeed");

    assert_eq!(report.schema, COMPUTE_PROVISIONING_REPORT_SCHEMA);
    assert_eq!(report.workload_id, "eval-42");
    assert!(report.admitted);
    assert_eq!(report.status, "succeeded");
    assert_eq!(report.substrate_selected.as_deref(), Some("local-node"));
    assert_eq!(
        report.strategy_selected,
        Some(SandboxStrategy::BwrapEphemeral)
    );
    assert!(report.execution_result.is_some());

    assert!(allocated.load(Ordering::SeqCst));
    assert!(executed.load(Ordering::SeqCst));
    assert!(released.load(Ordering::SeqCst));
}

#[tokio::test]
async fn compute_provisioning_rejects_when_fleet_insufficient() {
    let allocated = Arc::new(AtomicBool::new(false));
    let executed = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));

    let port = MockSubstratePort {
        allocated: Arc::clone(&allocated),
        executed: Arc::clone(&executed),
        released: Arc::clone(&released),
    };

    let intent = ComputeProvisioningIntent {
        workload_id: "heavy-gpu-job".to_string(),
        requirements: ComputeRequirements {
            requires_gpu: true,
            ..Default::default()
        },
        command: vec!["llama-server".to_string()],
        timeout_ms: 10000,
    };

    let fleet = vec![SubstrateObservation {
        name: "cpu-only-node".to_string(),
        kind: SubstrateKind::LocalBwrap,
        available_vcpus: 8,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_disk_bytes: 100 * 1024 * 1024 * 1024,
        has_gpu: false, // No GPU!
        supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
    }];

    let report = execute_compute_provisioning(&intent, &fleet, &port)
        .await
        .expect("use case handles rejection gracefully");

    assert!(!report.admitted);
    assert_eq!(report.status, "rejected");
    assert!(report.execution_result.is_none());
    assert!(!allocated.load(Ordering::SeqCst));
    assert!(!executed.load(Ordering::SeqCst));
    assert!(!released.load(Ordering::SeqCst));
}
