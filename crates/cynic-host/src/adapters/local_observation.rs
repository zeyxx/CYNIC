use std::path::Path;

use crate::domain::packages::PackageObservedState;
use crate::domain::principal::{PrincipalObservedState, PrincipalPolicy};
use crate::domain::privilege::{PrivilegeObservedState, PrivilegePolicy};
use crate::domain::services::{ServiceObservedState, ServiceObservedUnit, ServicePolicy};
use crate::ports::{
    HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
    HostServiceObservationPort,
};
use crate::process_observation::{command_exit_success, command_stdout};
use crate::runtime_layout::current_runtime_layout;
use crate::service_supervisor_observation::observe_service_supervisor;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHostObservationAdapter;

impl HostPrincipalObservationPort for LocalHostObservationAdapter {
    fn observe_principal(&self, policy: &PrincipalPolicy) -> PrincipalObservedState {
        let capabilities = current_runtime_layout().capabilities();
        let resolver_command = capabilities.name_service.resolver_command;
        let passwd = command_stdout(&resolver_command, &["passwd", policy.runtime_user.as_str()]);
        let group = command_stdout(&resolver_command, &["group", policy.runtime_group.as_str()]);
        let observed_home_dir = passwd
            .as_deref()
            .and_then(|line| line.split(':').nth(5))
            .map(str::to_string);

        PrincipalObservedState {
            user_present: passwd.is_some(),
            group_present: group.is_some(),
            observed_home_dir,
            present_state_dirs: policy
                .state_dirs
                .iter()
                .filter(|path| Path::new(path.as_str()).is_dir())
                .cloned()
                .collect(),
        }
    }
}

impl HostPrivilegeObservationPort for LocalHostObservationAdapter {
    fn observe_privilege(&self, policy: &PrivilegePolicy) -> PrivilegeObservedState {
        let helper_dir_present = Path::new(&policy.helper_dir).is_dir();
        let sudoers_projection_present = Path::new(&policy.sudoers_dir).is_dir();
        let sudo = current_runtime_layout().sudo_bin();
        let denial_proof_ready =
            command_exit_success(&sudo, &["-n", "/bin/sh", "-c", "true"]).is_some_and(|ok| !ok);

        PrivilegeObservedState {
            helper_dir_present,
            sudoers_projection_present,
            denial_proof_ready,
        }
    }
}

impl HostPackageObservationPort for LocalHostObservationAdapter {
    fn observe_packages(
        &self,
        policy: &crate::domain::packages::PackagePolicy,
    ) -> PackageObservedState {
        PackageObservedState {
            installed_packages: policy
                .bootstrap_packages
                .iter()
                .filter(|pkg| package_present(pkg))
                .cloned()
                .collect(),
        }
    }
}

impl HostServiceObservationPort for LocalHostObservationAdapter {
    fn observe_services(&self, policy: &ServicePolicy) -> ServiceObservedState {
        let supervisor_observation =
            observe_service_supervisor("local-host-substrate".to_string(), policy);
        ServiceObservedState {
            units: supervisor_observation
                .services
                .iter()
                .map(service_observation_to_substrate_unit)
                .collect(),
        }
    }
}

fn package_present(package: &str) -> bool {
    let layout = current_runtime_layout();
    let capabilities = layout.capabilities();
    let argv = capabilities
        .package_manager
        .kind
        .query_argv(package, &layout);
    command_exit_success(&argv[0], &string_args(&argv[1..])).unwrap_or(false)
        || command_exit_success(&layout.which_bin(), &[package]).unwrap_or(false)
}

fn string_args(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

fn service_observation_to_substrate_unit(
    unit: &cynic_protocol::ServiceSupervisorUnitObservation,
) -> ServiceObservedUnit {
    ServiceObservedUnit {
        unit_name: unit.unit_name.clone(),
        present: unit_present(unit),
        enabled: unit_enabled(unit),
        active: unit.status == "active",
    }
}

fn unit_present(unit: &cynic_protocol::ServiceSupervisorUnitObservation) -> bool {
    match unit.selected_provider.as_str() {
        "systemd" => unit.load_state == "loaded",
        "openrc" | "process" | "process_port" => unit.probe_status == "ok",
        _ => unit.status == "active" || unit.status == "degraded",
    }
}

fn unit_enabled(unit: &cynic_protocol::ServiceSupervisorUnitObservation) -> bool {
    match unit.selected_provider.as_str() {
        "systemd" => matches!(
            unit.unit_file_state.as_str(),
            "enabled" | "enabled-runtime" | "static" | "generated"
        ),
        "openrc" | "process" | "process_port" => unit.status == "active",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::service_observation_to_substrate_unit;
    use cynic_protocol::ServiceSupervisorUnitObservation;

    fn unit(
        selected_provider: &str,
        load_state: &str,
        unit_file_state: &str,
        status: &'static str,
        probe_status: &'static str,
    ) -> ServiceSupervisorUnitObservation {
        ServiceSupervisorUnitObservation {
            service_id: "svc".to_string(),
            unit_name: "svc.service".to_string(),
            health_port: None,
            health_kind: None,
            desired_provider: None,
            selected_provider: selected_provider.to_string(),
            selected_scope: "system".to_string(),
            target_source: "test".to_string(),
            desired_state: "enabled_and_active".to_string(),
            restart_policy: "on-failure".to_string(),
            probe_status,
            probe_detail: "test".to_string(),
            provider_attempts: Vec::new(),
            load_state: load_state.to_string(),
            unit_file_state: unit_file_state.to_string(),
            active_state: status.to_string(),
            sub_state: "unknown".to_string(),
            restart_count: None,
            active_enter_timestamp_monotonic: None,
            status,
        }
    }

    #[test]
    fn systemd_supervisor_observation_maps_to_substrate_unit() {
        let observed = service_observation_to_substrate_unit(&unit(
            "systemd", "loaded", "enabled", "active", "ok",
        ));

        assert!(observed.present);
        assert!(observed.enabled);
        assert!(observed.active);
    }

    #[test]
    fn openrc_supervisor_observation_does_not_require_systemctl() {
        let observed = service_observation_to_substrate_unit(&unit(
            "openrc", "unknown", "unknown", "active", "ok",
        ));

        assert!(observed.present);
        assert!(observed.enabled);
        assert!(observed.active);
    }

    #[test]
    fn unknown_supervisor_observation_remains_blocked() {
        let observed = service_observation_to_substrate_unit(&unit(
            "openrc", "unknown", "unknown", "unknown", "blocked",
        ));

        assert!(!observed.present);
        assert!(!observed.enabled);
        assert!(!observed.active);
    }
}
