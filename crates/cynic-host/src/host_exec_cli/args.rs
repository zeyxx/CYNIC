use std::collections::BTreeSet;

use crate::runtime_layout::current_runtime_layout;

pub(crate) fn usage() -> String {
<<<<<<< HEAD
    concat!(
        "usage: cynic-host-exec <",
        "bridge-bundle --output-dir <dir> | bridge-plan [--bundle-dir <dir>] | ",
        "bridge-install [--bundle-dir <dir>] --apply | bridge-status | ",
        "host-exec-validate-request [--policy <path>] [--declared-services svc1,svc2] < request.json | ",
        "runtime-seed-plan --request <request.json> --observation <observation.json> | ",
        "runtime-seed-reconcile --request <request.json> [--apply] | ",
        "storage-identity-enroll --endpoint <url> --namespace <ns> --database <db> ",
        "--admin-user <user> [--username <runtime-user>] [--token-duration <duration>] ",
        "[--session-duration <duration>] [--apply] < admin-secret | ",
        "storage-schema-migrate --endpoint <url> --namespace <ns> --database <db> ",
        "--admin-user <user> --schema-file <path> --expected-sha256 <sha256:hex> ",
        "[--timeout-seconds <seconds>] [--apply] < admin-secret | ",
        "transport-matrix | ssh-inspect --host <[principal@]hostname|ip> | ",
        "ssh-trust-admit --host <[principal@]hostname|ip> --fingerprint <sha256> ",
        "--attestation-source <console|idrac|...> --yes | ",
        "bootstrap-status|bootstrap-plan|bootstrap-chain|bootstrap-evidence ",
        "[--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | ",
        "principal-prepare [--profile <network-node|proxmox-hypervisor> | --node <name> ",
        "[--registry <path>]] [--apply] | hardware-baseline-enroll --apply | ",
        "service-reconcile --apply | network-baseline <observe|bundle|plan|apply-report|apply> ",
        "[--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] ",
        "[--dry-run|--apply] | network-actuate ",
        "--profile <provider-fallback-route|synapse-route|wireguard-mesh|direct-management-link> ",
        "[--target <name>] [--registry <path>] [--apply]>"
    )
    .to_string()
=======
    "usage: cynic-host-exec <bridge-bundle --output-dir <dir> | bridge-plan [--bundle-dir <dir>] | bridge-install [--bundle-dir <dir>] --apply | bridge-status | host-exec-validate-request [--policy <path>] [--declared-services svc1,svc2] < request.json | transport-matrix | ssh-inspect --host <hostname|ip> | bootstrap-status [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-plan [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-chain [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | bootstrap-evidence [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] | principal-prepare [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--apply] | hardware-baseline-enroll --apply | service-reconcile --apply | network-baseline <observe|bundle|plan|apply-report|apply> [--profile <network-node|proxmox-hypervisor> | --node <name> [--registry <path>]] [--dry-run|--apply] | network-actuate --profile <provider-fallback-route|synapse-route|wireguard-mesh|direct-management-link> [--target <name>] [--registry <path>] [--apply]>".to_string()
>>>>>>> github/main
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
        assert_eq!(
            host_exec_bundle_dir(&[]),
            "/var/lib/cynic/staging/host-exec"
        );
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
