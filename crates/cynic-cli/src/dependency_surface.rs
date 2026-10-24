//! `cynic dependency` — the object's real name, at the level it deserves.
//!
//! Until 2026-09-28 these lived as `cynic runtime deps`, `deps-plan`,
//! `deps-audit` and `deps-upgrade`, each documented as "compatibility alias →
//! `cynic deps …`". That target never existed: there is no `cynic deps`, so
//! the four were not aliases at all — they were the only way to reach
//! `deps_surface::execute_deps*`. The comment was the lie, and a lie in a
//! doc comment is how a surface loses an owner.
//!
//! `deps` is an abbreviation, and an abbreviation of the workspace's own
//! dependencies is not a verb. The object is `dependency`; it belongs at the
//! top level next to `update`, not buried under `runtime`, which owns path
//! resolution, evidence, disk and local inference. A surface projects; it does
//! not own. This one owns only the name.
//!
//! `cynic update` also reaches `execute_deps_upgrade`, with a terser name and
//! no `--yes`/`--apply`. That duplication is untouched here: `cynic update` is
//! not a hyphen violation, so retiring it is a separate decision, recorded
//! rather than taken.

use std::process::ExitCode;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum DependencyCommand {
    /// Current declared vs resolved dependency state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// What an upgrade would change, without changing it.
    Plan {
        #[arg(long)]
        json: bool,
    },
    /// Advisory and drift audit over the dependency graph.
    Audit {
        #[arg(long)]
        json: bool,
    },
    /// Resolve and upgrade stale components.
    Upgrade {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        confirm: Option<String>,
        #[arg(long)]
        only: Vec<String>,
        /// Explicit source binary path for first-contact bootstrap.
        #[arg(long)]
        source: Option<String>,
    },
}

pub(crate) fn execute_dependency_command(command: DependencyCommand) -> ExitCode {
    match command {
        DependencyCommand::Status { json } => crate::deps_surface::execute_deps(json),
        DependencyCommand::Plan { json } => crate::deps_surface::execute_deps_plan(json),
        DependencyCommand::Audit { json } => crate::deps_surface::execute_deps_audit(json),
        DependencyCommand::Upgrade {
            json,
            yes,
            apply,
            confirm,
            only,
            source,
        } => crate::deps_surface::execute_deps_upgrade(
            json,
            apply,
            yes,
            confirm.as_deref(),
            &only,
            source.as_deref(),
        ),
    }
}
