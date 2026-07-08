from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class NetworkActuatorRequest:
    profile: str
    commands: tuple[tuple[str, ...], ...]
    agent_id: str
    timeout_seconds: int = 30
    apply: bool = False

    @classmethod
    def from_json(cls, payload: dict[str, Any]) -> "NetworkActuatorRequest":
        raw_commands = payload.get("commands")
        commands: list[tuple[str, ...]] = []
        if isinstance(raw_commands, list):
            for command in raw_commands:
                if isinstance(command, list) and all(isinstance(item, str) for item in command):
                    commands.append(tuple(command))
        return cls(
            profile=str(payload.get("profile", "")),
            commands=tuple(commands),
            agent_id=str(payload.get("agent_id", "")),
            timeout_seconds=int(payload.get("timeout_seconds", 30) or 30),
            apply=bool(payload.get("apply", False)),
        )


@dataclass(frozen=True)
class NetworkActuatorValidation:
    ok: bool
    violations: tuple[str, ...]


class NetworkActuatorPolicy:
    def __init__(self, policy: dict[str, Any]) -> None:
        self.policy = policy

    @property
    def helper_path(self) -> str:
        return str(self.policy.get("helper_path", "/usr/local/libexec/cynic-network-actuator"))

    @property
    def sudoers_subject(self) -> str:
        return str(self.policy.get("sudoers_subject", "cynic"))

    def sudoers_rule(self) -> str:
        return f"{self.sudoers_subject} ALL=(root) NOPASSWD: {self.helper_path}"

    def validate_request(self, request: NetworkActuatorRequest, *, dry_run: bool = False) -> NetworkActuatorValidation:
        violations: list[str] = []
        if not request.profile.strip():
            violations.append("missing_profile")
        if not request.agent_id.strip():
            violations.append("missing_agent_id")
        if not request.commands:
            violations.append("missing_commands")
        if request.timeout_seconds > int(self.policy.get("max_timeout_seconds", 60)):
            violations.append("timeout_exceeds_policy")
        if not dry_run and not request.apply:
            violations.append("apply_flag_required")

        profile_policy = self.policy.get("allowed_profiles", {}).get(request.profile)
        if profile_policy is None:
            violations.append("profile_not_allowed")
            return NetworkActuatorValidation(ok=False, violations=tuple(sorted(set(violations))))

        allowed_prefixes = [
            tuple(str(part) for part in prefix)
            for prefix in profile_policy.get("command_prefixes", [])
        ]
        forbidden_tokens = set(self.policy.get("forbidden_tokens", []))
        for command in request.commands:
            if any(token in forbidden_tokens for token in command):
                violations.append("forbidden_token")
            if any(any(shell in token for shell in ("|", "&&", ";", ">", "$(", "`")) for token in command):
                violations.append("shell_syntax_forbidden")
            if not any(command[: len(prefix)] == prefix for prefix in allowed_prefixes):
                violations.append("command_prefix_not_allowed")

        return NetworkActuatorValidation(ok=not violations, violations=tuple(sorted(set(violations))))
