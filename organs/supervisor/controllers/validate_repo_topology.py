#!/usr/bin/env python3
"""Validate the CYNIC repository topology contract.

This is a narrow structural gate. It validates the machine-readable contract and
the safest invariants that can be checked before the historical tree is fully
migrated.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import yaml


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]
CONTRACT_PATH = WORKSPACE_ROOT / "axioms/repo-topology-v1.json"
THEME_CONTRACT_PATH = WORKSPACE_ROOT / "axioms/organism-theme-topology-v1.json"
DOCUMENT_THEME_INVENTORY_PATH = WORKSPACE_ROOT / "axioms/document-theme-inventory-v1.json"
CONTRACTS_ROOT = WORKSPACE_ROOT / "axioms"
MCP_SRC_ROOT = WORKSPACE_ROOT / "crates/cynic-mcp/src"
ORGANS_ROOT = WORKSPACE_ROOT / "organs"
RUNTIME_STATE_GLOBS = [
    "infra/organ-anvil/*.json",
    "infra/organ-anvil/*.jsonl",
    "organs/*/state/*.json",
    "organs/*/state/*.jsonl",
    "organs/*/state/**/*.json",
    "organs/*/state/**/*.jsonl",
]
PROVIDER_MUTATION_ROOTS = ["scripts", "infra/scripts", "organs/lifecycle/scripts"]
PROVIDER_MUTATION_SUFFIXES = (".py", ".sh")
TRANSITIONAL_WRAPPER_SUFFIXES = (".py", ".sh", ".ps1")
PROVIDER_MUTATION_RE = re.compile(
    r"\b("
    r"pct\s+(create|start|exec|stop|destroy|set)|"
    r"qm\s+(create|start|stop|destroy|set|clone|exec)|"
    r"provision_vm|ProxmoxAdapter|"
    r"/api2/json/nodes/[^\"'\s]+/(qemu|lxc)/[^\"'\s]+/(agent/exec|config|status)"
    r")\b",
    re.IGNORECASE,
)
TRANSITIONAL_PATH_REQUIRED_FIELDS = {
    "path",
    "owner",
    "replacement",
    "expiry_condition",
    "allowed_file_kinds",
    "forbidden_concerns",
}
MISPLACED_ORGAN_REQUIRED_FIELDS = {
    "path",
    "status",
    "owner",
    "target",
    "replacement",
    "expiry_condition",
}
LEGACY_HERMES_X_PATH_MARKER = "scripts/hermes-x"
LEGACY_HERMES_X_ROOT = Path("scripts") / "hermes-x"
LEGACY_HERMES_X_WRAPPER_MARKER = "from organs.twitter.src."
LEGACY_HERMES_X_FUNC_RE = re.compile(r"^\s*(def|class)\s+", re.MULTILINE)
INFRA_HERMES_X_REF_MARKER = "@CYNIC_PROJECT_ROOT@/scripts/hermes-x"
LEGACY_HERMES_X_REFERENCE_ALLOWED_GLOBS = {
    "axioms/repo-topology-v1.json",
    "docs/superpowers/**",
    "docs/foundation/CYNIC_DEPRECATED_SURFACE_REPORT*.md",
    "data/reports/**",
    ".cortex/plans/**",
    "organs/*/state/*.jsonl",
    "organs/*/state/**/*.jsonl",
    "organs/twitter/MANIFEST.yaml",
    "organs/browser/MANIFEST.yaml",
    "organs/twitter/src/entrypoints/hermes_x.py",
    "organs/supervisor/controllers/validate_repo_topology.py",
    "organs/supervisor/tests/test_validate_repo_topology.py",
}
LEGACY_HERMES_X_INFRA_SCAN_SUFFIXES = {
    ".conf",
    ".service",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}
LEGACY_HERMES_X_SYSTEMD_PREFIX = "hermes-x-"
LEGACY_HERMES_X_SYSTEMD_DEP_RE = re.compile(
    r"^\s*(After|Before|Requires|RequiresMountsFor|Wants|PartOf|WantedBy|Also)\s*=\s*(.+)$"
)
LEGACY_HERMES_X_UNIT_TOKEN_RE = re.compile(r"\bhermes-x-[A-Za-z0-9_.@%-]+(?:\\.service)?\b")
LEGACY_HERMES_X_DOMAIN_ALIAS_RE = re.compile(r"\bHERMES_AGENT_DOMAIN_ALLOW=.*\bhermes-x\b")
LEGACY_HERMES_X_SCAN_SUFFIXES = {
    ".json",
    ".jsonl",
    ".md",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}
DEPLOYMENT_PROJECTION_PATH_RE = re.compile(r"(?:/home/|/Users/|\\\\[A-Za-z]:\\|/Bureau/|%h/.*?/Bureau/)")
THEME_REQUIRED_FIELDS = {
    "id",
    "class",
    "owns",
    "canonical_sources",
    "allowed_projection_paths",
    "must_not_own",
    "drift_rule",
}
DOC_ZONE_REQUIRED_FIELDS = {
    "path",
    "class",
    "may_own",
    "must_not_own",
    "canonicality",
}
ROOT_CLASS_REQUIRED_FIELDS = {"path", "class", "owns", "must_not_own"}
DOCUMENT_INVENTORY_REQUIRED_FIELDS = {"path", "theme", "status", "canonical_target"}
DOCUMENT_INVENTORY_SENSITIVE_MARKDOWN_ROOTS = [
    "docs/internal/",
    "docs/hackathon/",
    "docs/research/",
    "docs/reference/",
    "data/reports/",
    "packages/",
]


@dataclass
class Finding:
    severity: str
    kind: str
    path: str
    detail: str


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_yaml(path: Path) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def relative(path: Path) -> str:
    try:
        return str(path.relative_to(WORKSPACE_ROOT))
    except ValueError:
        return str(path)


def git_ls_files(*pathspecs: str) -> list[str]:
    command = ["git", "-C", str(WORKSPACE_ROOT), "ls-files", *pathspecs]
    output = subprocess.check_output(command, text=True)
    return [line for line in output.splitlines() if line]


def git_ls_untracked(*pathspecs: str) -> list[str]:
    command = [
        "git",
        "-C",
        str(WORKSPACE_ROOT),
        "ls-files",
        "--others",
        "--exclude-standard",
        *pathspecs,
    ]
    output = subprocess.check_output(command, text=True)
    return [line for line in output.splitlines() if line]


def path_matches_any(path: str, patterns: set[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def validate_json_contracts() -> list[Finding]:
    findings: list[Finding] = []
    if not CONTRACTS_ROOT.exists():
        return [
            Finding(
                "critical",
                "contracts_root_missing",
                relative(CONTRACTS_ROOT),
                "axioms must exist because machine-readable contracts live there.",
            )
        ]

    for path in sorted(CONTRACTS_ROOT.rglob("*.json")):
        try:
            load_json(path)
        except json.JSONDecodeError as exc:
            findings.append(
                Finding(
                    "critical",
                    "invalid_json_contract",
                    relative(path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            )
    return findings


def validate_contract_file_policy(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    policy = contract.get("contract_file_policy", {})
    allowed_extensions = set(policy.get("allowed_extensions", [".json"]))
    extension_exceptions = set(policy.get("extension_exceptions", []))
    allowed_subdirectories = {
        str(item.get("path", "")).rstrip("/") + "/"
        for item in policy.get("allowed_subdirectories", [])
        if item.get("path")
    }
    required_fields = policy.get("json_required_top_level_fields", ["schema", "status"])
    allowed_statuses = set(policy.get("allowed_statuses", []))
    schema_prefix = policy.get("schema_prefix")

    if not CONTRACTS_ROOT.exists():
        return findings

    for item in policy.get("allowed_subdirectories", []):
        item_path = str(item.get("path", ""))
        for field in ("path", "class", "owner", "reason"):
            if not item.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "contract_subdirectory_metadata_missing",
                        item_path or "contract_file_policy.allowed_subdirectories",
                        f"allowed_subdirectories entries must declare {field}.",
                    )
                )

    for path in sorted(item for item in CONTRACTS_ROOT.iterdir() if item.is_dir()):
        rel = relative(path).rstrip("/") + "/"
        if rel not in allowed_subdirectories:
            findings.append(
                Finding(
                    "critical",
                    "contract_subdirectory_undeclared",
                    rel,
                    "Subdirectories under axioms must be declared in repo-topology-v1.contract_file_policy.allowed_subdirectories.",
                )
            )

    for path in sorted(item for item in CONTRACTS_ROOT.rglob("*") if item.is_file()):
        rel = relative(path)
        if rel in extension_exceptions:
            continue
        if path.suffix not in allowed_extensions:
            findings.append(
                Finding(
                    "critical",
                    "contract_file_extension_forbidden",
                    rel,
                    f"axioms files must use one of {sorted(allowed_extensions)} or declare an explicit extension exception.",
                )
            )

    for path in sorted(CONTRACTS_ROOT.rglob("*.json")):
        try:
            payload = load_json(path)
        except json.JSONDecodeError:
            continue
        rel = relative(path)
        for field in required_fields:
            if not payload.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "contract_required_field_missing",
                        rel,
                        f"JSON contracts under axioms must declare top-level {field!r}.",
                    )
                )
        status = payload.get("status")
        if allowed_statuses and status and status not in allowed_statuses:
            findings.append(
                Finding(
                    "critical",
                    "contract_status_unknown",
                    rel,
                    f"status {status!r} is not declared in repo-topology-v1.contract_file_policy.allowed_statuses.",
                )
            )
        schema = payload.get("schema")
        if schema_prefix and schema and not str(schema).startswith(str(schema_prefix)):
            findings.append(
                Finding(
                    "critical",
                    "contract_schema_prefix_invalid",
                    rel,
                    f"schema {schema!r} must start with {schema_prefix!r}.",
                )
            )
    return findings


def validate_topology_contract() -> list[Finding]:
    findings: list[Finding] = []
    if not CONTRACT_PATH.exists():
        return [
            Finding(
                "critical",
                "repo_topology_contract_missing",
                relative(CONTRACT_PATH),
                "repo-topology-v1.json is the machine contract for filesystem concern boundaries.",
            )
        ]

    contract = load_json(CONTRACT_PATH)
    if contract.get("schema") != "cynic.repo_topology.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_repo_topology_schema",
                relative(CONTRACT_PATH),
                f"expected cynic.repo_topology.v1, got {contract.get('schema')!r}",
            )
        )

    for name, value in sorted(contract.get("authorities", {}).items()):
        authority = WORKSPACE_ROOT / value
        if not authority.exists():
            findings.append(
                Finding(
                    "critical",
                    "topology_authority_missing",
                    relative(authority),
                    f"authority {name!r} is declared by repo-topology-v1 but does not exist.",
                )
            )

    declared_root_paths: set[str] = set()
    for root_class in contract.get("root_classes", []):
        path_value = root_class.get("path")
        if not path_value:
            findings.append(
                Finding(
                    "critical",
                    "root_class_path_missing",
                    relative(CONTRACT_PATH),
                    "root_classes entries must declare path.",
                )
            )
            continue
        declared_root_paths.add(str(path_value).rstrip("/") + "/")
        for field in sorted(ROOT_CLASS_REQUIRED_FIELDS - {"path"}):
            if not root_class.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "root_class_metadata_missing",
                        str(path_value),
                        f"root_classes entries must declare {field}.",
                    )
                )
        path = WORKSPACE_ROOT / path_value
        if not path.exists():
            findings.append(
                Finding(
                    "medium",
                    "declared_topology_root_missing",
                    relative(path),
                    f"declared root class {root_class.get('class')!r} does not exist yet.",
                )
            )

    policy = contract.get("top_level_root_policy", {})
    ignored_roots = set(policy.get("ignored_roots", []))
    if policy:
        for path in sorted(item for item in WORKSPACE_ROOT.iterdir() if item.is_dir()):
            name = path.name
            if name in ignored_roots:
                continue
            rel = f"{name}/"
            if rel not in declared_root_paths:
                findings.append(
                    Finding(
                        "critical",
                        "top_level_root_unclassified",
                        rel,
                        "Every top-level directory must be declared in repo-topology-v1.root_classes or ignored by top_level_root_policy.",
                    )
                )

    return findings


def validate_theme_topology_contract() -> list[Finding]:
    findings: list[Finding] = []
    if not THEME_CONTRACT_PATH.exists():
        return [
            Finding(
                "critical",
                "theme_topology_contract_missing",
                relative(THEME_CONTRACT_PATH),
                "organism-theme-topology-v1.json is the machine contract for thematic ownership.",
            )
        ]

    try:
        contract = load_json(THEME_CONTRACT_PATH)
    except json.JSONDecodeError as exc:
        return [
            Finding(
                "critical",
                "theme_topology_contract_invalid_json",
                relative(THEME_CONTRACT_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if contract.get("schema") != "cynic.organism_theme_topology.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_theme_topology_schema",
                relative(THEME_CONTRACT_PATH),
                f"expected cynic.organism_theme_topology.v1, got {contract.get('schema')!r}",
            )
        )

    for name, value in sorted(contract.get("authorities", {}).items()):
        authority = WORKSPACE_ROOT / value
        if not authority.exists():
            findings.append(
                Finding(
                    "critical",
                    "theme_topology_authority_missing",
                    relative(authority),
                    f"authority {name!r} is declared by organism-theme-topology-v1 but does not exist.",
                )
            )

    themes = contract.get("themes")
    if not isinstance(themes, list) or not themes:
        findings.append(
            Finding(
                "critical",
                "theme_topology_themes_missing",
                relative(THEME_CONTRACT_PATH),
                "organism-theme-topology-v1 must declare a non-empty themes list.",
            )
        )
        themes = []

    seen_theme_ids: set[str] = set()
    canonical_owner_by_path: dict[str, str] = {}
    shared_sources = set(contract.get("shared_canonical_sources", []))
    for index, theme in enumerate(themes):
        item_path = str(theme.get("id") or f"themes[{index}]")
        for field in sorted(THEME_REQUIRED_FIELDS):
            if not theme.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "theme_topology_theme_metadata_missing",
                        item_path,
                        f"themes entries must declare {field}.",
                    )
                )
        theme_id = str(theme.get("id", ""))
        if theme_id:
            if theme_id in seen_theme_ids:
                findings.append(
                    Finding(
                        "critical",
                        "theme_topology_duplicate_theme_id",
                        item_path,
                        f"theme id {theme_id!r} is declared more than once.",
                    )
                )
            seen_theme_ids.add(theme_id)

        for source in theme.get("canonical_sources", []):
            source_path = str(source)
            if not (WORKSPACE_ROOT / source_path).exists():
                findings.append(
                    Finding(
                        "critical",
                        "theme_topology_canonical_source_missing",
                        source_path,
                        f"canonical source declared by theme {theme_id!r} does not exist.",
                    )
                )
            previous_owner = canonical_owner_by_path.get(source_path)
            if previous_owner and source_path not in shared_sources:
                findings.append(
                    Finding(
                        "critical",
                        "theme_topology_canonical_source_ambiguous",
                        source_path,
                        f"canonical source is owned by both {previous_owner!r} and {theme_id!r}; add shared_canonical_sources only when the overlap is intentional.",
                    )
                )
            canonical_owner_by_path[source_path] = theme_id

    doc_zones = contract.get("doc_zones")
    if not isinstance(doc_zones, list) or not doc_zones:
        findings.append(
            Finding(
                "critical",
                "theme_topology_doc_zones_missing",
                relative(THEME_CONTRACT_PATH),
                "organism-theme-topology-v1 must declare a non-empty doc_zones list.",
            )
        )
        doc_zones = []

    for index, zone in enumerate(doc_zones):
        item_path = str(zone.get("path") or f"doc_zones[{index}]")
        for field in sorted(DOC_ZONE_REQUIRED_FIELDS):
            if not zone.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "theme_topology_doc_zone_metadata_missing",
                        item_path,
                        f"doc_zones entries must declare {field}.",
                    )
                )
        path_value = zone.get("path")
        if path_value and not (WORKSPACE_ROOT / str(path_value)).exists():
            findings.append(
                Finding(
                    "medium",
                    "theme_topology_doc_zone_missing",
                    str(path_value),
                    "Declared thematic document zone does not exist yet.",
                )
            )

    promotion = contract.get("promotion_rule")
    if not isinstance(promotion, dict) or not promotion.get("from") or not promotion.get("to") or not promotion.get("requires"):
        findings.append(
            Finding(
                "critical",
                "theme_topology_promotion_rule_incomplete",
                relative(THEME_CONTRACT_PATH),
                "promotion_rule must declare from, to and requires lists.",
            )
        )

    return findings


def validate_document_theme_inventory_contract() -> list[Finding]:
    findings: list[Finding] = []
    if not DOCUMENT_THEME_INVENTORY_PATH.exists():
        return [
            Finding(
                "critical",
                "document_theme_inventory_missing",
                relative(DOCUMENT_THEME_INVENTORY_PATH),
                "document-theme-inventory-v1.json must classify sensitive scattered docs by theme.",
            )
        ]

    try:
        inventory = load_json(DOCUMENT_THEME_INVENTORY_PATH)
    except json.JSONDecodeError as exc:
        return [
            Finding(
                "critical",
                "document_theme_inventory_invalid_json",
                relative(DOCUMENT_THEME_INVENTORY_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if inventory.get("schema") != "cynic.document_theme_inventory.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_document_theme_inventory_schema",
                relative(DOCUMENT_THEME_INVENTORY_PATH),
                f"expected cynic.document_theme_inventory.v1, got {inventory.get('schema')!r}",
            )
        )

    try:
        theme_contract = load_json(THEME_CONTRACT_PATH)
    except (json.JSONDecodeError, FileNotFoundError):
        theme_contract = {}
    theme_ids = {str(item.get("id")) for item in theme_contract.get("themes", []) if item.get("id")}
    allowed_statuses = set(inventory.get("allowed_statuses", []))
    documents = inventory.get("documents", [])
    if not isinstance(documents, list):
        findings.append(
            Finding(
                "critical",
                "document_theme_inventory_documents_not_list",
                relative(DOCUMENT_THEME_INVENTORY_PATH),
                "documents must be a list.",
            )
        )
        documents = []

    inventory_paths: set[str] = set()
    for index, item in enumerate(documents):
        item_path = str(item.get("path") or f"documents[{index}]")
        for field in sorted(DOCUMENT_INVENTORY_REQUIRED_FIELDS):
            if field not in item:
                findings.append(
                    Finding(
                        "critical",
                        "document_theme_inventory_metadata_missing",
                        item_path,
                        f"documents entries must declare {field}.",
                    )
                )
        path_value = item.get("path")
        if path_value:
            path = str(path_value)
            inventory_paths.add(path)
            if not (WORKSPACE_ROOT / path).exists():
                findings.append(
                    Finding(
                        "critical",
                        "document_theme_inventory_path_missing",
                        path,
                        "Inventory entry points to a file that does not exist.",
                    )
                )
        theme = item.get("theme")
        if theme_ids and theme and str(theme) not in theme_ids:
            findings.append(
                Finding(
                    "critical",
                    "document_theme_inventory_theme_unknown",
                    item_path,
                    f"theme {theme!r} is not declared in organism-theme-topology-v1.",
                )
            )
        status = item.get("status")
        if allowed_statuses and status and str(status) not in allowed_statuses:
            findings.append(
                Finding(
                    "critical",
                    "document_theme_inventory_status_unknown",
                    item_path,
                    f"status {status!r} is not declared in allowed_statuses.",
                )
            )

    try:
        visible_paths = sorted(set(git_ls_files()) | set(git_ls_untracked()))
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            Finding(
                "critical",
                "document_theme_inventory_scan_failed",
                ".",
                f"unable to query git visible files: {exc}",
            )
        ]

    for path in visible_paths:
        if not path.endswith(".md"):
            continue
        if not any(path.startswith(root) for root in DOCUMENT_INVENTORY_SENSITIVE_MARKDOWN_ROOTS):
            continue
        if path not in inventory_paths:
            findings.append(
                Finding(
                    "critical",
                    "sensitive_markdown_missing_theme_inventory",
                    path,
                    "Sensitive markdown files must be classified in document-theme-inventory-v1.json before they can enter the repo.",
                )
            )

    return findings


def validate_current_transitional_paths(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    declared = contract.get("current_transitional_paths", [])
    for item in declared:
        path_value = str(item.get("path", ""))
        for field in sorted(TRANSITIONAL_PATH_REQUIRED_FIELDS):
            if not item.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "transitional_path_metadata_missing",
                        path_value or "current_transitional_paths",
                        f"current_transitional_paths entries must declare {field}.",
                    )
                )
        if not path_value:
            continue
        path = WORKSPACE_ROOT / path_value
        if not path.exists():
            findings.append(
                Finding(
                    "medium",
                    "transitional_path_stale",
                    path_value,
                    "Declared transitional path does not exist; remove the exception or create the bounded replacement.",
                )
            )
    return findings


def transitional_wrapper_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path"))
        for item in contract.get("transitional_wrapper_file_exceptions", [])
        if item.get("path")
    }


def validate_transitional_wrapper_file_exceptions(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(
        validate_exception_metadata(
            contract,
            "transitional_wrapper_file_exceptions",
            {"path", "status", "owner", "classification", "replacement", "expiry_condition"},
        )
    )
    allowed = transitional_wrapper_exception_paths(contract)
    try:
        untracked = sorted(
            path
            for path in git_ls_untracked(*PROVIDER_MUTATION_ROOTS)
            if path.endswith(TRANSITIONAL_WRAPPER_SUFFIXES)
        )
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            Finding(
                "critical",
                "transitional_wrapper_scan_failed",
                ".",
                f"unable to query git untracked transitional wrappers: {exc}",
            )
        ]

    for path in untracked:
        if path not in allowed:
            findings.append(
                Finding(
                    "critical",
                    "unclassified_new_transitional_wrapper",
                    path,
                    "New wrappers in transitional roots must declare transitional_wrapper_file_exceptions metadata or move behind an organ entrypoint/MCP operation.",
                )
            )

    for path in sorted(allowed):
        if not (WORKSPACE_ROOT / path).exists():
            findings.append(
                Finding(
                    "medium",
                    "transitional_wrapper_exception_stale",
                    path,
                    "Transitional wrapper exception does not match an existing file.",
                )
            )
    return findings


def misplaced_organ_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path", "")).rstrip("/") + "/"
        for item in contract.get("misplaced_organ_exceptions", [])
        if item.get("path")
    }


def validate_misplaced_organ_exceptions(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(
        validate_exception_metadata(
            contract,
            "misplaced_organ_exceptions",
            MISPLACED_ORGAN_REQUIRED_FIELDS,
            directory_paths=True,
        )
    )
    allowed = misplaced_organ_exception_paths(contract)
    scripts_root = WORKSPACE_ROOT / "scripts"
    if not scripts_root.exists():
        return findings

    for manifest in sorted(scripts_root.rglob("MANIFEST.yaml")):
        organ_dir = relative(manifest.parent).rstrip("/") + "/"
        if organ_dir not in allowed:
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_under_transitional_scripts",
                    relative(manifest),
                    "MANIFEST.yaml under scripts/ means an organ is hiding in a transitional wrapper root; move it under organs/ or declare misplaced_organ_exceptions metadata.",
                )
            )

    for path in sorted(allowed):
        if not (WORKSPACE_ROOT / path / "MANIFEST.yaml").exists():
            findings.append(
                Finding(
                    "medium",
                    "misplaced_organ_exception_stale",
                    path,
                    "Misplaced organ exception does not match a scripts/*/MANIFEST.yaml.",
                )
            )
    return findings


def iter_repo_text_files() -> list[Path]:
    ignored_parts = {".git", ".venv", "__pycache__", "node_modules", "target"}
    files: list[Path] = []
    for path in WORKSPACE_ROOT.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = set(path.relative_to(WORKSPACE_ROOT).parts)
        if rel_parts & ignored_parts:
            continue
        if path.suffix not in LEGACY_HERMES_X_SCAN_SUFFIXES:
            continue
        files.append(path)
    return files


def validate_no_active_legacy_hermes_x_references() -> list[Finding]:
    findings: list[Finding] = []
    for path in sorted(iter_repo_text_files()):
        rel = relative(path)
        if path_matches_any(rel, LEGACY_HERMES_X_REFERENCE_ALLOWED_GLOBS):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (FileNotFoundError, UnicodeDecodeError):
            continue
        for line_no, line in enumerate(lines, start=1):
            if LEGACY_HERMES_X_PATH_MARKER in line:
                findings.append(
                    Finding(
                        "critical",
                        "active_legacy_hermes_x_reference",
                        f"{rel}:{line_no}",
                        "Active source surfaces must not reference scripts/hermes-x; use organ-twitter paths or a documented historical/legacy exception.",
                    )
                )
    return findings


def validate_legacy_hermes_x_wrapper_contract() -> list[Finding]:
    findings: list[Finding] = []
    root = WORKSPACE_ROOT / LEGACY_HERMES_X_ROOT
    if not root.exists():
        return findings

    for path in sorted(root.rglob("*.py")):
        rel = relative(path)
        if path.name.startswith("__init__"):
            continue
        try:
            source = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue

        if LEGACY_HERMES_X_FUNC_RE.search(source):
            findings.append(
                Finding(
                    "critical",
                    "legacy_hermes_x_runtime_implementation",
                    rel,
                    "scripts/hermes-x should only contain compatibility shims; move runtime implementations to organs/twitter/src."
                )
            )

        if LEGACY_HERMES_X_WRAPPER_MARKER not in source and rel not in {
            "scripts/hermes-x/analysis/__init__.py",
            "scripts/hermes-x/consumers/__init__.py",
            "scripts/hermes-x/core/__init__.py",
            "scripts/hermes-x/data_architecture/__init__.py",
            "scripts/hermes-x/tests/__init__.py",
        }:
            findings.append(
                Finding(
                    "critical",
                    "legacy_hermes_x_wrapper_missing_twitter_redirect",
                    rel,
                    "compatibility wrappers under scripts/hermes-x must import from organs/twitter/src.* rather than implement logic inline."
                )
            )

    return findings


def validate_no_infra_hermes_x_direct_refs() -> list[Finding]:
    findings: list[Finding] = []
    infra_root = WORKSPACE_ROOT / "infra"
    if not infra_root.exists():
        return findings

    for path in sorted(infra_root.rglob("*")):
        if not path.is_file() or path.suffix not in LEGACY_HERMES_X_INFRA_SCAN_SUFFIXES:
            continue
        rel = relative(path)
        try:
            for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                if LEGACY_HERMES_X_DOMAIN_ALIAS_RE.search(line):
                    findings.append(
                        Finding(
                            "medium",
                            "legacy_hermes_x_domain_alias",
                            f"{rel}:{line_no}",
                            "Active infra must not carry explicit legacy hermes-x domain filters in service env.",
                        )
                    )
                if INFRA_HERMES_X_REF_MARKER in line:
                    findings.append(
                        Finding(
                            "critical",
                            "infra_legacy_hermes_x_ref",
                            f"{rel}:{line_no}",
                            "systemd/infra should not target scripts/hermes-x runtime paths; route via organs/twitter entrypoints."
                        )
                    )
        except UnicodeDecodeError:
            continue

    return findings


def validate_legacy_hermes_x_systemd_migration() -> list[Finding]:
    findings: list[Finding] = []
    service_roots = [WORKSPACE_ROOT / "infra" / "systemd", WORKSPACE_ROOT / "organs"]
    for root in service_roots:
        if not root.exists():
            continue

        for path in sorted(root.rglob("*.service")):
            rel = relative(path)
            if path.name.startswith(LEGACY_HERMES_X_SYSTEMD_PREFIX):
                findings.append(
                    Finding(
                        "medium",
                        "legacy_hermes_x_systemd_filename",
                        rel,
                        "Systemd unit name still uses legacy hermes-x-* namespace; migrate to organ-twitter canonical naming or document why it remains.",
                    )
                )

            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except UnicodeDecodeError:
                continue

            for line_no, line in enumerate(lines, start=1):
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                match = LEGACY_HERMES_X_SYSTEMD_DEP_RE.match(stripped)
                if not match:
                    continue
                deps = match.group(2)
                for token in deps.replace(",", " ").split():
                    if LEGACY_HERMES_X_UNIT_TOKEN_RE.search(token):
                        findings.append(
                            Finding(
                                "medium",
                                "legacy_hermes_x_systemd_dependency",
                                f"{rel}:{line_no}",
                                "Systemd dependency graph still targets legacy hermes-x-* unit names; migrate or add a canonical alias policy.",
                            )
                        )

    return findings


def declared_paths(contract: dict[str, Any], key: str) -> set[str]:
    return {
        str(item.get("path", "")).rstrip("/") + "/"
        for item in contract.get(key, [])
        if item.get("path")
    }


def validate_exception_metadata(
    contract: dict[str, Any],
    key: str,
    required: set[str],
    *,
    directory_paths: bool = False,
) -> list[Finding]:
    findings: list[Finding] = []
    for index, item in enumerate(contract.get(key, [])):
        path = item.get("path")
        item_path = str(path or f"{key}[{index}]")
        for field in sorted(required):
            if not item.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "topology_exception_metadata_missing",
                        item_path,
                        f"{key} entries must declare {field}.",
                    )
                )
        if directory_paths and path and not str(path).endswith("/"):
            findings.append(
                Finding(
                    "critical",
                    "topology_exception_path_not_directory",
                    item_path,
                    f"{key} paths must end with '/' to avoid ambiguous prefix matching.",
                )
            )
    return findings


def allowed_organ_maturity_statuses(contract: dict[str, Any]) -> set[str]:
    policy = contract.get("organ_maturity_policy", {})
    statuses = policy.get("allowed_statuses")
    if statuses:
        return {str(item).lower() for item in statuses}
    manifest_statuses = (
        contract.get("organ_shape", {})
        .get("manifest_policy", {})
        .get("allowed_statuses", [])
    )
    return {str(item).lower() for item in manifest_statuses}


def validate_organ_maturity_policy(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    policy = contract.get("organ_maturity_policy")
    if not isinstance(policy, dict):
        return [
            Finding(
                "critical",
                "organ_maturity_policy_missing",
                relative(CONTRACT_PATH),
                "repo-topology-v1 must declare organ_maturity_policy so organ states use one vocabulary.",
            )
        ]
    for field in ("allowed_statuses", "status_meanings", "minimum_exception_fields", "shape_exception_fields", "rule"):
        if not policy.get(field):
            findings.append(
                Finding(
                    "critical",
                    "organ_maturity_policy_metadata_missing",
                    relative(CONTRACT_PATH),
                    f"organ_maturity_policy must declare {field}.",
                )
            )

    allowed = allowed_organ_maturity_statuses(contract)
    for status in allowed:
        if status not in {str(key).lower() for key in policy.get("status_meanings", {}).keys()}:
            findings.append(
                Finding(
                    "critical",
                    "organ_maturity_status_meaning_missing",
                    relative(CONTRACT_PATH),
                    f"organ maturity status {status!r} must have a status_meanings entry.",
                )
            )

    for key in ("organ_boundary_exceptions", "organ_shape_exceptions"):
        for item in contract.get(key, []):
            status = item.get("status")
            path = str(item.get("path") or key)
            if status and str(status).lower() not in allowed:
                findings.append(
                    Finding(
                        "critical",
                        "organ_maturity_status_unknown",
                        path,
                        f"{key} status {status!r} is not declared in organ_maturity_policy.allowed_statuses.",
                    )
                )
    return findings


def validate_organ_boundaries(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    if not ORGANS_ROOT.exists():
        return findings

    exception_paths = declared_paths(contract, "organ_boundary_exceptions")
    findings.extend(
        validate_exception_metadata(
            contract,
            "organ_boundary_exceptions",
            {"path", "status", "owner", "replacement", "expiry_condition"},
            directory_paths=True,
        )
    )

    for organ_dir in sorted(path for path in ORGANS_ROOT.iterdir() if path.is_dir()):
        rel = relative(organ_dir).rstrip("/") + "/"
        if (organ_dir / "MANIFEST.yaml").exists():
            continue
        if rel in exception_paths:
            continue
        findings.append(
            Finding(
                "critical",
                "organ_without_manifest",
                rel,
                "Every persistent organ directory needs MANIFEST.yaml or an explicit organ_boundary_exceptions entry.",
            )
        )

    for rel in sorted(exception_paths):
        if not (WORKSPACE_ROOT / rel).exists():
            findings.append(
                Finding(
                    "medium",
                    "declared_organ_boundary_exception_missing",
                    rel,
                    "Declared organ boundary exception does not exist in the current tree.",
                )
            )

    return findings


def validate_organ_manifest_policy(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    if not ORGANS_ROOT.exists():
        return findings

    policy = contract.get("organ_shape", {}).get("manifest_policy", {})
    identity_fields = policy.get("identity_fields_any", ["id", "name"])
    required_fields = policy.get("required_fields", ["status", "description"])
    allowed_statuses = {str(item).lower() for item in policy.get("allowed_statuses", [])}
    case_insensitive = policy.get("status_case") == "case_insensitive"

    for manifest in sorted(ORGANS_ROOT.glob("*/MANIFEST.yaml")):
        rel = relative(manifest)
        try:
            payload = load_yaml(manifest) or {}
        except yaml.YAMLError as exc:
            findings.append(
                Finding("critical", "organ_manifest_yaml_invalid", rel, str(exc))
            )
            continue
        if not isinstance(payload, dict):
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_not_mapping",
                    rel,
                    "MANIFEST.yaml must be a YAML mapping.",
                )
            )
            continue
        if not any(payload.get(field) for field in identity_fields):
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_identity_missing",
                    rel,
                    f"MANIFEST.yaml must declare at least one identity field from {identity_fields}.",
                )
            )
        for field in required_fields:
            if not payload.get(field):
                findings.append(
                    Finding(
                        "critical",
                        "organ_manifest_required_field_missing",
                        rel,
                        f"MANIFEST.yaml must declare {field}.",
                    )
                )
        status = payload.get("status")
        status_value = str(status).lower() if case_insensitive else str(status)
        if status and allowed_statuses and status_value not in allowed_statuses:
            findings.append(
                Finding(
                    "critical",
                    "organ_manifest_status_unknown",
                    rel,
                    f"status {status!r} is not declared in organ_shape.manifest_policy.allowed_statuses.",
                )
            )

    return findings


def has_python_sources(path: Path) -> bool:
    src = path / "src"
    return src.exists() and any(src.rglob("*.py"))


def validate_python_organ_shape(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    if not ORGANS_ROOT.exists():
        return findings

    required_paths = contract.get("organ_shape", {}).get("python_executable_required_paths", [])
    if not required_paths:
        return findings

    exception_paths = declared_paths(contract, "organ_shape_exceptions")
    findings.extend(
        validate_exception_metadata(
            contract,
            "organ_shape_exceptions",
            {"path", "status", "owner", "missing_paths", "replacement", "expiry_condition"},
            directory_paths=True,
        )
    )

    for organ_dir in sorted(path for path in ORGANS_ROOT.iterdir() if path.is_dir()):
        if not (organ_dir / "MANIFEST.yaml").exists() or not has_python_sources(organ_dir):
            continue
        missing = [item for item in required_paths if not (organ_dir / item).exists()]
        if not missing:
            continue
        rel = relative(organ_dir).rstrip("/") + "/"
        if rel in exception_paths:
            continue
        findings.append(
            Finding(
                "critical",
                "python_organ_shape_incomplete",
                rel,
                f"Python organs need {required_paths}; missing {missing}. Add the boundary or declare an organ_shape_exceptions entry.",
            )
        )

    for item in contract.get("organ_shape_exceptions", []):
        path_value = item.get("path")
        if not path_value:
            continue
        organ_dir = WORKSPACE_ROOT / path_value
        declared_missing = set(item.get("missing_paths", []))
        actual_missing = {path for path in required_paths if not (organ_dir / path).exists()}
        if organ_dir.exists() and declared_missing != actual_missing:
            findings.append(
                Finding(
                    "critical",
                    "organ_shape_exception_stale",
                    str(path_value),
                    f"declares missing_paths={sorted(declared_missing)} but actual missing paths are {sorted(actual_missing)}.",
                )
            )

    return findings


def runtime_state_exception_patterns(contract: dict[str, Any]) -> set[str]:
    patterns: set[str] = set()
    for item in contract.get("runtime_state_commit_exceptions", []):
        path = item.get("path")
        if path:
            patterns.add(str(path))
        for path in item.get("paths", []):
            patterns.add(str(path))
        for pattern in item.get("patterns", []):
            patterns.add(str(pattern))
    return patterns


def validate_runtime_state_commit_exceptions(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(
        validate_exception_metadata(
            contract,
            "runtime_state_commit_exceptions",
            {"status", "owner", "commit_policy", "replacement", "expiry_condition"},
        )
    )
    allowed = runtime_state_exception_patterns(contract)
    try:
        tracked = [
            path
            for path in git_ls_files()
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in RUNTIME_STATE_GLOBS)
        ]
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            Finding(
                "critical",
                "tracked_runtime_state_scan_failed",
                ".",
                f"unable to query git tracked runtime state: {exc}",
            )
        ]

    for path in sorted(tracked):
        if not path_matches_any(path, allowed):
            findings.append(
                Finding(
                    "critical",
                    "tracked_runtime_state_without_exception",
                    path,
                    "Tracked runtime state must be declared in runtime_state_commit_exceptions or removed from source control.",
                )
            )

    for pattern in sorted(allowed):
        if not any(fnmatch.fnmatchcase(path, pattern) for path in tracked):
            findings.append(
                Finding(
                    "medium",
                    "runtime_state_exception_stale",
                    pattern,
                    "Runtime state exception does not match any tracked file.",
                )
            )
    return findings


def provider_mutation_exception_paths(contract: dict[str, Any]) -> set[str]:
    return {
        str(item.get("path"))
        for item in contract.get("provider_mutation_script_exceptions", [])
        if item.get("path")
    }


def validate_provider_mutation_script_exceptions(contract: dict[str, Any]) -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(
        validate_exception_metadata(
            contract,
            "provider_mutation_script_exceptions",
            {"path", "status", "owner", "replacement", "guard", "expiry_condition"},
        )
    )
    allowed = provider_mutation_exception_paths(contract)
    try:
        tracked = sorted(
            set(git_ls_files(*PROVIDER_MUTATION_ROOTS))
            | set(git_ls_untracked(*PROVIDER_MUTATION_ROOTS))
            | {path for path in allowed if (WORKSPACE_ROOT / path).exists()}
        )
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            Finding(
                "critical",
                "provider_mutation_script_scan_failed",
                ".",
                f"unable to query git tracked provider scripts: {exc}",
            )
        ]

    for path in sorted(tracked):
        if not path.endswith(PROVIDER_MUTATION_SUFFIXES):
            continue
        full_path = WORKSPACE_ROOT / path
        if not full_path.exists():
            continue
        try:
            content = full_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            content = full_path.read_text(encoding="utf-8", errors="ignore")
        if PROVIDER_MUTATION_RE.search(content) and path not in allowed:
            findings.append(
                Finding(
                    "critical",
                    "provider_mutation_script_without_exception",
                    path,
                    "Provider mutation scripts in transitional paths need provider_mutation_script_exceptions metadata or must be migrated behind MCP/organ adapters.",
                )
            )

    for path in sorted(allowed):
        if not (WORKSPACE_ROOT / path).exists():
            findings.append(
                Finding(
                    "medium",
                    "provider_mutation_exception_stale",
                    path,
                    "Provider mutation exception does not match a tracked file.",
                )
            )
    return findings


def validate_mcp_no_absolute_project_paths() -> list[Finding]:
    findings: list[Finding] = []
    if not MCP_SRC_ROOT.exists():
        return findings

    forbidden = re.compile(r"(/home/[^\\\"'\\s]+|/Users/[^\\\"'\\s]+|Bureau/|privacy-ethereum-framework)")
    for path in sorted(MCP_SRC_ROOT.rglob("*.rs")):
        for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if forbidden.search(line):
                findings.append(
                    Finding(
                        "critical",
                        "hardcoded_project_path_in_mcp",
                        f"{relative(path)}:{line_no}",
                        "portable MCP proxy code must use explicit parameters or CYNIC_PROJECT_ROOT, not host/project-specific paths.",
                    )
                )
    return findings


def validate_no_hardcoded_deployment_projection_paths() -> list[Finding]:
    findings: list[Finding] = []
    projection_roots = [
        WORKSPACE_ROOT / "infra" / "systemd",
    ]
    projection_roots.extend(
        sorted((WORKSPACE_ROOT / "organs").glob("*/systemd"))
    )

    for root in projection_roots:
        if not root.exists():
            continue
        for path in sorted(root.rglob("*")):
            if not path.is_file() or path.suffix not in {".service", ".timer", ".sh", ".conf"}:
                continue
            rel = relative(path)
            try:
                for line_no, line in enumerate(
                    path.read_text(encoding="utf-8").splitlines(), start=1
                ):
                    stripped = line.strip()
                    if not stripped or stripped.startswith("#"):
                        continue
                    if DEPLOYMENT_PROJECTION_PATH_RE.search(stripped):
                        findings.append(
                            Finding(
                                "critical",
                                "hardcoded_deployment_projection_path",
                                f"{rel}:{line_no}",
                                "Deployment projections must use registry/runtime placeholders (@CYNIC_PROJECT_ROOT@ or explicit env vars), not host paths.",
                            )
                        )
            except UnicodeDecodeError:
                continue
    return findings


def validate() -> list[Finding]:
    findings: list[Finding] = []
    findings.extend(validate_json_contracts())
    findings.extend(validate_theme_topology_contract())
    findings.extend(validate_document_theme_inventory_contract())
    topology_findings = validate_topology_contract()
    findings.extend(topology_findings)
    if not any(item.kind in {"repo_topology_contract_missing", "unexpected_repo_topology_schema"} for item in topology_findings):
        contract = load_json(CONTRACT_PATH)
        findings.extend(validate_contract_file_policy(contract))
        findings.extend(validate_organ_maturity_policy(contract))
        findings.extend(validate_current_transitional_paths(contract))
        findings.extend(validate_transitional_wrapper_file_exceptions(contract))
        findings.extend(validate_misplaced_organ_exceptions(contract))
        findings.extend(validate_organ_boundaries(contract))
        findings.extend(validate_organ_manifest_policy(contract))
        findings.extend(validate_python_organ_shape(contract))
        findings.extend(validate_runtime_state_commit_exceptions(contract))
        findings.extend(validate_provider_mutation_script_exceptions(contract))
        findings.extend(validate_no_active_legacy_hermes_x_references())
        findings.extend(validate_legacy_hermes_x_wrapper_contract())
        findings.extend(validate_no_infra_hermes_x_direct_refs())
        findings.extend(validate_legacy_hermes_x_systemd_migration())
        findings.extend(validate_no_hardcoded_deployment_projection_paths())
    findings.extend(validate_mcp_no_absolute_project_paths())
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate CYNIC repo topology contract.")
    parser.add_argument("--json", action="store_true", help="Emit machine-readable report.")
    args = parser.parse_args()

    findings = validate()
    report = {
        "schema": "cynic.repo_topology_validation.v1",
        "ok": not any(item.severity == "critical" for item in findings),
        "findings": [asdict(item) for item in findings],
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        if report["ok"]:
            print("repo topology contract ok")
        else:
            print("repo topology contract failed")
            for item in findings:
                print(f"{item.severity}: {item.kind}: {item.path}: {item.detail}")

    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
