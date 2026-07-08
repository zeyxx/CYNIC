"""Git membrane closure domain service."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class GitMembraneClosurePolicy:
    target_required_fields: set[str]
    target_classifications: set[str]
    wave_required_fields: set[str]
    execution_required_fields: set[str]
    proof_required_fields: set[str]


@dataclass(frozen=True)
class GitMembraneClosureService:
    workspace_root: Path
    plan_path: Path
    policy: GitMembraneClosurePolicy
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    git_ls_untracked: Callable[..., list[str]]
    git_metadata_writable_probe: Callable[[], bool]
    finding: FindingFactory

    def validate_plan_contract(self) -> list[Any]:
        from organs.supervisor.src.core.git_membrane_contract import validate_plan_contract

        return validate_plan_contract(self)

    def summary(self, selected_wave: int | None = None) -> dict[str, Any]:
        from organs.supervisor.src.core.git_membrane_reporting import summary

        return summary(self, selected_wave=selected_wave)

    def matter_report(self) -> dict[str, Any]:
        from organs.supervisor.src.core.git_membrane_reporting import matter_report

        return matter_report(self)

    def git_metadata_writable(self) -> bool:
        return self.git_metadata_writable_probe()

    def commands(self, selected_wave: int | None = None) -> dict[str, Any]:
        from organs.supervisor.src.core.git_membrane_reporting import commands

        return commands(self, selected_wave=selected_wave)

    def preflight(self) -> dict[str, Any]:
        from organs.supervisor.src.core.git_membrane_reporting import preflight

        return preflight(self)

    def check(self, selected_wave: int | None = None) -> dict[str, Any]:
        from organs.supervisor.src.core.git_membrane_reporting import check

        return check(self, selected_wave=selected_wave)
