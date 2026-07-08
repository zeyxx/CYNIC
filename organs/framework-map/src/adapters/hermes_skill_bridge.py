"""Inventory and bridge Hermes skills with repo-local CYNIC skills."""

from __future__ import annotations

import ast
from pathlib import Path
import re
from typing import Any

SKIP_REPO_PARTS = frozenset(
    {
        ".git",
        ".venv",
        "node_modules",
        "__pycache__",
        ".pytest_cache",
        ".cynic",
        "external",
        "target",
        "tmp",
    }
)


class HermesSkillBridgeDiscovery:
    def discover(self, *, repo_root: Path, hermes_skill_root: Path | None = None) -> dict[str, Any]:
        root = hermes_skill_root or (Path.home() / ".hermes" / "skills")
        hermes_skills = _scan_skill_root(root, source="hermes_local")
        repo_skills = _scan_repo_skills(repo_root)
        topic_overlap = sorted(_topic_set(hermes_skills) & _topic_set(repo_skills))
        import_candidates = _match_candidates(hermes_skills, repo_skills, mode="import")
        export_candidates = _match_candidates(repo_skills, hermes_skills, mode="export")
        repo_operational = [skill for skill in repo_skills if skill.get("path_class") == "organ_runtime"]
        repo_artifacts = [skill for skill in repo_skills if skill.get("path_class") != "organ_runtime"]

        return {
            "schema": "cynic.hermes_skill_bridge.v1",
            "hermes_skill_root": str(root),
            "repo_root": str(repo_root),
            "hermes_skill_count": len(hermes_skills),
            "repo_skill_count": len(repo_skills),
            "repo_operational_skill_count": len(repo_operational),
            "repo_artifact_skill_count": len(repo_artifacts),
            "shared_topic_count": len(topic_overlap),
            "shared_topics": topic_overlap,
            "hermes_skills": hermes_skills,
            "repo_skills": repo_skills,
            "import_candidates": import_candidates,
            "export_candidates": export_candidates,
            "gaps": _gaps(repo_operational, repo_artifacts, topic_overlap, import_candidates, export_candidates),
        }


def _scan_skill_root(root: Path, *, source: str) -> list[dict[str, Any]]:
    if not root.exists():
        return []
    records: list[dict[str, Any]] = []
    for path in sorted(root.rglob("SKILL.md")):
        records.append(_skill_record(path, root=root, source=source))
    return records


