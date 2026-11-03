use crate::deps_surface_model::{DepsPlanReport, DepsReport, FrictionlessHints, PlanAction};
use cynic_pkg::{
    ProviderChannel, action_priority, channel_auto_applicable, has_auto_handler, needs_upgrade,
    plan_for_dep, reason_for_dep, resolve_principal_install_layout,
};

pub(crate) fn build_plan(deps: &DepsReport, observed_at_unix_ms: u64) -> DepsPlanReport {
    let layout = resolve_principal_install_layout();
    let mut actions = Vec::new();
    let mut priority = 10u32;

    for dep in &deps.deps {
        if !needs_upgrade(dep.id, dep.status.as_str(), dep.install_authority_status.as_str()) {
            continue;
        }

        let action_priority = action_priority(dep.tier, priority);
        priority += 1;

        let reason = reason_for_dep(
            dep.id,
            dep.status.as_str(),
            dep.install_authority_status.as_str(),
            dep
                .pin
                .as_ref()
                .and_then(|pin| pin.min_version.clone())
                .or_else(|| dep.pin.as_ref().and_then(|pin| pin.min_build.map(|b| b.to_string()))),
            dep.tooling_install_provenance.as_ref(),
            dep.expected_tooling_install_provenance.as_ref(),
        );

        let (commands, auto_applicable) = if let Some(plan) = plan_for_dep(dep.id, &reason, &layout) {
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
            model_start: "cynic models start <alias> --yes --json",
            pins_edit: "edit infra/ecosystem-pins.json then: cynic deps",
        },
    }
}
