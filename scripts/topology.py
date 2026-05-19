#!/usr/bin/env python3
"""Organism Topology — discover, parse, verify, render.

Tier 2 INFRASTRUCTURE: Aggregates MANIFEST.yaml files into a unified topology view.

K15 Consumer: session-init.sh injects TOPOLOGY.md at session start.
              make lint-topology verifies MANIFESTs vs live state.

Usage:
    python3 scripts/topology.py              # render TOPOLOGY.md
    python3 scripts/topology.py --verify     # render + verify vs live state
    python3 scripts/topology.py --graph      # print producer→consumer graph
    python3 scripts/topology.py --json       # output topology.json (machine-readable)
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional


__version__ = "0.1.0"


def find_manifests(root: Path) -> list[Path]:
    """Discover all MANIFEST.yaml files under root."""
    manifests: list[Path] = []
    for p in sorted(root.rglob("MANIFEST.yaml")):
        manifests.append(p)
    return manifests


def parse_manifest(path: Path) -> dict[str, Any]:
    """Parse a MANIFEST.yaml file. Uses a minimal YAML parser to avoid external deps."""
    # We use PyYAML if available, else a simple fallback
    try:
        import yaml  # type: ignore[import-untyped]
        with open(path) as f:
            data = yaml.safe_load(f) or {}
    except ImportError:
        # Fallback: parse key-value pairs at top level only
        data = _parse_yaml_fallback(path)

    data["_path"] = str(path)
    data["_relative"] = str(path.relative_to(Path.cwd()) if path.is_relative_to(Path.cwd()) else path)
    return data


def _parse_yaml_fallback(path: Path) -> dict[str, Any]:
    """Minimal YAML parser for flat key-value manifests. Handles lists and scalars."""
    data: dict[str, Any] = {}
    current_key: Optional[str] = None
    current_list: Optional[list[Any]] = None

    with open(path) as f:
        for line in f:
            stripped = line.rstrip()
            if not stripped or stripped.startswith("#"):
                continue

            # Detect list item under current key
            if stripped.startswith("  - ") and current_key is not None:
                item = stripped[4:].strip()
                if current_list is None:
                    current_list = []
                    data[current_key] = current_list
                current_list.append(item)
                continue

            # Top-level key: value
            if ":" in stripped and not stripped.startswith(" "):
                if current_list is not None:
                    current_list = None
                key, _, val = stripped.partition(":")
                key = key.strip()
                val = val.strip()
                current_key = key
                if val and val != "|":
                    # Remove quotes
                    if val.startswith('"') and val.endswith('"'):
                        val = val[1:-1]
                    data[key] = val
                elif val == "|":
                    data[key] = ""  # multiline block scalar (simplified)
                else:
                    data[key] = None
                continue

    return data


def get_active_timers() -> dict[str, dict[str, str]]:
    """Query systemctl --user list-timers and return {unit_name: {next, last}}."""
    timers: dict[str, dict[str, str]] = {}
    try:
        result = subprocess.run(
            ["systemctl", "--user", "list-timers", "--no-pager", "--plain"],
            capture_output=True, text=True, timeout=10
        )
        for line in result.stdout.splitlines():
            parts = line.split()
            # Format: NEXT ... LAST ... PASSED UNIT ACTIVATES
            # We look for .timer entries
            for i, part in enumerate(parts):
                if part.endswith(".timer"):
                    activates = parts[i + 1] if i + 1 < len(parts) else ""
                    timers[part] = {"activates": activates}
                    # Also index by service name
                    if activates.endswith(".service"):
                        timers[activates] = {"timer": part}
                    break
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        pass
    return timers


def get_active_services() -> set[str]:
    """Query systemctl --user for active services."""
    active: set[str] = set()
    try:
        result = subprocess.run(
            ["systemctl", "--user", "list-units", "--type=service", "--state=active",
             "--no-pager", "--plain", "--no-legend"],
            capture_output=True, text=True, timeout=10
        )
        for line in result.stdout.splitlines():
            parts = line.split()
            if parts and parts[0].endswith(".service"):
                active.add(parts[0])
    except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.SubprocessError):
        pass
    return active


def verify_manifest(manifest: dict[str, Any], timers: dict[str, dict[str, str]],
                    active_services: set[str], root: Path) -> list[dict[str, str]]:
    """Verify a manifest's declared state vs live reality. Returns list of issues."""
    issues: list[dict[str, str]] = []
    manifest_id = manifest.get("id", "unknown")

    # Verify crons
    for cron in manifest.get("crons", []) or []:
        if not isinstance(cron, dict):
            continue
        systemd_unit = cron.get("systemd", "")
        cron_type = cron.get("type", "oneshot")
        cron_name = cron.get("name", systemd_unit)

        if not systemd_unit:
            continue

        if cron_type == "long-running":
            # Check if service is active
            if systemd_unit not in active_services:
                issues.append({
                    "module": manifest_id,
                    "type": "cron_missing",
                    "detail": f"daemon {systemd_unit} not active",
                    "severity": "error",
                })
        else:
            # Check if timer exists
            timer_name = systemd_unit.replace(".service", ".timer")
            if timer_name not in timers:
                issues.append({
                    "module": manifest_id,
                    "type": "cron_missing",
                    "detail": f"timer {timer_name} not found in list-timers",
                    "severity": "error",
                })

    # Verify output path patterns exist (check at least one match)
    for output in manifest.get("outputs", []) or []:
        if not isinstance(output, dict):
            continue
        path_pattern = output.get("path", "")
        if not path_pattern or path_pattern.startswith("REST ") or path_pattern.startswith("kernel "):
            continue  # Skip REST endpoints and kernel paths
        if path_pattern.startswith("~"):
            path_pattern = str(Path.home()) + path_pattern[1:]

        # Replace {placeholders} with glob wildcards
        import re
        glob_pattern = re.sub(r"\{[^}]+\}", "*", path_pattern)

        # Check if at least one file matches
        from pathlib import PurePosixPath
        parent = Path(glob_pattern).parent
        name_pattern = Path(glob_pattern).name

        if parent.exists():
            matches = list(parent.glob(name_pattern))
            if not matches:
                # Try from project root
                full_path = root / glob_pattern
                parent_full = full_path.parent
                if parent_full.exists():
                    matches = list(parent_full.glob(full_path.name))

                if not matches:
                    issues.append({
                        "module": manifest_id,
                        "type": "output_missing",
                        "detail": f"no files match {output.get('path', '')}",
                        "severity": "warning",
                    })

    return issues


