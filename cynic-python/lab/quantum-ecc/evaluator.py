"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — cargo build+eval runner.

Runs the ecdsa.fail benchmark and returns gate×qubit product.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""
import logging
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class EvalResult:
    gate_count: int
    qubit_count: int
    product: int
    correctness: bool
    raw_output: str


def parse_eval_output(output: str) -> EvalResult:
    """Parse cargo eval_circuit stdout + score.json into EvalResult.

    The benchmark scores as avg_toffoli × peak_qubits and writes score.json.
    Raises ValueError if required fields are missing.
    """

    def extract_float(pattern: str) -> float:
        match = re.search(pattern, output, re.MULTILINE)
        if not match:
            raise ValueError(f"Pattern '{pattern}' not found in eval output: {output[:300]!r}")
        return float(match.group(1))

    toffoli = int(extract_float(r"avg executed Toffoli\s*:\s*([\d.]+)"))
    qubits = int(extract_float(r"qubits\s*:\s*([\d.]+)"))
    product = toffoli * qubits
    correctness = "experiment OK" in output

    return EvalResult(
        gate_count=toffoli,
        qubit_count=qubits,
        product=product,
        correctness=correctness,
        raw_output=output,
    )


def run_eval(repo_dir: Path) -> EvalResult:
    """Run cargo build_circuit then eval_circuit. Raises RuntimeError on compile failure."""
    for step in ("build_circuit", "eval_circuit"):
        proc = subprocess.run(
            ["cargo", "run", "--release", "--bin", step],
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        if proc.returncode != 0:
            logging.error(
                "cargo_step_failed",
                extra={"step": step, "stderr": proc.stderr[:500]},
            )
            raise RuntimeError(
                f"cargo {step} failed (exit {proc.returncode}): {proc.stderr[:200]}"
            )

    result = parse_eval_output(proc.stdout)
    logging.info(
        "eval_complete",
        extra={
            "toffoli": result.gate_count,
            "qubits": result.qubit_count,
            "product": result.product,
            "correctness": result.correctness,
        },
    )
    return result
