from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CANONICAL_AUDIT = PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "data_architecture_audit.py"
LEGACY_AUDIT = PROJECT_ROOT / "scripts" / "hermes-x" / "data_architecture" / "audit.py"


def test_canonical_data_architecture_audit_uses_organ_path_adapter():
    source = CANONICAL_AUDIT.read_text()

    assert "from hermes_paths import" not in source
    assert "organs.twitter.src.adapters.hermes_paths" in source
    assert "Status: EXPIRED_EXPERIMENT" in source


def test_legacy_data_architecture_audit_path_is_closed():
    assert not LEGACY_AUDIT.exists()
