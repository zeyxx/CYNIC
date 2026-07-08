from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CANONICAL_PROXY = PROJECT_ROOT / "organs" / "twitter" / "src" / "adapters" / "hermes_x_proxy.py"
LEGACY_PROXY = PROJECT_ROOT / "scripts" / "hermes-x" / "core" / "x_proxy.py"
LEGACY_TREE = "scripts" + "/" + "hermes-x"


def test_canonical_proxy_does_not_depend_on_legacy_scripts_tree():
    source = CANONICAL_PROXY.read_text()

    assert LEGACY_TREE not in source
    assert "LEGACY_CORE" not in source
    assert "from x_proxy import" not in source


def test_legacy_proxy_is_only_a_compatibility_shim():
    source = LEGACY_PROXY.read_text()

    assert "from organs.twitter.src.adapters.hermes_x_proxy import *" in source
    assert "CAPTURE_OPS" not in source
    assert "class XProxy" not in source
