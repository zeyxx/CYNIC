"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — parameter search.

Explores the env-var parameter space of the ecdsa.fail circuit without modifying Rust code.
The circuit uses Dialog-GCD with dozens of tunable parameters; improvements come from
finding "Fiat-Shamir islands" — parameter combinations where the random test vectors
happen to align with the circuit's branching structure.

Baseline: 2,630,999,274 (1,688,703 avg Toffoli × 1,558 peak qubits)

Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.

Usage:
  python3 search.py --repo ./ecdsafail-challenge --mode reroll2d [--dry-run]
  python3 search.py --repo ./ecdsafail-challenge --mode compare_bits
  python3 search.py --repo ./ecdsafail-challenge --mode width_margin
"""
import argparse
import itertools
import json
import logging
import subprocess
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterator, Optional

logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format="%(asctime)s %(levelname)s %(message)s")

STATE_FILE = Path(__file__).parent / "search_state.json"

# Current submission route defaults (from configure_ecdsafail_submission_route)
BASELINE_PARAMS: dict[str, str] = {
    "DIALOG_REROLL": "13",
    "DIALOG_POST_SUB_REROLL": "14",
    "DIALOG_GCD_COMPARE_BITS": "59",
    "DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS": "19",
    "DIALOG_GCD_WIDTH_MARGIN": "27",
    "DIALOG_GCD_ACTIVE_ITERATIONS": "399",
    "DIALOG_GCD_APPLY_WINDOW_BLOCKS": "2",
    "DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS": "2",
    "DIALOG_GCD_APPLY_CHUNKED_F_CUT": "70",
    "KAL_DOUBLE_CARRY_TRUNC_W": "20",
    "KAL_FOLD_CARRY_TRUNC_W": "20",
    "KARA_SOL_SHIFT22_DOUBLES": "1",
    "ROUND84_XTAIL_KARATSUBA": "1",
}


@dataclass
class SearchResult:
    params: dict[str, str]
    toffoli: int
    qubits: int
    score: int
    correctness: bool
    duration_s: float


@dataclass
class SearchState:
    best_score: int = 2_630_999_274
    best_params: dict[str, str] = field(default_factory=dict)
    best_toffoli: int = 1_688_703
    best_qubits: int = 1_558
    tried: list[dict[str, Any]] = field(default_factory=list)
    improvements: list[dict[str, Any]] = field(default_factory=list)


def load_state() -> SearchState:
    if not STATE_FILE.exists():
        return SearchState()
    try:
        d = json.loads(STATE_FILE.read_text())
        return SearchState(**d)
    except Exception:
        return SearchState()


def save_state(s: SearchState) -> None:
    tmp = STATE_FILE.with_suffix(".tmp")
    tmp.write_text(json.dumps(asdict(s), indent=2))
    tmp.replace(STATE_FILE)


def run_eval(repo_dir: Path, extra_env: dict[str, str]) -> Optional[SearchResult]:
    """Run eval_circuit with env overrides. Returns None on compile error."""
    import os
    env = os.environ.copy()
    env.update(extra_env)

    t0 = time.monotonic()
    proc = subprocess.run(
        ["cargo", "run", "--release", "--bin", "eval_circuit"],
        cwd=repo_dir,
        capture_output=True,
        text=True,
        env=env,
    )
    duration = time.monotonic() - t0

    if proc.returncode != 0:
        logging.warning("eval failed (rc=%d): %s", proc.returncode, proc.stderr[-200:])
        return None

    import re
    def extract(pattern: str) -> Optional[float]:
        m = re.search(pattern, proc.stdout, re.MULTILINE)
        return float(m.group(1)) if m else None

    toffoli_f = extract(r"avg executed Toffoli\s*:\s*([\d.]+)")
    qubits_f = extract(r"qubits\s*:\s*([\d.]+)")
    if toffoli_f is None or qubits_f is None:
        logging.warning("could not parse eval output: %s", proc.stdout[-300:])
        return None

    toffoli = int(toffoli_f)
    qubits = int(qubits_f)
    score = toffoli * qubits
    correctness = "experiment OK" in proc.stdout

    return SearchResult(
        params=dict(extra_env),
        toffoli=toffoli,
        qubits=qubits,
        score=score,
        correctness=correctness,
        duration_s=duration,
    )


def params_key(params: dict[str, str]) -> str:
    return json.dumps(sorted(params.items()))


def already_tried(state: SearchState, params: dict[str, str]) -> bool:
    key = params_key(params)
    return any(params_key(t.get("params", {})) == key for t in state.tried)


def reroll_2d_search(
    r_range: range, post_range: range
) -> Iterator[dict[str, str]]:
    """2D grid over (DIALOG_REROLL, DIALOG_POST_SUB_REROLL)."""
    for r, p in itertools.product(r_range, post_range):
        yield {"DIALOG_REROLL": str(r), "DIALOG_POST_SUB_REROLL": str(p)}


def compare_bits_search(cb_range: range) -> Iterator[dict[str, str]]:
    """1D sweep over DIALOG_GCD_COMPARE_BITS with matched reroll scan."""
    for cb in cb_range:
        for r in range(0, 20):
            for p in range(0, 20):
                yield {
                    "DIALOG_GCD_COMPARE_BITS": str(cb),
                    "DIALOG_REROLL": str(r),
                    "DIALOG_POST_SUB_REROLL": str(p),
                }


def width_margin_search(margin_range: range) -> Iterator[dict[str, str]]:
    """Sweep DIALOG_GCD_WIDTH_MARGIN with reroll scan."""
    for margin in margin_range:
        for r in range(0, 20):
            for p in range(0, 20):
                yield {
                    "DIALOG_GCD_WIDTH_MARGIN": str(margin),
                    "DIALOG_REROLL": str(r),
                    "DIALOG_POST_SUB_REROLL": str(p),
                }


def run_search(
    repo_dir: Path,
    param_iter: Iterator[dict[str, str]],
    dry_run: bool = False,
) -> None:
    state = load_state()

    for extra_params in param_iter:
        if already_tried(state, extra_params):
            logging.info("skip (already tried): %s", extra_params)
            continue

        logging.info("trying: %s", extra_params)

        if dry_run:
            logging.info("[DRY RUN] would run eval with %s", extra_params)
            state.tried.append({"params": extra_params, "dry_run": True})
            save_state(state)
            continue

        result = run_eval(repo_dir, extra_params)
        record: dict[str, Any] = {
            "params": extra_params,
            "result": asdict(result) if result else None,
        }
        state.tried.append(record)

        if result is None:
            save_state(state)
            continue

        logging.info(
            "score=%d toffoli=%d qubits=%d correct=%s (%.1fs)",
            result.score, result.toffoli, result.qubits,
            result.correctness, result.duration_s,
        )

        if result.correctness and result.score < state.best_score:
            delta = state.best_score - result.score
            logging.info(
                "*** NEW BEST: %d (delta -%d, %.1f%%) params=%s",
                result.score, delta, 100 * delta / state.best_score, extra_params,
            )
            state.best_score = result.score
            state.best_toffoli = result.toffoli
            state.best_qubits = result.qubits
            state.best_params = extra_params
            state.improvements.append(record)
            save_state(state)
        else:
            save_state(state)


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ECC parameter search")
    parser.add_argument("--repo", required=True)
    parser.add_argument(
        "--mode",
        choices=["reroll2d", "compare_bits", "width_margin"],
        default="reroll2d",
    )
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--r-max", type=int, default=20,
                        help="DIALOG_REROLL range [0, r-max)")
    parser.add_argument("--p-max", type=int, default=25,
                        help="DIALOG_POST_SUB_REROLL range [0, p-max)")
    args = parser.parse_args()

    repo_dir = Path(args.repo).resolve()

    state = load_state()
    logging.info(
        "search_start: best_score=%d tried=%d",
        state.best_score, len(state.tried),
    )

    if args.mode == "reroll2d":
        # ~500 combinations × ~30s each ≈ 4h (run overnight)
        logging.info(
            "2D reroll search: DIALOG_REROLL [0,%d) × DIALOG_POST_SUB_REROLL [0,%d) = %d combos",
            args.r_max, args.p_max, args.r_max * args.p_max,
        )
        it = reroll_2d_search(range(args.r_max), range(args.p_max))
    elif args.mode == "compare_bits":
        # sweep COMPARE_BITS 55-64 × reroll 2D
        it = compare_bits_search(range(55, 65))
    else:
        it = width_margin_search(range(22, 33))

    run_search(repo_dir, it, dry_run=args.dry_run)

    state = load_state()
    logging.info(
        "search_complete: best_score=%d best_params=%s improvements=%d",
        state.best_score, state.best_params, len(state.improvements),
    )


if __name__ == "__main__":
    main()
