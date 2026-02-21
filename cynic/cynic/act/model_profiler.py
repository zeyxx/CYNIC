"""
CYNIC ModelProfiler — LLM performance profiles per task_type (T07)

Tracks latency, cost, quality (q_score), and model used per task type.
Feeds LLMRouter: after 5+ samples, profiler recommends cheapest model
that achieves WAG-level quality (q_score >= 61.8).

Rolling window: F(11)=89 entries per (model, task_type) pair.
Persists to ~/.cynic/model_profiles.json (lightweight JSON, no DB needed).

Usage:
    profiler = ModelProfiler()
    profiler.record("code_review", "haiku", latency_ms=450, cost_usd=0.001, q_score=72.0)
    profile = profiler.profile("code_review")
    # → {"best_model": "haiku", "avg_q": 72.0, "avg_latency_ms": 450, "samples": 1}
"""
from __future__ import annotations

import json
import logging
import os
import time
from collections import defaultdict
from typing import Any, List


from cynic.core.phi import WAG_MIN, fibonacci

logger = logging.getLogger("cynic.act.model_profiler")

_PROFILE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "model_profiles.json")
_MAX_SAMPLES   = fibonacci(11)  # 89 per (task_type, model) — BURN axiom

# Task types recognized for profiling
TASK_TYPES = frozenset({
    "code_review", "refactor", "debug", "explain", "generate",
    "test", "docs", "architecture", "security", "general",
})


class ModelSample:
    """One LLM call sample for a (task_type, model) pair."""
    __slots__ = ("task_type", "model", "latency_ms", "cost_usd", "q_score", "ts")

    def __init__(
        self,
        task_type: str,
        model: str,
        latency_ms: float,
        cost_usd: float,
        q_score: float,
        ts: float | None = None,
    ) -> None:
        self.task_type  = task_type
        self.model      = model
        self.latency_ms = latency_ms
        self.cost_usd   = cost_usd
        self.q_score    = q_score
        self.ts         = ts or time.time()

    def to_dict(self) -> dict[str, Any]:
        return {
            "task_type":  self.task_type,
            "model":      self.model,
            "latency_ms": self.latency_ms,
            "cost_usd":   self.cost_usd,
            "q_score":    self.q_score,
            "ts":         self.ts,
        }

    @staticmethod
    def from_dict(d: dict[str, Any]) -> ModelSample:
        return ModelSample(
            task_type  = d.get("task_type", "general"),
            model      = d.get("model", "unknown"),
            latency_ms = float(d.get("latency_ms", 0.0)),
            cost_usd   = float(d.get("cost_usd", 0.0)),
            q_score    = float(d.get("q_score", 50.0)),
            ts         = float(d.get("ts", time.time())),
        )


