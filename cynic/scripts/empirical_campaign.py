#!/usr/bin/env python3
"""
Empirical Judgment Campaign — BLOCKER #3 Validation

Run 100+ real PERCEIVE→JUDGE→DECIDE cycles on asdfasdfa codebase.
This validates CYNIC's core loop and learning capability.

Usage:
  python -m scripts.empirical_campaign [--max-judgments 100] [--repo-path ../]
"""
from __future__ import annotations

import asyncio
import json
import logging
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("empirical_campaign")


async def empirical_campaign(
    max_judgments: int = 100,
    repo_path: Path = None,
) -> dict[str, Any]:
    """
    Run empirical judgment campaign.

    Args:
        max_judgments: Number of real judgments to collect
        repo_path: Repository to analyze (defaults to parent of CYNIC)

    Returns:
        Campaign results: {
            "campaign_id": str,
            "timestamp": str,
            "total_judgments": int,
            "judgments": list,
            "metrics": {
                "mean_latency_ms": float,
                "qtable_states": int,
                "consensus_rate": float,
                ...
            }
        }
    """
    if repo_path is None:
        # Default: parent of CYNIC kernel
        cynic_path = Path(__file__).parent.parent
        repo_path = cynic_path.parent

    campaign_id = datetime.now().isoformat()[:19].replace(":", "-")
    logger.info(f"Starting empirical campaign {campaign_id}")
    logger.info(f"Repository: {repo_path}")
    logger.info(f"Max judgments: {max_judgments}")

    # Access kernel via HTTP API (not via import - kernel runs separately)
    import httpx
    from cynic.core.judgment import Cell
    from cynic.core.consciousness import ConsciousnessLevel

    kernel_url = "http://localhost:8000"

    # Verify kernel is alive
    try:
        health_resp = httpx.get(f"{kernel_url}/health", timeout=5)
        health_resp.raise_for_status()
        logger.info("Kernel is alive ✓")
    except Exception as e:
        logger.error(f"Kernel not responding at {kernel_url}: {e}")
        return {"error": f"Kernel not available: {e}", "campaign_id": campaign_id}

    client = httpx.AsyncClient(base_url=kernel_url, timeout=60.0)

    # Collect code files from repo
    logger.info("Scanning repository for code files...")
    code_files = list(repo_path.rglob("*.py"))[:max_judgments]
    logger.info(f"Found {len(code_files)} Python files")

    # Campaign metrics
    judgments: list[dict] = []
    latencies: list[float] = []
    q_scores: list[float] = []
    verdicts_count = {"HOWL": 0, "WAG": 0, "GROWL": 0, "BARK": 0}

    # Run judgments
    logger.info(f"Starting judgment loop ({len(code_files)} files)...")

    for i, code_file in enumerate(code_files, 1):
        try:
            # Read file content
            try:
                content = code_file.read_text()[:1000]  # First 1000 chars
            except Exception as e:
                logger.warning(f"Could not read {code_file}: {e}")
                continue

            # Create cell for judgment
            cell = Cell(
                cell_id=f"empirical_{i:04d}",
                reality="CODE",
                content=content,
                budget_usd=0.01,
            )

            # Run judgment via HTTP API
            t0 = time.perf_counter()
            try:
                # Send judgment request to kernel
                payload = {
                    "cell": {
                        "cell_id": cell.cell_id,
                        "reality": cell.reality,
                        "content": cell.content,
                        "budget_usd": cell.budget_usd,
                    }
                }
                resp = await client.post("/judge", json=payload)
                resp.raise_for_status()
                judgment_data = resp.json()
                latency_ms = (time.perf_counter() - t0) * 1000

                # Extract judgment from response
                q_score = judgment_data.get("q_score", 0)
                verdict = judgment_data.get("verdict", "BARK")
                confidence = judgment_data.get("confidence", 0)
                judgment_id = judgment_data.get("judgment_id", f"emp_{i}")

                # Collect metrics
                latencies.append(latency_ms)
                q_scores.append(q_score)
                verdicts_count[verdict] += 1

                # Store judgment
                judgments.append({
                    "id": judgment_id,
                    "cell_id": cell.cell_id,
                    "file": str(code_file.relative_to(repo_path)),
                    "q_score": q_score,
                    "verdict": verdict,
                    "confidence": confidence,
                    "latency_ms": latency_ms,
                    "timestamp": datetime.now().isoformat(),
                })

                if i % 10 == 0:
                    logger.info(
                        f"[{i}/{len(code_files)}] Q={q_score:.1f} "
                        f"Verdict={verdict} Latency={latency_ms:.0f}ms"
                    )

            except Exception as e:
                logger.error(f"Judgment failed for {code_file}: {e}")
                continue

        except Exception as e:
            logger.error(f"Cell creation failed for {code_file}: {e}")
            continue

    # Fetch final kernel state
    logger.info("Campaign complete. Fetching final metrics...")

    import statistics

    # Get final Q-Table stats from kernel
    try:
        stats_resp = await client.get("/stats")
        stats_resp.raise_for_status()
        stats_data = stats_resp.json()
        qtable_states = stats_data.get("learning", {}).get("states", 0)
        qtable_updates = stats_data.get("learning", {}).get("total_updates", 0)
    except Exception as e:
        logger.warning(f"Could not fetch Q-Table stats: {e}")
        qtable_states = 0
        qtable_updates = 0

    metrics = {
        "total_judgments": len(judgments),
        "mean_latency_ms": statistics.mean(latencies) if latencies else 0,
        "median_latency_ms": statistics.median(latencies) if latencies else 0,
        "max_latency_ms": max(latencies) if latencies else 0,
        "mean_q_score": statistics.mean(q_scores) if q_scores else 0,
        "median_q_score": statistics.median(q_scores) if q_scores else 0,
        "qtable_states": qtable_states,
        "qtable_updates": qtable_updates,
        "verdicts": verdicts_count,
    }

    # Close HTTP client
    await client.aclose()

    campaign_result = {
        "campaign_id": campaign_id,
        "timestamp": datetime.now().isoformat(),
        "metrics": metrics,
        "judgments": judgments,
    }

    # Save results
    output_path = Path(__file__).parent.parent / ".cynic" / "campaigns" / f"{campaign_id}.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w") as f:
        json.dump(campaign_result, f, indent=2, default=str)

    logger.info(f"Campaign results saved to {output_path}")

    # Print summary
    print("\n" + "=" * 70)
    print("EMPIRICAL CAMPAIGN SUMMARY")
    print("=" * 70)
    print(f"Campaign ID: {campaign_id}")
    print(f"Judgments: {metrics['total_judgments']}")
    print(f"Mean latency: {metrics['mean_latency_ms']:.0f}ms")
    print(f"Mean Q-Score: {metrics['mean_q_score']:.1f}")
    print(f"Q-Table states: {metrics['qtable_states']}")
    print(f"Verdicts: {metrics['verdicts']}")
    print("=" * 70 + "\n")

    return campaign_result


def main() -> None:
    """CLI entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="Run empirical CYNIC judgment campaign")
    parser.add_argument("--max-judgments", type=int, default=100, help="Number of judgments to run")
    parser.add_argument("--repo-path", type=Path, default=None, help="Repository path to analyze")

    args = parser.parse_args()

    # Run campaign
    try:
        result = asyncio.run(
            empirical_campaign(
                max_judgments=args.max_judgments,
                repo_path=args.repo_path,
            )
        )
        if "error" in result:
            logger.error(result["error"])
            sys.exit(1)
    except KeyboardInterrupt:
        logger.info("Campaign interrupted by user")
        sys.exit(0)
    except Exception as e:
        logger.error(f"Campaign failed: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
