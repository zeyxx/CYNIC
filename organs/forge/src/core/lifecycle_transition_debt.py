"""Lifecycle transition debt report helpers."""

from __future__ import annotations


def _append_debt(
    debts: list[dict],
    *,
    layer: str,
    severity: str,
    code: str,
    description: str,
    evidence: dict | None = None,
    next_action: str,
) -> None:
    debts.append(
        {
            "layer": layer,
            "severity": severity,
            "code": code,
            "description": description,
            "evidence": evidence or {},
            "next_action": next_action,
        }
    )


def _summary_by_layer(debts: list[dict], layers: list[str]) -> dict:
    summary = {layer: 0 for layer in layers}
    for debt in debts:
        layer = debt.get("layer")
        if layer in summary:
            summary[layer] += 1
    return summary


def build_lifecycle_transition_debt_report(
    contract_report: dict,
    activation_report: dict | None = None,
    applied_evidence: dict | None = None,
    verified_evidence: dict | None = None,
) -> dict:
    activation_report = activation_report or {}
    applied_evidence = applied_evidence or {}
    verified_evidence = verified_evidence or {}
    contract = contract_report.get("transition_contract", {})
    layers = contract.get(
        "layers",
        ["desired", "declared", "observed", "effective", "applied", "verified"],
    )
    debts: list[dict] = []

    if contract_report.get("status") != "ok":
        _append_debt(
            debts,
            layer="declared",
            severity="critical",
            code="transition_contract_invalid",
            description="The registry lifecycle transition contract is invalid.",
            evidence={"errors": contract_report.get("errors", [])},
            next_action="Fix infra/registry.json:lifecycle.kernel_release.workflow.transition_contract.",
        )

    blockers = activation_report.get("blockers", [])
    if blockers:
        _append_debt(
            debts,
            layer="observed",
            severity="critical",
            code="runtime_blockers_present",
            description="Runtime observation still reports lifecycle blockers.",
            evidence={"blockers": blockers},
            next_action="Resolve lifecycle-blockers before dispatching lifecycle work.",
        )

    lifecycle_ready = activation_report.get("lifecycle_nodes_ready")
    activation_requests = activation_report.get("activation_request_summary", [])
    activation_request_count = activation_report.get("activation_request_count")
    if activation_request_count is None:
        activation_request_count = len(activation_report.get("requests", []))
    if lifecycle_ready is True and activation_request_count:
        _append_debt(
            debts,
            layer="effective",
            severity="high",
            code="ready_but_activation_requests_generated",
            description=(
                "Lifecycle nodes are marked ready while activation requests are still generated."
            ),
            evidence={
                "activation_request_count": activation_request_count,
                "activation_request_summary": activation_requests,
            },
            next_action=(
                "Reconcile activation request generation with activation evidence, or dispatch "
                "and read back the activation requests."
            ),
        )

    if not applied_evidence.get("runner_result"):
        _append_debt(
            debts,
            layer="applied",
            severity="high" if activation_request_count else "medium",
            code="missing_runner_result_readback",
            description="No applied-layer runner result is attached to this transition report.",
            evidence={
                "required": ["claimed_agent_task", "runner_result"],
                "activation_request_count": activation_request_count,
            },
            next_action=(
                "Use cynic_lifecycle_dispatch(profile=...) or lifecycle activation dispatch, "
                "then read back the claimed task and runner result."
            ),
        )

    if not verified_evidence.get("completed_agent_task"):
        _append_debt(
            debts,
            layer="verified",
            severity="medium",
            code="missing_completed_task_readback",
            description="No verified-layer completed task read-back is attached to this report.",
            evidence={
                "required": ["completed_agent_task", "service_health_or_activation_evidence"]
            },
            next_action=(
                "After dispatch completion, read back the completed task and service or "
                "activation evidence before claiming the lifecycle transition is verified."
            ),
        )

    return {
        "schema": "cynic.lifecycle_transition_debt.v1",
        "status": "ok" if not debts else "debt",
        "lifecycle": contract_report.get("lifecycle", "kernel_release"),
        "orchestrator_organ": contract_report.get("orchestrator_organ"),
        "layers": layers,
        "summary": {
            "debt_count": len(debts),
            "by_layer": _summary_by_layer(debts, layers),
            "activation_request_count": activation_request_count,
            "lifecycle_nodes_ready": lifecycle_ready,
        },
        "debts": debts,
    }
