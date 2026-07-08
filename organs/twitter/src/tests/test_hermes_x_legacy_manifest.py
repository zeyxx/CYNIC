from pathlib import Path

import yaml

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
LEGACY_ROOT = PROJECT_ROOT / "scripts" / "hermes-x"
LEGACY_MANIFEST = LEGACY_ROOT / "MANIFEST.yaml"


def test_legacy_hermes_x_manifest_only_declares_existing_files():
    manifest = yaml.safe_load(LEGACY_MANIFEST.read_text())
    declared = [
        *[item["name"] for item in manifest.get("scripts", [])],
        *[item["name"] for item in manifest.get("assets", [])],
    ]

    missing = [name for name in declared if not (LEGACY_ROOT / name).exists()]

    assert missing == []