class ModelProfiler:
    """
    Tracks LLM performance by (task_type, model).

    After _MIN_SAMPLES per pair, profile() returns a recommendation.
    The recommended model minimises cost while achieving WAG-level quality.
    """

    _MIN_SAMPLES = 3  # F(4) — minimum samples before recommending

    def __init__(self, path: str | None = None) -> None:
        self._path = path or _PROFILE_PATH
        # _samples[task_type][model] = List[ModelSample]
        self._samples: dict[str, dict[str, list[ModelSample]]] = defaultdict(
            lambda: defaultdict(list)
        )
        self._total_recorded = 0
        self._load()

    # ── Recording ─────────────────────────────────────────────────────────────

    def record(
        self,
        task_type: str,
        model: str,
        latency_ms: float,
        cost_usd: float,
        q_score: float,
    ) -> None:
        """Record one LLM call result."""
        task_type = task_type.lower().replace(" ", "_")
        sample = ModelSample(
            task_type=task_type,
            model=model,
            latency_ms=max(0.0, latency_ms),
            cost_usd=max(0.0, cost_usd),
            q_score=max(0.0, min(100.0, q_score)),
        )
        bucket = self._samples[task_type][model]
        bucket.append(sample)
        # Rolling cap per (task_type, model) pair
        if len(bucket) > _MAX_SAMPLES:
            bucket[:] = bucket[-_MAX_SAMPLES:]

        self._total_recorded += 1
        if self._total_recorded % 21 == 0:  # F(8)
            self._save()

        logger.debug(
            "ModelProfiler: %s/%s q=%.1f lat=%.0fms cost=$%.4f",
            task_type, model, q_score, latency_ms, cost_usd,
        )

    # ── Querying ──────────────────────────────────────────────────────────────

    def profile(self, task_type: str) -> dict[str, Any]:
        """
        Summarise all models for a given task_type.

        Returns:
          {
            "task_type": str,
            "best_model": str,     # cheapest model with avg q >= WAG_MIN
            "models": {
              model: {
                "avg_q": float, "avg_latency_ms": float,
                "avg_cost_usd": float, "samples": int
              }
            },
            "recommendation_ready": bool,  # True if best_model has >= MIN_SAMPLES
          }
        """
        task_type = task_type.lower().replace(" ", "_")
        model_data = self._samples.get(task_type, {})

        summary: dict[str, Any] = {}
        for model, samples in model_data.items():
            if not samples:
                continue
            avg_q      = sum(s.q_score for s in samples) / len(samples)
            avg_lat    = sum(s.latency_ms for s in samples) / len(samples)
            avg_cost   = sum(s.cost_usd for s in samples) / len(samples)
            summary[model] = {
                "avg_q":          round(avg_q, 2),
                "avg_latency_ms": round(avg_lat, 1),
                "avg_cost_usd":   round(avg_cost, 6),
                "samples":        len(samples),
            }

        # Best model: cheapest model with avg_q >= WAG_MIN and >= MIN_SAMPLES
        best = self._pick_best(summary)
        ready = (
            best is not None
            and summary.get(best, {}).get("samples", 0) >= self._MIN_SAMPLES
        )
        return {
            "task_type":            task_type,
            "best_model":           best,
            "models":               summary,
            "recommendation_ready": ready,
        }

    def all_profiles(self) -> dict[str, Any]:
        """Return profile for every known task_type."""
        result = {}
        for task_type in list(self._samples.keys()):
            result[task_type] = self.profile(task_type)
        return result

    def stats(self) -> dict[str, Any]:
        task_count = len(self._samples)
        total_samples = sum(
            len(samples)
            for models in self._samples.values()
            for samples in models.values()
        )
        return {
            "total_recorded": self._total_recorded,
            "task_types":     task_count,
            "total_samples":  total_samples,
        }

    # ── Private ───────────────────────────────────────────────────────────────

    def _pick_best(self, summary: dict[str, Any]) -> str | None:
        """Pick the cheapest WAG-quality model. Falls back to best avg_q if none qualify."""
        qualified = [
            m for m, d in summary.items()
            if d["avg_q"] >= WAG_MIN and d["samples"] >= self._MIN_SAMPLES
        ]
        if qualified:
            return min(qualified, key=lambda m: summary[m]["avg_cost_usd"])
        # No model has WAG quality yet — return highest avg_q model
        if summary:
            return max(summary, key=lambda m: summary[m]["avg_q"])
        return None

    def _save(self) -> None:
        try:
            os.makedirs(os.path.dirname(self._path), exist_ok=True)
            data: dict[str, Any] = {}
            for task_type, models in self._samples.items():
                data[task_type] = {
                    model: [s.to_dict() for s in samples]
                    for model, samples in models.items()
                }
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump(data, fh, indent=2)
        except OSError as exc:
            logger.debug("ModelProfiler: save failed: %s", exc)

    def _load(self) -> None:
        try:
            if not os.path.exists(self._path):
                return
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            for task_type, models in data.items():
                for model, samples_raw in models.items():
                    self._samples[task_type][model] = [
                        ModelSample.from_dict(d) for d in samples_raw
                        if isinstance(d, dict)
                    ]
        except OSError as exc:
            logger.debug("ModelProfiler: load failed: %s", exc)
