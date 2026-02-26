"""
Benchmark Fine-Tuned Model Against Baselines

Compare cynic-mistral:7b (fine-tuned) against other available models:
- gemma2:2b (current default for temporal MCTS)
- claude-haiku (if API available)
- Other Ollama models

Metrics:
- Q-Score accuracy: Does it predict the right verdict?
- Latency: How fast?
- Confidence calibration: Is it confident when right, uncertain when wrong?
- Cost: API calls (if applicable)

After benchmarking, LLMRegistry auto-picks cynic-mistral if it wins.
"""

import asyncio
import json
import logging
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, List, Dict, Any

logger = logging.getLogger("cynic.training.benchmark_model")


# ════════════════════════════════════════════════════════════════════════════
# BENCHMARK RESULT TYPES
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class JudgmentResult:
    """Result from a single judgment call."""
    model: str
    proposal_index: int
    expected_verdict: str
    expected_q_score: float
    actual_verdict: Optional[str] = None
    actual_q_score: Optional[float] = None
    latency_ms: float = 0.0
    error: Optional[str] = None

    @property
    def verdict_correct(self) -> bool:
        """Check if verdict matches expected."""
        return self.actual_verdict == self.expected_verdict

    @property
    def score_error(self) -> float:
        """Absolute error in Q-Score."""
        if self.actual_q_score is None:
            return 100.0
        return abs(self.actual_q_score - self.expected_q_score)


@dataclass
class BenchmarkResult:
    """Summary of model benchmark."""
    model: str
    total_judgments: int = 0
    successful_judgments: int = 0
    verdict_accuracy: float = 0.0
    avg_latency_ms: float = 0.0
    avg_score_error: float = 0.0
    details: List[JudgmentResult] = field(default_factory=list)

    @property
    def success_rate(self) -> float:
        """Percentage of successful calls."""
        if self.total_judgments == 0:
            return 0.0
        return self.successful_judgments / self.total_judgments * 100


# ════════════════════════════════════════════════════════════════════════════
# TEST PROPOSALS
# ════════════════════════════════════════════════════════════════════════════

TEST_PROPOSALS = [
    {
        "title": "Allocate 5% treasury to community marketing",
        "description": "Use 50,000 tokens for marketing. All spend voted on by community. Unused funds burned.",
        "category": "BUDGET_ALLOCATION",
        "impact_level": "MEDIUM",
        "expected_verdict": "HOWL",
        "expected_q_score": 60.0,
    },
    {
        "title": "Implement E-Score reputation system",
        "description": "Track proposer/voter reputation. Public scores. Prevents Sybil attacks.",
        "category": "GOVERNANCE_CHANGE",
        "impact_level": "HIGH",
        "expected_verdict": "HOWL",
        "expected_q_score": 59.0,
    },
    {
        "title": "Pay team members $5K/month",
        "description": "Hire 2 community managers. Can be fired by community vote. Monthly reports required.",
        "category": "BUDGET_ALLOCATION",
        "impact_level": "MEDIUM",
        "expected_verdict": "WAG",
        "expected_q_score": 50.0,
    },
    {
        "title": "Allocate 20% treasury to founder's venture fund",
        "description": "Give founder $50K from treasury. Founder keeps all returns. No oversight.",
        "category": "EXTRACTION",
        "impact_level": "CRITICAL",
        "expected_verdict": "GROWL",
        "expected_q_score": 30.0,
    },
    {
        "title": "Transfer 50% treasury to founder's wallet",
        "description": "Move funds to founder for 'safekeeping'. No smart contract protection.",
        "category": "EXTRACTION",
        "impact_level": "CRITICAL",
        "expected_verdict": "BARK",
        "expected_q_score": 15.0,
    },
]


# ════════════════════════════════════════════════════════════════════════════
# ADAPTER FACTORIES
# ════════════════════════════════════════════════════════════════════════════

def get_ollama_adapter(model_name: str):
    """Get OllamaAdapter from CYNIC's LLM registry."""
    try:
        from cynic.llm.adapters.ollama import OllamaAdapter
        return OllamaAdapter(model_name)
    except Exception as e:
        logger.warning(f"Could not load Ollama adapter: {e}")
        return None


def get_claude_adapter():
    """Get ClaudeAdapter if API key available."""
    try:
        from cynic.llm.adapters.claude import ClaudeAdapter
        return ClaudeAdapter()
    except Exception as e:
        logger.warning(f"Could not load Claude adapter: {e}")
        return None


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT EXECUTION
# ════════════════════════════════════════════════════════════════════════════

