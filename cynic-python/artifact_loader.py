#!/usr/bin/env python3
"""
Artifact protocol loader for cortexes.

Reads cortex manifest from ~/.cynic/cortex-manifests/<cortex_id>.json
and loads artifacts from ~/.cynic/organisms/artifacts/.

Usage:
    from artifact_loader import load_cortex_artifacts
    artifacts = load_cortex_artifacts("claude-code")
    domain_discovery = artifacts.get("domain_discovery_complete")
"""

import json
from pathlib import Path
from typing import Dict, Any, Optional


def find_artifact(artifact_name: str, artifact_base: str = "~/.cynic/organisms/artifacts") -> Optional[Path]:
    """Find artifact file by name, searching maturity levels."""
    base = Path(artifact_base).expanduser()

    # Search in order: validated, deferred, dead
    for maturity in ["validated", "deferred", "dead"]:
        maturity_dir = base / maturity
        if not maturity_dir.exists():
            continue

        # Search all subdirectories
        for artifact_file in maturity_dir.rglob(f"*{artifact_name}*"):
            if artifact_file.is_file():
                return artifact_file

    return None


def load_cortex_artifacts(
    cortex_id: str,
    manifest_dir: str = "~/.cynic/cortex-manifests",
    artifact_base: str = "~/.cynic/organisms/artifacts"
) -> Dict[str, Any]:
    """
    Load artifacts for a cortex based on its manifest.

    Args:
        cortex_id: Cortex identifier (e.g., "claude-code")
        manifest_dir: Directory containing cortex manifests
        artifact_base: Base directory for artifacts

    Returns:
        Dictionary mapping artifact names to loaded content
    """
    manifest_path = Path(manifest_dir).expanduser() / f"{cortex_id}.json"

    if not manifest_path.exists():
        print(f"  ℹ No manifest found for {cortex_id}")
        return {}

    try:
        with open(manifest_path) as f:
            manifest = json.load(f)
    except json.JSONDecodeError as e:
        print(f"  ✗ Failed to parse manifest {manifest_path}: {e}")
        return {}

    artifacts = {}
    for consumer in manifest.get("consumes", []):
        consumer_id = consumer["consumer_id"]
        print(f"  • {consumer_id}:")

        for artifact_name in consumer.get("artifacts", []):
            artifact_path = find_artifact(artifact_name, artifact_base)

            if not artifact_path:
                print(f"    ⚠ {artifact_name}: not found")
                continue

            try:
                with open(artifact_path) as af:
                    if artifact_path.suffix == ".json":
                        artifacts[artifact_name] = json.load(af)
                    else:
                        artifacts[artifact_name] = af.read()
                    print(f"    ✓ {artifact_name}")
            except (json.JSONDecodeError, IOError) as e:
                print(f"    ✗ {artifact_name}: {e}")

    return artifacts


def report_consumer_status(registry_path: str = "~/.cynic/organisms/consumers/consumer_registry.json") -> None:
    """Load and report status of all registered consumers."""
    reg_path = Path(registry_path).expanduser()

    if not reg_path.exists():
        print("  ℹ No consumer registry found")
        return

    try:
        with open(reg_path) as f:
            registry = json.load(f)
    except json.JSONDecodeError as e:
        print(f"  ✗ Failed to parse registry: {e}")
        return

    consumers_with_blockers = [
        c for c in registry.get("consumers", [])
        if c.get("inputs", [{}])[0].get("blocker")
    ]

    if consumers_with_blockers:
        print("  Blocked consumers:")
        for consumer in consumers_with_blockers:
            blocker = consumer["inputs"][0].get("blocker", "unknown")
            print(f"    ⏸ {consumer['id']}: {blocker}")
    else:
        print("  ✓ All consumers unblocked")


if __name__ == "__main__":
    import sys

    cortex_id = sys.argv[1] if len(sys.argv) > 1 else "claude-code"

    print(f"⏳ Loading artifacts for {cortex_id}...")
    artifacts = load_cortex_artifacts(cortex_id)
    print(f"\n✓ Loaded {len(artifacts)} artifacts")

    print(f"\n⏳ Checking consumer status...")
    report_consumer_status()
