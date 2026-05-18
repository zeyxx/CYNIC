"""
Tier 1 EXPERIMENTAL: 4-arm benchmark collector for CYNIC Dogs vs Claude models.

Research question: Do CYNIC Dogs (Qwen 7B + enrichment + crystals) outperform
  Claude models on token verdict accuracy against CultScreener ground truth?

Arms:
  cynic_dogs      — kernel /judge with enrichment + crystals
  haiku_naive     — Claude Haiku 4.5, mint+symbol only
  sonnet_naive    — Claude Sonnet 4.6, mint+symbol only
  sonnet_enriched — Claude Sonnet 4.6, enriched stimulus from Dogs arm

Success condition: rho(Dogs) > rho(Sonnet naive)
  AND adjacent_match(Dogs) >= adjacent_match(Sonnet enriched)

Timeline: Active until 2026-06-17 (see MANIFEST.yaml)
Status: ACTIVE (started 2026-05-17)
Owned by: T. / CYNIC meta-question experiment
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import subprocess
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

__version__ = "0.1.0"

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# parents[2] = heuristics/ (meta-question → experiments → heuristics)
CALIBRATION_PATH = (
    Path(__file__).resolve().parents[2] / "data" / "calibration_results_real.json"
)

DEFAULT_OUTPUT = Path(__file__).resolve().parent / "benchmark_results.jsonl"

CYNIC_REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
CYNIC_API_KEY = os.environ.get("CYNIC_API_KEY", "")

CLAUDE_HAIKU_MODEL = "claude-haiku-4-5"
CLAUDE_SONNET_MODEL = "claude-sonnet-4-6"

CLAUDE_RATE_LIMIT_SLEEP = 3  # seconds between Claude CLI calls

VALID_ARMS = ("cynic_dogs", "haiku_naive", "sonnet_naive", "sonnet_enriched")

PROMPT_TEMPLATE = """\
You are evaluating a Solana token. Score it on exactly 6 axioms.
Each score must be between 0.05 and 0.618 (phi-inverse ceiling).

FIDELITY: Is it faithful to its claimed purpose?
PHI: Is the holder distribution proportional and harmonious?
VERIFY: Can the claims be independently verified on-chain?
CULTURE: Does it follow established token standards?
BURN: Is it efficiently structured with minimal waste?
SOVEREIGNTY: Is control distributed, not concentrated?

Return ONLY a JSON object, no explanation:
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "culture": 0.XX, "burn": 0.XX, "sovereignty": 0.XX}}