def _scan_repo_skills(repo_root: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in sorted(repo_root.rglob("SKILL.md")):
        if any(part in SKIP_REPO_PARTS for part in path.parts):
            continue
        records.append(_skill_record(path, root=repo_root, source="repo"))
    return records


def _skill_record(path: Path, *, root: Path, source: str) -> dict[str, Any]:
    text = path.read_text(encoding="utf-8", errors="replace")
    frontmatter, body = _split_frontmatter(text)
    tags = _normalize_tags(frontmatter.get("tags"))
    name = str(frontmatter.get("name") or path.parent.name).strip()
    description = str(frontmatter.get("description") or _first_heading_or_line(body)).strip()
    relative = str(path.relative_to(root))
    topics = sorted(set(tags) | set(_keywords(name, description, body)))
    return {
        "name": name,
        "description": description,
        "tags": tags,
        "topics": topics,
        "source": source,
        "path": str(path),
        "relative_path": relative,
        "path_class": _path_class(relative, source),
    }


def _split_frontmatter(text: str) -> tuple[dict[str, Any], str]:
    if not text.startswith("---\n"):
        return {}, text
    end = text.find("\n---\n", 4)
    if end == -1:
        return {}, text
    header = text[4:end]
    body = text[end + 5 :]
    payload: dict[str, Any] = {}
    for line in header.splitlines():
        if ":" not in line:
            continue
        key, raw = line.split(":", 1)
        key = key.strip()
        value = raw.strip()
        payload[key] = _parse_frontmatter_value(value)
    return payload, body


def _parse_frontmatter_value(value: str) -> Any:
    if not value:
        return ""
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        if "'" not in inner and '"' not in inner:
            return [item.strip() for item in inner.split(",") if item.strip()]
        try:
            parsed = ast.literal_eval(value)
        except Exception:
            return value
        return parsed
    return value.strip("\"'")


def _normalize_tags(value: Any) -> list[str]:
    if isinstance(value, list):
        return sorted({str(item).strip().lower() for item in value if str(item).strip()})
    if isinstance(value, str) and value:
        if "," in value:
            return sorted({item.strip().lower() for item in value.split(",") if item.strip()})
        return [value.strip().lower()]
    return []


def _first_heading_or_line(body: str) -> str:
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("#"):
            return stripped.lstrip("#").strip()
        return stripped
    return ""


def _keywords(*parts: str) -> list[str]:
    text = " ".join(parts).lower()
    found: set[str] = set()
    for keyword in (
        "cynic",
        "hermes",
        "refactoring",
        "refactor",
        "makefile",
        "transmutation",
        "architecture",
        "hexagonal",
        "documentation",
        "automation",
        "audit",
        "solana",
        "submission",
        "twitter",
        "x",
        "browser",
        "kanban",
        "memory",
        "gateway",
        "compliance",
        "python",
    ):
        if re.search(rf"\b{re.escape(keyword)}\b", text):
            found.add(keyword)
    return sorted(found)


def _path_class(relative: str, source: str) -> str:
    if source != "repo":
        return "hermes_local"
    if relative.startswith("organs/"):
        return "organ_runtime"
    if relative.startswith("data/") or "/build_artifacts/" in relative:
        return "artifact"
    if relative.startswith("docs/"):
        return "documentation"
    return "repo_misc"


def _topic_set(skills: list[dict[str, Any]]) -> set[str]:
    topics: set[str] = set()
    for skill in skills:
        topics.update(str(topic) for topic in skill.get("topics", []))
    return topics


def _match_candidates(
    left: list[dict[str, Any]],
    right: list[dict[str, Any]],
    *,
    mode: str,
) -> list[dict[str, Any]]:
    candidates: list[dict[str, Any]] = []
    for left_item in left:
        left_topics = set(left_item.get("topics", []))
        if not left_topics:
            continue
        for right_item in right:
            overlap = sorted(left_topics & set(right_item.get("topics", [])))
            if len(overlap) < 2:
                continue
            candidates.append(
                {
                    "mode": mode,
                    "from": left_item["relative_path"],
                    "to": right_item["relative_path"],
                    "shared_topics": overlap,
                }
            )
    return sorted(candidates, key=lambda item: (-len(item["shared_topics"]), item["from"], item["to"]))[:20]


def _gaps(
    repo_operational: list[dict[str, Any]],
    repo_artifacts: list[dict[str, Any]],
    topic_overlap: list[str],
    import_candidates: list[dict[str, Any]],
    export_candidates: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    gaps: list[dict[str, Any]] = []
    if not repo_operational:
        gaps.append(
            {
                "kind": "missing_repo_operational_skills",
                "reason": "The repo currently has no durable organ-runtime SKILL.md surfaces to bridge with Hermes.",
            }
        )
    if repo_artifacts:
        gaps.append(
            {
                "kind": "repo_skill_artifact_bias",
                "reason": "Observed repo skills are mostly build artifacts or emergence outputs rather than durable organ skills.",
                "count": len(repo_artifacts),
            }
        )
    if not topic_overlap:
        gaps.append(
            {
                "kind": "no_shared_topics",
                "reason": "Hermes local skills and repo-local skills currently have no meaningful thematic overlap.",
            }
        )
    if not import_candidates:
        gaps.append(
            {
                "kind": "no_import_candidates",
                "reason": "No strong repo-to-Hermes skill import candidates were detected from topic overlap.",
            }
        )
    if not export_candidates:
        gaps.append(
            {
                "kind": "no_export_candidates",
                "reason": "No strong Hermes-to-repo skill export candidates were detected from topic overlap.",
            }
        )
    return gaps
