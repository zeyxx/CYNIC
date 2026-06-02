"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — main optimization loop.

Usage:
  python3 harness.py --repo ./ecdsafail-challenge [--max-iters 20]

Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""
import argparse
import logging
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from agents.inspector import InspectionResult, inspect_circuit, read_circuit_files
from agents.optimizer import generate_optimization
from evaluator import EvalResult, run_eval
from state import CircuitState, load_state, save_state

logging.basicConfig(level=logging.INFO, stream=sys.stdout,
                    format="%(asctime)s %(levelname)s %(message)s")

STATE_FILE = Path(__file__).parent / "state.json"

LEVER_PRIORITY = [
    "windowed_scalar_mult",
    "projective_coordinates",
    "toffoli_ladder_ancilla_reuse",
    "karatsuba_field_mult",
]


def load_lever_descriptions() -> dict[str, str]:
    """Parse levers.md into {lever_name: description} dict."""
    levers_md = Path(__file__).parent / "levers.md"
    content = levers_md.read_text()
    descriptions: dict[str, str] = {}
    current_name: str | None = None
    current_lines: list[str] = []

    for line in content.split("\n"):
        if line.startswith("## Lever "):
            if current_name:
                descriptions[current_name] = "\n".join(current_lines).strip()
            current_name = line.split(": ", 1)[1].strip()
            current_lines = []
        elif current_name:
            current_lines.append(line)

    if current_name:
        descriptions[current_name] = "\n".join(current_lines).strip()

    return descriptions


def select_lever(inspection: InspectionResult, tried: list[str]) -> str | None:
    """Select highest-priority untried lever applicable to current circuit structure."""
    unavailable: set[str] = set()
    if inspection.windowed:
        unavailable.add("windowed_scalar_mult")
    if inspection.projective:
        unavailable.add("projective_coordinates")

    for lever in LEVER_PRIORITY:
        if lever not in tried and lever not in unavailable:
            return lever
    return None


def backup_circuit(repo_dir: Path) -> dict[str, str]:
    """Save current src/point_add/*.rs content keyed by filename."""
    point_add_dir = repo_dir / "src" / "point_add"
    return {f.name: f.read_text() for f in sorted(point_add_dir.glob("*.rs"))}


def restore_circuit(repo_dir: Path, backup: dict[str, str]) -> None:
    """Restore src/point_add/*.rs from backup dict."""
    point_add_dir = repo_dir / "src" / "point_add"
    for name, content in backup.items():
        (point_add_dir / name).write_text(content)


def write_modified_circuit(repo_dir: Path, content: str) -> None:
    """Write modified content to src/point_add/mod.rs."""
    (repo_dir / "src" / "point_add" / "mod.rs").write_text(content)


def commit_improvement(repo_dir: Path, lever: str, result: EvalResult) -> None:
    """Git commit the improvement inside the challenge repo."""
    subprocess.run(["git", "add", "-A"], cwd=repo_dir, check=True, capture_output=True)
    msg = (
        f"opt({lever}): {result.product:,} "
        f"({result.gate_count:,}g x {result.qubit_count:,}q)"
    )
    subprocess.run(["git", "commit", "-m", msg], cwd=repo_dir, check=True, capture_output=True)
    logging.info("committed improvement: lever=%s product=%d", lever, result.product)


def run_loop(repo_dir: Path, max_iterations: int) -> None:
    """Main optimization loop."""
    if not os.environ.get("GEMINI_API_KEY"):
        raise RuntimeError("GEMINI_API_KEY not set")

    lever_descriptions = load_lever_descriptions()
    state = load_state(STATE_FILE)

    logging.info("loop_start: best_product=%d tried=%s", state.best_product, state.tried_levers)

    for iteration in range(max_iterations):
        logging.info("--- iteration %d ---", iteration)

        circuit_content = read_circuit_files(repo_dir)
        inspection = inspect_circuit(circuit_content)
        logging.info(
            "inspection: windowed=%s projective=%s recommended=%s reasoning=%s",
            inspection.windowed, inspection.projective,
            inspection.recommended_lever, inspection.reasoning,
        )

        lever = select_lever(inspection, state.tried_levers)
        if lever is None:
            logging.info("no levers remaining — stopping")
            break

        logging.info("selected lever: %s", lever)
        opt = generate_optimization(circuit_content, lever, lever_descriptions.get(lever, ""))

        if not opt.changed:
            logging.info("optimizer returned unchanged code for lever=%s", lever)
            state.tried_levers.append(lever)
            save_state(state, STATE_FILE)
            continue

        backup = backup_circuit(repo_dir)
        write_modified_circuit(repo_dir, opt.modified_content)

        try:
            result = run_eval(repo_dir)
        except RuntimeError as e:
            logging.warning("eval failed for lever=%s: %s", lever, e)
            restore_circuit(repo_dir, backup)
            state.tried_levers.append(lever)
            save_state(state, STATE_FILE)
            continue

        baseline = state.best_product if state.best_product > 0 else (state.baseline_product or 0)
        improved = result.correctness and (baseline == 0 or result.product < baseline)

        record: dict[str, Any] = {
            "iteration": iteration,
            "lever": lever,
            "product": result.product,
            "gate_count": result.gate_count,
            "qubit_count": result.qubit_count,
            "correctness": result.correctness,
            "accepted": improved,
        }
        state.iterations.append(record)

        if improved:
            state.best_product = result.product
            state.best_gate_count = result.gate_count
            state.best_qubit_count = result.qubit_count
            save_state(state, STATE_FILE)
            commit_improvement(repo_dir, lever, result)
            logging.info(
                "IMPROVED: lever=%s product=%d (delta=%d)",
                lever, result.product, baseline - result.product,
            )
        else:
            restore_circuit(repo_dir, backup)
            state.tried_levers.append(lever)
            save_state(state, STATE_FILE)
            logging.info(
                "rejected: lever=%s product=%d correctness=%s",
                lever, result.product, result.correctness,
            )

    logging.info(
        "loop_complete: best_product=%d iterations=%d",
        state.best_product, len(state.iterations),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Quantum ECC circuit optimizer harness")
    parser.add_argument("--repo", required=True, help="Path to cloned ecdsafail-challenge repo")
    parser.add_argument("--max-iters", type=int, default=20, help="Max optimization iterations")
    args = parser.parse_args()

    repo_dir = Path(args.repo).resolve()
    if not (repo_dir / "src" / "point_add").exists():
        print(f"ERROR: {repo_dir}/src/point_add not found. Run bootstrap.sh first.")
        sys.exit(1)

    run_loop(repo_dir, args.max_iters)


if __name__ == "__main__":
    main()
