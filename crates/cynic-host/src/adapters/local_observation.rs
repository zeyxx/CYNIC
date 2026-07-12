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
        ServiceObservedState {
            units: policy
                .units
                .iter()
                .map(|unit| ServiceObservedUnit {
                    unit_name: unit.unit_name.clone(),
                    present: systemctl_known(&unit.unit_name),
                    enabled: systemctl_enabled(&unit.unit_name),
                    active: systemctl_active(&unit.unit_name),
                })
                .collect(),
        }
    }
}

fn package_present(package: &str) -> bool {
    let layout = current_runtime_layout();
    command_exit_success(&layout.package_query_bin(), &["-s", package]).unwrap_or(false)
        || command_exit_success(&layout.rpm_bin(), &["-q", package]).unwrap_or(false)
        || command_exit_success(&layout.which_bin(), &[package]).unwrap_or(false)
}

fn systemctl_known(unit: &str) -> bool {
    let service_manager = current_runtime_layout()
        .capabilities()
        .service_manager
        .command;
    command_exit_success(&service_manager, &["status", unit]).is_some()
}

fn systemctl_enabled(unit: &str) -> bool {
    let service_manager = current_runtime_layout()
        .capabilities()
        .service_manager
        .command;
    command_stdout(&service_manager, &["is-enabled", unit])
        .is_some_and(|stdout| stdout == "enabled" || stdout == "static")
}

fn systemctl_active(unit: &str) -> bool {
    let service_manager = current_runtime_layout()
        .capabilities()
        .service_manager
        .command;
    command_stdout(&service_manager, &["is-active", unit]).is_some_and(|stdout| stdout == "active")
}
