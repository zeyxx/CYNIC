"""
CYNIC Axiomatic Benchmarker - 9 Lenses Evaluation.
Records objective performance across the 9 Engineering Lenses.
"""
from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, asdict, field
from pathlib import Path
from typing import Dict, List, Optional
from cynic.kernel.core.mathematics import MultiDimensionalMathematics

logger = logging.getLogger("cynic.organism.benchmarking")

@dataclass
class LensesScore:
    ai_infra: float = 0.5
    backend: float = 0.5
    ml_platform: float = 0.5
    data_engineer: float = 0.5
    security: float = 0.5
    sre: float = 0.5
    blockchain: float = 0.5
    robotics: float = 0.5
    solutions_architect: float = 0.5

    def to_vector(self) -> List[float]:
        return [
            self.ai_infra, self.backend, self.ml_platform, 
            self.data_engineer, self.security, self.sre, 
            self.blockchain, self.robotics, self.solutions_architect
        ]

@dataclass
class BenchmarkMetric:
    model_id: str
    axiom: str
    tokens_per_sec: float
    total_latency_ms: float
    lenses: LensesScore = field(default_factory=LensesScore)
    timestamp: float = field(default_factory=time.time)

    @property
    def phi_quality_score(self) -> float:
        """The geometric/euclidean truth of the 9 lenses."""
        return MultiDimensionalMathematics.l2_norm(self.lenses.to_vector())

class AxiomaticBenchmarker:
    def __init__(self, storage_path: str = "audit/benchmarks.jsonl"):
        self.path = Path(storage_path)
        self.path.parent.mkdir(parents=True, exist_ok=True)

    def record_metric(self, metric: BenchmarkMetric):
        data = asdict(metric)
        data["phi_quality_score"] = metric.phi_quality_score
        try:
            with open(self.path, "a") as f:
                f.write(json.dumps(data) + "\n")
            logger.info(f"Benchmarker: Recorded L2 Phi {metric.phi_quality_score:.4f} for {metric.model_id}")
        except Exception as e:
            logger.error(f"Benchmarker: Failed to record: {e}")

    def get_average_performance(self, model_id: str, axiom: str) -> Dict[str, float]:
        metrics = []
        if self.path.exists():
            with open(self.path, "r") as f:
                for line in f:
                    try:
                        data = json.loads(line)
                        if data["model_id"] == model_id and data["axiom"] == axiom:
                            metrics.append(data)
                    except: pass
        
        if not metrics:
            return {"avg_tps": 0.0, "avg_phi": 0.5}
            
        avg_tps = sum(m["tokens_per_sec"] for m in metrics) / len(metrics)
        avg_phi = sum(m["phi_quality_score"] for m in metrics) / len(metrics)
        
        return {"avg_tps": avg_tps, "avg_phi": avg_phi}

_benchmarker: Optional[AxiomaticBenchmarker] = None
def get_benchmarker() -> AxiomaticBenchmarker:
    global _benchmarker
    if _benchmarker is None: _benchmarker = AxiomaticBenchmarker()
    return _benchmarker
