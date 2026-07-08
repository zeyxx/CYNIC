from organs.twitter.src.core.ingest_signals import assign_domain, build_social_stimulus


def test_assign_domain_prefers_narrative_mapping_over_keywords():
    tweet = {
        "text": "solana infrastructure discussion",
        "narratives": ["attention-market"],
    }
    domains = {"D1": {"keywords": ["solana"]}}
    narrative_mappings = {"D4": ["attention-market"]}

    assert assign_domain(tweet, domains, narrative_mappings) == "D4"


def test_assign_domain_uses_keywords_when_no_narrative_match():
    tweet = {"text": "new solana validator client", "narratives": []}
    domains = {"D2": {"keywords": ["validator"]}}

    assert assign_domain(tweet, domains, {}) == "D2"


def test_assign_domain_defaults_to_d1():
    assert assign_domain({"text": "unclassified", "narratives": []}, {}, {}) == "D1"


def test_build_social_stimulus_contains_dog_contract_sections():
    stimulus = build_social_stimulus(
        {
            "text": "Useful $SOL signal",
            "author_screen_name": "alice",
            "signal_score": 5,
            "author_tier": "influencer",
            "cashtags": ["SOL"],
            "narratives": ["analysis"],
            "views": 12000,
            "likes": 42,
            "replies": 7,
            "bookmarks": 3,
            "author_followers_count": 100000,
            "engagement_rate": 0.0123,
            "interaction_type": "original",
            "author_verified": True,
            "lang": "en",
        },
        "D2",
    )

    assert "[DOMAIN: D2]" in stimulus
    assert "[METRICS]" in stimulus
    assert "author: @alice" in stimulus
    assert "cashtags: SOL" in stimulus
    assert "narratives: analysis" in stimulus
    assert "[AXIOM EVIDENCE]" in stimulus
    assert "[QUESTION]" in stimulus
