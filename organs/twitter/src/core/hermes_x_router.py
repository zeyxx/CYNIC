"""Hermes X component routing contract for organ-twitter."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class HermesXComponent:
    name: str
    script: Path
    extra_paths: tuple[Path, ...] = ()


def component_table(project_root: Path) -> dict[str, HermesXComponent]:
    """Build the organ-owned Hermes X compatibility routing table."""
    entrypoints = project_root / "organs" / "twitter" / "src" / "entrypoints"
    adapters = project_root / "organs" / "twitter" / "src" / "adapters"
    cynic_python = project_root / "services" / "cynic-python"

    return {
        "x-ingest": HermesXComponent("x-ingest", entrypoints / "x_ingest.py"),
        "x-login": HermesXComponent("x-login", entrypoints / "hermes_x_login.py"),
        "feedback-loop": HermesXComponent(
            "feedback-loop",
            entrypoints / "gemini_learn_from_verdicts.py",
        ),
        "gemini-briefing": HermesXComponent(
            "gemini-briefing",
            entrypoints / "gemini_briefing_consumer.py",
        ),
        "task-runner": HermesXComponent("task-runner", entrypoints / "hermes_task_runner.py"),
        "k15-consumer": HermesXComponent(
            "k15-consumer",
            entrypoints / "k15_observation_consumer.py",
            extra_paths=(cynic_python / "heuristics", cynic_python),
        ),
        "behavior-logger": HermesXComponent("behavior-logger", entrypoints / "behavior_logger.py"),
        "browser-hub": HermesXComponent("browser-hub", adapters / "browser_hub.py"),
        "search-executor": HermesXComponent("search-executor", adapters / "search_executor.py"),
        "search-generator": HermesXComponent("search-generator", entrypoints / "search_generation.py"),
        "curation": HermesXComponent("curation", entrypoints / "curate_domain_signals.py"),
        "navigator": HermesXComponent("navigator", entrypoints / "organic_navigator.py"),
        "x-recovery": HermesXComponent("x-recovery", entrypoints / "hermes_x_recovery.py"),
        "x-poster": HermesXComponent("x-poster", entrypoints / "x_poster.py"),
        "k15-emitter": HermesXComponent("k15-emitter", adapters / "k15_emitter.py"),
    }


def resolve_component(
    components: dict[str, HermesXComponent],
    name: str,
) -> HermesXComponent:
    """Resolve and verify a Hermes X component from a routing table."""
    try:
        component = components[name]
    except KeyError as exc:
        known = ", ".join(sorted(components))
        raise ValueError(f"unknown Hermes X component {name!r}; known: {known}") from exc
    if not component.script.exists():
        raise FileNotFoundError(f"Hermes X component script missing: {component.script}")
    return component
