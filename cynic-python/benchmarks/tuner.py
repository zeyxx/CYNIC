#!/usr/bin/env python3
"""
Parameter tuner: analyze observations and propose llama-server optimizations.
Bridges hardware profiling + judgment quality to self-evolving inference organ.

Version: 0.1.0
"""

import json
import glob
import logging
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class TuningRecommendation:
    """A proposed parameter change and its rationale."""
    parameter: str  # e.g., "parallel", "ctx-size", "threads", "gpu-layers"
    current_value: int
    proposed_value: int
    rationale: str  # e.g., "reduce swap thrashing", "improve GPU utilization"
    expected_impact: str  # e.g., "latency: -20%", "success_rate: +5%"
    priority: str  # P0=critical, P1=high, P2=medium, P3=low


class HardwareAnalyzer:
    """Analyze hardware profiles to identify bottlenecks."""

    def __init__(self, profiles: List[Dict]):
        """
        Args:
            profiles: list of HardwareSnapshot dicts from hardware_profiler.py
        """
        self.profiles = profiles

    def detect_bottleneck(self) -> Optional[str]:
        """Identify primary bottleneck: CPU, memory, swap, thermal, GPU."""
        if not self.profiles:
            return None

        swap_percents = [p.get("swap_percent", 0) for p in self.profiles]
        memory_percents = [p.get("memory_percent", 0) for p in self.profiles]
        cpu_percents = [p.get("cpu_percent", 0) for p in self.profiles]
        thermals = [p.get("thermal_celsius") for p in self.profiles if p.get("thermal_celsius")]

        avg_swap = sum(swap_percents) / len(swap_percents) if swap_percents else 0
        avg_mem = sum(memory_percents) / len(memory_percents) if memory_percents else 0
        avg_cpu = sum(cpu_percents) / len(cpu_percents) if cpu_percents else 0

        # Priority: swap > memory > CPU > thermal
        if avg_swap > 30:
            return "swap_thrashing"
        elif avg_mem > 80:
            return "memory_pressure"
        elif avg_cpu > 75:
            return "cpu_saturation"
        elif thermals and max(thermals) > 85:
            return "thermal_throttling"
        else:
            return "none"


class JudgmentQualityAnalyzer:
    """Analyze judgment observations to detect quality degradation."""

    def __init__(self, observations: List[Dict]):
        """
        Args:
            observations: list of JudgeObservation dicts
        """
        self.observations = observations

    def success_rate(self) -> float:
        """Fraction of successful judgments (error=null)."""
        if not self.observations:
            return 0.0
        successful = sum(1 for o in self.observations if o.get("error") is None)
        return successful / len(self.observations)

    def mean_latency_ms(self) -> float:
        """Average judgment latency."""
        if not self.observations:
            return 0.0
        valid = [o.get("latency_ms", 0) for o in self.observations if o.get("error") is None]
        return sum(valid) / len(valid) if valid else 0.0

    def mean_q_score(self) -> float:
        """Average confidence score."""
        if not self.observations:
            return 0.0
        valid = [o.get("q_score", 0) for o in self.observations if o.get("q_score", -1) >= 0]
        return sum(valid) / len(valid) if valid else 0.0

    def consistency(self) -> float:
        """Q-score variance (lower = more consistent)."""
        if not self.observations:
            return 0.0
        valid = [o.get("q_score", 0) for o in self.observations if o.get("q_score", -1) >= 0]
        if len(valid) < 2:
            return 0.0

        mean = sum(valid) / len(valid)
        variance = sum((x - mean) ** 2 for x in valid) / len(valid)
        return variance ** 0.5  # stdev


