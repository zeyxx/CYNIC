use serde::Serialize;

use crate::surface_registry::{CanonicalCliSurface, canonical_cli_surfaces};

#[derive(Serialize)]
pub(crate) struct CommandsManifest {
    pub(crate) schema: &'static str,
    pub(crate) entrypoint: &'static str,
    pub(crate) domains: Vec<CommandDomain>,
    pub(crate) canonical_surfaces: Vec<CanonicalCliSurface>,
}

#[derive(Serialize)]
pub(crate) struct CommandDomain {
    pub(crate) name: &'static str,
    pub(crate) commands: Vec<&'static str>,
}

pub(crate) fn commands_manifest() -> CommandsManifest {
    CommandsManifest {
        schema: "cynic.cli.surface_manifest.v1",
        entrypoint: "cynic",
        canonical_surfaces: canonical_cli_surfaces(),
        domains: vec![
            CommandDomain {
                name: "fleet",
                commands: vec![
                    "cynic fleet report --json [--registry <path>] [--proxmox-json <path>] [--proxmox-list-text <path|->] [--watchdog-json <path>]",
                ],
            },
            CommandDomain {
                name: "runtime",
                commands: vec![
                    "cynic runtime paths --json [--registry <path>]",
                    "cynic runtime validate --json [--registry <path>] [--lab-registry <path>]",
                    "cynic runtime suite --json [--registry <path>] [--profile <name>]",
                    "cynic runtime plan --json [--registry <path>] [--profile <name>] [--proof <path>]",
                    "cynic runtime harness --json [--kind <closed_local_lab|conformance|staging|canary|production>]",
                    "cynic runtime conformance --json [--profile <name> | --node <name>] [--registry <path>]",
                    "cynic runtime metrics --json [--profile <name> | --node <name>] [--registry <path>] [--proof <runtime-proof-bundle.json>]",
                    "cynic runtime repo-access --json [--profile <path>]",
                ],
            },
            CommandDomain {
                name: "dev",
                commands: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cynic dev kpi-score --json",
                    "cynic dev framework --json",
                    "cynic dev surface-audit --json",
                ],
            },
            CommandDomain {
                name: "bootstrap",
                commands: vec![
                    "cynic bootstrap chain [--profile <name> | --node <name>] [--registry <path>]",
                    "cynic bootstrap constellation --json [--registry <path>]",
                    "cynic bootstrap bridge <bundle|plan|install|rootless-smoke|status> [--output-dir <dir> | --bundle-dir <dir> | --sandbox-root <dir>] [--apply]",
                    "cynic bootstrap framework bundle --json --output-dir <dir>",
                    "cynic bootstrap host <observe|bundle|plan|evidence|proof|supervisor|principal|e2e|apply-report|apply> [--profile <name> | --node <name>] [--registry <path>] [--dry-run|--apply]",
                    "cynic bootstrap host e2e [--profile <name> | --node <name>] [--registry <path>] [--runtime-profile <name>] --dry-run",
                    "cynic bootstrap host supervisor [--profile <name> | --node <name>] [--registry <path>] [--provider <auto|systemd-system|systemd-user|openrc|process-port>]",
                    "cynic bootstrap host principal [--profile <name> | --node <name>] [--registry <path>] [--apply]",
                ],
            },
            CommandDomain {
                name: "lab",
                commands: vec![
                    "cynic lab plan --json [--registry <path>]",
                    "cynic lab provision --json [--registry <path>] --apply",
                    "cynic lab bundle --json [--registry <path>] --output-dir <dir>",
                    "cynic lab guest-bootstrap bundle --json --output-dir <dir> (alias of cynic bootstrap framework bundle)",
                    "cynic lab image plan --json [--registry <path>]",
                    "cynic lab image resolve --json [--registry <path>]",
                    "cynic lab image fetch --json [--registry <path>] --apply",
                ],
            },
            CommandDomain {
                name: "transport",
                commands: vec!["cynic transport matrix"],
            },
            CommandDomain {
                name: "network",
                commands: vec![
                    "cynic network chain [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network runtime [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network proof [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network evidence [--registry <path>]",
                    "cynic network dns <status|bundle|plan> [--registry <path>] [--resolve-dns] [--output-dir <dir>]",
                    "cynic network mesh plan [--registry <path>] [--provider <wireguard|headscale|tailscale|static>]",
                    "cynic network mesh bundle [--registry <path>] [--provider <wireguard|headscale|tailscale|static>] --output-dir <dir>",
                    "cynic network mesh apply-plan --bundle-dir <dir>",
                    "cynic network mesh apply-profile --bundle-dir <dir> --profile <name> --apply [--closed-environment]",
                    "cynic network mesh status [--registry <path>] [--resolve-dns] [--include-records] [--observe-runtime]",
                    "cynic network provider-fallback <status|hosts> [--registry <path>] [--hosts-path <path>] [--check-connectivity] [--output-dir <dir>]",
                    "cynic network actuator <status|bundle|plan|apply> [--registry <path>] [--output-dir <dir> | --bundle-dir <dir>] [--profile <name>] [--apply]",
                ],
            },
            CommandDomain {
                name: "audit",
                commands: vec!["cynic audit node-host --json [--max-commits <n>]"],
            },
        ],
    }
}
