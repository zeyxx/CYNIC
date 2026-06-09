"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — parallel parameter search.

Explores the env-var parameter space of the ecdsa.fail circuit without modifying Rust code.
Each combination runs build_circuit → eval_circuit in an isolated temp directory.
Uses concurrent.futures for parallel execution across available CPU cores.

Baseline: 2,630,999,274 (1,688,703 avg Toffoli × 1,558 peak qubits)

Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.

Usage:
  python3 search.py --repo ./ecdsafail-challenge --mode reroll2d [--workers 12]
  python3 search.py --repo ./ecdsafail-challenge --mode reroll2d --dry-run
  python3 search.py --repo ./ecdsafail-challenge --mode compare_bits
"""
import argparse
import itertools
import json
import logging
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterator, Optional

logging.basicConfig(
    level=logging.INFO,
    stream=sys.stdout,
    format="%(asctime)s %(levelname)s %(message)s",
)

STATE_FILE = Path(__file__).parent / "search_state.json"


@dataclass
class EvalResult:
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
    tried_keys: list[str] = field(default_factory=list)
    improvements: list[dict[str, Any]] = field(default_factory=list)
    total_run: int = 0


def load_state() -> SearchState:
    if not STATE_FILE.exists():
        return SearchState()
    try:
        d = json.loads(STATE_FILE.read_text())
        return SearchState(**d)
    except Exception:
        return SearchState()


def save_state(s: SearchState) -> None:
    import uuid
    tmp = STATE_FILE.parent / f".search_state_{uuid.uuid4().hex[:8]}.tmp"
    tmp.write_text(json.dumps(asdict(s), indent=2))
    tmp.replace(STATE_FILE)


def params_key(params: dict[str, str]) -> str:
    return json.dumps(sorted(params.items()))


def run_one(
    build_bin: str,
    eval_bin: str,
    extra_env: dict[str, str],
) -> Optional[EvalResult]:
    """Run build_circuit + eval_circuit in a temp dir with env overrides."""
    env = os.environ.copy()
    env.update(extra_env)

    tmpdir = tempfile.mkdtemp(prefix="qecc_")
    try:
        t0 = time.monotonic()

        # Step 1: build_circuit → ops.bin
        build_proc = subprocess.run(
            [build_bin],
            cwd=tmpdir,
            capture_output=True,
            text=True,
            env=env,
        )
        if build_proc.returncode != 0:
            return None

        # Step 2: eval_circuit → reads ops.bin, outputs metrics
        eval_proc = subprocess.run(
            [eval_bin],
            cwd=tmpdir,
            capture_output=True,
            text=True,
            env=env,
        )
        duration = time.monotonic() - t0

        if eval_proc.returncode != 0:
            return None

        output = eval_proc.stdout

        def extract(pattern: str) -> Optional[float]:
            m = re.search(pattern, output, re.MULTILINE)
            return float(m.group(1)) if m else None

        toffoli_f = extract(r"avg executed Toffoli\s*:\s*([\d.]+)")
        qubits_f = extract(r"qubits\s*:\s*([\d.]+)")
        if toffoli_f is None or qubits_f is None:
            return None

        toffoli = int(toffoli_f)
        qubits = int(qubits_f)

        return EvalResult(
            params=dict(extra_env),
            toffoli=toffoli,
            qubits=qubits,
            score=toffoli * qubits,
            correctness="experiment OK" in output,
            duration_s=duration,
        )
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)


def reroll_2d_search(r_max: int, p_max: int) -> list[dict[str, str]]:
    return [
        {"DIALOG_REROLL": str(r), "DIALOG_POST_SUB_REROLL": str(p)}
        for r, p in itertools.product(range(r_max), range(p_max))
    ]


def compare_bits_search(cb_range: range, r_max: int, p_max: int) -> list[dict[str, str]]:
    return [
        {
            "DIALOG_GCD_COMPARE_BITS": str(cb),
            "DIALOG_REROLL": str(r),
            "DIALOG_POST_SUB_REROLL": str(p),
        }
        for cb, r, p in itertools.product(cb_range, range(r_max), range(p_max))
    ]


def width_margin_search(margin_range: range, r_max: int, p_max: int) -> list[dict[str, str]]:
    return [
        {
            "DIALOG_GCD_WIDTH_MARGIN": str(m),
            "DIALOG_REROLL": str(r),
            "DIALOG_POST_SUB_REROLL": str(p),
        }
        for m, r, p in itertools.product(margin_range, range(r_max), range(p_max))
    ]


def run_search(
    repo_dir: Path,
    combos: list[dict[str, str]],
    workers: int,
    dry_run: bool,
    build_bin: Optional[str] = None,
    eval_bin: Optional[str] = None,
) -> None:
    build_bin = build_bin or str(repo_dir / "target" / "release" / "build_circuit")
    eval_bin = eval_bin or str(repo_dir / "target" / "release" / "eval_circuit")

    state = load_state()

    # Filter already-tried
    pending = [c for c in combos if params_key(c) not in set(state.tried_keys)]
    logging.info(
        "search_start: %d combos total, %d pending, %d already tried, workers=%d",
        len(combos), len(pending), len(combos) - len(pending), workers,
    )

    if dry_run:
        for c in pending[:5]:
            logging.info("[DRY RUN] would try: %s", c)
        logging.info("[DRY RUN] %d combos total — not running", len(pending))
        return

    eta_s = len(pending) * 25 / workers
    logging.info(
        "ETA: ~%dm%ds at ~25s/combo with %d workers",
        int(eta_s // 60), int(eta_s % 60), workers,
    )

    completed = 0
    with ProcessPoolExecutor(max_workers=workers) as pool:
        futures = {
            pool.submit(run_one, build_bin, eval_bin, c): c
            for c in pending
        }

        for future in as_completed(futures):
            c = futures[future]
            key = params_key(c)
            state.tried_keys.append(key)
            state.total_run += 1
            completed += 1

            try:
                result = future.result()
            except Exception as e:
                logging.warning("worker error params=%s: %s", c, e)
                save_state(state)
                continue

            if result is None:
                logging.warning("eval returned None for params=%s", c)
                save_state(state)
                continue

            status = "OK" if result.correctness else "FAIL"
            logging.info(
                "[%d/%d] score=%d toffoli=%d qubits=%d %s params=%s (%.1fs)",
                completed, len(pending),
                result.score, result.toffoli, result.qubits,
                status, c, result.duration_s,
            )

            if result.correctness and result.score < state.best_score:
                delta = state.best_score - result.score
                logging.info(
                    "*** NEW BEST: %d (delta -%d, -%.2f%%) params=%s",
                    result.score, delta, 100 * delta / state.best_score, c,
                )
                state.best_score = result.score
                state.best_toffoli = result.toffoli
                state.best_qubits = result.qubits
                state.best_params = c
                state.improvements.append(asdict(result))

            save_state(state)

    state = load_state()
    logging.info(
        "search_complete: best_score=%d best_params=%s improvements=%d total_run=%d",
        state.best_score, state.best_params,
        len(state.improvements), state.total_run,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ECC parallel parameter search")
    parser.add_argument("--repo", default=None,
                        help="Repo dir (expects target/release/ inside). "
                             "Ignored if --build-bin and --eval-bin are set.")
    parser.add_argument("--build-bin", default=None, help="Path to build_circuit binary")
    parser.add_argument("--eval-bin", default=None, help="Path to eval_circuit binary")
    parser.add_argument("--mode", choices=["reroll2d", "compare_bits", "width_margin"],
                        default="reroll2d")
    parser.add_argument("--workers", type=int, default=12)
    parser.add_argument("--r-min", type=int, default=0)
    parser.add_argument("--r-max", type=int, default=20)
    parser.add_argument("--p-max", type=int, default=25)
    parser.add_argument("--cb-min", type=int, default=55, help="compare_bits: min COMPARE_BITS")
    parser.add_argument("--cb-max", type=int, default=65, help="compare_bits: max COMPARE_BITS (exclusive)")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    # Resolve binary paths to absolute (workers run from tmpdir — relative paths break)
    build_bin: Optional[str] = str(Path(args.build_bin).resolve()) if args.build_bin else None
    eval_bin: Optional[str] = str(Path(args.eval_bin).resolve()) if args.eval_bin else None

    if build_bin is None or eval_bin is None:
        if args.repo is None:
            print("ERROR: provide --repo or both --build-bin and --eval-bin")
            sys.exit(1)
        repo_dir = Path(args.repo).resolve()
        build_bin = build_bin or str(repo_dir / "target" / "release" / "build_circuit")
        eval_bin = eval_bin or str(repo_dir / "target" / "release" / "eval_circuit")
    else:
        repo_dir = Path(build_bin).parent  # used only for label

    for b in (build_bin, eval_bin):
        if not Path(b).exists():
            print(f"ERROR: binary not found: {b}")
            sys.exit(1)

    if args.mode == "reroll2d":
        combos = reroll_2d_search(args.r_max, args.p_max)
        # Filter to r_min..r_max range
        combos = [c for c in combos if int(c["DIALOG_REROLL"]) >= args.r_min]
        logging.info(
            "mode=reroll2d: REROLL [%d,%d) × POST_SUB [0,%d) = %d combos",
            args.r_min, args.r_max, args.p_max, len(combos),
        )
    elif args.mode == "compare_bits":
        combos = compare_bits_search(range(args.cb_min, args.cb_max), args.r_max, args.p_max)
        logging.info(
            "mode=compare_bits: CB [%d,%d) × REROLL [0,%d) × POST_SUB [0,%d) = %d combos",
            args.cb_min, args.cb_max, args.r_max, args.p_max, len(combos),
        )
    else:
        combos = width_margin_search(range(22, 33), args.r_max, args.p_max)

    run_search(
        repo_dir, combos, workers=args.workers, dry_run=args.dry_run,
        build_bin=build_bin, eval_bin=eval_bin,
    )


if __name__ == "__main__":
    main()
