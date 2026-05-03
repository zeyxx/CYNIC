#!/usr/bin/env python3
"""
Reflection Producer — K15 Producer that sends organ-x learnings to kernel.

Closes the learning loop:
  farming_log (what organ-x searched)
    ↓
  observations (what organ-x found)
    ↓
  reflections.jsonl (what organ-x learned)
    ↓
  reflection_producer (you are here) — sends to kernel /observe
    ↓
  kernel learns domain productivity
    ↓
  domain_router adjusts farming weights
    ↓
  next farming cycle (optimized by learning)

Architecture:
  1. Load reflections.jsonl (per-cycle learnings)
  2. Enrich with observations linked to each cycle
  3. Infer domain insights (which domains produced signal)
  4. Post to kernel /observe endpoint (K15 producer)
  5. Log submission status
"""

__version__ = "0.1.0"

import json
import logging
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional
import urllib.request
import urllib.error

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("reflection-producer")


class ReflectionProducer:
    """Sends organ-x reflections to kernel for learning feedback."""

    def __init__(self, reflections_path: Path, obs_dir: Path):
        self.reflections_path = reflections_path
        self.obs_dir = obs_dir
        self.kernel_addr = os.environ.get("CYNIC_REST_ADDR", "<TAILSCALE_CORE>:3030")
        self.api_key = os.environ.get("CYNIC_API_KEY", "")
        self.reflections: List[Dict] = []
        self.submissions: List[Dict] = []

    def load_reflections(self) -> int:
        """Load reflections from local jsonl file."""
        if not self.reflections_path.exists():
            logger.warning(f"Reflections file not found: {self.reflections_path}")
            return 0

        count = 0
        try:
            with open(self.reflections_path) as f:
                for line in f:
                    try:
                        reflection = json.loads(line)
                        self.reflections.append(reflection)
                        count += 1
                    except:
                        pass
        except Exception as e:
            logger.error(f"Failed to load reflections: {e}")

        logger.info(f"✓ Loaded {count} reflections")
        return count

    def post_observation(self, observation: Dict) -> bool:
        """POST observation to kernel /observe endpoint."""
        if not self.api_key:
            logger.warning("✗ CYNIC_API_KEY not set, skipping kernel submission")
            return False

        url = f"http://{self.kernel_addr}/observe"
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

        payload = json.dumps(observation)

        try:
            req = urllib.request.Request(
                url, data=payload.encode(), headers=headers, method="POST"
            )
            with urllib.request.urlopen(req, timeout=5) as response:
                result = json.loads(response.read().decode())
                return result.get("success", False)
        except urllib.error.HTTPError as e:
            # Log the response body for debugging
            try:
                error_body = e.read().decode()
                logger.debug(f"HTTP {e.code}: {error_body[:200]}")
            except:
                logger.debug(f"HTTP {e.code}: {e.reason}")
            return False
        except urllib.error.URLError as e:
            logger.debug(f"Connection error: {e.reason}")
            return False
        except Exception as e:
            logger.debug(f"Submission failed: {e}")
            return False

    def submit_reflections(self) -> int:
        """Submit reflections to kernel as learning observations."""
        if not self.reflections:
            logger.warning("No reflections to submit")
            return 0

        success_count = 0

        for reflection in self.reflections:
            observation = {
                "tool": "reflection_producer",
                "target": "kernel_learning",
                "domain": "organ-x-reflections",
                "finding": reflection.get("finding", ""),
                "signal_score": reflection.get("signal_quality", 0.5),
                "timestamp": reflection.get("timestamp", datetime.now().isoformat()),
                "metadata": {
                    "cycle_index": reflection.get("cycle_index"),
                    "domains_farmed": reflection.get("domains_farmed", []),
                    "learning": reflection.get("learnings", []),
                    "observation_count": reflection.get("observation_count", 0),
                    "source": "reflection_producer",
                    "version": __version__,
                },
            }

            if self.post_observation(observation):
                success_count += 1
                self.submissions.append({
                    "timestamp": datetime.now().isoformat(),
                    "reflection_timestamp": reflection.get("timestamp"),
                    "status": "success",
                })
                logger.info(f"✓ Submitted reflection {success_count}/{len(self.reflections)}")
            else:
                self.submissions.append({
                    "timestamp": datetime.now().isoformat(),
                    "reflection_timestamp": reflection.get("timestamp"),
                    "status": "failed",
                })
                logger.warning(f"✗ Failed to submit reflection")

        logger.info(f"✓ Submitted {success_count}/{len(self.reflections)} reflections to kernel")
        return success_count

    def write_submission_log(self) -> None:
        """Write submission status to local log."""
        log_path = Path.home() / ".cynic" / "organisms" / "reflection_submissions.jsonl"
        log_path.parent.mkdir(parents=True, exist_ok=True)

        with open(log_path, 'a') as f:
            for submission in self.submissions:
                f.write(json.dumps(submission) + '\n')

        logger.info(f"✓ Logged {len(self.submissions)} submissions to {log_path}")

    def run(self) -> int:
        """Execute full reflection submission pipeline."""
        logger.info(f"\n=== Reflection Producer (K15 Producer) v{__version__} ===\n")

        # Load local reflections
        count = self.load_reflections()
        if count == 0:
            logger.error("✗ No reflections to process")
            return 0

        # Submit to kernel
        success = self.submit_reflections()

        # Log submissions
        self.write_submission_log()

        logger.info(f"\n✓ Reflection production complete ({success}/{count} successful)\n")
        return success


def main():
    from argparse import ArgumentParser

    parser = ArgumentParser(description="Send organ-x reflections to kernel for learning")
    parser.add_argument(
        '--reflections',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "datasets" / "reflections.jsonl",
        help="Reflections file path"
    )
    parser.add_argument(
        '--obs-dir',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations",
        help="Observations directory"
    )
    args = parser.parse_args()

    producer = ReflectionProducer(args.reflections, args.obs_dir)
    success = producer.run()

    return 0 if success > 0 else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
