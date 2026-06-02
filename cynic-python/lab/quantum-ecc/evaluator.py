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
    """Parse cargo eval stdout into EvalResult. Raises ValueError on missing fields."""

    def extract(field: str) -> str:
        match = re.search(rf"^{field}:\s*(\S+)", output, re.MULTILINE)
        if not match:
            raise ValueError(f"Field '{field}' not found in eval output: {output!r}")
        return match.group(1)

    return EvalResult(
        gate_count=int(extract("gates")),
        qubit_count=int(extract("qubits")),
        product=int(extract("product")),
        correctness=extract("correctness").upper() == "PASS",
        raw_output=output,
    )


def run_eval(repo_dir: Path) -> EvalResult:
    """Run cargo build then cargo eval in repo_dir. Raises RuntimeError on compile failure."""
    for step in ("build", "eval"):
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
            "gate_count": result.gate_count,
            "qubit_count": result.qubit_count,
            "product": result.product,
            "correctness": result.correctness,
        },
    )
    return result
