"""
CYNIC DogSoul Tests (β2)

Covers DogSoul cross-session identity memory:
  - Fresh start when no file exists
  - Load/save round-trip (YAML front matter)
  - update() running average
  - session counter
  - Top signals tracking
  - Error resilience (bad file, no write permission)

Uses tmp_path fixture — no real ~/.cynic/ writes.
"""
from __future__ import annotations

import pytest
from pathlib import Path

from cynic.core.soul import DogSoul, _parse_front_matter, _render_front_matter


# ── Front matter helpers ──────────────────────────────────────────────────

class TestFrontMatterHelpers:
    def test_render_and_parse_round_trip(self):
        fields = {
            "dog_id": "SCHOLAR",
            "total_judgments": 42,
            "avg_q_score": 55.3,
            "session_count": 3,
            "last_seen": "2026-02-18T10:00:00",
            "top_signals": ["clean code (7×)", "type hints (3×)"],
        }
        rendered = _render_front_matter(fields)
        parsed, _ = _parse_front_matter(rendered + "\nbody text")
        assert parsed["dog_id"] == "SCHOLAR"
        assert parsed["total_judgments"] == 42
        assert abs(parsed["avg_q_score"] - 55.3) < 0.01
        assert parsed["session_count"] == 3
        assert len(parsed["top_signals"]) == 2
        assert "clean code (7×)" in parsed["top_signals"]

    def test_parse_no_front_matter_returns_empty(self):
        text = "# Just markdown\nno front matter"
        fields, body = _parse_front_matter(text)
        assert fields == {}
        assert "Just markdown" in body

    def test_parse_unclosed_front_matter_returns_empty(self):
        text = "---\ndog_id: FOO\n# no closing ---"
        fields, _ = _parse_front_matter(text)
        assert fields == {}

    def test_render_list_field(self):
        fields = {"top_signals": ["a (1×)", "b (2×)"]}
        rendered = _render_front_matter(fields)
        assert "top_signals:" in rendered
        assert '- "a (1×)"' in rendered

    def test_render_float_precision(self):
        fields = {"avg_q_score": 52.381}
        rendered = _render_front_matter(fields)
        # Should not have excessive decimals
        assert "52.38" in rendered


# ── DogSoul core ─────────────────────────────────────────────────────────

class TestDogSoulFreshStart:
    def test_fresh_soul_has_zero_stats(self, tmp_path):
        soul = DogSoul.load("GUARDIAN", soul_root=tmp_path)
        assert soul.total_judgments == 0
        assert soul.avg_q_score == 0.0
        assert soul.session_count == 0
        assert soul.top_signals == []

    def test_dog_id_uppercased(self, tmp_path):
        soul = DogSoul.load("scholar", soul_root=tmp_path)
        assert soul.dog_id == "SCHOLAR"

    def test_path_is_under_soul_root(self, tmp_path):
        soul = DogSoul("JANITOR", soul_root=tmp_path)
        assert soul.path == tmp_path / "janitor" / "soul.md"


class TestDogSoulUpdate:
    def test_update_increments_total_judgments(self, tmp_path):
        soul = DogSoul("GUARDIAN", soul_root=tmp_path)
        soul.update(q_score=70.0)
        assert soul.total_judgments == 1

    def test_update_computes_running_average(self, tmp_path):
        soul = DogSoul("GUARDIAN", soul_root=tmp_path)
        soul.update(q_score=60.0)
        soul.update(q_score=80.0)
        assert abs(soul.avg_q_score - 70.0) < 0.01

    def test_update_running_avg_multiple(self, tmp_path):
        soul = DogSoul("SAGE", soul_root=tmp_path)
        for i in range(5):
            soul.update(q_score=50.0)
        assert abs(soul.avg_q_score - 50.0) < 0.01
        assert soul.total_judgments == 5

    def test_update_tracks_signals(self, tmp_path):
        soul = DogSoul("SCHOLAR", soul_root=tmp_path)
        soul.update(q_score=65.0, signals=["clean code", "type hints"])
        soul.update(q_score=55.0, signals=["clean code"])
        # "clean code" hit 2×, "type hints" 1×
        assert any("clean code" in s for s in soul.top_signals)
        assert any("2×" in s for s in soul.top_signals)

    def test_top_signals_capped_at_seven(self, tmp_path):
        soul = DogSoul("ANALYST", soul_root=tmp_path)
        for i in range(10):
            soul.update(q_score=50.0, signals=[f"signal_{i}"])
        assert len(soul.top_signals) <= 7

    def test_on_session_start_increments(self, tmp_path):
        soul = DogSoul("ORACLE", soul_root=tmp_path)
        soul.on_session_start()
        soul.on_session_start()
        assert soul.session_count == 2


class TestDogSoulPersistence:
    def test_save_creates_file(self, tmp_path):
        soul = DogSoul("GUARDIAN", soul_root=tmp_path)
        soul.update(q_score=75.0)
        soul.save()
        assert soul.path.exists()

    def test_save_load_round_trip(self, tmp_path):
        soul = DogSoul("ARCHITECT", soul_root=tmp_path)
        soul.update(q_score=60.0)
        soul.update(q_score=80.0)
        soul.on_session_start()
        soul.save()

        loaded = DogSoul.load("ARCHITECT", soul_root=tmp_path)
        assert loaded.total_judgments == 2
        assert abs(loaded.avg_q_score - 70.0) < 0.1
        assert loaded.session_count == 1

    def test_load_missing_file_returns_fresh(self, tmp_path):
        soul = DogSoul.load("DEPLOYER", soul_root=tmp_path)
        assert soul.total_judgments == 0
        assert soul.dog_id == "DEPLOYER"

    def test_last_seen_set_on_save(self, tmp_path):
        soul = DogSoul("SCOUT", soul_root=tmp_path)
        soul.save()
        assert soul.last_seen != ""
        assert "2026" in soul.last_seen or "2025" in soul.last_seen

    def test_load_corrupted_file_returns_fresh(self, tmp_path):
        """Corrupted soul.md → falls back to fresh soul, no crash."""
        dog_dir = tmp_path / "scholar"
        dog_dir.mkdir(parents=True)
        (dog_dir / "soul.md").write_text("<<<not valid yaml>>>", encoding="utf-8")
        soul = DogSoul.load("SCHOLAR", soul_root=tmp_path)
        assert soul.total_judgments == 0  # Fresh start

    def test_to_dict_returns_expected_keys(self, tmp_path):
        soul = DogSoul("JANITOR", soul_root=tmp_path)
        soul.update(q_score=45.0)
        d = soul.to_dict()
        assert "dog_id" in d
        assert "total_judgments" in d
        assert "avg_q_score" in d
        assert "session_count" in d
        assert "top_signals" in d

    def test_accumulated_signals_preserved_across_load(self, tmp_path):
        """Signals saved and reloaded from top_signals list."""
        soul = DogSoul("CARTOGRAPHER", soul_root=tmp_path)
        soul.update(q_score=70.0, signals=["graph traversal", "topology"])
        soul.update(q_score=65.0, signals=["graph traversal"])
        soul.save()

        loaded = DogSoul.load("CARTOGRAPHER", soul_root=tmp_path)
        assert any("graph traversal" in s for s in loaded.top_signals)
