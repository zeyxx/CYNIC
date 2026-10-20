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
                    "cynic forensics postmortem [--json] [--boot-id <id>]",
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
                name: "hub",
                commands: vec![
                    "cynic hub upload --repo <id> --path <local> [--type model|dataset|space] --yes [--json]",
                    "cynic hub job submit [--script <file|url> | --image <img>] [--args...] --yes [--json]",
                    "cynic hub job poll --id <job-id> [--json]",
                    "cynic hub job list [--all] [--json]",
                    "cynic hub job logs --id <job-id> [--json]",
                    "cynic hub job cancel --id <job-id> --yes [--json]",
                    "cynic hub sched submit --schedule <cron|@alias> [--script | --image] --yes [--json]",
                    "cynic hub sched list [--json]",
                    "cynic hub sched delete --id <id> --yes [--json]",
                ],
            },
            CommandDomain {
                name: "audit",
                commands: vec![
                    "cynic audit crate [--json]",
                    "cynic audit secret [--json]",
                    "cynic audit node host [--json]",
                    "cynic audit paths [--json]",
                    "cynic audit branch [--json]",
                    "cynic audit all [--json]",
                    "cynic audit conformance [--profile <name>] [--node <name>] [--registry <path>] [--json]",
                    "cynic audit metrics [--profile <name>] [--node <name>] [--registry <path>] [--proof <path>] [--json]",
                    "cynic audit harness [--kind <name>] [--json]",
                    "cynic audit external [--json] [--registry <path>]",
                    "cynic audit gate-receipt [--path <path>] [--json]",
                ],
            },
            CommandDomain {
                name: "collaborator",
                commands: vec![
                    "cynic collaborator status [--json] [--registry <path>]",
                    "cynic collaborator promote <collaborator_id> --to-tier <tier> --yes [--evidence <ref>] [--operation <op>] [--registry <path>]",
                    "cynic collaborator demote <collaborator_id> --to-tier <tier> --yes [--evidence <ref>] [--operation <op>] [--registry <path>]",
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
                name: "mcp",
                commands: vec!["cynic mcp serve"],
            },
            CommandDomain {
                name: "daemon",
                commands: vec![
                    "cynic daemon kernel [--reset]",
                    "cynic daemon node [--config <path>] [--watchdog]",
                ],
            },
            CommandDomain {
                name: "supervise",
                commands: vec!["cynic supervise observe [--role <id>]"],
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
                    "cynic network mesh apply plan --bundle-dir <dir>",
                    "cynic network mesh target attestation --target-id <guest> --transport qemu-guest-agent --transport-target <guest> --expected-network-boundary <network> [--harness-kind <kind>]",
                    "cynic network mesh apply target --bundle-dir <dir> --profile <name> --target-attestation <json> --apply [--harness-kind <kind>]",
                    "cynic network mesh apply profile --bundle-dir <dir> --profile <name> --apply [--closed-environment]",
                    "cynic network mesh status [--registry <path>] [--resolve-dns] [--include-records] [--observe-runtime]",
                    "cynic network mesh coordination [--registry <path>] [--endpoint <url>] [--api-key <token>] [--check-health]",
                    "cynic network provider fallback <status|hosts> [--registry <path>] [--hosts-path <path>] [--check-connectivity] [--output-dir <dir>]",
                    "cynic network actuator <status|bundle|plan|apply> [--registry <path>] [--output-dir <dir> | --bundle-dir <dir>] [--profile <name>] [--apply]",
                ],
            },
            CommandDomain {
                name: "host",
                commands: vec![
                    "cynic host <bridge|membrane|transport|ssh|bootstrap|runner|capability|principal|seed|storage|hardware|service|network|request> <verb> [flags] [--apply]",
                    "cynic host bridge <bundle|plan|install|status> [--bundle-dir <dir>] [--apply]",
                    "cynic host service reconcile --node <node> --registry <path> <unit> --apply",
                    "cynic host network <baseline|actuate> --profile <name> --target <node> --registry <path> [--apply]",
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
                    "cynic node node-identity --node-id <id> [--signing-key-path <path>] [--apply] [--json]",
                    "cynic node csr-create --node-id <id> --role <role> [--key-path <path>] [--json]",
                    "cynic node cert-inspect [--cert-path <path>] [--json]",
                    "cynic node cert-verify --kernel-pubkey-hex <hex> [--cert-path <path>] [--json]",
                    "cynic node host-audit [--json] [--max-commits <n>]",
                    "cynic node host cpu [--json]",
                    "cynic node host memory [--json]",
                    "cynic node host disk status [--json]",
                    "cynic node host disk clean [--apply] [--json]",
                    "cynic node host capacity [--json]",
                    "cynic node compute provision --vmid <n> --hostname <name> --template-volid <volid> [--ip-cidr <cidr> --gateway <ip> --nameserver <ip> --features <flag>] [--node <name>] [--apply [--hypervisor <ssh-target>]]",
                ],
            },
            CommandDomain {
                name: "source",
                commands: vec![
                    "cynic source status [--root <path>] [--json]",
                    "cynic source release [--root <path>] [--dry-run] [--apply] [--json]",
                    "cynic source sync [--root <path>] [--apply] [--json]",
                    "cynic source worktree [--root <path>] [--workspace-store <path>] [--apply] [--json]",
                    "cynic source divergence [--root <path>] [--json]",
                    "cynic source mirrors [--json]",
                    "cynic source receipt verify [--path <path>] [--json]",
                ],
            },
            CommandDomain {
                name: "storage",
                commands: vec![
                    "cynic storage ping [--endpoint <url>] [--json]",
                    "cynic storage query --sql <surrealql> [--endpoint <url>] [--json]",
                    "cynic storage capabilities [--url <url>] [--json]",
                    "cynic storage credential [--endpoint <url>] [--json]",
                ],
            },
            CommandDomain {
                name: "git",
                commands: vec![
                    "cynic git branch graph [--json]",
                    "cynic git branch status [--root <path>] [--json]",
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
                    "cynic task list [--kind <k>] [--status <s>] [--priority <p>] [--json] [--ledger <path>]",
                    "cynic task add --title <t> [--priority <p>] [--kind <kairotic|operational>] [--status <s>] [--link <ref>] [--task-id <id>] [--json] [--ledger <path>]",
                    "cynic task status --task-id <id> --status <s> [--json] [--ledger <path>]",
                ],
            },
            CommandDomain {
                name: "federation",
                commands: vec!["cynic federation status [--json]", "cynic federation enroll status [--json]"],
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
                    "cynic build run --json",
                    "cynic build submit --json",
                    "cynic build trigger --commit <sha> [--phase check|build] [--package <pkg>] [--json]",
                    "cynic build pool eligibility [--node <id>] [--observed-labels a,b] [--json]",
                    "cynic build proofs list [--commit <sha>] [--limit N] [--json]",
                    "cynic build invoice --json",
                    "cynic build proof --json",
                    "cynic build workload observe --json",
                    "cynic build workload list --json",
                    "cynic build webhook serve --json",
                    "cynic build credentials provision --json",
                    "cynic build discover --json [--kind local|remote] [--target <ssh-target>]",
                    "cynic build reclaim --json",
                    "cynic build runner-token [--json]",
                    "cynic build runs [--json]",
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
            CommandDomain {
                name: "know",
                commands: vec![
                    "cynic know ask <question> [--limit <n>] [--commits <n>] [--prompt-only] [--endpoint <url>] [--model <name>] [--json]",
                ],
            },
            CommandDomain {
                name: "idea",
                commands: vec![
                    "cynic idea list [--json] [--domain <domain>] [--status <status>] [--freshness <freshness>] [--ledger <path>]",
                    "cynic idea add --title <title> [--description <text>] [--domain <domain>] [--status <status>] [--freshness <freshness>] [--idea-id <id>] [--json] [--ledger <path>]",
                    "cynic idea status --idea-id <idea-id> [--status <status>] [--freshness <freshness>] [--json] [--ledger <path>]",
                    "cynic idea consume --idea-id <idea-id> --consumer <consumer> [--json] [--ledger <path>]",
                ],
            },
            CommandDomain {
                name: "wiki",
                commands: vec![
                    "cynic wiki list [--kind <kind>] [--status <status>] [--registry <path>] [--json]",
                    "cynic wiki search <query> [--limit <n>] [--json]",
                    "cynic wiki show <article-id> [--registry <path>] [--json]",
                ],
            },
            CommandDomain {
                name: "coordination",
                commands: vec![
                    "cynic coordination append --agent <id> --kind <kind> --body <text> [--ref <anchor>] [--log <path>] [--json]",
                    "cynic coordination read [--kind <kind>] [--limit <n>] [--log <path>] [--json]",
                    "cynic coordination send --to <agent:id|role:r> --summary <140c> --body <text> [--priority 0-3] [--in-reply-to <msg>] [--json]",
                    "cynic coordination inbox [--unopened] [--full] [--limit <n>] [--log <path>] [--json]",
                    "cynic coordination ack <msg_id> [--state delivered|opened] [--json]",
                    "cynic coordination ledger [--json] [--claims <path>] [--log <path>] [--ledger <path>]",
                    "cynic coordination metrics [--json] [--claims <path>] [--log <path>] [--root <path>]",
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
                            && !token.starts_with('#')
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
