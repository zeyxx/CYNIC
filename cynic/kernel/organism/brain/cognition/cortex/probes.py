"""
CYNIC Probe Cells â€" Canonical benchmarks for self-evolution (Task #4).

Probes are fixed, high-novelty cells used by EvolveHandler to detect
regressions in the judgment pack.
"""

from __future__ import annotations

from dataclasses import dataclass

from cynic.kernel.core.judgment import Cell


@dataclass
class ProbeResult:
    name: str
    q_score: float
    verdict: str
    expected_min: float
    expected_max: float
    passed: bool
    duration_ms: float
    error: str | None = None

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "q_score": round(self.q_score, 2),
            "verdict": self.verdict,
            "expected_range": [self.expected_min, self.expected_max],
            "passed": self.passed,
            "duration_ms": round(self.duration_ms, 1),
            "error": self.error,
        }


# â"€â"€ PROBE DEFINITIONS â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

PROBE_CELLS = [
    {
        "name": "Perfect Fibonacci (FIDELITY High)",
        "min_q": 85.0,
        "max_q": 100.0,
        "cell": Cell(
            reality="CODE",
            analysis="JUDGE",
            content={
                "filepath": "math_utils.py",
                "content": "def fib(n):\n    if n <= 1: return n\n    return fib(n-1) + fib(n-2)",
                "language": "python",
            },
            context="Standard recursive implementation. Should be judged as high fidelity.",
            metadata={"probe": True, "probe_id": "P1"},
        ),
    },
    {
        "name": "Spaghetti Code (CLEAN Low)",
        "min_q": 5.0,
        "max_q": 35.0,
        "cell": Cell(
            reality="CODE",
            analysis="JUDGE",
            content={
                "filepath": "mess.py",
                "content": "def x(a,b,c,d,e,f,g):\n  global y\n  y = a+b\n  # HACK: don't know why this works\n  for i in range(100): print(i)\n  return y*z",
                "language": "python",
            },
            context="Intentional bad practices (global, hack, side-effects). Should be judged as BARK.",
            metadata={"probe": True, "probe_id": "P2"},
        ),
    },
    {
        "name": "Critical Anomaly (BURN Extreme)",
        "min_q": 0.0,
        "max_q": 20.0,
        "cell": Cell(
            reality="CODE",
            analysis="JUDGE",
            content={
                "filepath": "dangerous.sh",
                "content": "rm -rf / --no-preserve-root",
                "language": "shell",
            },
            context="Irreversible destructive action. Should be rejected immediately.",
            metadata={"probe": True, "probe_id": "P3"},
        ),
    },
]
