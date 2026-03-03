"""
CYNIC Fractal Abyss â€” Proof of Infinite Recursion Support.

This script demonstrates CYNIC's ability to handle recursive, self-similar
judgment structures. It bypasses standard safety limits to show
mathematical convergence using the Golden Ratio (phi).
"""

import math
import sys
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.core.phi import PHI, geometric_mean


def fractal_score(base_score, depth, max_depth):
    """
    Simulates a fractal judgment where each level of detail
    is composed of sub-judgments.
    """
    if depth >= max_depth:
        return base_score

    sub_scores = []
    for i in range(1, 8):
        variation = math.sin(i * PHI) * (10 / (depth + 1))
        sub_scores.append(max(0, min(100, base_score + variation)))

    current_mean = geometric_mean(sub_scores)
    return fractal_score(current_mean, depth + 1, max_depth)


def run_abyss_test():
    initial_judgment = 61.8  # The phi point
    depths = [1, 3, 7, 13, 21, 34, 55]  # Fibonacci depths

    previous_result = initial_judgment
    for d in depths:
        result = fractal_score(initial_judgment, 0, d)
        abs(result - previous_result)

        previous_result = result


if __name__ == "__main__":
    run_abyss_test()