TOKEN TO EVALUATE:
{stimulus}"""


# ---------------------------------------------------------------------------
# Verdict mapping
# ---------------------------------------------------------------------------

def q_score_to_verdict(q: float) -> str:
    """Map a scalar q_score to a verdict tier name."""
    if q >= 0.528:
        return "Howl"
    if q >= 0.382:
        return "Wag"
    if q >= 0.236:
        return "Growl"
    return "Bark"


# ---------------------------------------------------------------------------
# Kernel /judge call (cynic_dogs arm)
# ---------------------------------------------------------------------------

def call_kernel_judge(mint: str) -> dict:
    """
    POST to /judge and return the parsed response dict.

    Input contract: mint is a valid Solana base58 address string.
    Output: dict with keys verdict, q_score (float), axioms (dict),
            stimulus_content (str|None), elapsed_ms (int).
    Failure mode: raises RuntimeError with description if HTTP or JSON error.
    """
    url = "http://" + CYNIC_REST_ADDR + "/judge"
    payload = json.dumps({
        "content": mint,
        "domain": "token-analysis",
        "crystals": True,
    }).encode("utf-8")

    headers = {
        "Content-Type": "application/json",
        "Authorization": "Bearer " + CYNIC_API_KEY,
    }

    req = urllib.request.Request(url, data=payload, headers=headers, method="POST")
    t0 = time.monotonic()
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            body = resp.read().decode("utf-8")
    except urllib.error.HTTPError as exc:
        raise RuntimeError("kernel /judge HTTP " + str(exc.code) + ": " + exc.reason) from exc
    except urllib.error.URLError as exc:
        raise RuntimeError("kernel /judge unreachable: " + str(exc.reason)) from exc
    except TimeoutError as exc:
        raise RuntimeError("kernel /judge timeout (60s)") from exc

    elapsed_ms = int((time.monotonic() - t0) * 1000)

    try:
        data = json.loads(body)
    except json.JSONDecodeError as exc:
        raise RuntimeError("kernel /judge JSON parse error: " + str(exc)) from exc

    # Extract q_score — may be nested dict or flat float
    raw_qs = data.get("q_score", {})
    if isinstance(raw_qs, dict):
        total = float(raw_qs.get("total", 0.0))
        axioms = {k: v for k, v in raw_qs.items() if k != "total"}
    else:
        total = float(raw_qs) if raw_qs else 0.0
        axioms = {}

    return {
        "verdict": data.get("verdict", q_score_to_verdict(total)),
        "q_score": total,
        "axioms": axioms,
        "stimulus_content": data.get("stimulus_content"),
        "elapsed_ms": elapsed_ms,
    }


# ---------------------------------------------------------------------------
# Claude CLI call
# ---------------------------------------------------------------------------

def call_claude(stimulus: str, model: str) -> dict:
    """
    Run: claude -p <prompt> --model <model> --output-format json

    Input contract: stimulus is the text inserted into PROMPT_TEMPLATE.
    Output: dict with q_score (float), verdict (str), axioms (dict), elapsed_ms (int).
    Failure mode: raises RuntimeError if subprocess fails or JSON not parseable.
    """
    prompt = PROMPT_TEMPLATE.format(stimulus=stimulus)

    t0 = time.monotonic()
    try:
        result = subprocess.run(
            ["claude", "-p", prompt, "--model", model, "--output-format", "json"],
            capture_output=True,
            text=True,
            timeout=120,
        )
    except FileNotFoundError as exc:
        raise RuntimeError("claude CLI not found on PATH") from exc
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError("claude CLI timed out after 120s") from exc

    elapsed_ms = int((time.monotonic() - t0) * 1000)

    if result.returncode != 0:
        raise RuntimeError(
            "claude CLI exited " + str(result.returncode) + ": " + result.stderr[:300]
        )

    raw_output = result.stdout.strip()

    # --output-format json wraps the response; extract text content.
    try:
        outer = json.loads(raw_output)
        text_content = ""
        if isinstance(outer, dict):
            for key in ("result", "content", "text", "output"):
                val = outer.get(key)
                if isinstance(val, str) and val:
                    text_content = val
                    break
            if not text_content:
                text_content = raw_output
        else:
            text_content = raw_output
    except json.JSONDecodeError:
        text_content = raw_output

    # Parse the 6-axiom JSON object from the text content
    match = re.search(r'\{[^{}]*"fidelity"[^{}]*\}', text_content, re.DOTALL)
    if not match:
        raise RuntimeError(
            "No axiom JSON found in Claude response. Preview: " + text_content[:300]
        )

    try:
        axioms = json.loads(match.group())
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            "Axiom JSON parse error: " + str(exc) + ". Raw: " + match.group()
        ) from exc

    required = ("fidelity", "phi", "verify", "culture", "burn", "sovereignty")
    for key in required:
        if key not in axioms:
            raise RuntimeError("Missing axiom key '" + key + "' in: " + str(axioms))

    # Geometric mean of 6 axiom scores (floor at 1e-9 to avoid log(0))
    values = [max(float(axioms[k]), 1e-9) for k in required]
    q_score = math.exp(sum(math.log(v) for v in values) / len(values))

    return {
        "verdict": q_score_to_verdict(q_score),
        "q_score": q_score,
        "axioms": {k: axioms[k] for k in required},
        "elapsed_ms": elapsed_ms,
    }


# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------

def preflight_kernel() -> bool:
    """Check kernel health endpoint. Returns True if reachable."""
    url = "http://" + CYNIC_REST_ADDR + "/health"
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=5):
            pass
        print("[preflight] kernel at " + CYNIC_REST_ADDR + " is reachable", flush=True)
        return True
    except Exception as exc:  # noqa: BLE001
        print("[preflight] kernel NOT reachable at " + CYNIC_REST_ADDR + ": " + str(exc), flush=True)
        return False


def preflight_claude() -> bool:
    """Run a trivial claude call to confirm CLI is available."""
    print("[preflight] testing claude CLI...", flush=True)
    try:
        result = subprocess.run(
            [
                "claude", "-p", "Say hello",
                "--model", CLAUDE_HAIKU_MODEL,
                "--output-format", "json",
            ],
            capture_output=True,
            text=True,
            timeout=60,
        )
        if result.returncode == 0:
            print("[preflight] claude CLI OK", flush=True)
            return True
        print(
            "[preflight] claude CLI returned " + str(result.returncode)
            + ": " + result.stderr[:200],
            flush=True,
        )
        return False
    except FileNotFoundError:
        print("[preflight] claude CLI not found on PATH", flush=True)
        return False
    except subprocess.TimeoutExpired:
        print("[preflight] claude CLI preflight timed out", flush=True)
        return False


# ---------------------------------------------------------------------------
# JSONL record builder
# ---------------------------------------------------------------------------

def make_record(
    mint: str,
    symbol: str,
    arm: str,
    q_score: float,
    verdict: str,
    axioms: dict,
    ground_truth_tier: str,
    ground_truth_verdict: str,
    conviction: float,
    elapsed_ms: int,
) -> str:
    """Serialise one result row as a JSON line."""
    record = {
        "mint": mint,
        "symbol": symbol,
        "arm": arm,
        "q_score": q_score,
        "verdict": verdict,
        "axioms": axioms,
        "ground_truth_tier": ground_truth_tier,
        "ground_truth_verdict": ground_truth_verdict,
        "conviction": conviction,
        "elapsed_ms": elapsed_ms,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    return json.dumps(record)


# ---------------------------------------------------------------------------
# Main collection loop
# ---------------------------------------------------------------------------

def run_collection(arms: list, output_path: Path) -> None:
    """
    Load calibration tokens, run requested arms, append results to output_path.

    Input contract: arms is a non-empty subset of VALID_ARMS.
    Output: JSONL appended to output_path (one line per token x arm).
    Failure mode: per-token errors are logged and skipped; collection continues.
    """
    print("[collect] Loading calibration tokens from " + str(CALIBRATION_PATH), flush=True)
    if not CALIBRATION_PATH.exists():
        print("[collect] ERROR: calibration file not found: " + str(CALIBRATION_PATH), flush=True)
        sys.exit(1)

    with CALIBRATION_PATH.open("r", encoding="utf-8") as fh:
        cal_data = json.load(fh)

    tokens = cal_data.get("results", [])
    if not tokens:
        print("[collect] ERROR: no results in calibration file", flush=True)
        sys.exit(1)

    print("[collect] " + str(len(tokens)) + " tokens, arms: " + str(arms), flush=True)
    print("[collect] Output: " + str(output_path), flush=True)

    # Preflight
    need_dogs = "cynic_dogs" in arms
    need_claude = any(a in arms for a in ("haiku_naive", "sonnet_naive", "sonnet_enriched"))

    kernel_ok = preflight_kernel() if need_dogs else True
    claude_ok = preflight_claude() if need_claude else True

    if need_dogs and not kernel_ok:
        print(
            "[collect] WARNING: kernel unreachable — cynic_dogs arm will fail per token",
            flush=True,
        )
    if need_claude and not claude_ok:
        print(
            "[collect] WARNING: claude CLI unavailable — Claude arms will fail per token",
            flush=True,
        )

    output_path.parent.mkdir(parents=True, exist_ok=True)

    total_written = 0
    total_errors = 0

    with output_path.open("a", encoding="utf-8") as out_fh:
        for idx, token in enumerate(tokens, start=1):
            mint = token.get("mint", "")
            symbol = token.get("symbol", "UNKNOWN")
            ground_truth_tier = token.get("conviction_tier", "")
            ground_truth_verdict = token.get("expected_verdict", "")
            conviction = float(token.get("conviction", 0.0))

            print(
                "\n[" + str(idx) + "/" + str(len(tokens)) + "] "
                + symbol + " (" + mint[:8] + "...)",
                flush=True,
            )

            # Arm 1: cynic_dogs — must run first per token to get stimulus_content for arm 4
            stimulus_content = None

            if "cynic_dogs" in arms:
                # Rate limit: kernel allows 10 req/min on /judge
                if idx > 1:
                    time.sleep(7)
                try:
                    dogs_res = call_kernel_judge(mint)
                    stimulus_content = dogs_res.get("stimulus_content")
                    line = make_record(
                        mint=mint,
                        symbol=symbol,
                        arm="cynic_dogs",
                        q_score=dogs_res["q_score"],
                        verdict=dogs_res["verdict"],
                        axioms=dogs_res["axioms"],
                        ground_truth_tier=ground_truth_tier,
                        ground_truth_verdict=ground_truth_verdict,
                        conviction=conviction,
                        elapsed_ms=dogs_res["elapsed_ms"],
                    )
                    out_fh.write(line + "\n")
                    out_fh.flush()
                    total_written += 1
                    sc_note = " [no stimulus_content]" if stimulus_content is None else ""
                    print(
                        "  cynic_dogs  q=" + format(dogs_res["q_score"], ".3f")
                        + " verdict=" + dogs_res["verdict"] + sc_note,
                        flush=True,
                    )
                except RuntimeError as exc:
                    print("  cynic_dogs  ERROR: " + str(exc), flush=True)
                    total_errors += 1

            # Arm 2: haiku_naive
            if "haiku_naive" in arms:
                if need_claude:
                    time.sleep(CLAUDE_RATE_LIMIT_SLEEP)
                naive_stimulus = "Solana token: " + symbol + "\nToken mint: " + mint
                try:
                    haiku_res = call_claude(naive_stimulus, CLAUDE_HAIKU_MODEL)
                    line = make_record(
                        mint=mint,
                        symbol=symbol,
                        arm="haiku_naive",
                        q_score=haiku_res["q_score"],
                        verdict=haiku_res["verdict"],
                        axioms=haiku_res["axioms"],
                        ground_truth_tier=ground_truth_tier,
                        ground_truth_verdict=ground_truth_verdict,
                        conviction=conviction,
                        elapsed_ms=haiku_res["elapsed_ms"],
                    )
                    out_fh.write(line + "\n")
                    out_fh.flush()
                    total_written += 1
                    print(
                        "  haiku_naive q=" + format(haiku_res["q_score"], ".3f")
                        + " verdict=" + haiku_res["verdict"],
                        flush=True,
                    )
                except RuntimeError as exc:
                    print("  haiku_naive ERROR: " + str(exc), flush=True)
                    total_errors += 1

            # Arm 3: sonnet_naive
            if "sonnet_naive" in arms:
                if need_claude:
                    time.sleep(CLAUDE_RATE_LIMIT_SLEEP)
                naive_stimulus = "Solana token: " + symbol + "\nToken mint: " + mint
                try:
                    sonnet_res = call_claude(naive_stimulus, CLAUDE_SONNET_MODEL)
                    line = make_record(
                        mint=mint,
                        symbol=symbol,
                        arm="sonnet_naive",
                        q_score=sonnet_res["q_score"],
                        verdict=sonnet_res["verdict"],
                        axioms=sonnet_res["axioms"],
                        ground_truth_tier=ground_truth_tier,
                        ground_truth_verdict=ground_truth_verdict,
                        conviction=conviction,
                        elapsed_ms=sonnet_res["elapsed_ms"],
                    )
                    out_fh.write(line + "\n")
                    out_fh.flush()
                    total_written += 1
                    print(
                        "  sonnet_naive q=" + format(sonnet_res["q_score"], ".3f")
                        + " verdict=" + sonnet_res["verdict"],
                        flush=True,
                    )
                except RuntimeError as exc:
                    print("  sonnet_naive ERROR: " + str(exc), flush=True)
                    total_errors += 1

            # Arm 4: sonnet_enriched — requires stimulus_content from Dogs arm
            if "sonnet_enriched" in arms:
                if stimulus_content is None:
                    print(
                        "  sonnet_enriched SKIP: stimulus_content unavailable"
                        " (Dogs arm missing or returned None)",
                        flush=True,
                    )
                else:
                    if need_claude:
                        time.sleep(CLAUDE_RATE_LIMIT_SLEEP)
                    try:
                        enriched_res = call_claude(stimulus_content, CLAUDE_SONNET_MODEL)
                        line = make_record(
                            mint=mint,
                            symbol=symbol,
                            arm="sonnet_enriched",
                            q_score=enriched_res["q_score"],
                            verdict=enriched_res["verdict"],
                            axioms=enriched_res["axioms"],
                            ground_truth_tier=ground_truth_tier,
                            ground_truth_verdict=ground_truth_verdict,
                            conviction=conviction,
                            elapsed_ms=enriched_res["elapsed_ms"],
                        )
                        out_fh.write(line + "\n")
                        out_fh.flush()
                        total_written += 1
                        print(
                            "  sonnet_enriched q=" + format(enriched_res["q_score"], ".3f")
                            + " verdict=" + enriched_res["verdict"],
                            flush=True,
                        )
                    except RuntimeError as exc:
                        print("  sonnet_enriched ERROR: " + str(exc), flush=True)
                        total_errors += 1

    print(
        "\n[collect] Done. "
        + str(total_written) + " records written, "
        + str(total_errors) + " errors."
        + " Output: " + str(output_path),
        flush=True,
    )


# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

def resolve_arms(arms_arg: str) -> list:
    """
    Expand --arms argument to list of arm names.

    Valid input values: 'all', 'dogs', 'claude', or comma-separated arm names.
    Output: list of arm name strings from VALID_ARMS.
    Failure mode: prints error and sys.exit(1) on unknown arm name.
    """
    if arms_arg == "all":
        return list(VALID_ARMS)
    if arms_arg == "dogs":
        return ["cynic_dogs"]
    if arms_arg == "claude":
        return ["haiku_naive", "sonnet_naive", "sonnet_enriched"]
    # comma-separated explicit arms
    selected = [a.strip() for a in arms_arg.split(",") if a.strip()]
    for arm in selected:
        if arm not in VALID_ARMS:
            print(
                "[args] ERROR: unknown arm '" + arm + "'. Valid: " + ", ".join(VALID_ARMS),
                flush=True,
            )
            sys.exit(1)
    return selected


def main() -> None:
    """Entry point."""
    print(
        "[collect] version " + __version__
        + " | " + datetime.now(timezone.utc).isoformat(),
        flush=True,
    )

    parser = argparse.ArgumentParser(
        description="4-arm benchmark collector: CYNIC Dogs vs Claude models."
    )
    parser.add_argument(
        "--arms",
        default="all",
        help=(
            "Arms to run: 'all' (default), 'dogs', 'claude', "
            "or comma-separated from: " + ", ".join(VALID_ARMS)
        ),
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Output JSONL file path (default: " + str(DEFAULT_OUTPUT) + ")",
    )

    args = parser.parse_args()
    arms = resolve_arms(args.arms)
    output_path = Path(args.output)

    run_collection(arms=arms, output_path=output_path)


if __name__ == "__main__":
    main()
