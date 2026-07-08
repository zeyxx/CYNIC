from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CANONICAL_EMITTER = PROJECT_ROOT / "organs" / "twitter" / "src" / "adapters" / "k15_emitter.py"
LEGACY_EMITTER = PROJECT_ROOT / "scripts" / "hermes-x" / "consumers" / "k15_emitter.py"
LEGACY_TREE = "scripts" + "/" + "hermes-x"


def test_canonical_k15_emitter_is_not_sourced_from_legacy_scripts_tree():
    source = CANONICAL_EMITTER.read_text()

    assert LEGACY_TREE not in source
    assert "from k15_emitter import *" not in source


def test_legacy_k15_emitter_is_only_a_compatibility_shim():
    source = LEGACY_EMITTER.read_text()

    assert "from organs.twitter.src.adapters.k15_emitter import *" in source
    assert "def emit_observation" not in source
    assert "requests.post" not in source