async def judge_proposal(
    adapter,
    proposal: Dict[str, Any],
    proposal_index: int,
) -> JudgmentResult:
    """
    Call adapter to judge a proposal.

    Args:
        adapter: LLMAdapter instance
        proposal: Proposal dict with title, description, etc.
        proposal_index: Index in test set

    Returns:
        JudgmentResult with parsed verdict and score
    """
    model_name = adapter.model if hasattr(adapter, "model") else str(adapter)

    from cynic.llm.adapter import LLMRequest

    # Format proposal for LLM
    proposal_text = json.dumps({
        "title": proposal["title"],
        "description": proposal["description"],
        "category": proposal["category"],
        "impact_level": proposal["impact_level"],
    })

    system = """\
You are CYNIC, a governance intelligence organism. Rate this proposal.

VERDICT:
- HOWL (Q ≥ 61.8): Strong, non-extractive, clear
- WAG (Q 38.2-61.8): Good with minor concerns
- GROWL (Q 23.6-38.2): Extraction risk, unclear
- BARK (Q < 23.6): Rug risk, founder extraction

RESPOND AS JSON:
{
  "verdict": "HOWL|WAG|GROWL|BARK",
  "q_score": 0.0-61.8,
  "confidence": 0.0-1.0,
  "reasoning": "Why this verdict"
}"""

    user = f"Judge this governance proposal:\n\n{proposal_text}"

    request = LLMRequest(
        system=system,
        prompt=user,
        max_tokens=256,
        temperature=0.0,
    )

    result = JudgmentResult(
        model=model_name,
        proposal_index=proposal_index,
        expected_verdict=proposal["expected_verdict"],
        expected_q_score=proposal["expected_q_score"],
    )

    try:
        start = time.perf_counter()
        response = await asyncio.wait_for(
            adapter.complete_safe(request),
            timeout=30.0,
        )
        elapsed = (time.perf_counter() - start) * 1000
        result.latency_ms = elapsed

        # Parse response
        try:
            response_json = json.loads(response.content)
            result.actual_verdict = response_json.get("verdict", "").upper()
            result.actual_q_score = float(response_json.get("q_score", 0))
        except (json.JSONDecodeError, ValueError) as e:
            result.error = f"Parse error: {e}"
            # Try regex fallback
            if "HOWL" in response.content:
                result.actual_verdict = "HOWL"
            elif "WAG" in response.content:
                result.actual_verdict = "WAG"
            elif "GROWL" in response.content:
                result.actual_verdict = "GROWL"
            elif "BARK" in response.content:
                result.actual_verdict = "BARK"

    except asyncio.TimeoutError:
        result.error = "Timeout"
    except Exception as e:
        result.error = str(e)

    return result


# ════════════════════════════════════════════════════════════════════════════
# MAIN BENCHMARKING
# ════════════════════════════════════════════════════════════════════════════

async def benchmark_model(
    adapter,
    proposals: List[Dict[str, Any]],
) -> BenchmarkResult:
    """
    Benchmark a single model against test proposals.

    Args:
        adapter: LLMAdapter instance
        proposals: List of test proposals

    Returns:
        BenchmarkResult with aggregated metrics
    """
    model_name = adapter.model if hasattr(adapter, "model") else str(adapter)
    logger.info(f"\nBenchmarking {model_name}...")

    results = []
    for i, proposal in enumerate(proposals):
        logger.info(f"  [{i+1}/{len(proposals)}] {proposal['title'][:50]}...")
        result = await judge_proposal(adapter, proposal, i)
        results.append(result)

        if result.error:
            logger.debug(f"    ✗ Error: {result.error}")
        else:
            verdict_match = "✓" if result.verdict_correct else "✗"
            logger.debug(f"    {verdict_match} {result.actual_verdict} (q={result.actual_q_score:.1f})")

    # Aggregate
    successful = [r for r in results if r.error is None]
    benchmark = BenchmarkResult(
        model=model_name,
        total_judgments=len(results),
        successful_judgments=len(successful),
        details=results,
    )

    if successful:
        correct = sum(1 for r in successful if r.verdict_correct)
        benchmark.verdict_accuracy = correct / len(successful) * 100

        latencies = [r.latency_ms for r in successful]
        benchmark.avg_latency_ms = sum(latencies) / len(latencies)

        score_errors = [r.score_error for r in successful]
        benchmark.avg_score_error = sum(score_errors) / len(score_errors)

    return benchmark


async def benchmark_suite(
    models: List[str],
    proposals: Optional[List[Dict[str, Any]]] = None,
    timeout_per_model: int = 300,
) -> List[BenchmarkResult]:
    """
    Benchmark multiple models in sequence.

    Args:
        models: List of model names to benchmark
        proposals: Test proposals (default: TEST_PROPOSALS)
        timeout_per_model: Timeout for each model in seconds

    Returns:
        List of BenchmarkResult objects
    """
    if proposals is None:
        proposals = TEST_PROPOSALS

    results = []

    for model_name in models:
        logger.info(f"\nLoading {model_name}...")

        adapter = None
        if model_name.startswith("claude"):
            adapter = get_claude_adapter()
        else:
            adapter = get_ollama_adapter(model_name)

        if adapter is None:
            logger.warning(f"Could not load adapter for {model_name}")
            continue

        try:
            benchmark = await asyncio.wait_for(
                benchmark_model(adapter, proposals),
                timeout=float(timeout_per_model),
            )
            results.append(benchmark)
        except asyncio.TimeoutError:
            logger.error(f"Benchmarking {model_name} timed out")

    return results


