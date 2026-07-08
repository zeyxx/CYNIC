"""Hermes Agent capability discovery adapter."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
import os
from pathlib import Path
import subprocess
import tomllib
from typing import Any

from organs.contracts.src.core.agent_framework_capabilities import (
    AgentFrameworkCapabilities,
    AgentFrameworkCapability,
    sorted_capabilities,
    utc_now_iso,
)


@dataclass(frozen=True)
class CommandResult:
    argv: tuple[str, ...]
    returncode: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.returncode == 0


CommandRunner = Callable[[tuple[str, ...]], CommandResult]


def subprocess_runner(argv: tuple[str, ...]) -> CommandResult:
    try:
        completed = subprocess.run(
            argv,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except OSError as exc:
        return CommandResult(argv=argv, returncode=127, stdout="", stderr=str(exc))
    except subprocess.TimeoutExpired as exc:
        return CommandResult(
            argv=argv,
            returncode=124,
            stdout=exc.stdout or "",
            stderr=exc.stderr or "command timed out",
        )
    return CommandResult(
        argv=argv,
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


class HermesAgentCapabilityScanner:
    """Probe Hermes Agent without making it an organ-forge dependency."""

    def __init__(
        self,
        *,
        binary: str = "hermes",
        runner: CommandRunner = subprocess_runner,
        skill_roots: tuple[Path, ...] | None = None,
        source_root: Path | None = None,
    ) -> None:
        self.binary = binary
        self.runner = runner
        self.skill_roots = skill_roots if skill_roots is not None else default_skill_roots()
        self.source_root = source_root

    def scan(self, observed_at: str | None = None) -> AgentFrameworkCapabilities:
        version_result = self.runner((self.binary, "--version"))
        help_result = self.runner((self.binary, "--help"))
        skill_evidence = list(self._skill_evidence())
        source_evidence, source_text, source_summary, source_gaps = self._source_evidence()
        command_text = "\n".join((version_result.stdout, version_result.stderr, help_result.stdout, help_result.stderr))
        skill_text = "\n".join(str(item.get("text_excerpt", "")) for item in skill_evidence)
        capability_text = f"{command_text}\n{skill_text}\n{source_text}".lower()

        capabilities: list[AgentFrameworkCapability] = []
        if version_result.ok or help_result.ok:
            self._add_capability(
                capabilities,
                "agent_task_execution",
                ("cynic.agent_task.v1",),
                ("cynic.agent_task_result.v1",),
                ("kernel_task_dispatch", "external_process"),
                "observed",
            )
        if self._mentions_repo_review(capability_text):
            self._add_capability(
                capabilities,
                "repo_review",
                ("cynic.git_matter_review_request.v1",),
                ("cynic.git_matter_review.v1",),
                ("repo_read", "repo_write_requires_claim"),
                "inferred",
            )
        self._add_source_capabilities(capabilities, capability_text, source_summary)

        evidence = [
            self._command_evidence("version", version_result),
            self._command_evidence("help", help_result),
        ]
        evidence.extend(skill_evidence)
        evidence.extend(source_evidence)

        return AgentFrameworkCapabilities(
            framework="hermes-agent",
            version=self._version_from_result(version_result),
            observed_at=observed_at or utc_now_iso(),
            capabilities=sorted_capabilities(capabilities),
            evidence=tuple(evidence),
            source_summary=source_summary,
            coverage_gaps=tuple(source_gaps),
        )

    def _source_evidence(self) -> tuple[list[dict[str, Any]], str, dict[str, Any], list[dict[str, Any]]]:
        if self.source_root is None:
            return [], "", {"source_root": None, "source_files_observed": 0}, [
                {"kind": "missing_source_root", "reason": "--source-root was not provided"}
            ]
        root = self.source_root
        if not root.exists():
            return [], "", {"source_root": str(root), "source_files_observed": 0}, [
                {"kind": "missing_source_root", "path": str(root)}
            ]

        evidence: list[dict[str, Any]] = []
        text_parts: list[str] = []
        observed_files: list[str] = []
        for relative in ("README.md", "pyproject.toml", "package.json"):
            path = root / relative
            if not path.exists():
                continue
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError as exc:
                evidence.append({"kind": "source_file", "path": str(path), "status": "blocked", "error": str(exc)})
                continue
            observed_files.append(relative)
            text_parts.append(text)
            evidence.append(
                {
                    "kind": "source_file",
                    "path": str(path),
                    "status": "observed",
                    "text_excerpt": text[:1000],
                }
            )
        top_dirs = sorted(
            item.name
            for item in root.iterdir()
            if item.is_dir() and not item.name.startswith(".") and item.name not in {"node_modules", "__pycache__"}
        )[:200]
        text_parts.append(" ".join(top_dirs))
        summary = {
            "source_root": str(root),
            "source_files_observed": len(observed_files),
            "observed_files": observed_files,
            "top_level_directories": top_dirs,
            "pyproject_extras": self._pyproject_extras(root / "pyproject.toml"),
        }
        return evidence, "\n".join(text_parts), summary, []

    def _add_source_capabilities(
        self,
        capabilities: list[AgentFrameworkCapability],
        text: str,
        source_summary: dict[str, Any],
    ) -> None:
        dirs = set(source_summary.get("top_level_directories", []))
        extras = set(source_summary.get("pyproject_extras", []))
        rules = (
            ("terminal_cli", ("tui", "slash-command", "prompt_toolkit"), ("repo_read", "external_process")),
            ("model_provider_routing", ("openrouter", "openai", "providers", "hermes model"), ("network",)),
            ("messaging_gateway", ("telegram", "discord", "slack", "whatsapp", "signal", "gateway"), ("network",)),
            ("closed_learning_loop", ("learning loop", "skills self-improve", "honcho", "hindsight"), ("repo_read",)),
            ("skill_lifecycle", ("skills", "agentskills", "skill creation"), ("repo_read", "repo_write_requires_claim")),
            ("scheduled_automation", ("cron", "scheduled", "croniter"), ("external_process",)),
            ("subagent_delegation", ("subagents", "parallel workstreams", "rpc"), ("external_process",)),
            ("terminal_backend_multiplexing", ("docker", "ssh", "singularity", "modal", "daytona"), ("external_process", "network")),
            ("mcp_tooling", ("mcp", "optional-mcps"), ("external_process", "network")),
            ("acp_adapter", ("acp", "agent-client-protocol"), ("external_process",)),
            ("web_dashboard", ("dashboard", "fastapi", "web"), ("network",)),
            ("browser_computer_use", ("browser", "computer-use", "agent-browser"), ("external_process", "network")),
            ("voice_io", ("voice", "tts", "stt", "whisper", "edge-tts"), ("network",)),
            ("research_trajectory_generation", ("batch trajectory", "trajectory compression", "batch_runner"), ("external_process",)),
        )
        for capability_id, needles, side_effects in rules:
            if any(needle in text for needle in needles) or capability_id.replace("_", "-") in dirs or capability_id in extras:
                self._add_capability(
                    capabilities,
                    capability_id,
                    ("cynic.agent_task.v1",),
                    ("cynic.agent_task_result.v1",),
                    side_effects,
                    "declared" if source_summary.get("source_files_observed") else "inferred",
                )

    @staticmethod
    def _add_capability(
        capabilities: list[AgentFrameworkCapability],
        capability_id: str,
        input_schemas: tuple[str, ...],
        output_schemas: tuple[str, ...],
        side_effects: tuple[str, ...],
        confidence: str,
    ) -> None:
        if any(item.id == capability_id for item in capabilities):
            return
        capabilities.append(
            AgentFrameworkCapability(
                id=capability_id,
                input_schemas=input_schemas,
                output_schemas=output_schemas,
                side_effects=side_effects,
                confidence=confidence,
            )
        )

    @staticmethod
    def _pyproject_extras(path: Path) -> list[str]:
        if not path.exists():
            return []
        try:
            payload = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError):
            return []
        project = payload.get("project", {})
        optional = project.get("optional-dependencies", {}) if isinstance(project, dict) else {}
        return sorted(str(key) for key in optional) if isinstance(optional, dict) else []

    def _skill_evidence(self) -> tuple[dict[str, Any], ...]:
        evidence: list[dict[str, Any]] = []
        for root in self.skill_roots:
            if not root.exists():
                continue
            for path in sorted(root.rglob("SKILL.md"))[:50]:
                try:
                    text = path.read_text(encoding="utf-8", errors="replace")
                except OSError as exc:
                    evidence.append({"kind": "skill", "path": str(path), "status": "blocked", "error": str(exc)})
                    continue
                evidence.append(
                    {
                        "kind": "skill",
                        "path": str(path),
                        "status": "observed",
                        "text_excerpt": text[:1000],
                    }
                )
        return tuple(evidence)

    @staticmethod
    def _mentions_repo_review(text: str) -> bool:
        repo_terms = ("repo", "repository", "git")
        review_terms = ("review", "audit", "diff", "pull request")
        return any(term in text for term in repo_terms) and any(term in text for term in review_terms)

    @staticmethod
    def _version_from_result(result: CommandResult) -> str:
        if not result.ok:
            return "unknown"
        for line in (result.stdout + "\n" + result.stderr).splitlines():
            value = line.strip()
            if value:
                return value
        return "unknown"

    @staticmethod
    def _command_evidence(name: str, result: CommandResult) -> dict[str, Any]:
        return {
            "kind": "command",
            "name": name,
            "argv": list(result.argv),
            "returncode": result.returncode,
            "status": "observed" if result.ok else "blocked",
            "stdout": result.stdout[:4000],
            "stderr": result.stderr[:4000],
        }


def default_skill_roots() -> tuple[Path, ...]:
    raw = os.environ.get("HERMES_AGENT_SKILL_ROOTS", "").strip()
    if raw:
        return tuple(Path(item).expanduser() for item in raw.split(os.pathsep) if item)
    return (Path("~/.cynic/organs/hermes").expanduser(),)
