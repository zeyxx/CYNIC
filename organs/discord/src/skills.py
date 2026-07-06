"""Superteam skill catalog and reality-anchored CYNIC capability assessment."""
from __future__ import annotations

import json
import os
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable
from urllib import error, request

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, kernel_url as build_kernel_url
from organs.contracts.src.surface_contracts import json_loads_or_default


def _resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in [start, *start.parents]:
        if (candidate / "infra").is_dir() and (candidate / "organs").is_dir() and (candidate / ".git").exists():
            return candidate

    return Path(__file__).resolve().parent


REPO_ROOT = _resolve_project_root()

SUPERTEAM_SKILLS: dict[str, tuple[str, ...]] = {
    "Frontend": ("All", "React", "Svelte", "Angular", "Vue", "SolidJS", "Redux", "Elm", "Other"),
    "Backend": (
        "Javascript",
        "Typescript",
        "Node.js",
        "PHP",
        "Laravel",
        "Python",
        "Django",
        "Kotlin",
        "Swift",
        "Java",
        "C++",
        "C",
        "Ruby",
        "Ruby on Rails",
        "Go",
        "MySQL",
        "Postgres",
        "MongoDB",
        "Pearl",
        "Scala",
        "Elixir",
        "Haskell",
        "Erlang",
        "Deno",
        "Dart",
        "ASP.NET",
    ),
    "Blockchain": ("All", "Rust", "Solidity", "Move", "Other"),
    "Mobile": ("All", "Android", "iOS", "Flutter", "React Native", "Other"),
    "Design": (
        "UI/UX Design",
        "Graphic Design",
        "Illustration",
        "Game Design",
        "Presentation Design",
    ),
    "Community": ("All", "Community Manager", "Discord Moderator", "Other"),
    "Growth": ("All", "Business Development", "Digital Marketing", "Marketing", "Other"),
    "Content": ("All", "Research", "Photography", "Video", "Video Editing", "Writing", "Social Media", "Other"),
    "Other": ("All", "Data Analytics", "Operations", "Product Feedback", "Product Manager"),
}

LEVEL_LABELS = {
    0: "absent",
    1: "observed",
    2: "prototype",
    3: "operational",
    4: "autonomous",
    5: "judge-confirmed",
}


@dataclass(frozen=True)
class EvidenceProbe:
    kind: str
    path: str
    note: str


@dataclass(frozen=True)
class SkillAssessment:
    category: str
    skill: str
    level: int
    confidence: float
    status: str
    evidence: tuple[dict[str, str], ...]
    judge_verdict: dict[str, object] | None = None


