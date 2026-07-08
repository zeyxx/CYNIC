"""Validation service for the LXC exec bridge contract."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class LxcExecBridgeContractService:
    workspace_root: Path
    contract_path: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        if not self.contract_path.exists():
            return [
                self.finding(
                    "critical",
                    "lxc_exec_bridge_contract_missing",
                    self.relative(self.contract_path),
                    "lxc-exec-bridge-v1.json must define bounded LXC execution before storage activation can rely on it.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "lxc_exec_bridge_contract_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.lxc_exec_bridge.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_lxc_exec_bridge_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.lxc_exec_bridge.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_not_normative",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must be normative before fleet_node_exec may use it.",
                )
            )

        authority = contract.get("authority")
        if not isinstance(authority, dict):
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_authority_missing",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must declare authority.",
                )
            )
            authority = {}
        for field in ("infra_contract", "organ", "policy_file"):
            if not authority.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_authority_field_missing",
                        self.relative(self.contract_path),
                        f"authority must declare {field}.",
                    )
                )

        principles = set(contract.get("principles") or [])
        for principle in (
            "deny_by_default",
            "no_raw_shell",
            "no_agent_direct_pct_exec",
            "root_owned_helper_only",
            "sudoers_allows_helper_not_pct",
            "structured_json_request",
            "audit_every_attempt",
        ):
            if principle not in principles:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_principle_missing",
                        self.relative(self.contract_path),
                        f"principles must include {principle}.",
                    )
                )

        runtime = contract.get("runtime_shape")
        if not isinstance(runtime, dict):
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_runtime_shape_missing",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must declare runtime_shape.",
                )
            )
            runtime = {}
        expected_runtime = {
            "helper_path": "/usr/local/libexec/cynic-lxc-exec-bridge",
            "source_path": "organs/fleet/src/entrypoints/lxc_exec_bridge_helper.py",
            "request_transport": "stdin_json",
            "response_transport": "stdout_json",
            "bundle_command": "make lxc-exec-bridge-bundle out=/tmp/cynic-lxc-bridge",
            "sudoers_subject": "cynic",
            "sudoers_rule_shape": "cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-lxc-exec-bridge",
        }
        for field, expected in expected_runtime.items():
            if runtime.get(field) != expected:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_runtime_shape_drift",
                        self.relative(self.contract_path),
                        f"runtime_shape.{field} must be {expected!r}, got {runtime.get(field)!r}.",
                    )
                )
        forbidden_sudoers = str(runtime.get("forbidden_sudoers_shape") or "")
        for token in ("/usr/sbin/pct", "/usr/bin/pct", "/usr/bin/bash", "/bin/sh"):
            if token not in forbidden_sudoers:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_forbidden_sudoers_shape_incomplete",
                        self.relative(self.contract_path),
                        f"forbidden_sudoers_shape must name {token}.",
                    )
                )

        env = runtime.get("mcp_transport_env") if isinstance(runtime.get("mcp_transport_env"), dict) else {}
        if "CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET" not in env:
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_transport_env_missing",
                    self.relative(self.contract_path),
                    "runtime_shape.mcp_transport_env must declare CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET.",
                )
            )

        request = contract.get("request")
        if not isinstance(request, dict):
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_request_contract_missing",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must declare request.",
                )
            )
            request = {}
        required_fields = set(request.get("required_fields") or [])
        for field in ("node", "vmid", "purpose", "argv", "agent_id"):
            if field not in required_fields:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_request_field_missing",
                        self.relative(self.contract_path),
                        f"request.required_fields must include {field}.",
                    )
                )

        policy = contract.get("policy")
        if not isinstance(policy, dict):
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_policy_missing",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must declare policy.",
                )
            )
            policy = {}
        if policy.get("allowed_command_shape") != "argv_prefix_allowlist_per_purpose":
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_allowed_command_shape_invalid",
                    self.relative(self.contract_path),
                    "policy.allowed_command_shape must be argv_prefix_allowlist_per_purpose.",
                )
            )
        if policy.get("max_timeout_seconds") != 60:
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_timeout_drift",
                    self.relative(self.contract_path),
                    "policy.max_timeout_seconds must remain 60.",
                )
            )
        allowed_purposes = set(policy.get("allowed_purposes") or [])
        for purpose in ("read_only", "service_control"):
            if purpose not in allowed_purposes:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_allowed_purpose_missing",
                        self.relative(self.contract_path),
                        f"policy.allowed_purposes must include {purpose}.",
                    )
                )

        mcp = contract.get("mcp_integration")
        if not isinstance(mcp, dict):
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_mcp_integration_missing",
                    self.relative(self.contract_path),
                    "LXC exec bridge contract must declare mcp_integration.",
                )
            )
            mcp = {}
        if mcp.get("operation") != "fleet.node.exec":
            findings.append(
                self.finding(
                    "critical",
                    "lxc_exec_bridge_mcp_operation_invalid",
                    self.relative(self.contract_path),
                    "mcp_integration.operation must be fleet.node.exec.",
                )
            )
        transition = str(mcp.get("lxc_transition") or "")
        for marker in ("precondition_missing", "CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET"):
            if marker not in transition:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_transition_guard_missing",
                        self.relative(self.contract_path),
                        f"mcp_integration.lxc_transition must mention {marker}.",
                    )
                )
        evidence = set(mcp.get("activation_evidence") or [])
        for item in (
            "source_helper_tests_pass",
            "helper_exists_root_owned",
            "sudoers_rule_matches_contract",
            "policy_validates",
            "dry_run_read_only_command_passes",
            "raw_pct_exec_not_allowed_to_agent",
            "activation_status_reports_active",
        ):
            if item not in evidence:
                findings.append(
                    self.finding(
                        "critical",
                        "lxc_exec_bridge_activation_evidence_missing",
                        self.relative(self.contract_path),
                        f"mcp_integration.activation_evidence must include {item}.",
                    )
                )

        return findings
