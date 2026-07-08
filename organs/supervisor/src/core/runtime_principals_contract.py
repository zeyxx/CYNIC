"""Runtime principals contract validation service."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class RuntimePrincipalsContractService:
    workspace_root: Path
    contract_path: Path
    agent_policy_path: Path
    required_fields: set[str]
    load_json: Callable[[Path], Any]
    load_toml: Callable[[Path], dict[str, Any]]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        if not self.contract_path.exists():
            return [
                self.finding(
                    "critical",
                    "runtime_principals_contract_missing",
                    self.relative(self.contract_path),
                    "runtime-principals-v1.json must define the dedicated organism OS user and cortex CLI posture.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except Exception as exc:
            if hasattr(exc, "msg") and hasattr(exc, "lineno") and hasattr(exc, "colno"):
                detail = f"{exc.msg} at line {exc.lineno} column {exc.colno}"
            else:
                detail = str(exc)
            return [
                self.finding(
                    "critical",
                    "runtime_principals_contract_invalid_json",
                    self.relative(self.contract_path),
                    detail,
                )
            ]

        for field in sorted(self.required_fields):
            if not contract.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_metadata_missing",
                        self.relative(self.contract_path),
                        f"runtime-principals-v1 must declare {field}.",
                    )
                )

        if contract.get("schema") != "cynic.runtime_principals.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_runtime_principals_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.runtime_principals.v1, got {contract.get('schema')!r}",
                )
            )

        dedicated_user = str(contract.get("dedicated_user", {}).get("name") or "")
        if not dedicated_user:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_dedicated_user_missing",
                    self.relative(self.contract_path),
                    "dedicated_user.name must declare the organism OS account.",
                )
            )

        cortex_policy = contract.get("cortex_cli_policy", {})
        required_agents = [str(item) for item in cortex_policy.get("required_agents", [])]
        required_runtime_user = str(cortex_policy.get("required_runtime_user") or dedicated_user)
        required_posture = str(cortex_policy.get("required_posture") or "")
        projection_path = str(cortex_policy.get("projection") or self.agent_policy_path.relative_to(self.workspace_root))
        projection = self.workspace_root / projection_path

        if not required_agents:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_required_agents_missing",
                    self.relative(self.contract_path),
                    "cortex_cli_policy.required_agents must list the cortex CLIs governed by the runtime user contract.",
                )
            )
        if required_runtime_user != dedicated_user:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_user_mismatch",
                    self.relative(self.contract_path),
                    "cortex_cli_policy.required_runtime_user must match dedicated_user.name.",
                )
            )
        if required_posture not in contract.get("postures", {}):
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_posture_missing",
                    self.relative(self.contract_path),
                    "cortex_cli_policy.required_posture must reference a declared posture.",
                )
            )

        findings.extend(self._validate_activation(contract.get("activation", {})))
        findings.extend(self._validate_legacy_sudoers(contract))
        findings.extend(
            self._validate_agent_policy(
                projection=projection,
                projection_path=projection_path,
                dedicated_user=dedicated_user,
                required_agents=required_agents,
                required_runtime_user=required_runtime_user,
                required_posture=required_posture,
                posture=contract.get("postures", {}).get(required_posture, {}),
            )
        )
        return findings

    def _validate_activation(self, activation: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        activation_script = str(activation.get("script") or "")
        activation_script_text = ""
        if activation_script:
            activation_script_path = self.workspace_root / activation_script
            if not activation_script_path.is_file():
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_activation_script_missing",
                        activation_script,
                        "runtime principals activation script must exist.",
                    )
                )
            else:
                try:
                    activation_script_text = activation_script_path.read_text(encoding="utf-8")
                except UnicodeDecodeError:
                    activation_script_text = ""
        else:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_activation_script_missing",
                    self.relative(self.contract_path),
                    "activation.script must declare the bounded incarnation script.",
                )
            )

        for command in activation.get("required_script_commands", []):
            command_text = str(command)
            if activation_script_text:
                is_bash_impl = f"{command_text})" in activation_script_text
                is_python_impl = f'"{command_text}"' in activation_script_text or f"'{command_text}'" in activation_script_text
                if not (is_bash_impl or is_python_impl):
                    findings.append(
                        self.finding(
                            "critical",
                            "runtime_principals_activation_command_missing",
                            activation_script,
                            f"runtime principals activation script must implement command {command_text!r}.",
                        )
                    )
        source_text = activation_script_text + "\n" + self._runtime_principals_source_text()
        for command in activation.get("operator_commands", []):
            command_text = str(command)
            if command_text and command_text not in source_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_operator_command_unprojected",
                        self.relative(self.contract_path),
                        f"operator command {command_text!r} must be present in runtime principals generated CLI/help/status projection.",
                    )
                )
        findings.extend(self._validate_primary_dependency_updates(activation, source_text))
        findings.extend(self._validate_lifecycle_operator_sequence(activation, source_text))
        return findings

    def _validate_lifecycle_operator_sequence(self, activation: dict[str, Any], source_text: str) -> list[Any]:
        findings: list[Any] = []
        sequence = activation.get("lifecycle_operator_sequence")
        if sequence is None:
            return findings
        if not isinstance(sequence, dict):
            return [
                self.finding(
                    "critical",
                    "runtime_principals_lifecycle_sequence_invalid",
                    self.relative(self.contract_path),
                    "activation.lifecycle_operator_sequence must be an object when declared.",
                )
            ]

        profiles = sequence.get("profiles")
        steps = sequence.get("steps")
        if not isinstance(profiles, list) or not all(isinstance(item, str) and item for item in profiles):
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_lifecycle_sequence_profiles_invalid",
                    self.relative(self.contract_path),
                    "lifecycle_operator_sequence.profiles must be a non-empty list of profile names.",
                )
            )
            profiles = []
        if not isinstance(steps, list) or not all(isinstance(item, dict) for item in steps):
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_lifecycle_sequence_steps_invalid",
                    self.relative(self.contract_path),
                    "lifecycle_operator_sequence.steps must be a list of step objects.",
                )
            )
            steps = []

        activation_command = str(sequence.get("activation_command") or "")
        sequence_commands: list[tuple[str, str]] = []
        if activation_command:
            sequence_commands.extend(
                [
                    (activation_command, "cynic {profile} --dry-run"),
                    (activation_command, "cynic {profile}"),
                    (activation_command, "cynic {profile} --json"),
                    (activation_command, "cynic organism"),
                ]
            )
            for field in ("activation_operator_contract", "operator_sequence", "proof_commands", "done_when"):
                if field not in source_text:
                    findings.append(
                        self.finding(
                            "critical",
                            "runtime_principals_lifecycle_sequence_field_unprojected",
                            self.relative(self.contract_path),
                            f"activation lifecycle operator sequence field {field!r} must be projected by runtime principals.",
                        )
                    )

        for field in sequence.get("required_projection_fields", []):
            field_text = str(field)
            if field_text and field_text not in source_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_lifecycle_sequence_field_unprojected",
                        self.relative(self.contract_path),
                        f"lifecycle operator sequence field {field_text!r} must be projected by runtime principals.",
                    )
                )

        for profile in profiles:
            for step in steps:
                template = str(step.get("command_template") or "")
                if not template:
                    findings.append(
                        self.finding(
                            "critical",
                            "runtime_principals_lifecycle_sequence_step_missing_command",
                            self.relative(self.contract_path),
                            f"lifecycle operator sequence step for profile {profile!r} must declare command_template.",
                        )
                    )
                    continue
                name = str(step.get("name") or "")
                mutates_mempool = step.get("mutates_mempool")
                rendered_template = template.format(profile=profile)
                if rendered_template.endswith(" --json") and rendered_template.startswith(
                    ("cynic activate", "cynic mcp-reload", "cynic gate-2", "cynic deploy")
                ):
                    if name != "dispatch_json" or mutates_mempool is not True:
                        findings.append(
                            self.finding(
                                "critical",
                                "runtime_principals_lifecycle_json_dispatch_misclassified",
                                self.relative(self.contract_path),
                                f"lifecycle JSON dispatch {rendered_template!r} must use name='dispatch_json' and mutates_mempool=true.",
                            )
                        )
                sequence_commands.append((profile, template))

        for profile, template in sequence_commands:
            command = template.format(profile=profile)
            if command not in source_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_lifecycle_sequence_command_unprojected",
                        self.relative(self.contract_path),
                        f"lifecycle operator sequence command {command!r} must be projected by runtime principals.",
                    )
                )
        return findings

    def _validate_primary_dependency_updates(self, activation: dict[str, Any], source_text: str) -> list[Any]:
        findings: list[Any] = []
        updates = activation.get("primary_dependency_updates")
        if not isinstance(updates, dict):
            return [
                self.finding(
                    "critical",
                    "runtime_principals_primary_updates_missing",
                    self.relative(self.contract_path),
                    "activation.primary_dependency_updates must declare native cortex CLI update commands.",
                )
            ]

        operator_command = str(updates.get("operator_command") or "")
        if operator_command and operator_command not in source_text:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_update_operator_command_unprojected",
                    self.relative(self.contract_path),
                    f"primary dependency update operator command {operator_command!r} must be projected by runtime principals.",
                )
            )

        allowed = updates.get("allowed_update_commands")
        if not isinstance(allowed, dict) or not allowed:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_allowed_update_commands_missing",
                    self.relative(self.contract_path),
                    "primary_dependency_updates.allowed_update_commands must map each cortex CLI to a native update command.",
                )
            )
            return findings

        expected_update_commands = {
            "codex": ["codex", "update"],
            "agy": ["agy", "update"],
            "hermes": ["make", "hermes-update"],
        }
        for agent, command in sorted(allowed.items()):
            if not isinstance(command, list) or not all(isinstance(part, str) and part for part in command):
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_allowed_update_command_invalid",
                        self.relative(self.contract_path),
                        f"allowed update command for {agent!r} must be a non-empty list of strings.",
                    )
                )
                continue
            expected_command = expected_update_commands.get(str(agent), [str(agent), "update"])
            if command != expected_command:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_allowed_update_command_not_native",
                        self.relative(self.contract_path),
                        f"allowed update command for {agent!r} must be {expected_command!r}.",
                    )
                )
            command_literal = repr(command)
            double_quote_literal = "[" + ", ".join(f'"{part}"' for part in command) + "]"
            if command_literal not in source_text and double_quote_literal not in source_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_allowed_update_command_unimplemented",
                        self.relative(self.contract_path),
                        f"allowed update command for {agent!r} is not implemented by runtime principals.",
                    )
                )

        forbidden_patterns = updates.get("forbidden_update_patterns")
        if not isinstance(forbidden_patterns, list) or not forbidden_patterns:
            findings.append(
                self.finding(
                    "critical",
                    "runtime_principals_forbidden_update_patterns_missing",
                    self.relative(self.contract_path),
                    "primary_dependency_updates.forbidden_update_patterns must declare blocked non-native update paths.",
                )
            )
            forbidden_patterns = []
        infra_contract_path = self.workspace_root / "axioms" / "infra-consumption-v1.json"
        if infra_contract_path.is_file():
            try:
                infra_contract = self.load_json(infra_contract_path)
            except Exception:
                infra_contract = {}
            infra_forbidden = set(
                str(item)
                for item in infra_contract.get("native_lifecycle", {}).get("forbidden_on_cortex", [])
                if item
            )
            missing_infra_forbidden = sorted(infra_forbidden - {str(item) for item in forbidden_patterns})
            if missing_infra_forbidden:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_update_forbidden_patterns_incomplete",
                        self.relative(self.contract_path),
                        "primary_dependency_updates.forbidden_update_patterns must include infra-consumption native_lifecycle.forbidden_on_cortex entries: "
                        + ", ".join(missing_infra_forbidden),
                    )
                )
        for field in updates.get("required_projection_fields", []):
            field_text = str(field)
            if field_text and field_text not in source_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_update_projection_field_unprojected",
                        self.relative(self.contract_path),
                        f"primary dependency update projection field {field_text!r} must be implemented by runtime principals.",
                    )
                )
        return findings

    def _runtime_principals_source_text(self) -> str:
        roots = [
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_actions.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_content.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_lifecycle_reports.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_organism_reports.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_organism_text.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_reports.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_runtime.py",
            self.workspace_root / "organs/supervisor/src/core/runtime_principals_runtime_reports.py",
            self.workspace_root / "organs/supervisor/src/entrypoints",
        ]
        parts: list[str] = []
        for root in roots:
            if root.is_file():
                try:
                    parts.append(root.read_text(encoding="utf-8"))
                except UnicodeDecodeError:
                    pass
                continue
            if not root.is_dir():
                continue
            for path in sorted(root.glob("runtime_principals*.py")):
                try:
                    parts.append(path.read_text(encoding="utf-8"))
                except UnicodeDecodeError:
                    continue
        return "\n".join(parts)

    def _validate_legacy_sudoers(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        activation = contract.get("activation", {})
        legacy_provision = self.workspace_root / "infra/monitoring/provision_cynic.sh"
        if not legacy_provision.exists():
            return findings
        try:
            legacy_text = legacy_provision.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            legacy_text = ""
        for forbidden in activation.get("forbidden_legacy_sudoers", []):
            if str(forbidden) in legacy_text:
                findings.append(
                    self.finding(
                        "critical",
                        "runtime_principals_legacy_sudoers_forbidden",
                        self.relative(legacy_provision),
                        f"legacy provisioning must not grant forbidden sudoers rule {forbidden!r}.",
                    )
                )
        return findings

    def _validate_agent_policy(
        self,
        *,
        projection: Path,
        projection_path: str,
        dedicated_user: str,
        required_agents: list[str],
        required_runtime_user: str,
        required_posture: str,
        posture: dict[str, Any],
    ) -> list[Any]:
        findings: list[Any] = []
        if not projection.exists():
            return [
                self.finding(
                    "critical",
                    "runtime_principals_projection_missing",
                    projection_path,
                    "runtime principals projection file is missing.",
                )
            ]

        try:
            agent_policy = self.load_toml(projection)
        except Exception as exc:
            return [
                self.finding(
                    "critical",
                    "runtime_principals_projection_invalid_toml",
                    projection_path,
                    f"unable to parse runtime principals projection: {exc}",
                )
            ]

        runtime = agent_policy.get("runtime", {})
        if runtime.get("authority") != self.relative(self.contract_path):
            findings.append(
                self.finding(
                    "critical",
                    "agent_policy_runtime_authority_mismatch",
                    projection_path,
                    "infra/agent-policy.toml must declare runtime.authority = axioms/runtime-principals-v1.json.",
                )
            )
        if runtime.get("dedicated_user") != dedicated_user:
            findings.append(
                self.finding(
                    "critical",
                    "agent_policy_dedicated_user_mismatch",
                    projection_path,
                    f"runtime.dedicated_user must be {dedicated_user!r}.",
                )
            )

        agents = agent_policy.get("agents", {})
        for agent in required_agents:
            item = agents.get(agent)
            if not isinstance(item, dict):
                findings.append(
                    self.finding(
                        "critical",
                        "agent_policy_required_agent_missing",
                        projection_path,
                        f"agent policy must declare agents.{agent}.",
                    )
                )
                continue
            if item.get("runtime_user") != required_runtime_user:
                findings.append(
                    self.finding(
                        "critical",
                        "agent_policy_runtime_user_mismatch",
                        f"{projection_path}:agents.{agent}",
                        f"{agent} must run as runtime_user {required_runtime_user!r}.",
                    )
                )
            if item.get("execution_posture") != required_posture:
                findings.append(
                    self.finding(
                        "critical",
                        "agent_policy_execution_posture_mismatch",
                        f"{projection_path}:agents.{agent}",
                        f"{agent} must use execution_posture {required_posture!r}.",
                    )
                )

        concerns = agent_policy.get("concerns", {})
        for concern_name in posture.get("forbidden_nopasswd_concerns", []):
            concern = concerns.get(str(concern_name), {})
            if concern.get("nopasswd") is not False:
                findings.append(
                    self.finding(
                        "critical",
                        "agent_policy_destructive_concern_nopasswd",
                        f"{projection_path}:concerns.{concern_name}",
                        f"bounded_yolo requires concern {concern_name!r} to remain passworded or absent.",
                    )
                )

        deny_commands = set(str(item) for item in agent_policy.get("deny", {}).get("commands", []))
        for command in posture.get("forbidden_commands", []):
            if str(command) not in deny_commands:
                findings.append(
                    self.finding(
                        "critical",
                        "agent_policy_forbidden_command_missing",
                        projection_path,
                        f"bounded_yolo deny list must include {command!r}.",
                    )
                )
        return findings
