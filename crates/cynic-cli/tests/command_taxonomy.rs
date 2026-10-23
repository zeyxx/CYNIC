//! Command taxonomy guard: the surface the operator types.
//!
//! WHY THIS EXISTS. `contracts/foundation.json` → `no_hyphen_in_cli_commands`
//! is a constitutional rule, and a constitutional rule that nothing executes is
//! a wish. The rule had been an owner instruction for weeks with no authority
//! file, and the workspace paid for it with a `dev` surface that no binary could
//! reach — a command nobody can type cannot carry a rule.
//!
//! WHY IT FAILS TODAY, on purpose. The guard reflects the real `clap` surface
//! (not the source text: source parsing already gave this repository two wrong
//! answers about the command tree) and reports every hyphenated command. 59
//! were measured on 2026-08-28 from the built binary: 31 under `runtime`,
//! 15 under `network`, and the rest scattered. This test is RED until they are
//! gone, and that redness is the debt made visible rather than described.
//!
//! WHAT A HYPHEN ACTUALLY COSTS. A hyphenated name is how a flat verb-noun set
//! hides a missing noun level: `cynic runtime inference-status` reads as one
//! idea but is a verb (`runtime`) wrapping a noun (`inference`) wrapping a verb
//! (`status`). The compiler sees one string, so no tool can see the taxonomy
//! the name is hiding. The fix is never a longer name — it is the level the
//! name is asking for. `clap` cannot express a subcommand containing a space,
//! so every rename here is a structural split into two levels, which is why
//! these are not a mechanical search-and-replace.

use clap::{CommandFactory, Parser};

use cynic_cli::Cli;

/// Depth to walk. Beyond 2 the tree is a detail of one domain; the rule is
/// about what an operator types, and it is stated the same way at every level.
const MAX_DEPTH: usize = 2;

fn collect(command: &clap::Command, path: &mut Vec<String>, out: &mut Vec<String>, depth: usize) {
    for sub in command.get_subcommands().filter(|s| !s.is_hide_set()) {
        path.push(sub.get_name().to_string());
        if sub.get_name().contains('-') {
            out.push(path.join(" "));
        }
        if depth < MAX_DEPTH {
            collect(sub, path, out, depth + 1);
        }
        path.pop();
    }
}

fn hyphenated_commands() -> Vec<String> {
    let cli = Cli::command();
    let mut out = Vec::new();
    let mut path = Vec::new();
    collect(&cli, &mut path, &mut out, 0);
    out.sort();
    out
}

#[test]
fn no_cli_command_carries_a_hyphen() {
    let offenders = hyphenated_commands();
    assert!(
        offenders.is_empty(),
        "{} command(s) carry a hyphen, which contracts/foundation.json forbids \
         (no_hyphen_in_cli_commands). Each one is a flat verb-noun set standing in \
         for a missing noun level: split it into two command levels rather than \
         joining words with a dash.\n\noffenders:\n  {}\n\nTo read the current \
         surface: `cynic --help` and the subcommand help. To see the whole tree: \
         `cynic audit branch --json` does not help here — this guard reflects clap \
         directly so it needs no built binary.",
        offenders.len(),
        offenders.join("\n  ")
    );
}

#[test]
fn the_guard_actually_walks_the_surface() {
    // A guard that always passes is a guard that measures nothing. Assert the
    // walker reaches real subcommands, so a refactor that empties the surface
    // cannot quietly turn this file into a no-op.
    let all = Cli::command();
    let top_level: Vec<String> = all
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(|s| s.get_name().to_string())
        .collect();
    assert!(
        top_level.len() >= 20,
        "expected the operator surface to have at least 20 top-level commands, found {} — \
         if the surface really shrank, this guard's own reachability assumption is stale",
        top_level.len()
    );
    assert!(
        all.get_subcommands().any(|s| s.get_name() == "audit"),
        "the gated audit entry point must remain reachable from the top level"
    );
}

#[test]
fn deploy_status_is_nested_and_does_not_require_a_commit() {
    let parsed = Cli::try_parse_from([
        "cynic",
        "build",
        "deploy",
        "status",
        "--registry",
        "infra/registry.json",
        "--locations",
        "infra/deploy-locations.json",
        "--json",
    ]);
    assert!(parsed.is_ok(), "deploy status argv was rejected");
}

#[test]
fn deploy_plan_keeps_the_existing_direct_verb_shape() {
    let commit = "a".repeat(40);
    let parsed = Cli::try_parse_from([
        "cynic",
        "build",
        "deploy",
        "--commit",
        commit.as_str(),
        "--json",
    ]);
    assert!(parsed.is_ok(), "deploy plan argv was rejected");
}

#[test]
fn deploy_plan_emits_opt_out_reasons_when_every_machine_is_skipped() {
    let directory = tempfile::tempdir().expect("temporary fixture directory");
    let policy_path = directory.path().join("policy.json");
    let locations_path = directory.path().join("locations.json");
    let registry_path = directory.path().join("registry.json");

    std::fs::write(
        &policy_path,
        serde_json::json!({
            "schema": "cynic.deploy_policy.v1",
            "status": "normative",
            "canary_order": ["cortex"],
            "stop_before_swap": true,
            "containment_ladder": [],
            "per_step_gates": {"health": []}
        })
        .to_string(),
    )
    .expect("write policy fixture");
    std::fs::write(
        &locations_path,
        serde_json::json!({
            "schema": "cynic.deploy_locations.v1",
            "status": "observed",
            "machines": [{
                "hostname": "cortex",
                "registry_node_id": "workstation-node",
                "os": "linux",
                "ssh_target": "local",
                "principal": "user",
                "deploy_path": "$HOME/.local/bin/cynic",
                "deploy_path_resolved": "/home/fixture/.local/bin/cynic",
                "path_owner": "user:user",
                "layout": "flat",
                "backup_pattern": "cynic.{timestamp}.bak",
                "readback": {
                    "sha256_command": "sha256sum {path}",
                    "identity_command": "{path} version --json",
                    "smoke_command": "{path} --version",
                    "smoke_expect": "exit 0"
                }
            }]
        })
        .to_string(),
    )
    .expect("write locations fixture");
    std::fs::write(
        &registry_path,
        serde_json::json!({
            "nodes": [{"id": "workstation-node", "deploy_target": false}]
        })
        .to_string(),
    )
    .expect("write registry fixture");

    let commit = "a".repeat(40);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_cynic"))
        .args(["build", "deploy", "--commit"])
        .arg(commit)
        .arg("--policy")
        .arg(&policy_path)
        .arg("--locations")
        .arg(&locations_path)
        .arg("--registry")
        .arg(&registry_path)
        .arg("--json")
        .output()
        .expect("run deploy plan command");

    assert!(
        output.status.success(),
        "deploy plan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse deploy plan JSON");
    assert_eq!(plan["steps"], serde_json::json!([]));
    assert_eq!(plan["skipped"][0]["machine"], "cortex");
    assert_eq!(
        plan["skipped"][0]["reason"],
        "registry deploy_target is false; the machine opted out"
    );
}
