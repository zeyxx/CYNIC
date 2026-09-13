use std::net::TcpListener;
use std::os::unix::fs::MetadataExt;

use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

use super::{apply_runtime_binding, observe_runtime_binding, plan_runtime_binding};
use crate::runtime_binding::types::RuntimeBindingSpec;

fn spec(root: &std::path::Path, port: u16) -> RuntimeBindingSpec {
    let metadata = std::fs::metadata(root).expect("metadata");
    RuntimeBindingSpec {
        authority_role: "kernel_primary".to_string(),
        service_name: "cynic-kernel".to_string(),
        unit_name: "cynic-kernel.service".to_string(),
        scheme: "http".to_string(),
        endpoint_host: "127.0.0.1".to_string(),
        port,
        network_plane: "test".to_string(),
        environment_path: root.join("env").display().to_string(),
        service_unit_path: root.join("cynic-kernel.service").display().to_string(),
        service_manager_argv: "/test/systemctl".to_string(),
        owner_id: metadata.uid(),
        group_id: metadata.gid(),
    }
}

#[test]
fn missing_unit_blocks_before_any_environment_mutation() {
    let directory = tempfile::tempdir().expect("tempdir");
    let spec = spec(directory.path(), 3030);
    let observation = observe_runtime_binding(&spec);
    let plan = plan_runtime_binding(&spec, &observation);

    assert_eq!(plan.status, "blocked");
    assert!(
        plan.blockers
            .contains(&"runtime_binding_service_unit_missing".to_string())
    );
}

#[test]
fn explicit_apply_preserves_other_environment_and_requires_socket_read_back() {
    let directory = tempfile::tempdir().expect("tempdir");
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("address").port();
    let spec = spec(directory.path(), port);
    std::fs::write(
        &spec.service_unit_path,
        format!("[Service]\nEnvironmentFile=-{}\n", spec.environment_path),
    )
    .expect("unit");
    std::fs::write(
        &spec.environment_path,
        "CYNIC_STATE_ROOT=/var/lib/cynic\nCYNIC_REST_ADDR=127.0.0.1:1\n",
    )
    .expect("env");
    let runner = MapArgvRunner {
        responses: vec![(
            vec![
                "/test/systemctl".to_string(),
                "restart".to_string(),
                "cynic-kernel.service".to_string(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            },
        )],
    };

    let report = apply_runtime_binding(&spec, true, &runner);

    assert_eq!(report.status, "converged");
    assert_eq!(report.after.endpoint_reachable, Some(true));
    let environment = std::fs::read_to_string(&spec.environment_path).expect("environment");
    assert!(environment.contains("CYNIC_STATE_ROOT=/var/lib/cynic"));
    assert!(environment.contains(&format!("CYNIC_REST_ADDR=127.0.0.1:{port}")));
}
