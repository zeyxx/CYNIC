use crate::deps_surface_model::{DepsPlanReport, DepsReport, FrictionlessHints, PlanAction};
use cynic_host::domain::ecosystem_package::{
    ProviderChannel, channel_auto_applicable, has_auto_handler, plan_for_dep,
    resolve_principal_install_layout,
};

pub(crate) fn build_plan(deps: &DepsReport, observed_at_unix_ms: u64) -> DepsPlanReport {
    let layout = resolve_principal_install_layout();
    let mut actions = Vec::new();
    let mut priority = 10u32;

    for dep in &deps.deps {
        let needs = matches!(dep.status.as_str(), "missing" | "stale" | "desired_upgrade")
            || dep.install_authority_status == "noncanonical";
        if !needs {
            continue;
        }

        let action_priority = if dep.tier == "required" {
            priority
        } else if dep.tier == "recommended" {
            priority + 100
        } else {
            priority + 200
        };
        priority += 1;

        let reason = match dep.status.as_str() {
            "missing" => format!("{} missing from PATH / principal bin", dep.id),
            "stale" => format!(
                "{} below pin ({:?})",
                dep.id,
                dep.pin.as_ref().and_then(|pin| {
                    pin.min_version
                        .clone()
                        .or_else(|| pin.min_build.map(|build| build.to_string()))
                })
            ),
            "desired_upgrade" => format!(
                "{} at minimum but below desired (e.g. newer GGUF support)",
                dep.id
            ),
            _other if dep.install_authority_status == "noncanonical" => format!(
                "{} tooling authority is noncanonical ({:?} observed, {:?} expected)",
                dep.id, dep.tooling_install_provenance, dep.expected_tooling_install_provenance
            ),
            other => other.to_string(),
        };

        let (commands, auto_applicable) = if let Some(plan) = plan_for_dep(dep.id, &reason, &layout)
        {
            (plan.summary_commands, plan.auto_applicable)
        } else {
            let auto = channel_auto_applicable(dep.channel)
                && dep.id != "systemd"
                && has_auto_handler(dep.id);
            (dep.watch.update_commands.clone(), auto)
        };

        actions.push(PlanAction {
            priority: action_priority,
            dep_id: dep.id.to_string(),
            reason,
            commands,
            mutates: true,
            upgrade_mode: dep.upgrade_mode,
            channel: dep.channel,
            install_principal: dep.install_principal,
            auto_applicable,
        });
    }
    actions.sort_by_key(|action| action.priority);

    let mut copy_paste = Vec::new();
    copy_paste.push("set -euo pipefail".into());
    copy_paste.push("# Prefer: cynic deps upgrade --yes".into());
    for action in &actions {
        copy_paste.push(format!(
            "# --- {} [{} / {:?}] : {} ---",
            action.dep_id,
            match action.channel {
                ProviderChannel::Scripted => "scripted",
                ProviderChannel::External => "external",
                ProviderChannel::DistroManaged => "distro_managed",
            },
            action.install_principal,
            action.reason
        ));
        for command in &action.commands {
            copy_paste.push(command.clone());
        }
    }
    copy_paste.push("cynic deps".into());

    DepsPlanReport {
        schema: "cynic.ecosystem_deps_plan.v1",
        surface: "tech_watch",
        observed_at_unix_ms,
        pins_path: deps.pins_path.clone(),
        actions,
        copy_paste,
        frictionless: FrictionlessHints {
            after_cli_change: "cynic deps upgrade --yes --only cynic",
            daily_check: "cynic deps",
            model_start: "cynic runtime inference-start <alias> --json",
            pins_edit: "edit infra/ecosystem-pins.json then: cynic deps",
        },
    }
}
