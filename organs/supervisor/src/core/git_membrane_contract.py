"""Git membrane closure plan validation."""

from __future__ import annotations

import subprocess
from typing import Any

from organs.supervisor.src.core.git_membrane import GitMembraneClosureService


def validate_plan_contract(service: GitMembraneClosureService) -> list[Any]:
    findings: list[Any] = []
    if not service.plan_path.exists():
        return [
            service.finding(
                "critical",
                "git_membrane_closure_plan_missing",
                service.relative(service.plan_path),
                "git-membrane-closure-plan-v1.json must define the ordered closure plan for open Git membranes.",
            )
        ]

    try:
        plan = service.load_json(service.plan_path)
    except Exception as exc:
        if hasattr(exc, "msg") and hasattr(exc, "lineno") and hasattr(exc, "colno"):
            detail = f"{exc.msg} at line {exc.lineno} column {exc.colno}"
        else:
            detail = str(exc)
        return [
            service.finding(
                "critical",
                "git_membrane_closure_plan_invalid_json",
                service.relative(service.plan_path),
                detail,
            )
        ]

    if plan.get("schema") != "cynic.git_membrane_closure_plan.v1":
        findings.append(
            service.finding(
                "critical",
                "unexpected_git_membrane_closure_plan_schema",
                service.relative(service.plan_path),
                f"expected cynic.git_membrane_closure_plan.v1, got {plan.get('schema')!r}",
            )
        )

    allowed_dispositions = set(plan.get("allowed_dispositions", []))
    if not allowed_dispositions:
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_allowed_dispositions_missing",
                service.relative(service.plan_path),
                "git-membrane-closure-plan-v1 must declare allowed_dispositions.",
            )
        )
    execution_requirements = plan.get("execution_requirements")
    if not isinstance(execution_requirements, dict):
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_execution_requirements_missing",
                service.relative(service.plan_path),
                "git-membrane-closure-plan-v1 must declare execution_requirements.",
            )
        )
        execution_requirements = {}
    for field in sorted(service.policy.execution_required_fields):
        if field not in execution_requirements or not execution_requirements.get(field):
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_execution_requirement_missing",
                    service.relative(service.plan_path),
                    f"execution_requirements must declare {field}.",
                )
            )
    if execution_requirements.get("requires_git_metadata_write") is not True:
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_requires_git_metadata_write",
                service.relative(service.plan_path),
                "execution_requirements.requires_git_metadata_write must be true.",
            )
        )
    if execution_requirements.get("claim_required") is not True:
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_claim_required",
                service.relative(service.plan_path),
                "execution_requirements.claim_required must be true.",
            )
        )
    if execution_requirements.get("preflight_command") != "make git-membrane-closure-preflight":
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_preflight_command_invalid",
                service.relative(service.plan_path),
                "execution_requirements.preflight_command must be make git-membrane-closure-preflight.",
            )
        )

    proof = plan.get("post_closure_proof")
    if not isinstance(proof, dict):
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_proof_missing",
                service.relative(service.plan_path),
                "git-membrane-closure-plan-v1 must declare post_closure_proof.",
            )
        )
        proof = {}
    for field in sorted(service.policy.proof_required_fields):
        value = proof.get(field)
        if not isinstance(value, list) or not value:
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_proof_metadata_missing",
                    service.relative(service.plan_path),
                    f"post_closure_proof must declare non-empty {field}.",
                )
            )

    waves = plan.get("waves")
    if not isinstance(waves, list) or not waves:
        findings.append(
            service.finding(
                "critical",
                "git_membrane_closure_waves_missing",
                service.relative(service.plan_path),
                "git-membrane-closure-plan-v1 must declare a non-empty waves list.",
            )
        )
        waves = []

    try:
        untracked = set(service.git_ls_untracked())
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            service.finding(
                "critical",
                "git_membrane_closure_git_scan_failed",
                ".",
                f"unable to query git untracked files for closure plan: {exc}",
            )
        ]

    seen_waves: set[int] = set()
    last_wave = 0
    target_paths: set[str] = set()
    for index, wave in enumerate(waves):
        item_path = f"waves[{index}]"
        for field in sorted(service.policy.wave_required_fields):
            if not wave.get(field):
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_wave_metadata_missing",
                        item_path,
                        f"waves entries must declare {field}.",
                    )
                )
        wave_number = wave.get("wave")
        if isinstance(wave_number, int):
            if wave_number in seen_waves:
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_duplicate_wave",
                        item_path,
                        f"wave {wave_number} is declared more than once.",
                    )
                )
            seen_waves.add(wave_number)
            if wave_number <= last_wave:
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_wave_order_invalid",
                        item_path,
                        "waves must be declared in strictly increasing order.",
                    )
                )
            last_wave = wave_number
        elif wave_number is not None:
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_wave_number_invalid",
                    item_path,
                    "wave must be an integer.",
                )
            )

        disposition = wave.get("disposition")
        if allowed_dispositions and disposition and str(disposition) not in allowed_dispositions:
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_disposition_unknown",
                    item_path,
                    f"disposition {disposition!r} is not declared in allowed_dispositions.",
                )
            )
        targets = wave.get("targets")
        if not isinstance(targets, list) or not targets:
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_targets_missing",
                    item_path,
                    "waves entries must declare non-empty targets.",
                )
            )
            continue
        for target_index, target in enumerate(targets):
            target_item = f"{item_path}.targets[{target_index}]"
            for field in sorted(service.policy.target_required_fields):
                if not target.get(field):
                    findings.append(
                        service.finding(
                            "critical",
                            "git_membrane_closure_target_metadata_missing",
                            target_item,
                            f"targets entries must declare {field}.",
                        )
                    )
            classification = target.get("classification")
            if classification and str(classification) not in service.policy.target_classifications:
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_target_classification_unknown",
                        target_item,
                        "classification %r is not in the allowed target classification vocabulary." % classification,
                    )
                )
            path_value = target.get("path")
            if not path_value:
                continue
            path = str(path_value)
            if path in target_paths:
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_duplicate_target",
                        path,
                        "closure plan targets must appear only once.",
                    )
                )
            target_paths.add(path)
            normalized = path.rstrip("/") + "/" if path.endswith("/") else path
            if not (service.workspace_root / path).exists():
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_target_missing",
                        path,
                        "closure plan target does not exist.",
                    )
                )
            if path.endswith("/") and not any(item.startswith(normalized) for item in untracked):
                findings.append(
                    service.finding(
                        "medium",
                        "git_membrane_closure_target_no_longer_open",
                        path,
                        "closure plan directory target has no visible untracked files; remove it from the open membrane plan or move it to done evidence.",
                    )
                )
            if not path.endswith("/") and path not in untracked and not (service.workspace_root / path).exists():
                findings.append(
                    service.finding(
                        "critical",
                        "git_membrane_closure_file_target_missing",
                        path,
                        "closure plan file target is neither visible nor present.",
                    )
                )

    for path in sorted(untracked):
        if not any(
            path.startswith(target) if target.endswith("/") else path == target
            for target in target_paths
        ):
            findings.append(
                service.finding(
                    "critical",
                    "git_membrane_closure_untracked_not_planned",
                    path,
                    "Every visible untracked file must be covered by git-membrane-closure-plan-v1.json before membrane closure can be claimed.",
                )
            )

    return findings
