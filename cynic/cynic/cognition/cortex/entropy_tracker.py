"""
EntropyTracker — Measure Information Density of Judgments

Principle: Every decision should reduce information entropy
  H(input)  = entropy of observations before judgment
  H(output) = entropy of judgment (based on confidence)
  Efficiency = H(input) - H(output) > 0

If efficiency ≤ 0: System is adding noise, not creating knowledge.
This detects:
  - Dead judgment paths (judgments that don't compress info)
  - Biased dogs (always output same verdict regardless of input)
  - Learning failures (confidence doesn't correlate with input diversity)

Formula:
  H(X) = -Σ p(x) log₂(p(x))  [Shannon entropy]

  H(input):
    - Count unique signals in observation set
    - Probability = frequency of signal type

  H(output):
    - Binary: verdict or confidence level?
    - For judgment: H ∝ -confidence*log₂(confidence) - (1-confidence)*log₂(1-confidence)
    - Low confidence → high entropy (uncertain)
    - High confidence → low entropy (certain)

  Efficiency = H(input) - H(output)
    - >0: Good (compressed observation → certain decision)
    - ≤0: Bad (observations didn't guide decision)
"""
from __future__ import annotations

import math
import logging
from dataclasses import dataclass, field
from typing import Any, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.cognition.neurons.dog_state import DogState

from cynic.core.formulas import CHAT_MESSAGE_CAP

logger = logging.getLogger("cynic.cognition.cortex.entropy_tracker")


# ═══════════════════════════════════════════════════════════════════════════
# ENTROPY MEASUREMENT
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class EntropyMetrics:
    """
    Information density metrics for one judgment.

    Tracks:
      - H_input: Entropy of observations before judgment
      - H_output: Entropy of judgment (based on confidence)
      - efficiency: H_input - H_output (information compression)
      - signal_count: How many observations fed into judgment
      - verdict: What verdict was reached
      - confidence: How certain about the verdict
    """
    dog_id: str
    cell_id: str
    h_input: float  # Entropy of observations
    h_output: float  # Entropy of judgment
    efficiency: float  # Compression ratio (input - output)
    signal_count: int  # How many signals observed
    verdict: str  # BARK/GROWL/WAG/HOWL
    confidence: float  # [0, 0.618] φ-bounded
    timestamp: float = field(default_factory=lambda: __import__("time").time())

    @property
    def is_efficient(self, threshold: float = 0.0) -> bool:
        """Is this judgment creating order (efficiency > threshold)?"""
        return self.efficiency > threshold

    def to_dict(self) -> dict[str, Any]:
        """Serialize for storage or analysis."""
        return {
            "dog_id": self.dog_id,
            "cell_id": self.cell_id,
            "h_input": round(self.h_input, 3),
            "h_output": round(self.h_output, 3),
            "efficiency": round(self.efficiency, 3),
            "signal_count": self.signal_count,
            "verdict": self.verdict,
            "confidence": round(self.confidence, 3),
            "timestamp": self.timestamp,
        }


# ═══════════════════════════════════════════════════════════════════════════
# ENTROPY CALCULATOR
# ═══════════════════════════════════════════════════════════════════════════


class EntropyCalculator:
    """Calculate Shannon entropy for observations and judgments."""

    @staticmethod
    def calculate_observation_entropy(signals: list[dict[str, Any]]) -> float:
        """
        Calculate entropy of observation set.

        H(input) = -Σ p(type) * log₂(p(type))

        Where p(type) = frequency of each signal type.

        **Args**:
          signals: List of observed signals (each with 'type' or 'category')

        **Returns**:
          H in bits (0 = all same type, max = all different types)
        """
        if not signals:
            return 0.0

        # Count signal types
        type_counts: dict[str, int] = {}
        for sig in signals:
            sig_type = sig.get("type", sig.get("category", "unknown"))
            type_counts[sig_type] = type_counts.get(sig_type, 0) + 1

        # Calculate probabilities and entropy
        n = len(signals)
        h = 0.0
        for count in type_counts.values():
            p = count / n
            if p > 0:
                h -= p * math.log2(p)

        return h

    @staticmethod
    def calculate_confidence_entropy(confidence: float) -> float:
        """
        Calculate entropy of judgment based on confidence.

        H(output) = -[p*log₂(p) + (1-p)*log₂(1-p)]

        Where p = confidence in the verdict.

        High confidence → low entropy (judgment is certain)
        Low confidence → high entropy (judgment is uncertain)

        **Args**:
          confidence: Certainty in judgment [0, 1]

        **Returns**:
          H in bits (0 = certain, 1.0 = 50/50 uncertain)
        """
        if confidence <= 0.0 or confidence >= 1.0:
            return 0.0  # Certain judgment

        p = confidence
        h = -(p * math.log2(p) + (1 - p) * math.log2(1 - p))
        return h

    @staticmethod
    def calculate_efficiency(h_input: float, h_output: float) -> float:
        """
        Calculate information compression ratio.

        efficiency = h_input - h_output

        Positive: Good (observation entropy reduced by judgment)
        Negative: Bad (judgment added entropy, not reduced it)
        Zero: Neutral (judgment didn't compress info)

        **Returns**:
          Efficiency in bits (can be negative)
        """
        return h_input - h_output


# ═══════════════════════════════════════════════════════════════════════════
# ENTROPY TRACKER
# ═══════════════════════════════════════════════════════════════════════════


