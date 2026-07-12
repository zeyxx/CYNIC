use std::collections::BTreeSet;

use crate::runtime_layout::current_runtime_layout;

pub(crate) fn usage() -> String {
    "usage: cynic-host-exec <bridge-bundle --output-dir <dir> | bridge-plan [--bundle-dir <dir>] | bridge-install [--bundle-dir <dir>] --apply | bridge-status | host-exec-validate-request [--policy <path>] [--declared-services svc1,svc2] < request.json | transport-matrix | bootstrap-status [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-plan [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-chain [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-evidence [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | principal-prepare [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--apply] | network-baseline <observe|bundle|plan|apply-report|apply> [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--dry-run|--apply]>".to_string()
}

pub(crate) fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

pub(crate) fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

pub(crate) fn host_exec_bundle_dir(args: &[String]) -> String {
    flag_value(args, "--bundle-dir").unwrap_or_else(default_host_exec_bundle_dir)
}

pub(crate) fn host_exec_policy_path(args: &[String]) -> String {
    flag_value(args, "--policy").unwrap_or_else(default_host_exec_policy_path)
}

pub(crate) fn declared_services(args: &[String]) -> BTreeSet<String> {
    flag_value(args, "--declared-services")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn default_host_exec_bundle_dir() -> String {
    current_runtime_layout().host_exec_bundle_dir
}

fn default_host_exec_policy_path() -> String {
    current_runtime_layout().host_exec_policy_path()
}

#[cfg(test)]
mod tests {
    use super::{declared_services, flag_value, host_exec_bundle_dir};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn flag_value_reads_following_token() {
        let args = args(&[
            "--profile",
            "network-node",
            "--registry",
            "infra/registry.json",
        ]);

        assert_eq!(
            flag_value(&args, "--registry"),
            Some("infra/registry.json".to_string())
        );
    }

    #[test]
    fn host_exec_bundle_dir_has_single_default() {
        assert_eq!(host_exec_bundle_dir(&[]), "/tmp/cynic-host-exec");
    }

    #[test]
    fn declared_services_are_trimmed_and_empty_values_removed() {
        let args = args(&["--declared-services", "cynic-kernel, ,surrealdb.service"]);

        let services = declared_services(&args);

        assert!(services.contains("cynic-kernel"));
        assert!(services.contains("surrealdb.service"));
        assert_eq!(services.len(), 2);
    }
}
