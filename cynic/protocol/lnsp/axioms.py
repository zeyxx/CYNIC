"""Layer 3: Axiom Evaluators for Judgment

Axioms are fundamental principles that guide CYNIC's judgment. Each axiom scorer
evaluates state against a principle and returns a score (0.0-1.0).

Implemented Axioms:
- FIDELITY: Observations match expected range
- PHI: Golden ratio balance and harmony
- VERIFY: Multiple sources agree
- CULTURE: State respects community norms
- BURN: No extraction or waste
"""
from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class AxiomEvaluator(ABC):
    """Abstract base class for axiom evaluators.

    Each axiom scorer evaluates aggregated state against a fundamental principle
    and returns a confidence score between 0.0 and 1.0.

    Attributes:
        axiom_name: Unique identifier for this axiom (e.g., "FIDELITY")
    """

    axiom_name: str

    @abstractmethod
    async def score(self, state: dict[str, Any]) -> float:
        """Score state against this axiom.

        Args:
            state: Aggregated state dictionary to evaluate

        Returns:
            Float between 0.0 and 1.0, where:
            - 0.0 = completely violates axiom
            - 0.5 = neutral/uncertain
            - 1.0 = perfectly satisfies axiom
        """
        pass


class FidelityEvaluator(AxiomEvaluator):
    """FIDELITY Axiom: Observations match expected range.

    Observations should fall within reasonable baselines. This axiom checks
    whether numeric values in state are within expected bounds (0-100 range
    for demonstration purposes).

    Score:
        - Ratio of in-range numeric values to total numeric values
        - Returns 1.0 if all values in expected range
        - Returns 0.0 if all values out of range
    """

    axiom_name = "FIDELITY"
    _baseline_min = 0.0
    _baseline_max = 100.0

    async def score(self, state: dict[str, Any]) -> float:
        """Evaluate state fidelity against expected baseline.

        Args:
            state: State dictionary to check

        Returns:
            Ratio of in-range numeric values to total numeric values (0.0-1.0)
        """
        if not state:
            return 0.5  # Neutral for empty state

        numeric_values = [
            v for v in state.values()
            if isinstance(v, int | float) and not isinstance(v, bool)
        ]

        if not numeric_values:
            return 0.5  # Neutral if no numeric values to check

        in_range_count = sum(
            1 for v in numeric_values
            if self._baseline_min <= v <= self._baseline_max
        )

        return in_range_count / len(numeric_values)


class PhiEvaluator(AxiomEvaluator):
    """PHI Axiom: Golden ratio balance and harmony.

    The golden ratio (φ = 1.618) represents perfect balance and harmony.
    This axiom checks if value distribution approaches this ratio.

    Score:
        - Based on proximity of value ratios to golden ratio
        - Returns higher scores when values are balanced
        - Returns 1.0 for perfect harmony
    """

    axiom_name = "PHI"
    _golden_ratio = 1.618

    async def score(self, state: dict[str, Any]) -> float:
        """Evaluate state harmony using golden ratio.

        Args:
            state: State dictionary to check

        Returns:
            Score based on proximity to golden ratio balance (0.0-1.0)
        """
        if not state:
            return 0.5  # Neutral for empty state

        numeric_values = [
            v for v in state.values()
            if isinstance(v, int | float) and not isinstance(v, bool) and v > 0
        ]

        if len(numeric_values) < 2:
            return 0.5  # Need at least 2 values to compute ratio

        # Compute ratios between consecutive values (sorted)
        sorted_values = sorted(numeric_values)
        ratios = [
            sorted_values[i + 1] / sorted_values[i]
            for i in range(len(sorted_values) - 1)
            if sorted_values[i] != 0
        ]

        if not ratios:
            return 0.5

        # Score based on proximity to golden ratio
        # Calculate average deviation from golden ratio
        deviations = [abs(r - self._golden_ratio) / self._golden_ratio for r in ratios]
        avg_deviation = sum(deviations) / len(deviations)

        # Convert deviation to score (lower deviation = higher score)
        # Max deviation considered is 100% (deviation of 1.0)
        score = 1.0 - min(avg_deviation, 1.0)
        return max(0.0, min(score, 1.0))


class VerifyEvaluator(AxiomEvaluator):
    """VERIFY Axiom: Multiple sources agree.

    Consensus among multiple sources increases confidence. This axiom rewards
    state dictionaries that contain multiple agreeing signals.

    Score:
        - Based on number of state keys (simulating multiple sources)
        - Returns 1.0 if 3+ keys present (good consensus)
        - Returns proportional scores for fewer keys
    """

    axiom_name = "VERIFY"

    async def score(self, state: dict[str, Any]) -> float:
        """Evaluate state consensus and verification.

        Args:
            state: State dictionary to check

        Returns:
            Score based on number of agreeing sources (0.0-1.0)
        """
        if not state:
            return 0.0  # No sources = no consensus

        # Reward for having multiple sources (keys)
        key_count = len(state)

        # Scoring: proportional to number of keys, maxed at 3+
        if key_count >= 3:
            return 1.0
        elif key_count == 2:
            return 0.67
        elif key_count == 1:
            return 0.33
        else:
            return 0.0


class CultureEvaluator(AxiomEvaluator):
    """CULTURE Axiom: State respects community norms.

    Community culture defines expected state structure and metrics.
    This axiom checks if state contains culturally expected fields.

    Score:
        - Based on presence of expected keys
        - Expected keys: process_count, memory_usage, cpu_usage
        - Returns ratio of expected keys present
    """

    axiom_name = "CULTURE"
    _expected_keys = {"process_count", "memory_usage", "cpu_usage"}

    async def score(self, state: dict[str, Any]) -> float:
        """Evaluate state cultural alignment.

        Args:
            state: State dictionary to check

        Returns:
            Ratio of expected keys present (0.0-1.0)
        """
        if not state:
            return 0.0  # No state = no cultural alignment

        present_keys = sum(
            1 for key in self._expected_keys
            if key in state
        )

        return present_keys / len(self._expected_keys)


class BurnEvaluator(AxiomEvaluator):
    """BURN Axiom: No extraction or waste.

    The organism should not waste resources or extract value unfairly.
    This axiom checks for waste_percent field and penalizes high waste.

    Score:
        - Based on low waste percentage
        - Returns 1.0 if waste_percent is 0-5%
        - Returns 0.0 if waste_percent is 100%+
        - Linear interpolation in between
    """

    axiom_name = "BURN"

    async def score(self, state: dict[str, Any]) -> float:
        """Evaluate state for waste and extraction.

        Args:
            state: State dictionary to check

        Returns:
            Score penalizing high waste (0.0-1.0)
        """
        if not state:
            return 0.5  # Neutral for empty state

        waste_percent = state.get("waste_percent", 0.0)

        # Ensure waste_percent is numeric
        if not isinstance(waste_percent, int | float):
            waste_percent = 0.0

        # Score inversely proportional to waste
        # 0% waste = 1.0, 100% waste = 0.0
        score = max(0.0, 1.0 - (waste_percent / 100.0))
        return float(min(score, 1.0))
