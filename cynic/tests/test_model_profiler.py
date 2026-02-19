"""
CYNIC ModelProfiler Tests (T07)

Tests LLM performance profiling per (task_type, model) pair.
No LLM, no DB — pure in-memory.
"""
from __future__ import annotations

import os
import json
import tempfile
import pytest
from cynic.act.model_profiler import ModelProfiler, ModelSample, TASK_TYPES
from cynic.core.phi import WAG_MIN


class TestModelSample:
    def test_create_defaults(self):
        s = ModelSample("code_review", "haiku", 400.0, 0.001, 72.0)
        assert s.task_type == "code_review"
        assert s.model == "haiku"
        assert s.latency_ms == 400.0
        assert s.cost_usd == 0.001
        assert s.q_score == 72.0
        assert s.ts > 0

    def test_to_dict_round_trip(self):
        s = ModelSample("debug", "ollama", 120.0, 0.0, 55.0, ts=1000.0)
        d = s.to_dict()
        s2 = ModelSample.from_dict(d)
        assert s2.task_type == s.task_type
        assert s2.model == s.model
        assert abs(s2.q_score - s.q_score) < 0.01

    def test_from_dict_defaults(self):
        s = ModelSample.from_dict({})
        assert s.task_type == "general"
        assert s.model == "unknown"
        assert s.q_score == 50.0


class TestModelProfilerRecord:
    def test_record_adds_sample(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("code_review", "haiku", 400.0, 0.001, 72.0)
        profile = p.profile("code_review")
        assert "haiku" in profile["models"]
        assert profile["models"]["haiku"]["samples"] == 1

    def test_record_normalises_task_type(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("Code Review", "haiku", 400.0, 0.001, 72.0)  # spaces + capital
        result = p.profile("code_review")
        assert result["task_type"] == "code_review"
        assert "haiku" in result["models"]  # model is present under normalised key

    def test_record_clamps_q_score(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("debug", "haiku", 100.0, 0.001, 999.0)  # way above 100
        assert p.profile("debug")["models"]["haiku"]["avg_q"] == 100.0

    def test_rolling_cap_at_89(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        for i in range(100):
            p.record("general", "haiku", 100.0, 0.001, 50.0)
        assert p.profile("general")["models"]["haiku"]["samples"] == 89

    def test_multiple_models_tracked_separately(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("debug", "haiku", 100.0, 0.001, 70.0)
        p.record("debug", "ollama", 50.0, 0.0, 55.0)
        models = p.profile("debug")["models"]
        assert "haiku" in models
        assert "ollama" in models
        assert models["haiku"]["avg_cost_usd"] > models["ollama"]["avg_cost_usd"]


class TestModelProfilerRecommendation:
    def test_no_recommendation_before_min_samples(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("refactor", "haiku", 400.0, 0.001, 72.0)  # 1 sample < 3 MIN
        profile = p.profile("refactor")
        assert profile["recommendation_ready"] is False

    def test_recommendation_ready_after_min_samples(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        for _ in range(3):
            p.record("refactor", "haiku", 400.0, 0.001, 72.0)
        profile = p.profile("refactor")
        assert profile["recommendation_ready"] is True
        assert profile["best_model"] == "haiku"

    def test_best_model_requires_wag_quality(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        # haiku: q < WAG_MIN (61.8) — below threshold
        for _ in range(3):
            p.record("test", "haiku", 100.0, 0.001, 50.0)
        # ollama: q >= WAG_MIN
        for _ in range(3):
            p.record("test", "ollama", 200.0, 0.0, 70.0)
        profile = p.profile("test")
        # ollama qualifies (WAG+), haiku doesn't
        assert profile["best_model"] == "ollama"

    def test_picks_cheapest_among_qualified(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        # Two models both WAG+ quality — haiku is cheaper
        for _ in range(3):
            p.record("explain", "opus", 2000.0, 0.10, 85.0)
        for _ in range(3):
            p.record("explain", "haiku", 400.0, 0.001, 78.0)
        profile = p.profile("explain")
        assert profile["best_model"] == "haiku"  # cheaper WAG-quality wins

    def test_fallback_to_best_q_when_none_qualify(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        # Both below WAG_MIN — pick the one with higher avg_q
        for _ in range(3):
            p.record("docs", "haiku", 100.0, 0.001, 40.0)
        for _ in range(3):
            p.record("docs", "ollama", 100.0, 0.0, 55.0)
        profile = p.profile("docs")
        assert profile["best_model"] == "ollama"  # higher avg_q wins


class TestModelProfilerPersistence:
    def test_save_and_reload(self, tmp_path):
        path = str(tmp_path / "profiles.json")
        p = ModelProfiler(path=path)
        for _ in range(5):
            p.record("architecture", "haiku", 500.0, 0.002, 75.0)
        p._save()

        p2 = ModelProfiler(path=path)
        profile = p2.profile("architecture")
        assert profile["models"]["haiku"]["samples"] == 5

    def test_load_tolerates_missing_file(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "nonexistent.json"))
        assert p.stats()["total_samples"] == 0

    def test_stats(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("debug", "haiku", 100.0, 0.001, 70.0)
        p.record("refactor", "ollama", 200.0, 0.0, 60.0)
        s = p.stats()
        assert s["task_types"] == 2
        assert s["total_samples"] == 2

    def test_all_profiles_returns_each_task_type(self, tmp_path):
        p = ModelProfiler(path=str(tmp_path / "profiles.json"))
        p.record("debug", "haiku", 100.0, 0.001, 70.0)
        p.record("test", "ollama", 200.0, 0.0, 60.0)
        all_p = p.all_profiles()
        assert "debug" in all_p
        assert "test" in all_p
