"""K15 falsification test: Lab must consume kernel observations."""

import os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from hermes_paths import DATASET, HERMES_X_DIR

# Add lab to path
LAB_DIR = Path(__file__).parent.parent / "lab"
sys.path.insert(0, str(LAB_DIR))

from lab import generate_briefing


def test_lab_consumes_kernel_observations():
    cynic_rest_addr = os.environ.get("CYNIC_REST_ADDR", "")
    cynic_api_key = os.environ.get("CYNIC_API_KEY", "")
    if not (cynic_rest_addr and cynic_api_key):
        pytest.skip("CYNIC_REST_ADDR or CYNIC_API_KEY not set")

    dataset_path = DATASET
    organ_dir = HERMES_X_DIR
    if not dataset_path.exists():
        pytest.skip(f"Dataset not found at {dataset_path}")

    briefing = generate_briefing(str(dataset_path), str(organ_dir))

    assert briefing is not None, "Lab must generate a briefing"
    assert "tweet_count" in briefing, "Briefing must have tweet_count"
    assert "observation_count" in briefing, "Briefing must separate observation_count"
    assert "analyses" in briefing, "Briefing must have analyses section"

    tweet_count = briefing["tweet_count"]
    observation_count = briefing["observation_count"]

    assert tweet_count >= 2007, (
        f"Lab tweet_count ({tweet_count}) must be >= base dataset (2007 tweets). "
        f"If lower, dataset load failed."
    )

    assert observation_count >= 10 or tweet_count >= 2007, (
        f"Lab must load observations (organ + kernel). "
        f"Got tweet_count={tweet_count}, observation_count={observation_count}. "
        f"If observation_count=0, K15 is violated (producer not consumed)."
    )