class ParameterTuner:
    """Propose parameter adjustments based on observations."""

    def __init__(self, hw_analyzer: HardwareAnalyzer, jq_analyzer: JudgmentQualityAnalyzer,
                 current_config: Dict[str, int]):
        """
        Args:
            hw_analyzer: hardware bottleneck analyzer
            jq_analyzer: judgment quality analyzer
            current_config: dict with keys like "parallel", "ctx-size", "threads", "gpu-layers"
        """
        self.hw = hw_analyzer
        self.jq = jq_analyzer
        self.config = current_config

    def recommend(self) -> List[TuningRecommendation]:
        """Generate a sequence of tuning recommendations."""
        recs = []

        # Swap thrashing: reduce context size or parallel
        if self.hw.detect_bottleneck() == "swap_thrashing":
            ctx_size = self.config.get("ctx-size", 4096)
            if ctx_size > 2048:
                recs.append(TuningRecommendation(
                    parameter="ctx-size",
                    current_value=ctx_size,
                    proposed_value=ctx_size // 2,
                    rationale="swap_thrashing detected: reduce context size to cut memory footprint",
                    expected_impact="latency: stable, memory: -50%",
                    priority="P0",
                ))

        # Memory pressure: reduce parallel or ctx-size
        elif self.hw.detect_bottleneck() == "memory_pressure":
            parallel = self.config.get("parallel", 1)
            if parallel > 1:
                recs.append(TuningRecommendation(
                    parameter="parallel",
                    current_value=parallel,
                    proposed_value=1,
                    rationale="memory_pressure detected: disable concurrent requests",
                    expected_impact="latency: per-request stable, throughput: -parallel%",
                    priority="P1",
                ))

        # CPU saturation: reduce threads
        elif self.hw.detect_bottleneck() == "cpu_saturation":
            threads = self.config.get("threads", 8)
            if threads > 4:
                recs.append(TuningRecommendation(
                    parameter="threads",
                    current_value=threads,
                    proposed_value=threads // 2,
                    rationale="cpu_saturation detected: reduce thread count to minimize context switching",
                    expected_impact="latency: -10% to +5%, cpu_load: -40%",
                    priority="P1",
                ))

        # Poor success rate: check if parallel is too high
        if self.jq.success_rate() < 0.95:
            parallel = self.config.get("parallel", 1)
            if parallel > 1:
                recs.append(TuningRecommendation(
                    parameter="parallel",
                    current_value=parallel,
                    proposed_value=1,
                    rationale="low success rate: concurrent requests may be timing out",
                    expected_impact="success_rate: +100%",
                    priority="P0",
                ))

        # High latency: check GPU layers
        latency_ms = self.jq.mean_latency_ms()
        if latency_ms > 10000:  # >10s
            gpu_layers = self.config.get("gpu-layers", 10)
            if gpu_layers < 20:
                recs.append(TuningRecommendation(
                    parameter="gpu-layers",
                    current_value=gpu_layers,
                    proposed_value=gpu_layers + 5,
                    rationale="high latency: increase GPU offload to accelerate computation",
                    expected_impact="latency: -15% to +5% (depends on GPU capacity)",
                    priority="P2",
                ))

        return sorted(recs, key=lambda r: {"P0": 0, "P1": 1, "P2": 2, "P3": 3}[r.priority])

    def apply_recommendation(self, rec: TuningRecommendation) -> Dict[str, int]:
        """Apply a single recommendation and return updated config."""
        self.config[rec.parameter] = rec.proposed_value
        return self.config.copy()


def load_latest_observations(dog_name: str = "gemma-4-e4b",
                           observations_dir: str = "observations") -> Tuple[List[Dict], List[Dict]]:
    """
    Load latest hardware + judgment observations for a dog.

    Returns: (hardware_profiles, judge_observations)
    """
    import json

    # Find latest benchmark file for this dog
    pattern = f"{observations_dir}/{dog_name}-*.json"
    files = sorted(glob.glob(pattern))

    judge_obs = []
    hw_profiles = []

    if files:
        # Load latest judge observations
        latest = files[-1]
        with open(latest) as f:
            data = json.load(f)
            judge_obs = data.get("observations", [])

    # Load latest hardware profile
    hw_pattern = f"{observations_dir}/*-hwprofile-*.json"
    hw_files = sorted(glob.glob(hw_pattern))
    if hw_files:
        with open(hw_files[-1]) as f:
            hw_data = json.load(f)
            hw_profiles = hw_data.get("snapshots", [])

    return hw_profiles, judge_obs


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    # Load observations
    hw_profiles, judge_obs = load_latest_observations()

    # Current llama-server config (from previous tuning session)
    current_config = {
        "parallel": 1,
        "ctx-size": 4096,
        "threads": 4,
        "gpu-layers": 10,
    }

    # Analyze
    hw = HardwareAnalyzer(hw_profiles)
    jq = JudgmentQualityAnalyzer(judge_obs)

    logger.info(f"Hardware bottleneck: {hw.detect_bottleneck()}")
    logger.info(f"Success rate: {jq.success_rate():.1%}")
    logger.info(f"Mean latency: {jq.mean_latency_ms():.1f}ms")
    logger.info(f"Mean q-score: {jq.mean_q_score():.3f}")
    logger.info(f"Consistency: {jq.consistency():.3f}")

    # Recommend
    tuner = ParameterTuner(hw, jq, current_config)
    recommendations = tuner.recommend()

    if recommendations:
        logger.info("\nRecommendations:")
        for i, rec in enumerate(recommendations, 1):
            logger.info(f"  {i}. [{rec.priority}] {rec.parameter}: {rec.current_value} → {rec.proposed_value}")
            logger.info(f"     Rationale: {rec.rationale}")
            logger.info(f"     Expected impact: {rec.expected_impact}")
    else:
        logger.info("\nNo recommendations — config is well-tuned.")
