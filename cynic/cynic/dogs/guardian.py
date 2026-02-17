"""
CYNIC Guardian Dog — Gevurah (Strength/Severity)

Non-LLM Dog. L3 REFLEX (<10ms anomaly detection).
Technology: scikit-learn IsolationForest

Responsibilities:
  - Anomaly detection in code diffs, transactions, market data
  - Security pattern matching (injection, XSS, rug pulls)
  - Hard VETO when danger exceeds φ² threshold

GUARDIAN is the immune system. It cannot be overridden by votes.
A GUARDIAN veto BLOCKS execution regardless of quorum (PBFT protocol).

Confidence deliberately capped at φ⁻² = 38.2% —
even GUARDIAN can be wrong about anomalies.
"""
from __future__ import annotations

import logging
import time
from typing import Any, Dict, List, Optional

import numpy as np
from sklearn.ensemble import IsolationForest

from cynic.core.phi import PHI_INV, PHI_INV_2, PHI_2, MAX_Q_SCORE, phi_bound_score
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.guardian")

# Danger threshold: anomaly score above φ² triggers veto
VETO_THRESHOLD = PHI_2     # 2.618 — exceptional danger
WARN_THRESHOLD = 1.0       # φ⁰ — notable anomaly


class GuardianDog(AbstractDog):
    """
    Guardian (Gevurah) — IsolationForest anomaly detector.

    Trained incrementally as CYNIC observes normal behavior.
    New observations that deviate significantly → BARK + potential VETO.

    The IsolationForest learns what is "normal" for each reality dimension.
    When code, transactions, or market data are anomalous → GUARDIAN reacts.
    """

    def __init__(self) -> None:
        super().__init__(DogId.GUARDIAN)
        # Separate model per reality dimension
        self._models: Dict[str, IsolationForest] = {}
        self._training_data: Dict[str, List[List[float]]] = {}
        self._min_training_samples = 13  # F(7) — need enough data to train
        self._trained_realities: set = set()

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.GUARDIAN,
            sefirot="Gevurah — Strength/Severity",
            consciousness_min=ConsciousnessLevel.REFLEX,
            uses_llm=False,
            supported_realities={"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"},
            supported_analyses={"PERCEIVE", "JUDGE", "DECIDE"},
            technology="scikit-learn IsolationForest",
            max_concurrent=4,  # Can run 4 parallel anomaly checks (F(3)=2, F(4)=3, F(5)=5... use F(4)=3+1)
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Analyze a cell for anomalies. May issue VETO.

        Feature extraction from cell content:
          - If content is dict: extract numeric values as features
          - If content is str: use length, entropy, character frequencies
          - Otherwise: use novelty, complexity, risk from cell metadata
        """
        start = time.perf_counter()

        features = self._extract_features(cell)
        reality = cell.reality
        anomaly_score, is_anomaly, danger_level = self._score_anomaly(reality, features)

        # Update training data
        self._update_training(reality, features)

        # Determine q_score (inverted: high anomaly → low q_score)
        # Normal: q_score near MAX_Q_SCORE (61.8)
        # Anomalous: q_score near 0
        q_score = phi_bound_score(MAX_Q_SCORE * (1.0 - min(anomaly_score, 1.0)))

        # VETO if danger is extreme
        veto = danger_level >= VETO_THRESHOLD

        # GUARDIAN confidence: lower than other Dogs by design (immune systems have false positives)
        confidence = PHI_INV_2  # 38.2% — intentionally conservative

        reasoning = (
            f"Anomaly score: {anomaly_score:.3f} "
            f"({'ANOMALY' if is_anomaly else 'NORMAL'}, "
            f"danger={danger_level:.2f})"
        )
        if veto:
            reasoning = f"*GROWL* VETO — {reasoning}"
            logger.warning("GUARDIAN VETO on cell %s: %s", cell.cell_id, reasoning)

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=q_score,
            confidence=confidence,
            reasoning=reasoning,
            evidence={"anomaly_score": anomaly_score, "danger_level": danger_level, "features": features},
            latency_ms=latency,
            veto=veto,
        )
        self.record_judgment(judgment)
        return judgment

    def _extract_features(self, cell: Cell) -> List[float]:
        """Extract numeric features from cell content for anomaly detection."""
        features = [
            cell.novelty,
            cell.complexity,
            cell.risk,
            float(cell.lod),
            float(cell.consciousness),
        ]

        content = cell.content
        if isinstance(content, dict):
            # Extract all numeric values
            for v in content.values():
                if isinstance(v, (int, float)):
                    features.append(float(v))
        elif isinstance(content, str):
            # String features: length, unique chars ratio, digit ratio
            if content:
                features.append(min(len(content) / 10000.0, 1.0))
                features.append(len(set(content)) / max(len(content), 1))
                features.append(sum(c.isdigit() for c in content) / max(len(content), 1))
            else:
                features.extend([0.0, 0.0, 0.0])
        elif isinstance(content, (int, float)):
            features.append(float(content))

        # Pad or truncate to 8 features (F(6)=8 — Fibonacci-aligned)
        while len(features) < 8:
            features.append(0.0)
        return features[:8]

    def _score_anomaly(
        self,
        reality: str,
        features: List[float],
    ) -> tuple[float, bool, float]:
        """
        Score anomaly using IsolationForest.

        Returns: (anomaly_score [0,1], is_anomaly, danger_level)
        IsolationForest returns score in [-1, 0] where -1 = anomaly.
        We invert to [0, 1] where 1 = extreme anomaly.
        """
        model = self._models.get(reality)
        if model is None or reality not in self._trained_realities:
            # Not enough data → use risk feature (index 2) as best guess
            risk = features[2] if len(features) > 2 else 0.5
            return (risk, risk > 0.5, risk)

        X = np.array([features])
        raw_score = model.score_samples(X)[0]  # [-1, 0] range
        # Invert: 0 = normal, 1 = extreme anomaly
        anomaly_score = 1.0 - (raw_score + 1.0)  # maps [-1,0] → [0,1]
        anomaly_score = max(0.0, min(1.0, anomaly_score))
        is_anomaly = model.predict(X)[0] == -1

        # Scale danger by cell.risk (amplifies high-risk anomalies)
        danger_level = anomaly_score * (1 + float(features[2]))  # features[2] = risk

        return anomaly_score, is_anomaly, danger_level

    def _update_training(self, reality: str, features: List[float]) -> None:
        """Incrementally update training data and retrain if enough samples."""
        if reality not in self._training_data:
            self._training_data[reality] = []
        self._training_data[reality].append(features)

        samples = self._training_data[reality]
        if len(samples) >= self._min_training_samples and len(samples) % 5 == 0:
            # Retrain every 5 new samples (online incremental learning)
            X = np.array(samples[-89:])  # Keep last F(11)=89 samples
            model = IsolationForest(
                contamination=PHI_INV_2,  # 38.2% expected anomaly rate
                random_state=42,
                n_estimators=13,           # F(7) trees
            )
            model.fit(X)
            self._models[reality] = model
            self._trained_realities.add(reality)

    async def health_check(self) -> DogHealth:
        trained = len(self._trained_realities)
        return DogHealth(
            dog_id=self.dog_id,
            status=HealthStatus.HEALTHY if trained > 0 else HealthStatus.DEGRADED,
            latency_p50_ms=self.avg_latency_ms,
            details=f"Trained realities: {trained}/7, samples: {sum(len(v) for v in self._training_data.values())}",
        )

    # Expose features for Guardian access
    def _extract_features(self, cell: Cell) -> List[float]:  # type: ignore[override]
        features = [cell.novelty, cell.complexity, cell.risk, float(cell.lod), float(cell.consciousness)]
        content = cell.content
        if isinstance(content, dict):
            for v in content.values():
                if isinstance(v, (int, float)):
                    features.append(float(v))
        elif isinstance(content, str) and content:
            features.append(min(len(content) / 10000.0, 1.0))
            features.append(len(set(content)) / max(len(content), 1))
            features.append(sum(c.isdigit() for c in content) / max(len(content), 1))
        else:
            features.extend([0.0, 0.0, 0.0])
        while len(features) < 8:
            features.append(0.0)
        return features[:8]
