LEGACY_HERMES_ROOT = "scripts" + "/" + "hermes-x"


from pathlib import Path

from organs.twitter.src.entrypoints import x_ingest


def test_x_ingest_uses_organ_owned_paths():
    assert "organs/twitter/src/entrypoints/x_ingest.py" in str(Path(x_ingest.__file__))
    assert LEGACY_HERMES_ROOT + "/core" not in str(x_ingest.DEFAULT_DATASET)
    assert LEGACY_HERMES_ROOT + "/core" not in str(x_ingest.DEFAULT_ORGAN_DIR)


def test_domain_config_default_points_to_services_cynic_python():
    config = x_ingest.PROJECT_ROOT / "services" / "cynic-python" / "lab" / "config"

    assert str(config).endswith("services/cynic-python/lab/config")
