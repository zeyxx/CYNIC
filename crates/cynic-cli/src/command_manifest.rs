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
                name: "operator",
                commands: vec![
                    "cynic status [--json]",
                    "cynic doctor [--json]",
                    "cynic version [--json]",
                    "cynic errors [--json]",
                    "cynic advanced",
                    "cynic models list [--json]",
                    "cynic models status [--json]",
                    "cynic models start [alias] --yes [--json] [--no-wait]",
                    "cynic models stop [alias] --yes [--json]",
                    "cynic chat [<prompt>] [--model <alias|id>] [--json]",
                ],
            },
            CommandDomain {
                name: "fleet",
                commands: vec![
                    "cynic fleet report --json [--registry <path>] [--proxmox-json <path>] [--proxmox-list-text <path|->] [--watchdog-json <path>]",
                    "cynic fleet discover --json [--registry <path>]",
                    "cynic fleet inventory --json [--registry <path>]",
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
                    "cynic runtime conformance --json [--profile <name> | --node <name>] [--registry <path>] [--build-evidence <sha256-digest>]",
                    "cynic runtime metrics --json [--profile <name> | --node <name>] [--registry <path>] [--proof <runtime-proof-bundle.json>]",
                    "cynic runtime proof-persist --json --proof <runtime-proof-persistence.json> [--endpoint <kernel-url>]",
                    "cynic runtime storage-capabilities --json [--url <surrealdb-url>]",
                    "cynic runtime execution-context --json [--host <name>]...",
                    "cynic runtime network-preflight --json [--host <name>]...",
                    "cynic runtime repo-access --json [--profile <path>]",
                    "cynic runtime report --json [--output <path> | --persist]",
                    "cynic runtime agent-matrix --json",
                    "cynic runtime agents --json",
                    "cynic runtime hermes-run --json --run-id <id>",
                    "cynic runtime inference --json",
                    "cynic runtime inference-vault --json",
                    "cynic runtime inference-status --json",
                    "cynic runtime inference-start <alias> --yes --json",
                    "cynic runtime inference-stop [alias] --yes --json",
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
                    "cynic runtime deps [--json]  # alias of cynic deps",
                    "cynic runtime deps-plan [--json]  # alias of cynic deps plan",
                    "cynic runtime deps-audit [--json]  # alias of cynic deps audit",
                    "cynic runtime deps-upgrade [--json]  # alias of cynic deps upgrade",
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
                    "cynic dev check --json [--architecture-only] [--source-root <path>] [--target-root <path>]",
                    "cynic dev opportunities [--json] [--frame <id>] [--output <path>]",
                ],
            },
            CommandDomain {
                name: "bootstrap",
                commands: vec![
                    "cynic bootstrap chain [--profile <name> | --node <name>] [--registry <path>]",
                    "cynic bootstrap constellation --json [--registry <path>]",
                    "cynic bootstrap bridge <bundle|plan|install|rootless-smoke|status> [--output-dir <dir> | --bundle-dir <dir> | --sandbox-root <dir>] [--apply]",
                    "cynic bootstrap framework bundle --json --output-dir <dir> [--role <role>] [--artifact-dir <dir>] [--registry <path>]",
                    "cynic bootstrap framework propagation-plan --json --bundle-dir <dir> [--registry <path>] [--output <path>]",
                    "cynic bootstrap framework propagate --json --bundle-dir <dir> [--registry <path>] [--output <path>] --yes",
                    "cynic bootstrap framework source-snapshot-plan --json --repository-root <dir> --source-commit <commit> --output-dir <dir> [--output <path>]",
                    "cynic bootstrap framework source-snapshot-create --json --repository-root <dir> --source-commit <commit> --output-dir <dir> [--output <path>] --yes",
                    "cynic bootstrap framework substrate-plan --json --node <id> [--registry <path>] [--observation <path>]",
                    "cynic bootstrap framework substrate-apply --json --node <id> [--registry <path>] --yes",
                    "cynic bootstrap framework lifecycle-audit --json --role <role> [--registry <path>]",
                    "cynic bootstrap framework enrollment-authority-plan --json --seed-manifest <path> --node <id> --staging-root <path> [--registry <path>]",
                    "cynic bootstrap framework enrollment-authority-apply --json --seed-manifest <path> --node <id> --staging-root <path> [--registry <path>] --yes",
                    "cynic bootstrap framework runtime-binding-plan --json [--registry <path>]",
                    "cynic bootstrap framework runtime-binding-apply --json [--registry <path>] --yes",
                    "cynic bootstrap framework provider-enrollment-plan --json --token-secret-file <path> [--registry <path>] [--ca-file <path>]",
                    "cynic bootstrap framework provider-enrollment-apply --json --token-secret-file <path> [--registry <path>] [--ca-file <path>] --yes",
                    "cynic bootstrap framework install-cli --json [--apply] [--source <path>] [--target <path>]",
                    "cynic bootstrap host <observe|bundle|plan|evidence|proof|supervisor|principal|e2e|apply-report|apply> [--profile <name> | --node <name>] [--registry <path>] [--dry-run|--apply]",
                    "cynic bootstrap host e2e [--profile <name> | --node <name>] [--registry <path>] [--runtime-profile <name>] --dry-run",
                    "cynic bootstrap host supervisor [--profile <name> | --node <name>] [--registry <path>] [--provider <auto|systemd-system|systemd-user|openrc|process-port>]",
                    "cynic bootstrap host principal [--profile <name> | --node <name>] [--registry <path>] [--apply]",
                ],
            },
            CommandDomain {
                name: "network",
                commands: vec![
                    "cynic network chain [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network runtime [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network proof [--registry <path>] [--resolve-dns] [--check-provider-connectivity]",
                    "cynic network evidence [--registry <path>]",
                    "cynic network dns <status|bundle|plan|activate> [--registry <path>] [--resolve-dns] [--json] [--output-dir <dir> | --bundle-dir <dir> --yes]",
                    "cynic network mesh plan [--registry <path>] [--provider <wireguard|headscale|tailscale|static>]",
                    "cynic network mesh bundle [--registry <path>] [--provider <wireguard|headscale|tailscale|static>] --output-dir <dir>",
                    "cynic network mesh apply-plan --bundle-dir <dir>",
                    "cynic network mesh target-attestation --target-id <guest> --transport qemu-guest-agent --transport-target <guest> --expected-network-boundary <network> [--harness-kind <kind>]",
                    "cynic network mesh apply-target --bundle-dir <dir> --profile <name> --target-attestation <json> --apply [--harness-kind <kind>]",
                    "cynic network mesh apply-profile --bundle-dir <dir> --profile <name> --apply [--closed-environment]",
                    "cynic network mesh status [--registry <path>] [--resolve-dns] [--include-records] [--observe-runtime]",
                    "cynic network mesh coordination [--registry <path>] [--endpoint <url>] [--api-key <token>] [--check-health]",
                    "cynic network provider-fallback <status|hosts> [--registry <path>] [--hosts-path <path>] [--check-connectivity] [--output-dir <dir>]",
                    "cynic network actuator <status|bundle|plan|apply> [--registry <path>] [--output-dir <dir> | --bundle-dir <dir>] [--profile <name>] [--apply]",
                ],
            },
            CommandDomain {
                name: "agent",
                commands: vec![
                    "cynic agent identify --agent-id <id> [--agent-kind <kind>] [--registry <path>] [--kernel-base <url>] [--json]",
                    "cynic agent register --agent-id <id> [--intent <text>] [--registry <path>] [--kernel-base <url>] [--json]",
                    "cynic agent claim --agent-id <id> --target <path> [--claim-type <file|crate>] [--ttl-seconds <n>] [--budget-actions <n>] [--budget-wall-seconds <n>] [--registry <path>] [--kernel-base <url>] [--json]",
                    "cynic agent release --agent-id <id> --target <path> --lease-id <id> [--fencing-token <n>] [--proof-ref <ref>]... [--registry <path>] [--kernel-base <url>] [--json]",
                    "cynic agent status [--agent-id <id|*>] [--registry <path>] [--kernel-base <url>] [--json]",
                    "cynic agent session-context [--registry <path>] [--json]",
                ],
            },
            CommandDomain {
                name: "node",
                commands: vec![
                    "cynic node status --node-id <id> [--json]",
                    "cynic node activate --node-id <id> [--yes] [--json]",
                    "cynic node identity-init --key-path <path> [--json]",
                    "cynic node csr-create --node-id <id> --role <role> --key-path <path> [--json]",
                    "cynic node cert-inspect --cert-path <path> [--json]",
                    "cynic node cert-verify --kernel-pubkey-hex <hex> --cert-path <path> [--json]",
                    "cynic node host-audit [--json] [--max-commits <n>]",
                ],
            },
            CommandDomain {
                name: "source",
                commands: vec![
                    "cynic source status [--root <path>] [--json]",
                    "cynic source release [--root <path>] [--dry-run] [--apply] [--json]",
                    "cynic source sync [--root <path>] [--apply] [--json]",
                    "cynic source worktree [--root <path>] [--workspace-store <path>] [--apply] [--json]",
                ],
            },
            CommandDomain {
                name: "git",
                commands: vec![
                    "cynic git branch-graph [--json]",
                    "cynic git branch-status [--root <path>] [--json]",
                    "cynic git reconcile [--root <path>] [--json]",
                    "cynic git hygiene [--root <path>] [--json]",
                ],
            },
            CommandDomain {
                name: "judge",
                commands: vec![
                    "cynic judge patch --branch <name> [--root <path>] [--json]",
                    "cynic judge runtime --subject <id> [--json]",
                    "cynic judge digest --subject <id> [--json]",
                ],
            },
            CommandDomain {
                name: "deps",
                commands: vec![
                    "cynic deps plan",
                    "cynic deps audit",
                    "cynic deps upgrade [--yes] [--apply --confirm <token>] [--only <id>]...",
                ],
            },
            CommandDomain {
                name: "session",
                commands: vec!["cynic session observe [--json]"],
            },
            CommandDomain {
                name: "update",
                commands: vec!["cynic update [<target>] [--json]"],
            },
            CommandDomain {
                name: "task",
                commands: vec![
                    "cynic task push --domain <d> --title <t> --queue-path <path> [--priority <p>] [--payload <json>] [--agent <id>] [--json]",
                    "cynic task list --domain <d> --status <s> --queue-path <path> [--json]",
                    "cynic task claim --domain <d> --agent <id> --queue-path <path> [--json]",
                    "cynic task complete --task-id <id> --agent <id> --queue-path <path> [--evidence <ref>] [--json]",
                    "cynic task fail --task-id <id> --reason <text> --queue-path <path> [--json]",
                ],
            },
            CommandDomain {
                name: "federation",
                commands: vec!["cynic federation status [--json]"],
            },
            CommandDomain {
                name: "mask",
                commands: vec![
                    "cynic mask forge --report <path> [--source <path>] [--json] [--yes]",
                    "cynic mask start [--alias <name>] [--json] [--yes]",
                ],
            },
            CommandDomain {
                name: "queue",
                commands: vec!["cynic queue [--json] [--apply]"],
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
                    "cynic lifecycle kernel-release --spec <path> [--state <path>] [--yes] [--json] [--registry <path>]",
                ],
            },
            CommandDomain {
                name: "build",
                commands: vec![
                    "cynic build discover --json [--kind local|remote] [--target <ssh-target>]",
                    "cynic build trigger --commit <sha> [--phase check|build] [--package <pkg>] [--json]",
                    "cynic build list-proofs [--commit <sha>] [--limit N] [--json]",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cli;
    use clap::CommandFactory;

    /// Every visible top-level clap subcommand must have a manifest domain,
    /// and every domain must match a real subcommand. `operator` is the
    /// documented multi-variant family (models/doctor/errors/status);
    /// `commands` is the manifest meta-command itself.
    #[test]
    fn manifest_domains_match_clap_surface() {
        let cli = Cli::command();
        let clap_names: Vec<String> = cli
            .get_subcommands()
            .filter(|s| !s.is_hide_set())
            .map(|s| s.get_name().to_string())
            .collect();
        let manifest = commands_manifest();
        let domains: Vec<&str> = manifest.domains.iter().map(|d| d.name).collect();

        let multi = ["models", "doctor", "errors", "status"];
        for name in &clap_names {
            if name == "commands" || multi.contains(&name.as_str()) {
                continue;
            }
            assert!(
                domains.contains(&name.as_str()),
                "clap subcommand `cynic {name}` has no manifest domain"
            );
        }
        assert!(domains.contains(&"operator"), "operator domain missing");
        for domain in &domains {
            assert!(
                clap_names.iter().any(|n| n == domain) || *domain == "operator",
                "manifest domain `{domain}` matches no clap subcommand"
            );
        }
    }
}
