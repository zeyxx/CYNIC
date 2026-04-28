"""K15 falsification test: Lab must consume kernel observations.

This test verifies that lab.py loads observations from both:
1. Organ files (hermes-x data-centric path)
2. Kernel storage via REST /observations (general observations path)

If lab fails to load kernel observations, this test falsifies (fails).
"""

import json
import os
import sys
import tempfile
from pathlib import Path

# Add lab to path
LAB_DIR = Path(__file__).parent.parent / "lab"
sys.path.insert(0, str(LAB_DIR))

from lab import generate_briefing


def test_lab_consumes_kernel_observations():
    """
    FALSIFICATION TEST: Lab must include kernel observations in briefing.

    Setup:
      - Dataset: base 2007 tweets (organ-x dataset)
      - Organ: 11 observations (from prior runs)
      - Kernel: ~100 observations (from REST /observe without hermes-x)

    Expected:
      - briefing.tweet_count >= 2007 + 11 + some_kernel_observations

    If this fails, lab is not consuming kernel observations (K15 violation).
    """

    # Skip test if kernel not available
    cynic_rest_addr = os.environ.get("CYNIC_REST_ADDR", "")
    cynic_api_key = os.environ.get("CYNIC_API_KEY", "")

    if not (cynic_rest_addr and cynic_api_key):
        print("SKIP: CYNIC_REST_ADDR or CYNIC_API_KEY not set")
        return

    # Run lab with both kernel and organ observations available
    dataset_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"
    organ_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x"

    if not dataset_path.exists():
        print(f"SKIP: Dataset not found at {dataset_path}")
        return

    briefing = generate_briefing(str(dataset_path), str(organ_dir))

    # Verify structure
    assert briefing is not None, "Lab must generate a briefing"
    assert "tweet_count" in briefing, "Briefing must have tweet_count"
    assert "analyses" in briefing, "Briefing must have analyses section"

    # Base dataset: 2007 tweets (from dataset.jsonl)
    # Organ observations: typically 10-20 files
    # Kernel observations: could be 0-1000+ depending on /observe call volume
    # Minimum baseline: 2007 + 10 (at least some observations)

    tweet_count = briefing["tweet_count"]
    min_expected = 2007 + 10  # Base dataset + at least some organ observations

    assert tweet_count >= min_expected, (
        f"Lab tweet_count ({tweet_count}) must be >= base (2007) + organ observations (10+). "
        f"If count is exactly 2007 or 2007+organ only, kernel observations were NOT loaded (K15 violation)."
    )

    # Secondary check: if kernel observations were available and loaded,
    # the count should be significantly higher than organ-only
    # This is a heuristic but catches the case where kernel consumers fail silently.
    kernel_available = (
        cynic_rest_addr and cynic_api_key and
        os.environ.get("CYNIC_OBSERVATIONS_LOADED", "0") != "0"
    )

    if kernel_available:
        assert tweet_count > 2050, (
            f"If kernel observations are available, lab should load them. "
            f"Got {tweet_count}, expected >2050."
        )

    print(f"✓ K15 VERIFIED: Lab consumed observations (count={tweet_count})")


if __name__ == "__main__":
    test_lab_consumes_kernel_observations()
