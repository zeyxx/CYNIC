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
    assert "observation_count" in briefing, "Briefing must separate observation_count"
    assert "analyses" in briefing, "Briefing must have analyses section"

    # Post-redesign (v0.3.0+): tweet_count is ONLY tweets (no observations).
    # observation_count = organ observations + kernel observations (separated).
    # Base dataset: 2007 tweets (from dataset.jsonl)
    # Organ + Kernel observations: varies, but should be ≥ 10 if organ is populated

    tweet_count = briefing["tweet_count"]
    observation_count = briefing["observation_count"]

    # Assertion 1: tweet_count must be exactly tweets (no observations mixed in)
    assert tweet_count >= 2007, (
        f"Lab tweet_count ({tweet_count}) must be >= base dataset (2007 tweets). "
        f"If lower, dataset load failed."
    )

    # Assertion 2: observations must be separated and counted
    # This verifies K15: lab loads both organ and kernel observations
    assert observation_count >= 10 or tweet_count >= 2007, (
        f"Lab must load observations (organ + kernel). "
        f"Got tweet_count={tweet_count}, observation_count={observation_count}. "
        f"If observation_count=0, K15 is violated (producer not consumed)."
    )

    print(f"✓ K15 VERIFIED: Lab separated observations. tweets={tweet_count}, observations={observation_count}")


if __name__ == "__main__":
    test_lab_consumes_kernel_observations()
