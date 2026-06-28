#!/usr/bin/env python3
"""Generate the CYNIC document theme inventory.

The inventory is intentionally deterministic and committed as JSON so cortex
sessions can review document drift without creating another markdown backlog.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
from pathlib import Path
from typing import Any


WORKSPACE_ROOT = Path(__file__).resolve().parents[1]
THEME_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/organism-theme-topology-v1.json"
INVENTORY_PATH = WORKSPACE_ROOT / "axioms/document-theme-inventory-v1.json"

SENSITIVE_ROOTS = [
    "docs/internal/",
    "docs/hackathon/",
    "docs/research/",
    "docs/reference/",
    "data/reports/",
    "packages/",
]

THEME_KEYWORDS = {
    "talaria_legal_genesis": [
        "talaria",
        "sas",
        "legal",
        "mica",
        "ownership",
        "cofounder",
        "caplogy",
        "facture",
        "associ",
    ],
    "cynic_protocol_framework": [
        "cynic",
        "token",
        "protocol",
        "e-score",
        "escore",
        "revenue",
        "waterfall",
        "fair-launch",
        "vivatech",
        "onepager",
    ],
    "odysseum_futardio_legacy": [
        "odysseum",
        "futard",
        "futardio",
        "incubator",
        "submission",
        "pitch-bc",
    ],
    "organism_runtime_topology": [
        "topology",
        "architecture",
        "organ",
        "organism",
        "hermes",
        "mcp",
        "runtime",
        "infrastructure",
        "wireguard",
        "agent",
        "dispatch",
        "soma",
    ],
    "data_and_evidence": [
        "data",
        "dataset",
        "report",
        "audit",
        "benchmark",
        "calibration",
        "research",
        "analysis",
        "ground-truth",
        "ground_truth",
    ],
}

DEFAULT_THEME_BY_ROOT = {
    "data/reports/": "data_and_evidence",
    "docs/research/": "data_and_evidence",
    "docs/reference/": "organism_runtime_topology",
    "docs/hackathon/": "cynic_protocol_framework",
    "docs/internal/": "cynic_protocol_framework",
    "packages/": "cynic_protocol_framework",
}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def git_files() -> list[str]:
    command = ["git", "-C", str(WORKSPACE_ROOT), "ls-files", "--cached", "--others", "--exclude-standard"]
    output = subprocess.check_output(command, text=True)
    return sorted(line for line in output.splitlines() if line)


def is_sensitive_doc(path: str) -> bool:
    if not any(path.startswith(root) for root in SENSITIVE_ROOTS):
        return False
    return path.endswith((".md", ".html", ".pdf", ".json", ".jsonl"))


def match_any(path: str, patterns: list[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) or path.startswith(pattern) for pattern in patterns)


def canonical_sources_by_theme(theme_contract: dict[str, Any]) -> dict[str, set[str]]:
    return {
        str(theme["id"]): set(str(item) for item in theme.get("canonical_sources", []))
        for theme in theme_contract.get("themes", [])
    }


def projection_patterns_by_theme(theme_contract: dict[str, Any]) -> dict[str, list[str]]:
    return {
        str(theme["id"]): [str(item) for item in theme.get("allowed_projection_paths", [])]
        for theme in theme_contract.get("themes", [])
    }


def score_theme(path: str) -> str:
    haystack = path.lower().replace("_", "-")
    scores: dict[str, int] = {}
    for theme, keywords in THEME_KEYWORDS.items():
        scores[theme] = sum(1 for keyword in keywords if keyword in haystack)
    if max(scores.values(), default=0) > 0:
        return sorted(scores.items(), key=lambda item: (-item[1], item[0]))[0][0]
    for root, theme in DEFAULT_THEME_BY_ROOT.items():
        if path.startswith(root):
            return theme
    return "data_and_evidence"


def classify_status(path: str, theme: str, canonical_sources: dict[str, set[str]], projection_patterns: dict[str, list[str]]) -> str:
    if path in canonical_sources.get(theme, set()):
        return "canonical"
    if path.startswith("data/reports/") or path.endswith((".json", ".jsonl", ".pdf")):
        return "evidence"
    if path.startswith(("docs/internal/", "docs/hackathon/")):
        return "draft"
    if path.startswith(("docs/research/", "docs/reference/")):
        return "evidence"
    if match_any(path, projection_patterns.get(theme, [])):
        return "projection"
    if path.startswith("packages/"):
        return "projection"
    return "draft"


def canonical_target(theme: str, theme_contract: dict[str, Any]) -> str | None:
    for item in theme_contract.get("themes", []):
        if item.get("id") != theme:
            continue
        sources = item.get("canonical_sources", [])
        return str(sources[0]) if sources else None
    return None


def build_inventory() -> dict[str, Any]:
    theme_contract = load_json(THEME_CONTRACT_PATH)
    canonical_sources = canonical_sources_by_theme(theme_contract)
    projection_patterns = projection_patterns_by_theme(theme_contract)
    files = [path for path in git_files() if is_sensitive_doc(path)]

    documents = []
    totals_by_theme: dict[str, int] = {}
    totals_by_status: dict[str, int] = {}
    for path in files:
        theme = score_theme(path)
        status = classify_status(path, theme, canonical_sources, projection_patterns)
        totals_by_theme[theme] = totals_by_theme.get(theme, 0) + 1
        totals_by_status[status] = totals_by_status.get(status, 0) + 1
        documents.append(
            {
                "path": path,
                "theme": theme,
                "status": status,
                "canonical_target": canonical_target(theme, theme_contract),
            }
        )

    return {
        "schema": "cynic.document_theme_inventory.v1",
        "status": "draft",
        "purpose": "Machine-readable classification of sensitive scattered docs by organism theme. This is an inventory, not a policy source.",
        "theme_contract": "axioms/organism-theme-topology-v1.json",
        "sensitive_roots": SENSITIVE_ROOTS,
        "allowed_statuses": ["canonical", "projection", "evidence", "draft", "archive", "delete_candidate"],
        "totals": {
            "documents": len(documents),
            "by_theme": dict(sorted(totals_by_theme.items())),
            "by_status": dict(sorted(totals_by_status.items())),
        },
        "documents": documents,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help=f"Write {INVENTORY_PATH.relative_to(WORKSPACE_ROOT)}")
    args = parser.parse_args()

    payload = build_inventory()
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.write:
        INVENTORY_PATH.write_text(text, encoding="utf-8")
    else:
        print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
