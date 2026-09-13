use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use cynic_lifecycle::ArgvRunner;

use super::decision::plan_runtime_binding;
use super::observe::{observe_runtime_binding, render_environment_projection};
use super::types::{
    RUNTIME_BINDING_REPORT_SCHEMA, RuntimeBindingApplyReport, RuntimeBindingCommandExecution,
    RuntimeBindingSpec,
};

pub fn apply_runtime_binding(
    spec: &RuntimeBindingSpec,
    explicit_apply: bool,
    runner: &dyn ArgvRunner,
) -> RuntimeBindingApplyReport {
    let before = observe_runtime_binding(spec);
    let plan = plan_runtime_binding(spec, &before);
    if plan.status != "planned" || !explicit_apply {
        let mut blockers = plan.blockers.clone();
        if plan.status == "planned" && !explicit_apply {
            blockers.push("explicit_apply_required".to_string());
        }
        return RuntimeBindingApplyReport {
            schema: RUNTIME_BINDING_REPORT_SCHEMA,
            status: if plan.status == "converged" {
                "converged"
            } else if plan.status == "blocked" {
                "blocked"
            } else {
                "planned"
            },
            mutates: false,
            requested_apply: explicit_apply,
            plan,
            before: before.clone(),
            executions: Vec::new(),
            after: before,
            proofs_observed: Vec::new(),
            blockers,
            rollback_performed: false,
        };
    }

    let path = Path::new(&spec.environment_path);
    let snapshot = match snapshot(path) {
        Ok(snapshot) => snapshot,
        Err(blocker) => {
            return RuntimeBindingApplyReport {
                schema: RUNTIME_BINDING_REPORT_SCHEMA,
                status: "blocked",
                mutates: false,
                requested_apply: true,
                plan,
                before: before.clone(),
                executions: Vec::new(),
                after: before,
                proofs_observed: Vec::new(),
                blockers: vec![blocker],
                rollback_performed: false,
            };
        }
    };
    let mut blockers = Vec::new();
    let mut executions = Vec::new();
    let existing = fs::read_to_string(path).ok();
    let projection = render_environment_projection(existing.as_deref(), &spec.rest_addr());
    if let Err(error) = atomic_write(
        path,
        projection.as_bytes(),
        0o640,
        spec.owner_id,
        spec.group_id,
    ) {
        blockers.push(error);
    } else {
        executions.push(restart(spec, runner));
        if executions.iter().any(|execution| !execution.success) {
            blockers.push("runtime_binding_service_restart_failed".to_string());
        }
    }
    let after_mutation = observe_runtime_binding(spec);
    if plan_runtime_binding(spec, &after_mutation).status != "converged" {
        blockers.push("runtime_binding_read_back_failed".to_string());
    }
    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        return RuntimeBindingApplyReport {
            schema: RUNTIME_BINDING_REPORT_SCHEMA,
            status: "converged",
            mutates: true,
            requested_apply: true,
            plan,
            before,
            executions,
            after: after_mutation,
            proofs_observed: vec![
                "kernel_environment_binding_read_back",
                "kernel_service_restart_exit_status",
                "kernel_endpoint_socket_read_back",
            ],
            blockers,
            rollback_performed: false,
        };
    }

    let rollback_performed = restore(path, snapshot)
        .map(|()| {
            let rollback_restart = restart(spec, runner);
            let success = rollback_restart.success;
            executions.push(rollback_restart);
            success
        })
        .unwrap_or(false);
    if !rollback_performed {
        blockers.push("runtime_binding_rollback_failed".to_string());
    }
    RuntimeBindingApplyReport {
        schema: RUNTIME_BINDING_REPORT_SCHEMA,
        status: "blocked",
        mutates: true,
        requested_apply: true,
        plan,
        before,
        executions,
        after: observe_runtime_binding(spec),
        proofs_observed: Vec::new(),
        blockers,
        rollback_performed,
    }
}

fn restart(spec: &RuntimeBindingSpec, runner: &dyn ArgvRunner) -> RuntimeBindingCommandExecution {
    let argv = vec![
        spec.service_manager_argv.clone(),
        "restart".to_string(),
        spec.unit_name.clone(),
    ];
    let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
    RuntimeBindingCommandExecution {
        argv,
        success: output.success(),
        exit_code: output.exit_code,
        stderr: output.stderr,
    }
}

#[derive(Debug)]
struct Snapshot {
    bytes: Option<Vec<u8>>,
    mode: Option<u32>,
    owner_id: Option<u32>,
    group_id: Option<u32>,
}

fn snapshot(path: &Path) -> Result<Snapshot, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let metadata = fs::metadata(path)
                .map_err(|error| format!("runtime_binding_snapshot_failed:{error}"))?;
            Ok(Snapshot {
                bytes: Some(bytes),
                mode: Some(metadata.permissions().mode() & 0o777),
                owner_id: Some(metadata.uid()),
                group_id: Some(metadata.gid()),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Snapshot {
            bytes: None,
            mode: None,
            owner_id: None,
            group_id: None,
        }),
        Err(error) => Err(format!("runtime_binding_snapshot_failed:{error}")),
    }
}

fn restore(path: &Path, snapshot: Snapshot) -> Result<(), String> {
    if let Some(bytes) = snapshot.bytes {
        atomic_write(
            path,
            &bytes,
            snapshot.mode.unwrap_or(0o640),
            snapshot.owner_id.unwrap_or(0),
            snapshot.group_id.unwrap_or(0),
        )
    } else if path.is_file() {
        fs::remove_file(path).map_err(|error| format!("runtime_binding_rollback_failed:{error}"))
    } else {
        Ok(())
    }
}

fn atomic_write(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    owner_id: u32,
    group_id: u32,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "runtime_binding_environment_parent_missing".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("runtime_binding_environment_parent_failed:{error}"))?;
    let temporary = parent.join(format!(".env.next-{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        std::os::unix::fs::chown(&temporary, Some(owner_id), Some(group_id))?;
        fs::rename(&temporary, path)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("runtime_binding_environment_write_failed:{error}"))
}
