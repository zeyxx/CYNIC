use cynic_host::update_lifecycle::{
    ApplyReport, KernelReleaseReport, LifecycleGateReport, LifecyclePlanReport,
    LifecyclePromoteReport, LifecycleStatusReport, OpsReport, RepairReport,
};

pub(crate) fn print_status_human(report: &LifecycleStatusReport) {
    println!("CYNIC update lifecycle");
    println!("  state:  {}", report.state_path);
    println!(
        "  phase:  {}  (authority={:?})",
        report.state.current_phase.as_str(),
        report.policy.harness.authority
    );
    println!(
        "  next:   {}",
        report.next_phase.unwrap_or("(production — cycle complete)")
    );
    println!(
        "  gate:   last_ok={:?}  can_promote={}",
        report.state.last_gate_ok, report.can_promote
    );
    if let Some(blocked) = &report.promote_blocked_reason {
        println!("  blocked promote: {blocked}");
    }
    println!("\n  progression:");
    for phase in &report.progression {
        let mark = if *phase == report.state.current_phase.as_str() {
            "*"
        } else {
            " "
        };
        println!("   {mark} {phase}");
    }
    println!("\n  allowed mutations:");
    for mutation in &report.policy.allowed_mutations {
        println!("   · {mutation:?}");
    }
    println!("\nNext:");
    println!("  cynic lifecycle report          # open the black box");
    println!("  cynic lifecycle repair          # plan fixes");
    println!("  cynic lifecycle repair --yes    # apply phase-safe fixes");
    println!("  cynic lifecycle gate --yes");
    println!("  cynic lifecycle plan");
    println!("  cynic lifecycle kernel-release --spec <path> [--state <path>] [--yes]");
    if report.can_promote {
        if let Some(next) = report.next_phase {
            println!("  cynic lifecycle promote --to {next} --yes");
        }
    }
    println!("  JSON: cynic lifecycle status --json");
}

pub(crate) fn print_plan_human(report: &LifecyclePlanReport) {
    println!("CYNIC lifecycle plan  phase={}\n", report.phase);
    for action in &report.planned_actions {
        let tag = if action.mutates { "MUTATE" } else { "read  " };
        let yes = if action.requires_yes { " [--yes]" } else { "" };
        println!("  [{tag}] {}{yes}", action.command);
        println!("          id={}", action.id);
    }
    println!("\nFrictionless:");
    for hint in &report.human {
        println!("  {hint}");
    }
}

pub(crate) fn print_gate_human(report: &LifecycleGateReport, executed: bool) {
    println!(
        "CYNIC lifecycle gate  phase={}  status={}\n",
        report.phase, report.status
    );
    if !executed {
        println!("  (plan only — re-run with --yes to execute)\n");
    }
    for gate in &report.results {
        let mark = if !gate.ran {
            "·"
        } else if gate.ok {
            "ok"
        } else {
            "FAIL"
        };
        let required = if gate.required {
            "required"
        } else {
            "optional"
        };
        println!("  [{mark}] {} ({required})  {}", gate.id, gate.command);
        println!("         {}", gate.detail);
    }
    println!("\nNext:");
    for next in &report.next {
        println!("  {next}");
    }
}

pub(crate) fn print_promote_human(report: &LifecyclePromoteReport) {
    println!("CYNIC lifecycle promote");
    println!("  status: {}", report.status);
    println!("  {} → {}", report.from, report.to.unwrap_or("(none)"));
    println!("  {}", report.message);
    if report.status == "ok" {
        println!("\n  current phase: {}", report.state.current_phase.as_str());
        println!("  re-gate: cynic lifecycle gate --yes");
    }
}

pub(crate) fn print_ops_human(report: &OpsReport) {
    println!("CYNIC operational report");
    println!(
        "  phase={}  authority={}  status={}  black_box={}",
        report.phase, report.authority, report.status, report.black_box
    );
    println!(
        "  findings: info={} warn={} error={} critical={} repairable={}",
        report.finding_counts.info,
        report.finding_counts.warning,
        report.finding_counts.error,
        report.finding_counts.critical,
        report.finding_counts.repairable
    );
    println!();
    for finding in &report.findings {
        if matches!(
            finding.severity,
            cynic_host::update_lifecycle::FindingSeverity::Info
        ) && finding.repair_command.is_none()
        {
            continue;
        }
        let severity = format!("{:?}", finding.severity).to_lowercase();
        let repair = if finding.phase_allows_repair {
            finding.repair_command.as_deref().unwrap_or("-")
        } else {
            "(not allowed in phase)"
        };
        println!("  [{severity}] {}  {}", finding.surface, finding.summary);
        println!("         repair: {repair}");
    }
    println!("\nNext:");
    for next in &report.next {
        println!("  • {next}");
    }
}

pub(crate) fn print_repair_human(report: &RepairReport, applied: bool) {
    println!(
        "CYNIC lifecycle repair  phase={}  status={}\n",
        report.phase, report.status
    );
    println!("  {}\n", report.message);
    if !applied {
        println!("  (plan only — re-run with --yes)\n");
    }
    for action in &report.actions {
        let tag = if action.mutates { "MUTATE" } else { "read  " };
        println!("  [{tag}] {}", action.command);
        println!("          id={}  {}", action.id, action.purpose);
    }
    if !report.results.is_empty() {
        println!("\nResults:");
        for result in &report.results {
            println!(
                "  {}  {}  {}",
                if result.ok { "ok  " } else { "FAIL" },
                result.id,
                result.detail.chars().take(120).collect::<String>()
            );
        }
    }
}

pub(crate) fn print_apply_human(report: &ApplyReport, applied: bool) {
    println!(
        "CYNIC lifecycle apply  phase={}  status={}\n",
        report.phase, report.status
    );
    println!("  {}\n", report.message);
    if !applied {
        println!("  (plan only — re-run with --yes)\n");
    }
    for action in &report.actions {
        let tag = if action.mutates { "MUTATE" } else { "read  " };
        println!("  [{tag}] {}", action.command);
    }
    if !report.results.is_empty() {
        println!("\nResults:");
        for result in &report.results {
            println!(
                "  {}  {}  exit={:?}",
                if result.ok { "ok  " } else { "FAIL" },
                result.id,
                result.exit_code
            );
        }
    }
}

pub(crate) fn print_kernel_release_human(report: &KernelReleaseReport, applied: bool) {
    println!(
        "CYNIC kernel release  node={}  phase={}  status={}",
        report.node_id, report.lifecycle_phase, report.status
    );
    println!(
        "  desired: version={} git={} sha256={}",
        report.desired_version, report.desired_git_sha, report.desired_sha256
    );
    if !applied {
        println!("  plan only: re-run with --yes");
    }
    for action in &report.actions {
        let marker = if action.mutates { "MUTATE" } else { "observe" };
        println!("  [{marker}] {}: {}", action.phase, action.description);
    }
    if let Some(identity) = &report.observed_after {
        println!(
            "  observed: node={} host={} pid={} version={} git={}",
            identity.node_id,
            identity.hostname,
            identity.pid,
            identity.build.version,
            identity.build.git_sha
        );
    }
    if let Some(event) = &report.proof_event {
        println!(
            "  proof: seq={} hash={} prev={}",
            event.seq, event.hash, event.prev_hash
        );
    }
    if let Some(error) = &report.error {
        println!("  error: {error}");
    }
}
