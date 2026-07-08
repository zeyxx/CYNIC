import json

from organs.twitter.src.entrypoints import search_generation


def test_extract_domains_from_skill_builds_queries():
    queries = search_generation.extract_domains_from_skill(
        """## Domain: Talaria Watch
- Track "$TALARIA" launch mentions
- Watch @MetaDAOProject announcements
- Phrase "futarchy launch"
"""
    )

    values = {query["query"] for query in queries}
    assert "$TALARIA solana" in values
    assert "@MetaDAOProject crypto" in values
    assert "futarchy launch" in values
    assert {query["domain"] for query in queries} == {"talaria_watch"}


def test_apply_yield_weights_boosts_high_yield_domain():
    queries = [{"query": "a", "domain": "security", "weight": 1.0}]
    weighted = search_generation.apply_yield_weights(
        queries,
        {"domains": {"security": {"signals": 30, "pct_of_total": 30}}},
    )

    assert weighted[0]["weight"] == 1.3


def test_deduplicate_removes_existing_queries_case_insensitive():
    new = [{"query": "Solana Rug Pull"}, {"query": "new signal"}]
    existing = [{"query": "solana rug pull"}]

    assert search_generation.deduplicate(new, existing) == [{"query": "new signal"}]


def test_write_and_load_tasks_round_trip(tmp_path):
    path = tmp_path / "search_tasks.jsonl"
    written = search_generation.write_tasks([{"query": "solana", "domain": "D1"}], path)

    assert written == 1
    loaded = search_generation.load_existing_tasks(path)
    assert loaded[0]["query"] == "solana"
    assert loaded[0]["status"] == "pending"
    assert "generated_at" in loaded[0]


def test_load_existing_tasks_skips_invalid_json(tmp_path):
    path = tmp_path / "search_tasks.jsonl"
    path.write_text(json.dumps({"query": "ok"}) + "\nnot-json\n")

    assert search_generation.load_existing_tasks(path) == [{"query": "ok"}]
