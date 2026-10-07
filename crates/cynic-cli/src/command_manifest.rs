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
                name: "forensics",
                commands: vec![
                    "cynic forensics evidence [--json] [--boot-id <id>]",
                    "cynic forensics timeline --boot-id <id> [--json]",
                    "cynic forensics post-mortem [--json] [--boot-id <id>]",
                    "cynic forensics status [--json]",
                    "cynic forensics services [--json] [--min-rate <rate>]",
                ],
            },
            CommandDomain {
                name: "runtime",
                commands: vec![
                    "cynic runtime paths --json [--registry <path>]",
                    "cynic runtime registry observe --json [--observation <path>] [--registry <path>] [--apply]",
                    "cynic runtime validate --json [--registry <path>] [--lab-registry <path>]",
                    "cynic runtime suite --json [--registry <path>] [--profile <name>]",
                    "cynic runtime plan --json [--registry <path>] [--profile <name>] [--proof <path>]",
                    "cynic runtime capacity --json",
                    "cynic runtime harness status --json",
                    "cynic runtime harness progress --json",
                    "cynic runtime conformance --json [--profile <name> | --node <name>] [--registry <path>] [--build-evidence <sha256-digest>]",
                    "cynic runtime metrics --json [--profile <name> | --node <name>] [--registry <path>] [--proof <runtime-proof-bundle.json>]",
                    "cynic runtime storage capabilities --json [--url <surrealdb-url>]",
                    "cynic runtime storage credential --json [--endpoint <url>]",
                    "cynic runtime storage client --json",
                    "cynic runtime execution context --json [--host <name>]...",
                    "cynic runtime resolver plan --json",
                    "cynic runtime repo access --json [--profile <path>]",
                    "cynic runtime proof persist --json --proof <runtime-proof-persistence.json> [--endpoint <kernel-url>]",
                    "cynic runtime report --json [--output <path> | --persist]",
                    "cynic runtime agent matrix --json",
                    "cynic runtime disk status --json",
                    "cynic runtime disk clean --json",
                    "cynic runtime memory --json",
                    "cynic runtime cpu --json",
                    "cynic runtime network status --json [--host <name>]...",
                    "cynic runtime network preflight --json [--host <name>]...",
                    "cynic runtime forgejo token --json",
                    "cynic runtime build artefact --json",
                    "cynic runtime queue --json [--apply]",
                ],
            },
            CommandDomain {
                name: "inference",
                commands: vec![
                    "cynic inference config [--json]",
                    "cynic inference vault [--json]",
                    "cynic inference status [--json]",
                    "cynic inference start <alias> --yes [--json]",
                    "cynic inference stop [alias] --yes [--json]",
                    "cynic inference forge [--json]",
                    "cynic inference discover [--json]",
                    "cynic inference hardware [--json]",
                    "cynic inference plan [--json]",
                    "cynic inference models [--json]",
                    "cynic inference hf [--json]",
                    "cynic inference trace [--json]",
                ],
            },
            CommandDomain {
                name: "audit",
                commands: vec![
                    "cynic audit crate [--json]",
                    "cynic audit secret [--json]",
                    "cynic audit node-host [--json]",
                    "cynic audit paths [--json]",
                    "cynic audit branch [--json]",
                    "cynic audit all [--json]",
                ],
            },
            CommandDomain {
                name: "secret",
                commands: vec![
                    "cynic secret status [--json]",
                    "cynic secret check [--json]",
                    "cynic secret resolve [--json]",
                    "cynic secret install [--json]",
                    "cynic secret use [--json]",
                    "cynic secret audit [--json]",
                ],
            },
            CommandDomain {
                name: "privilege",
                commands: vec![
                    "cynic privilege catalog [--json]",
                    "cynic privilege admit [--json]",
                    "cynic privilege exec [--json]",
                    "cynic privilege install [--json]",
                ],
            },
            CommandDomain {
                name: "discovery",
                commands: vec![
                    "cynic discovery capabilities [--json]",
                    "cynic discovery nodes [--json]",
                    "cynic discovery events [--watch] [--json]",
                    "cynic discovery discover --kind local|remote [--target <ssh>] [--json]",
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
                name: "dependency",
                commands: vec![
                    "cynic dependency status",
                    "cynic dependency plan",
                    "cynic dependency audit",
                    "cynic dependency upgrade [--yes] [--apply --confirm <token>] [--only <id>]...",
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
                name: "court",
                commands: vec!["cynic court audit [--scope <surfaces>] [--json]"],
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
    /// `commands` and `advanced` are manifest/meta help commands, not domains.
    #[test]
    fn manifest_domains_match_clap_surface() {
        let cli = Cli::command();
        let clap_names: Vec<String> = cli
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        let visible: Vec<String> = cli
            .get_subcommands()
            .filter(|s| !s.is_hide_set())
            .map(|s| s.get_name().to_string())
            .collect();
        let manifest = commands_manifest();
        let domains: Vec<&str> = manifest.domains.iter().map(|d| d.name).collect();

        let multi = ["models", "doctor", "errors", "status", "version"];
        for name in &visible {
            if name == "commands" || name == "advanced" || multi.contains(&name.as_str()) {
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

    /// Every command string in the manifest must resolve to a real clap
    /// subcommand path. This catches drift the domain-name test cannot see:
    /// a domain can be listed while its commands are stale or missing.
    #[test]
    fn manifest_commands_resolve_in_clap() {
        let cli = Cli::command();
        let manifest = commands_manifest();
        let mut checked = 0usize;
        for domain in &manifest.domains {
            for command in &domain.commands {
                let path = command
                    .split_whitespace()
                    .skip(1) // skip "cynic"
                    .take_while(|token| {
                        !token.starts_with("--")
                            && !token.starts_with('<')
                            && !token.starts_with('[')
                    })
                    .map(|token| token.to_string())
                    .collect::<Vec<_>>();
                if path.is_empty() {
                    continue;
                }
                checked += 1;
                let mut current = &cli;
                for (depth, segment) in path.iter().enumerate() {
                    let found = current
                        .get_subcommands()
                        .find(|s| s.get_name() == segment.as_str());
                    assert!(
                        found.is_some(),
                        "manifest command `{}` (domain `{}`) does not resolve at segment {depth}: `{segment}`",
                        command,
                        domain.name
                    );
                    current = found.unwrap();
                }
            }
        }
        assert!(
            checked > 100,
            "manifest command resolution checked too few commands: {checked}"
        );
    }
}
