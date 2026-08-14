pub(crate) fn print_integrations_status_human(
    report: &cynic_host::domain::ecosystem_package::IntegrationsStatusReport,
) {
    println!(
        "integrations: {} (ready {}/{})",
        report.status, report.summary.ready, report.summary.total
    );
    for integration in &report.integrations {
        let location = integration.path_or_endpoint.as_deref().unwrap_or("-");
        let principal = integration.principal_name.as_deref().unwrap_or("-");
        let next_command = integration.next_command.as_deref().unwrap_or("-");
        let dependency_tooling = integration
            .dependency
            .as_ref()
            .and_then(|dependency| dependency.tooling_authority.as_ref())
            .map(|authority| {
                format!(
                    "{:?}:{:?}->{:?}:{} payload={:?}->{:?} normalize_now={} uv={} runtime_principal={} auto_apply={} cmd={}",
                    authority.family,
                    authority.observed_provenance,
                    authority.expected_provenance,
                    authority.status,
                    authority.observed_payload_provenance,
                    authority.expected_payload_provenance,
                    authority.normalizable_now,
                    authority.requires_uv,
                    authority.requires_runtime_principal,
                    authority.can_auto_apply,
                    authority
                        .canonical_update_command
                        .as_deref()
                        .unwrap_or("-")
                )
            })
            .unwrap_or_else(|| "-".to_string());
        println!(
            "- {}: {} location={} principal={} connection={:?} dependency_tooling={} next_boundary={} reason={} next={}",
            integration.id,
            integration.status,
            location,
            principal,
            integration.connection.state,
            dependency_tooling,
            integration.next_boundary,
            integration.next_boundary_reason,
            next_command
        );
    }
}

pub(crate) fn print_integrations_plan_human(
    report: &cynic_host::domain::ecosystem_package::IntegrationsPlanReport,
) {
    println!(
        "integration plan: {} (actions {}/{})",
        report.status, report.summary.planned_actions, report.summary.total
    );
    for integration in &report.integrations {
        let dependency_tooling = integration
            .dependency
            .as_ref()
            .and_then(|dependency| dependency.tooling_authority.as_ref())
            .map(|authority| {
                format!(
                    "{:?}:{:?}->{:?}:{} payload={:?}->{:?} normalize_now={} uv={} runtime_principal={} auto_apply={} cmd={}",
                    authority.family,
                    authority.observed_provenance,
                    authority.expected_provenance,
                    authority.status,
                    authority.observed_payload_provenance,
                    authority.expected_payload_provenance,
                    authority.normalizable_now,
                    authority.requires_uv,
                    authority.requires_runtime_principal,
                    authority.can_auto_apply,
                    authority
                        .canonical_update_command
                        .as_deref()
                        .unwrap_or("-")
                )
            })
            .unwrap_or_else(|| "-".to_string());
        println!(
            "- {}: action={:?} apply={} privilege_required={} dependency_tooling={}",
            integration.id,
            integration.action,
            integration.requires_apply,
            integration.privilege_required,
            dependency_tooling
        );
        for step in &integration.execution.apply {
            println!(
                "    - {}",
                cynic_host::command_surface::render_command(step)
            );
        }
    }
}

pub(crate) fn print_integration_adopt_human(
    report: &cynic_host::domain::ecosystem_package::IntegrationApplyReport,
) {
    println!(
        "integration adopt: id={} status={} apply={} before={} after={}",
        report.integration_id,
        report.status,
        report.requested_apply,
        report.before.status,
        report.after.status
    );
    for blocker in &report.blockers {
        println!("- blocker: {blocker}");
    }
    if !report.requested_apply {
        println!(
            "> To execute this adoption, run: cynic integrations adopt {} --apply",
            report.integration_id
        );
    }
}

pub(crate) fn print_integration_connect_human(
    report: &cynic_host::domain::ecosystem_package::IntegrationConnectReport,
) {
    println!(
        "integration connect: id={} status={} apply={} before={} after={}",
        report.integration_id,
        report.status,
        report.requested_apply,
        report.before.status,
        report.after.status
    );
    for blocker in &report.blockers {
        println!("- blocker: {blocker}");
    }
    for step in &report.execution_plan.steps {
        println!(
            "- step[{}]: {}",
            step.role,
            cynic_host::command_surface::render_command(&step.command)
        );
    }
    if !report.requested_apply {
        println!(
            "> To execute this connection boundary, run: cynic integrations connect {} --apply",
            report.integration_id
        );
    }
}