def build_graph(manifests: list[dict[str, Any]]) -> list[dict[str, str]]:
    """Build producer→consumer edges from manifest outputs and inputs."""
    edges: list[dict[str, str]] = []

    for m in manifests:
        mid = m.get("id", "unknown")
        for output in m.get("outputs", []) or []:
            if not isinstance(output, dict):
                continue
            consumer = output.get("consumer", "")
            path = output.get("path", "")
            if consumer:
                for c in consumer.split(","):
                    c = c.strip()
                    if c:
                        edges.append({
                            "from": mid,
                            "to": c,
                            "via": path,
                        })

    return edges


def render_topology_md(manifests: list[dict[str, Any]], issues: list[dict[str, str]],
                       graph: list[dict[str, str]]) -> str:
    """Render TOPOLOGY.md content."""
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    lines: list[str] = []
    lines.append(f"# CYNIC Organism Topology (auto-generated {now})")
    lines.append("")
    lines.append("## Active Modules")
    lines.append("")
    lines.append("| Module | Domain | Tier | Status | Crons | Key Outputs |")
    lines.append("|--------|--------|------|--------|-------|-------------|")

    # Sort: tier 1 first, then tier 2, then tier 3
    sorted_manifests = sorted(manifests, key=lambda m: (
        int(m.get("tier", 9)),
        m.get("domain", ""),
        m.get("id", ""),
    ))

    for m in sorted_manifests:
        mid = m.get("id", "unknown")
        domain = m.get("domain", "?")
        tier = m.get("tier", "?")
        status = m.get("status", "?")

        # Count crons
        crons = m.get("crons", []) or []
        cron_names: list[str] = []
        for c in crons:
            if isinstance(c, dict):
                name = c.get("name", "")
                timer = c.get("timer", "")
                ctype = c.get("type", "")
                if ctype == "long-running":
                    cron_names.append(f"{name} (daemon)")
                elif timer:
                    cron_names.append(f"{name} ({timer})")
                else:
                    cron_names.append(name)
        cron_str = ", ".join(cron_names) if cron_names else "none"

        # Key outputs (compact)
        outputs = m.get("outputs", []) or []
        output_names: list[str] = []
        for o in outputs:
            if isinstance(o, dict):
                path = o.get("path", "")
                # Extract meaningful name from path
                if path.startswith("REST ") or path.startswith("kernel "):
                    output_names.append(path)
                else:
                    output_names.append(Path(path).stem.replace("_", " ").replace("{", "").replace("}", ""))
        output_str = ", ".join(output_names[:3])
        if len(output_names) > 3:
            output_str += f" (+{len(output_names) - 3})"

        lines.append(f"| {mid} | {domain} | {tier} | {status} | {cron_str} | {output_str} |")

    # Data flow section
    lines.append("")
    lines.append("## Data Flow (producer -> consumer)")
    lines.append("")
    if graph:
        # Group by source
        from collections import defaultdict
        by_source: dict[str, list[str]] = defaultdict(list)
        for edge in graph:
            by_source[edge["from"]].append(f"{edge['to']}")

        for source, targets in sorted(by_source.items()):
            unique_targets = sorted(set(targets))
            lines.append(f"- **{source}** -> {', '.join(unique_targets)}")
    else:
        lines.append("(no edges found)")

    # K15 unfulfilled section
    unfulfilled: list[str] = []
    for m in sorted_manifests:
        for consumer in m.get("k15_consumers", []) or []:
            if isinstance(consumer, dict):
                c_name = consumer.get("consumer", "")
                if "NOT YET BUILT" in c_name:
                    unfulfilled.append(f"- {m.get('id', '?')}: {c_name}")

        # Check scripts with status NOT YET BUILT
        for script in m.get("scripts", []) or []:
            if isinstance(script, dict) and script.get("status") == "NOT YET BUILT":
                unfulfilled.append(f"- {m.get('id', '?')}/{script.get('name', '?')}: DECLARED, NOT BUILT")

    if unfulfilled:
        lines.append("")
        lines.append("## Unfulfilled (K15)")
        lines.append("")
        for u in unfulfilled:
            lines.append(u)

    # Issues section (only if verify was run)
    if issues:
        lines.append("")
        lines.append("## Verification Issues")
        lines.append("")
        for issue in issues:
            sev = issue.get("severity", "?")
            lines.append(f"- [{sev}] {issue.get('module', '?')}: {issue.get('detail', '?')}")

    # Experiment MANIFESTs (separate table)
    experiments = [m for m in manifests if m.get("hypothesis")]
    if experiments:
        lines.append("")
        lines.append("## Experiments")
        lines.append("")
        lines.append("| ID | Status | Hypothesis (truncated) |")
        lines.append("|----|--------|----------------------|")
        for exp in experiments:
            hyp = (exp.get("hypothesis", "") or "")[:80]
            lines.append(f"| {exp.get('id', '?')} | {exp.get('status', '?')} | {hyp} |")

    lines.append("")
    return "\n".join(lines)