class EntropyTracker:
    """
    Track information density across all judgments.

    Accumulates metrics:
    - Average efficiency (H_input - H_output)
    - Min/max efficiency
    - Judgments with negative efficiency (noise generators)
    - Alert threshold: if avg_efficiency < 0, system is adding noise
    """

    def __init__(self, max_history: int = CHAT_MESSAGE_CAP) -> None:  # F(11) = 89 (imported from formulas.py)
        self.max_history = max_history
        self.metrics: list[EntropyMetrics] = []
        self.calculator = EntropyCalculator()

    async def track_judgment(
        self,
        dog_id: str,
        cell_id: str,
        signals: list[dict[str, Any]],
        verdict: str,
        confidence: float,
    ) -> EntropyMetrics:
        """
        Calculate and track entropy metrics for a judgment.

        **Args**:
          dog_id: Which dog made the judgment
          cell_id: Which cell was judged
          signals: Observations that led to judgment
          verdict: BARK/GROWL/WAG/HOWL
          confidence: Certainty [0, 0.618]

        **Returns**:
          EntropyMetrics with h_input, h_output, efficiency
        """
        # Calculate entropy of inputs
        h_input = self.calculator.calculate_observation_entropy(signals)

        # Calculate entropy of output (based on confidence)
        h_output = self.calculator.calculate_confidence_entropy(confidence)

        # Calculate efficiency
        efficiency = self.calculator.calculate_efficiency(h_input, h_output)

        # Create metrics record
        metrics = EntropyMetrics(
            dog_id=dog_id,
            cell_id=cell_id,
            h_input=h_input,
            h_output=h_output,
            efficiency=efficiency,
            signal_count=len(signals),
            verdict=verdict,
            confidence=confidence,
        )

        # Store in history (bounded cap)
        self.metrics.append(metrics)
        if len(self.metrics) > self.max_history:
            self.metrics.pop(0)

        # Log alert if efficiency is poor
        if efficiency <= 0.0:
            logger.warning(
                f"[{dog_id}] Low efficiency on {cell_id}: "
                f"H_in={h_input:.2f}, H_out={h_output:.2f}, eff={efficiency:.2f}"
            )
        elif efficiency > 1.0:
            logger.debug(
                f"[{dog_id}] High efficiency on {cell_id}: "
                f"eff={efficiency:.2f} (created {efficiency:.2f} bits of order)"
            )

        return metrics

    def get_statistics(self) -> dict[str, Any]:
        """
        Get summary statistics across all tracked judgments.

        **Returns**:
          Dictionary with:
            - avg_efficiency: Mean compression ratio
            - min_efficiency: Worst judgment
            - max_efficiency: Best judgment
            - neg_efficiency_count: How many judgments added noise
            - verdict_distribution: Count by verdict
            - alert: Is avg_efficiency < 0? (system adding noise)
        """
        if not self.metrics:
            return {
                "avg_efficiency": 0.0,
                "min_efficiency": 0.0,
                "max_efficiency": 0.0,
                "neg_efficiency_count": 0,
                "verdict_distribution": {},
                "alert": False,
            }

        efficiencies = [m.efficiency for m in self.metrics]
        verdicts = {}
        for m in self.metrics:
            verdicts[m.verdict] = verdicts.get(m.verdict, 0) + 1

        avg_eff = sum(efficiencies) / len(efficiencies)
        neg_count = sum(1 for e in efficiencies if e <= 0.0)

        return {
            "avg_efficiency": round(avg_eff, 3),
            "min_efficiency": round(min(efficiencies), 3),
            "max_efficiency": round(max(efficiencies), 3),
            "neg_efficiency_count": neg_count,
            "neg_efficiency_pct": round(neg_count / len(self.metrics) * 100, 1),
            "verdict_distribution": verdicts,
            "alert": avg_eff < 0.0,
            "total_tracked": len(self.metrics),
        }

    def get_dog_efficiency(self, dog_id: str) -> dict[str, Any]:
        """Get efficiency statistics for a specific dog."""
        dog_metrics = [m for m in self.metrics if m.dog_id == dog_id]

        if not dog_metrics:
            return {"dog_id": dog_id, "tracked": 0}

        efficiencies = [m.efficiency for m in dog_metrics]
        avg_eff = sum(efficiencies) / len(efficiencies)

        return {
            "dog_id": dog_id,
            "tracked": len(dog_metrics),
            "avg_efficiency": round(avg_eff, 3),
            "min_efficiency": round(min(efficiencies), 3),
            "max_efficiency": round(max(efficiencies), 3),
            "high_efficiency_count": sum(1 for e in efficiencies if e > 1.0),
            "low_efficiency_count": sum(1 for e in efficiencies if e <= 0.0),
        }

    def get_worst_judgments(self, limit: int = 5) -> list[EntropyMetrics]:
        """
        Get judgments with lowest efficiency (most noise).

        Useful for identifying which dogs/cells are problematic.

        **Returns**:
          List of EntropyMetrics sorted by efficiency (worst first)
        """
        sorted_metrics = sorted(self.metrics, key=lambda m: m.efficiency)
        return sorted_metrics[:limit]

    def get_best_judgments(self, limit: int = 5) -> list[EntropyMetrics]:
        """
        Get judgments with highest efficiency (most knowledge).

        **Returns**:
          List of EntropyMetrics sorted by efficiency (best first)
        """
        sorted_metrics = sorted(self.metrics, key=lambda m: m.efficiency, reverse=True)
        return sorted_metrics[:limit]
