from __future__ import annotations

from pathlib import Path

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.WORKSPACE_ROOT / "axioms" / "anti-polling-transition-v1.json"
EXPECTED_SCHEMA = "cynic.anti_polling_transition.v1"


def validate_anti_polling_transition_contract() -> list[host.Finding]:
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "anti_polling_transition_contract_missing",
                host.relative(CONTRACT_PATH),
                "anti-polling-transition-v1.json must exist to block new polling surfaces.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    findings: list[host.Finding] = []
    if contract.get("schema") != EXPECTED_SCHEMA:
        return [
            host.Finding(
                "critical",
                "unexpected_anti_polling_transition_schema",
                host.relative(CONTRACT_PATH),
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        ]

    authorities = contract.get("authorities") or {}
    for name, value in sorted(authorities.items()):
        path = host.WORKSPACE_ROOT / str(value)
        if not path.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "anti_polling_authority_missing",
                    str(value),
                    f"authority {name!r} points to a path that does not exist.",
                )
            )

    markers = contract.get("polling_markers")
    if not isinstance(markers, list) or not markers:
        findings.append(
            host.Finding(
                "critical",
                "anti_polling_markers_missing",
                host.relative(CONTRACT_PATH),
                "polling_markers must declare non-empty marker strings.",
            )
        )
        markers = []

    scan_roots = contract.get("scan_roots")
    if not isinstance(scan_roots, list) or not scan_roots:
        findings.append(
            host.Finding(
                "critical",
                "anti_polling_scan_roots_missing",
                host.relative(CONTRACT_PATH),
                "scan_roots must declare non-empty root paths.",
            )
        )
        scan_roots = []

    transitional = contract.get("transitional_polling_surfaces")
    if not isinstance(transitional, list) or not transitional:
        findings.append(
            host.Finding(
                "critical",
                "anti_polling_transitional_surfaces_missing",
                host.relative(CONTRACT_PATH),
                "transitional_polling_surfaces must declare every allowed remaining polling surface.",
            )
        )
        transitional = []

    allowlisted_paths: set[str] = set()
    for index, item in enumerate(transitional):
        path = str(item.get("path") or "")
        pointer = f"{host.relative(CONTRACT_PATH)}:transitional_polling_surfaces[{index}]"
        if not path:
            findings.append(host.Finding("critical", "anti_polling_surface_path_missing", pointer, "transitional polling surfaces must declare path."))
            continue
        allowlisted_paths.add(path)
        fs_path = host.WORKSPACE_ROOT / path
        if not fs_path.exists():
            findings.append(host.Finding("warning", "anti_polling_surface_path_missing_on_disk", path, "declared transitional polling surface does not exist on disk."))
        for field in ("current_mechanism", "target_mechanism", "status"):
            if not item.get(field):
                findings.append(host.Finding("critical", "anti_polling_surface_metadata_missing", path, f"transitional polling surface must declare {field}."))

    for root_value in scan_roots:
        root = host.WORKSPACE_ROOT / str(root_value)
        if not root.exists():
            findings.append(host.Finding("warning", "anti_polling_scan_root_missing", str(root_value), "scan root does not exist on disk."))
            continue
        for path in _iter_text_files(root):
            rel = host.relative(path)
            text = path.read_text(encoding="utf-8", errors="ignore")
            matched = [marker for marker in markers if marker in text]
            if not matched:
                continue
            if rel not in allowlisted_paths:
                findings.append(
                    host.Finding(
                        "warning",
                        "undeclared_polling_surface",
                        rel,
                        f"polling markers {matched!r} found under anti-polling scan roots without a transitional declaration.",
                    )
                )
    return findings


def _iter_text_files(root: Path):
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix not in {".py", ".md", ".json", ".service"}:
            continue
        yield path
