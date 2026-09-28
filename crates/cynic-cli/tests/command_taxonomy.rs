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

use clap::CommandFactory;

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
