from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CANONICAL_POSTER = PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "x_poster.py"
LEGACY_POSTER = PROJECT_ROOT / "scripts" / "hermes-x" / "core" / "x_poster.py"
LEGACY_TREE = "scripts" + "/" + "hermes-x"


def test_canonical_x_poster_uses_organ_paths_not_legacy_import_path():
    source = CANONICAL_POSTER.read_text()

    assert "from hermes_paths import" not in source
    assert LEGACY_TREE not in source
    assert "organs.twitter.src.adapters.hermes_paths" in source


def test_legacy_x_poster_is_closed():
    assert not LEGACY_POSTER.exists()
