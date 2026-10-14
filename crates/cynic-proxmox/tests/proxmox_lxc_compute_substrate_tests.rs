//! Tests for ProxmoxLxcSubstrateAdapter realizing ComputeSubstratePort.

use cynic_lifecycle::{ArgvOutput, ArgvRunner};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{ComputeRequirements, ComputeSubstratePort, SubstrateKind};
use cynic_proxmox::adapters::proxmox_lxc_compute_substrate::ProxmoxLxcSubstrateAdapter;
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

#[test]
fn proxmox_lxc_substrate_allocates_and_executes_container_command() {
    futures::executor::block_on(async {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let runner = Arc::new(MockRunner {
            recorded_commands: Arc::clone(&recorded),
            stdout: "cloned ok\n".to_string(),
        });

        let adapter =
            ProxmoxLxcSubstrateAdapter::new(runner, "root@192.168.0.37", 100, 200).unwrap();

        let reqs = ComputeRequirements {
            vcpus: 4,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            disk_bytes: 20 * 1024 * 1024 * 1024,
            requires_gpu: false,
            preferred_strategy: SandboxStrategy::LxcUnprivileged,
            allowed_strategies: vec![SandboxStrategy::LxcUnprivileged],
        };

        let allocation = adapter
            .allocate_runner("heavy-eval-1", &reqs)
            .await
            .unwrap();

        assert_eq!(allocation.substrate, SubstrateKind::ProxmoxLxc);
        assert_eq!(allocation.strategy, SandboxStrategy::LxcUnprivileged);
        assert_eq!(allocation.allocation_id, "lxc-200");

        let cmd = vec!["cynic".to_string(), "status".to_string()];
        let res = adapter
            .execute_command(&allocation.allocation_id, &cmd, 10000)
            .await
            .unwrap();

        assert_eq!(res.exit_code, 0);

        // Verify pct calls: clone, set, start, exec
        let calls = recorded.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert!(calls[0].contains(&"clone".to_string()));
        assert!(calls[1].contains(&"set".to_string()));
        assert!(calls[2].contains(&"start".to_string()));
        assert!(calls[3].contains(&"exec".to_string()));
    });
}