REALITY_PROBES: dict[tuple[str, str], tuple[EvidenceProbe, ...]] = {
    ("Backend", "Python"): (
        EvidenceProbe("code", "organs/discord/src/passive_proxy.py", "passive Discord/Superteam capture"),
        EvidenceProbe("code", "organs/telegram/src/pipeline.py", "Telegram signal pipeline"),
        EvidenceProbe("code", "services/cynic-python/sensors/spike_judge_consumer.py", "kernel judge consumer"),
        EvidenceProbe("service", "infra/systemd/discord-passive-proxy.service", "persistent capture service"),
        EvidenceProbe("service", "infra/systemd/discord-pipeline.service", "scheduled processing"),
    ),
    ("Backend", "Javascript"): (
        EvidenceProbe("code", "infra/hermes-extension/background.js", "browser extension runtime"),
        EvidenceProbe("package", "package.json", "root JS workspace"),
    ),
    ("Backend", "Typescript"): (
        EvidenceProbe("code", "crates/percolator-cli/src/v16/instructions.ts", "Solana instruction client"),
        EvidenceProbe("test", "crates/percolator-cli/tests/preflight.ts", "integration test harness"),
        EvidenceProbe("package", "crates/percolator-cli/package.json", "typed package"),
    ),
    ("Backend", "Node.js"): (
        EvidenceProbe("package", "package.json", "Node workspace"),
        EvidenceProbe("package", "crates/percolator-cli/package.json", "Node CLI package"),
        EvidenceProbe("test", "crates/percolator-cli/tests/runner.ts", "test runner"),
    ),
    ("Backend", "Go"): (
        EvidenceProbe("code", "crates/mcp-coord/main.go", "MCP coord bridge"),
        EvidenceProbe("package", "crates/mcp-coord/go.mod", "Go module"),
    ),
    ("Backend", "Postgres"): (
        EvidenceProbe("runtime", "organs/docker/state/state.json", "observed postgres container outside CYNIC"),
    ),
    ("Backend", "MongoDB"): (
        EvidenceProbe("runtime", "organs/docker/state/state.json", "observed mongo container outside CYNIC"),
    ),
    ("Frontend", "React"): (
        EvidenceProbe("package", "packages/cynic-ui/package.json", "React package"),
        EvidenceProbe("test", "packages/cynic-ui/tests/render.spec.ts", "frontend render test"),
        EvidenceProbe("package", "packages/talaria-demo/package.json", "React demo package"),
    ),
    ("Frontend", "Redux"): (
        EvidenceProbe("package", "package.json", "frontend dependency surface"),
    ),
    ("Blockchain", "Rust"): (
        EvidenceProbe("code", "crates/cynic-verdict-program/src/lib.rs", "on-chain verdict program"),
        EvidenceProbe("code", "crates/cynic-kernel/src/judge/mod.rs", "kernel judge engine"),
        EvidenceProbe("test", "crates/cynic-kernel/tests/integration_judge.rs", "judge integration tests"),
        EvidenceProbe("service", "infra/systemd/cynic-kernel.service", "kernel runtime service"),
    ),
    ("Blockchain", "Solidity"): (),
    ("Blockchain", "Move"): (),
    ("Mobile", "Android"): (),
    ("Mobile", "iOS"): (),
    ("Mobile", "Flutter"): (),
    ("Mobile", "React Native"): (),
    ("Design", "UI/UX Design"): (
        EvidenceProbe("code", "organs/discord/src/ui.py", "local opportunity and skill UI"),
        EvidenceProbe("package", "packages/cynic-ui/package.json", "frontend package"),
    ),
    ("Design", "Presentation Design"): (
        EvidenceProbe("artifact", "docs/hackathon/HACKATHON_SUBMISSION.md", "pitch/submission artifact"),
    ),
    ("Community", "Community Manager"): (
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "community signal extraction"),
        EvidenceProbe("service", "infra/systemd/discord-pipeline.service", "automated community ingestion"),
    ),
    ("Community", "Discord Moderator"): (
        EvidenceProbe("code", "organs/discord/src/passive_proxy.py", "passive Discord observation only"),
    ),
    ("Growth", "Business Development"): (
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "opportunity extraction"),
        EvidenceProbe("runtime", "organs/discord/src/ui.py", "bounty review queue"),
    ),
    ("Growth", "Digital Marketing"): (
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "content/growth classification"),
    ),
    ("Growth", "Marketing"): (
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "content/growth classification"),
    ),
    ("Content", "Research"): (
        EvidenceProbe("artifact", "docs/architecture/CCM-V2-ARCHITECTURE.md", "technical research artifact"),
        EvidenceProbe("artifact", "docs/research/SESSION-2026-03-25-L6L7-RESEARCH.md", "research notes"),
    ),
    ("Content", "Writing"): (
        EvidenceProbe("artifact", "docs/hackathon/HACKATHON_SUBMISSION.md", "public submission writing"),
        EvidenceProbe("artifact", "CHANGELOG.md", "release writing"),
    ),
    ("Content", "Social Media"): (
        EvidenceProbe("code", "organs/twitter/src/entrypoints/search_generation.py", "social search generation"),
        EvidenceProbe("code", "organs/twitter/src/entrypoints/x_ingest.py", "X ingestion daemon"),
    ),
    ("Content", "Video"): (),
    ("Content", "Video Editing"): (),
    ("Other", "Data Analytics"): (
        EvidenceProbe("code", "organs/discord/src/schema.py", "SQLite model"),
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "signal analysis"),
        EvidenceProbe("code", "services/cynic-python/heuristics/experiments/ab-test-enrichment/ab_test_divergence_enrichment.py", "measurement experiment"),
    ),
    ("Other", "Operations"): (
        EvidenceProbe("service", "infra/systemd/cynic-kernel.service", "kernel service"),
        EvidenceProbe("service", "infra/systemd/discord-passive-proxy.service", "capture service"),
        EvidenceProbe("code", "Makefile", "sovereign gate targets"),
    ),
    ("Other", "Product Feedback"): (
        EvidenceProbe("code", "organs/discord/src/ui.py", "review queue UI"),
        EvidenceProbe("code", "organs/discord/src/passive_proxy.py", "Superteam listing capture"),
        EvidenceProbe("runtime", "organs/discord/src/schema.py", "opportunity status/notes schema"),
    ),
    ("Other", "Product Manager"): (
        EvidenceProbe("code", "organs/discord/src/pipeline.py", "opportunity scoring"),
        EvidenceProbe("runtime", "organs/discord/src/ui.py", "decision queue"),
    ),
}

FALLBACK_PROBES: dict[str, tuple[EvidenceProbe, ...]] = {
    "Frontend": (),
    "Backend": (),
    "Blockchain": (),
    "Mobile": (),
    "Design": (),
    "Community": (),
    "Growth": (),
    "Content": (),
    "Other": (),
}


def catalog_rows() -> Iterable[tuple[str, str]]:
    for category, skills in SUPERTEAM_SKILLS.items():
        for skill in skills:
            yield category, skill


def probe_exists(probe: EvidenceProbe) -> bool:
    path = REPO_ROOT / probe.path
    return path.exists()


def materialize_evidence(probes: Iterable[EvidenceProbe]) -> tuple[dict[str, str], ...]:
    rows = []
    for probe in probes:
        if probe_exists(probe):
            rows.append({"kind": probe.kind, "path": probe.path, "note": probe.note})
    return tuple(rows)


