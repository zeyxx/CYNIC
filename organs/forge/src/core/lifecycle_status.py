"""Frictionless lifecycle status surface for humans and agents."""

from __future__ import annotations

from .lifecycle_transition_debt import build_lifecycle_transition_debt_report


def build_lifecycle_status_report(
    transition_contract: dict,
    activation_report: dict,
) -> dict:
    debt_report = build_lifecycle_transition_debt_report(
        transition_contract,
        activation_report=activation_report,
    )
    blockers = activation_report.get("blockers", [])
    can_dispatch = (
        activation_report.get("lifecycle_nodes_ready") is True
        and not blockers
        and debt_report["summary"]["by_layer"].get("effective", 0) == 0
    )
    if can_dispatch:
        state = "ready_for_dispatch"
        recommended_action = "Dispatch the desired lifecycle profile, then require applied and verified read-back."
    elif blockers:
        state = "needs_runtime_attention"
        recommended_action = "Resolve lifecycle blockers before dispatch."
    else:
        state = "needs_proof_reconciliation"
        recommended_action = "Resolve transition debt before claiming the lifecycle is ready."

    return {
        "schema": "cynic.lifecycle_status.v1",
        "status": "ok",
        "lifecycle": transition_contract.get("lifecycle", "kernel_release"),
        "state": state,
        "can_dispatch": can_dispatch,
        "recommended_action": recommended_action,
        "targets": {
            "build": activation_report.get("build_target"),
            "deploy": activation_report.get("deploy_target"),
        },
        "readiness": {
            "status_nodes_ready": activation_report.get("status_nodes_ready"),
            "fleet_nodes_ready": activation_report.get("fleet_nodes_ready"),
            "lifecycle_nodes_ready": activation_report.get("lifecycle_nodes_ready"),
        },
        "debt_summary": debt_report.get("summary", {}),
        "top_debts": debt_report.get("debts", [])[:5],
        "next_commands": {
            "inspect_debt": "python3 organs/forge/src/entrypoints/cli.py lifecycle-transition-debt",
            "inspect_constellation": "python3 organs/forge/src/entrypoints/cli.py lifecycle-constellation",
            "inspect_bootstrap_source": "python3 organs/forge/src/entrypoints/cli.py lifecycle-bootstrap-source-check",
            "dispatch_activation": "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-dispatch",
        },
    }
