use super::{HostArgvCommand, execute_argv_commands, execute_host_commands};

#[test]
fn every_legacy_mutation_surface_is_fail_closed() {
    let command = HostArgvCommand {
        role: "must_not_run".to_string(),
        argv: vec!["/bin/true".to_string()],
    };
    let argv_report = execute_argv_commands(&[command], true, true);
    let host_report = execute_host_commands(&[], true, true);

    assert_eq!(argv_report.execution_count, 0);
    assert_eq!(host_report.execution_count, 0);
    assert!(
        argv_report
            .blockers
            .contains(&"policy_admission_required".to_string())
    );
    assert!(
        host_report
            .blockers
            .contains(&"policy_admission_required".to_string())
    );
}