def render_topology_json(manifests: list[dict[str, Any]], issues: list[dict[str, str]],
                         graph: list[dict[str, str]]) -> str:
    """Render topology.json for machine consumption."""
    data = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "modules": [],
        "graph": graph,
        "issues": issues,
    }
    for m in manifests:
        entry = {
            "id": m.get("id", "unknown"),
            "tier": m.get("tier"),
            "domain": m.get("domain"),
            "status": m.get("status"),
            "path": m.get("_relative"),
            "crons": [c.get("name", "") for c in (m.get("crons") or []) if isinstance(c, dict)],
            "outputs_count": len(m.get("outputs") or []),
            "is_experiment": bool(m.get("hypothesis")),
        }
        data["modules"].append(entry)

    return json.dumps(data, indent=2)


def main() -> int:
    parser = argparse.ArgumentParser(description="CYNIC Organism Topology")
    parser.add_argument("--verify", action="store_true", help="Verify MANIFESTs vs live state")
    parser.add_argument("--graph", action="store_true", help="Print producer→consumer graph")
    parser.add_argument("--json", action="store_true", help="Output topology.json")
    parser.add_argument("--root", type=str, default=None, help="Project root (default: git toplevel)")
    parser.add_argument("--output", type=str, default=None, help="Output file path (default: stdout for --json/--graph, TOPOLOGY.md for default)")
    args = parser.parse_args()

    # Determine project root
    if args.root:
        root = Path(args.root)
    else:
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                capture_output=True, text=True, timeout=5
            )
            root = Path(result.stdout.strip())
        except (subprocess.TimeoutExpired, subprocess.SubprocessError):
            root = Path.cwd()

    # Discover + parse
    manifest_paths = find_manifests(root)
    if not manifest_paths:
        print("ERROR: No MANIFEST.yaml files found", file=sys.stderr)
        return 1

    manifests_raw = [parse_manifest(p) for p in manifest_paths]
    # Filter: only include manifests with an 'id' field (skip versioning/non-topology manifests)
    manifests = [m for m in manifests_raw if m.get("id")]
    skipped = len(manifests_raw) - len(manifests)
    msg = f"Found {len(manifests)} MANIFESTs"
    if skipped:
        msg += f" ({skipped} skipped — no 'id' field)"
    print(msg, file=sys.stderr)

    # Build graph
    graph = build_graph(manifests)

    # Verify if requested
    all_issues: list[dict[str, str]] = []
    if args.verify:
        timers = get_active_timers()
        active_services = get_active_services()
        for m in manifests:
            issues = verify_manifest(m, timers, active_services, root)
            all_issues.extend(issues)

        error_count = sum(1 for i in all_issues if i.get("severity") == "error")
        warn_count = sum(1 for i in all_issues if i.get("severity") == "warning")
        print(f"Verification: {error_count} errors, {warn_count} warnings", file=sys.stderr)

    # Render
    if args.json:
        output = render_topology_json(manifests, all_issues, graph)
        if args.output:
            Path(args.output).write_text(output)
            print(f"Written to {args.output}", file=sys.stderr)
        else:
            print(output)
    elif args.graph:
        for edge in graph:
            print(f"  {edge['from']} -> {edge['to']}  (via {edge['via']})")
    else:
        md = render_topology_md(manifests, all_issues, graph)
        output_path = Path(args.output) if args.output else root / "TOPOLOGY.md"
        output_path.write_text(md)
        print(f"Written to {output_path}", file=sys.stderr)

    # Exit code: 1 if verify found errors
    if args.verify and any(i.get("severity") == "error" for i in all_issues):
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
