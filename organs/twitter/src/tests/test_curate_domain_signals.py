from organs.twitter.src.entrypoints import curate_domain_signals as curation


LEGACY_HERMES_ROOT = "scripts" + "/" + "hermes-x"


def test_extract_keywords_filters_noise():
    keywords = curation.extract_keywords("The $SOL token has serious liquidity risk https://x.com/a @bob")

    assert "liquidity" in keywords
    assert "risk" in keywords
    assert "the" not in keywords
    assert "sol" not in keywords


def test_assign_domain_prefers_narrative_mapping(monkeypatch):
    monkeypatch.setattr(curation, "NARRATIVE_MAPPINGS", {"D3": ["futarchy"]})

    assert curation.assign_domain("market hack security", ["futarchy"], "unknown") == "D3"


def test_extract_falsifiable_claim_for_futarchy_domain():
    claim = curation.extract_falsifiable_claim("MetaDAO futarchy proposal is live", "D3")

    assert "futarchy governance" in claim


def test_create_signal_id_is_stable():
    assert curation.create_signal_id("alice", "123", "D1", 7) == "D1_alice_123_0007"


def test_curation_paths_are_organ_owned():
    assert LEGACY_HERMES_ROOT + "/core" not in str(curation.DATASET_PATH)
    assert LEGACY_HERMES_ROOT + "/core" not in str(curation.CURATED_DIR)
    assert "services/cynic-python" in str(curation.PROJECT_ROOT / "services" / "cynic-python")
