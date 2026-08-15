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
                    "cynic runtime registry-observation --json --observation <path> [--registry <path>] [--apply]",
                    "cynic runtime validate --json [--registry <path>] [--lab-registry <path>]",
                    "cynic runtime suite --json [--registry <path>] [--profile <name>]",
                    "cynic runtime plan --json [--registry <path>] [--profile <name>] [--proof <path>]",
                    "cynic runtime harness --json [--kind <closed_local_lab|conformance|staging|canary|production>]",
                    "cynic runtime conformance --json [--profile <name> | --node <name>] [--registry <path>]",
                    "cynic runtime metrics --json [--profile <name> | --node <name>] [--registry <path>] [--proof <runtime-proof-bundle.json>]",
                    "cynic runtime proof-persist --json --proof <runtime-proof-persistence.json> [--endpoint <kernel-url>]",
                    "cynic runtime build --json --source-snapshot <source-snapshot.json> --artifact <path> [--apply] -- <argv...>",
                    "cynic runtime storage-capabilities --json [--url <surrealdb-url>]",
                    "cynic runtime execution-context --json [--host <name>]...",
                    "cynic runtime network-preflight --json [--host <name>]...",
                    "cynic runtime repo-access --json [--profile <path>]",
                    "cynic runtime report --json [--output <path> | --persist]",
                    "cynic runtime epoch --json [--advance] [--endpoint <kernel-url>]",
                    "cynic runtime agent-matrix --json",
                    "cynic runtime inference --json",
                    "cynic runtime inference-vault --json",
                    "cynic runtime inference-status --json",
                    "cynic runtime inference-start <alias> --json",
                    "cynic runtime inference-stop [alias] --json",
                    "cynic runtime inference-discover --json",
                    "cynic runtime inference-discover --json --apply",
                    "cynic runtime inference-discover --json --apply --open-webui",
                    "cynic runtime inference-models --json [--persist]",
                    "cynic runtime inference-hf-status --json",
                    "cynic runtime inference-hf-categories --json",
                    "cynic runtime inference-hardware --json",
                    "cynic runtime inference-hf-search --json [--category chat|code|…] [--query TEXT] [--limit N]",
                    "cynic runtime inference-hf-suggest --json [--category chat] [--limit N]",
                    "cynic runtime inference-hf-download <repo> --json [--alias NAME] [--include <glob>]... [--no-profile] [--cache-only]",
                    "cynic runtime inference-plan <alias> --json [--apply]",
                    "cynic deps [--json]",
                    "cynic deps plan [--json]",
                    "cynic deps audit [--json]",
                    "cynic deps upgrade [--yes] [--json] [--apply --confirm <token>] [--only <id>]...",
                    "cynic runtime deps [--json]  # alias",
                    "cynic runtime deps-audit [--json]  # alias",
                ],
            },
            CommandDomain {
                name: "dev",
                commands: vec![
                    "cynic dev surfaces --json",
                    "cynic dev outcomes --json",
                    "cynic dev kpi-score --json",
                    "cynic dev framework --json",
                    "cynic dev crate-audit --json [--output <path>] [--persist]",
                    "cynic dev kernel-audit --json [--output <path>] [--persist]",
                    "cynic dev ownership-audit --json [--output <path>] [--persist]",
                    "cynic dev surface-audit --json [--output <path>] [--persist]",
                    "cynic dev surface-matrix --json [--output <path>] [--persist]",
                    "cynic dev agent-exposure --json [--output <path>] [--persist]",
                    "cynic dev check --json [--architecture-only] [--target-root <path>]",
                ],
            },
            CommandDomain {
                name: "bootstrap",
                commands: vec![
                    "cynic bootstrap chain [--profile <name> | --node <name>] [--registry <path>]",
                    "cynic bootstrap constellation --json [--registry <path>]",
                    "cynic bootstrap bridge <bundle|plan|install|rootless-smoke|status> [--output-dir <dir> | --bundle-dir <dir> | --sandbox-root <dir>] [--apply]",
                    "cynic bootstrap framework bundle --json --output-dir <dir>",
                    "cynic bootstrap framework install-cli --json [--apply] [--source <path>] [--target <path>]",
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
                commands: vec![
                    "cynic transport matrix",
                    "cynic transport security-audit",
                    "cynic transport ssh-inspect --host <hostname|ip>",
                ],
            },
            CommandDomain {
                name: "network",
                commands: vec![
                    "cynic network chain [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network runtime [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network proof [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network evidence [--registry <path>]",
                    "cynic network dns <status|bundle|plan> [--registry <path>] [--resolve-dns] [--json] [--output-dir <dir>]",
                    "cynic network mesh plan [--registry <path>] [--provider <wireguard|headscale|tailscale|static>]",
                    "cynic network mesh bundle [--registry <path>] [--provider <wireguard|headscale|tailscale|static>] --output-dir <dir>",
                    "cynic network mesh apply-plan --bundle-dir <dir>",
                    "cynic network mesh target-attestation --target-id <guest> --transport qemu-guest-agent --transport-target <guest> --expected-network-boundary <network> [--harness-kind <kind>]",
                    "cynic network mesh apply-target --bundle-dir <dir> --profile <name> --target-attestation <json> --apply [--harness-kind <kind>]",
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
            CommandDomain {
                name: "integrations",
                commands: vec![
                    "cynic integrations status [--json]",
                    "cynic integrations plan [--json]",
                    "cynic integrations adopt <id> [--json] [--apply]",
                    "cynic integrations connect <id> [--json] [--apply]",
                ],
            },
            CommandDomain {
                name: "lifecycle",
                commands: vec![
                    "cynic lifecycle evidence <operation> [-- <operation-flags>]",
                    "cynic lifecycle status [--json] [--state <path>]",
                    "cynic lifecycle plan [--json]",
                    "cynic lifecycle gate [--yes] [--json]",
                    "cynic lifecycle promote --to <phase> --yes [--json]",
                    "cynic lifecycle report [--json]   # operational findings (end black box)",
                    "cynic lifecycle repair [--yes] [--json]",
                    "cynic lifecycle apply [--yes] [--json]",
                    "cynic lifecycle kernel-release --spec <path> [--state <path>] [--yes] [--json]",
                ],
            },
            CommandDomain {
                name: "chat",
                commands: vec![
                    "cynic chat  # interactive TUI (default)",
                    "cynic chat --mock | --ods  # TUI offline / ODS endpoint",
                    "cynic chat <prompt> [--model <alias|id>] [--resume ID] [--stream] [--mock|--ods] [--json]",
                    "cynic chat --model <alias|id> --mock --json # model-aware TUI boot / promptless control",
                    "cynic chat --list-sessions [--json]",
                    "cynic chat --export <session-id> [--json]",
                ],
            },
        ],
    }
}