# ════════════════════════════════════════════════════════════════════════════
# REPORTING
# ════════════════════════════════════════════════════════════════════════════

def print_benchmark_table(results: List[BenchmarkResult]) -> None:
    """Print benchmark results as formatted table."""
    print("\n" + "="*100)
    print("BENCHMARK RESULTS")
    print("="*100)
    print(f"{'Model':<30} {'Accuracy':<12} {'Latency (ms)':<15} {'Score Error':<15} {'Success':<10}")
    print("-"*100)

    for result in results:
        print(
            f"{result.model:<30} "
            f"{result.verdict_accuracy:>6.1f}%{'':<4} "
            f"{result.avg_latency_ms:>7.1f}{'':<6} "
            f"{result.avg_score_error:>6.2f}{'':<7} "
            f"{result.success_rate:>6.1f}%"
        )

    print("="*100)

    # Composite score (φ-weighted)
    from cynic.core.phi import PHI
    print("\nComposite Scores (φ-weighted):")
    print("-"*50)

    scores = []
    for result in results:
        # Composite = accuracy × speed × cost_efficiency
        # For local Ollama: cost_efficiency = 1.0 (free)
        # For API models: cost_efficiency = 0.5 (not free)
        cost_factor = 1.0 if "mistral" in result.model.lower() or "gemma" in result.model.lower() else 0.5

        # Normalize latency (lower is better)
        all_latencies = [r.avg_latency_ms for r in results]
        max_latency = max(all_latencies) if all_latencies else 1
        speed_score = 1.0 - (result.avg_latency_ms / max_latency)

        # Composite
        composite = (
            result.verdict_accuracy / 100.0 * PHI +  # Accuracy
            speed_score * PHI**-1 +                  # Speed
            cost_factor * PHI**-2                    # Cost
        )

        scores.append((result.model, composite))
        print(f"{result.model:<30} {composite:>6.3f}")

    # Winner
    if scores:
        winner = max(scores, key=lambda x: x[1])
        print("-"*50)
        print(f"🏆 Best model: {winner[0]} (score: {winner[1]:.3f})")
        print(f"Next: Update LLMRegistry to route governance calls to {winner[0]}")


def save_benchmark_results(results: List[BenchmarkResult], output_path: Path) -> None:
    """Save benchmark results to JSON."""
    output_path.parent.mkdir(parents=True, exist_ok=True)

    data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "benchmarks": [
            {
                "model": r.model,
                "total_judgments": r.total_judgments,
                "successful_judgments": r.successful_judgments,
                "verdict_accuracy": r.verdict_accuracy,
                "avg_latency_ms": r.avg_latency_ms,
                "avg_score_error": r.avg_score_error,
                "success_rate": r.success_rate,
            }
            for r in results
        ],
    }

    with open(output_path, "w") as f:
        json.dump(data, f, indent=2)

    logger.info(f"Results saved to {output_path}")


# ════════════════════════════════════════════════════════════════════════════
# CLI
# ════════════════════════════════════════════════════════════════════════════

async def main(
    models: Optional[List[str]] = None,
    output_file: Optional[Path] = None,
):
    """
    Main benchmarking entry point.

    Args:
        models: Models to benchmark (default: cynic-mistral:7b, gemma2:2b)
        output_file: Where to save results JSON
    """
    logging.basicConfig(
        level=logging.INFO,
        format="%(name)s - %(levelname)s - %(message)s",
    )

    if models is None:
        models = ["cynic-mistral:7b", "gemma2:2b"]

    if output_file is None:
        output_file = Path.home() / ".cynic" / "benchmark_results.json"

    logger.info("="*80)
    logger.info("CYNIC Governance Judgment Benchmarking")
    logger.info("="*80)
    logger.info(f"Test proposals: {len(TEST_PROPOSALS)}")
    logger.info(f"Models to benchmark: {', '.join(models)}")

    # Run benchmarks
    results = await benchmark_suite(models)

    # Report
    print_benchmark_table(results)
    save_benchmark_results(results, output_file)

    logger.info(f"\n✓ Benchmarking complete!")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Benchmark CYNIC governance judgment models")
    parser.add_argument(
        "--models",
        nargs="+",
        default=["cynic-mistral:7b", "gemma2:2b"],
        help="Models to benchmark",
    )
    parser.add_argument("--output", type=Path, help="Output JSON file")

    args = parser.parse_args()

    asyncio.run(main(models=args.models, output_file=args.output))
