//! Tests for LocalBwrapSubstrateAdapter realizing ComputeSubstratePort.

use cynic_lifecycle::{ArgvOutput, ArgvRunner};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{ComputeRequirements, ComputeSubstratePort, SubstrateKind};
use cynic_sandbox::adapters::bwrap_substrate::LocalBwrapSubstrateAdapter;
use std::sync::{Arc, Mutex};

struct MockRunner {
    recorded_commands: Arc<Mutex<Vec<Vec<String>>>>,
    stdout: String,
}

impl ArgvRunner for MockRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        self.recorded_commands
            .lock()
            .unwrap()
            .push(argv.iter().map(|s| s.to_string()).collect());
        ArgvOutput {
            exit_code: Some(0),
            stdout: self.stdout.clone(),
            stderr: String::new(),
            spawn_error: None,
        }
    }
}

#[tokio::test]
async fn bwrap_substrate_allocates_and_executes_isolated_command() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let runner = Arc::new(MockRunner {
        recorded_commands: Arc::clone(&recorded),
        stdout: "hello sovereign world\n".to_string(),
    });

    let adapter = LocalBwrapSubstrateAdapter::new(runner);

    let reqs = ComputeRequirements::default();
    let allocation = adapter.allocate_runner("task-42", &reqs).await.unwrap();

    assert_eq!(allocation.substrate, SubstrateKind::LocalBwrap);
    assert_eq!(allocation.strategy, SandboxStrategy::BwrapEphemeral);
    assert_eq!(allocation.workload_id, "task-42");

    let cmd = vec!["echo".to_string(), "hello sovereign world".to_string()];
    let res = adapter
        .execute_command(&allocation.allocation_id, &cmd, 5000)
        .await
        .unwrap();

    assert_eq!(res.exit_code, 0);
    assert_eq!(res.stdout, "hello sovereign world\n");

    let calls = recorded.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let invoked = &calls[0];
    assert_eq!(invoked[0], "bwrap");
    assert!(invoked.contains(&"--unshare-net".to_string()));
    assert!(invoked.contains(&"echo".to_string()));
}