def level_from_evidence(evidence: tuple[dict[str, str], ...]) -> tuple[int, float]:
    kinds = {row["kind"] for row in evidence}
    code_count = sum(1 for row in evidence if row["kind"] == "code")
    test_count = sum(1 for row in evidence if row["kind"] == "test")
    service_count = sum(1 for row in evidence if row["kind"] == "service")
    package_count = sum(1 for row in evidence if row["kind"] == "package")
    runtime_count = sum(1 for row in evidence if row["kind"] == "runtime")

    if not evidence:
        return 0, 0.0
    if kinds <= {"artifact"}:
        return 2, min(0.38 + 0.06 * len(evidence), 0.50)

    level = 1
    confidence = 0.20
    if code_count or package_count:
        level = 2
        confidence = 0.34 + 0.06 * min(code_count + package_count, 3)
    if (code_count and (test_count or service_count or runtime_count)) or (package_count and test_count):
        level = 3
        confidence = 0.52 + 0.05 * min(test_count + service_count + runtime_count, 3)
    if code_count >= 2 and service_count >= 1 and (test_count >= 1 or runtime_count >= 1):
        level = 4
        confidence = 0.70 + 0.04 * min(code_count + test_count + service_count + runtime_count, 4)

    return level, min(round(confidence, 2), 0.86)


def maybe_judge_skill(category: str, skill: str, evidence: tuple[dict[str, str], ...]) -> dict[str, object] | None:
    """Optionally ask the kernel to judge one skill. Disabled unless explicitly enabled."""
    if os.environ.get("CYNIC_SKILL_JUDGE", "0") != "1" or not evidence:
        return None
    kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    if not kernel_url:
        return None

    payload = {
        "domain": "capability-assessment",
        "content": json.dumps(
            {
                "question": "Assess whether CYNIC can credibly claim this Superteam skill.",
                "category": category,
                "skill": skill,
                "evidence": evidence,
                "scale": LEVEL_LABELS,
                "instruction": "Penalize docs-only evidence. Reward shipped code, tests, services, and runtime traces.",
            },
            separators=(",", ":"),
        ),
    }
    body = json.dumps(payload).encode("utf-8")
    req = request.Request(
        build_kernel_url(kernel_url, "/judge"),
        data=body,
        headers=kernel_auth_headers(os.environ.get("CYNIC_API_KEY", "")),
        method="POST",
    )
    try:
        with request.urlopen(req, timeout=20) as resp:
            payload = json_loads_or_default(resp.read().decode("utf-8"), None)
            return payload if isinstance(payload, dict) else None
    except (error.URLError, TimeoutError, json.JSONDecodeError, OSError):
        return None


def assess_skill(category: str, skill: str) -> SkillAssessment:
    if skill == "All":
        children = [assess_skill(category, s) for s in SUPERTEAM_SKILLS[category] if s != "All"]
        if not children:
            return SkillAssessment(category, skill, 0, 0.0, LEVEL_LABELS[0], ())
        level = max(row.level for row in children)
        confidence = max(row.confidence for row in children)
        evidence = tuple(dict.fromkeys(json.dumps(item, sort_keys=True) for row in children for item in row.evidence))
        restored = tuple(json_loads_or_default(item, {}) for item in evidence[:8])
        return SkillAssessment(category, skill, level, round(confidence, 2), LEVEL_LABELS[level], restored)

    probes = REALITY_PROBES.get((category, skill), FALLBACK_PROBES.get(category, ()))
    evidence = materialize_evidence(probes)
    level, confidence = level_from_evidence(evidence)
    judge_verdict = maybe_judge_skill(category, skill, evidence)
    return SkillAssessment(category, skill, level, confidence, LEVEL_LABELS[level], evidence, judge_verdict)


def bootstrap_skill_assessments(conn: sqlite3.Connection) -> None:
    """Persist the observed Superteam catalog and local CYNIC evidence levels."""
    now = datetime.now(timezone.utc).isoformat()
    for category, skill in catalog_rows():
        conn.execute(
            """INSERT INTO skill_catalog (category, skill, source, first_seen, last_seen)
               VALUES (?, ?, 'superteam.fun:earn:all-skills:2026-06-14', ?, ?)
               ON CONFLICT(category, skill) DO UPDATE SET
                 source=excluded.source,
                 last_seen=excluded.last_seen""",
            (category, skill, now, now),
        )

        assessment = assess_skill(category, skill)
        conn.execute(
            """INSERT INTO skill_assessments
               (category, skill, level, confidence, status, evidence_json, judge_verdict_json, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(category, skill) DO UPDATE SET
                 level=excluded.level,
                 confidence=excluded.confidence,
                 status=excluded.status,
                 evidence_json=excluded.evidence_json,
                 judge_verdict_json=COALESCE(excluded.judge_verdict_json, skill_assessments.judge_verdict_json),
                 updated_at=excluded.updated_at""",
            (
                assessment.category,
                assessment.skill,
                assessment.level,
                assessment.confidence,
                assessment.status,
                json.dumps(list(assessment.evidence), separators=(",", ":")),
                json.dumps(assessment.judge_verdict, separators=(",", ":")) if assessment.judge_verdict else None,
                now,
            ),
        )
    conn.commit()
