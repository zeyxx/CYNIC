# Quantum ECC Optimization Harness — Phase A

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Python harness (`cynic-python/lab/quantum-ecc/`) that iteratively improves the [ecdsa.fail](https://github.com/ecdsafail/ecdsafail-challenge) quantum ECDSA circuit using AI agents — reducing the gate×qubit product below the current community best (6.5B).

**Architecture:** A CLI loop — inspect the current Rust circuit (`src/point_add/mod.rs`), select an untried optimization lever, call Claude to generate improved Rust, evaluate with `cargo run --bin eval`, commit improvements or revert failures. No CYNIC kernel dependency; uses the Anthropic SDK directly. State persists across runs in `state.json`.

**Tech Stack:** Python 3.11+, `anthropic` SDK, `subprocess` (list args only, no `shell=True`), `dataclasses`, `json`, `pytest`, Rust/cargo (external toolchain)

---

## File Map

```
cynic-python/lab/quantum-ecc/
├── bootstrap.sh          # Clone repo, verify toolchain, run baseline eval, init state.json
├── harness.py            # CLI entry point + main optimization loop
├── evaluator.py          # Run cargo build+eval, return EvalResult
├── state.py              # CircuitState dataclass + JSON read/write
├── levers.md             # Lever reference injected into agent prompts
├── agents/
│   ├── __init__.py
│   ├── inspector.py      # Claude Haiku: characterize circuit → InspectionResult
│   └── optimizer.py      # Claude Sonnet: generate improved Rust → OptimizationResult
└── tests/
    ├── __init__.py
    ├── test_evaluator.py  # Unit tests with mocked subprocess
    └── test_state.py      # Unit tests for state read/write
```

The cloned challenge repo lives at `cynic-python/lab/quantum-ecc/ecdsafail-challenge/` (gitignored).

---

## Task 1: Directory scaffold, gitignore, levers.md

**Files:**
- Create: `cynic-python/lab/quantum-ecc/levers.md`
- Modify: `.gitignore`

- [ ] **Step 1.1: Create directory structure**

```bash
mkdir -p /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/agents
mkdir -p /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/tests
```

- [ ] **Step 1.2: Add cloned repo to .gitignore**

In `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/.gitignore`, add after the `benchmarks/` section:

```
# Quantum ECC harness — cloned challenge repo (large, not part of CYNIC)
cynic-python/lab/quantum-ecc/ecdsafail-challenge/
cynic-python/lab/quantum-ecc/state.json
```

- [ ] **Step 1.3: Write levers.md**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/levers.md`:

```markdown
# Quantum ECC Optimization Levers

Ordered by estimated impact on gate×qubit product. All estimates are directional.

## Lever 1: windowed_scalar_mult
**Impact:** 4–8× reduction (highest priority)
**What:** Replace scalar-bit-by-bit point addition (256 controlled ECPointAdds) with
w=4 windowed arithmetic. Precompute 16 = 2^4 multiples of the base point as classical
constants. Implement a QROM lookup: 4 address qubits load one of 16 (x,y) pairs into
ancilla registers. Loop: 64 iterations of (QROM lookup → unconditional ECPointAdd →
QROM uncompute). Net result: 256 controlled adds → 64 unconditional adds + 64 QROM lookups.
**Applies when:** `windowed == false` in InspectionResult.
**Constraint:** Every QROM lookup must uncompute cleanly (ancilla returns to |0⟩).
**Do NOT change:** field arithmetic, modular inversion, coordinate system.

## Lever 2: projective_coordinates
**Impact:** 2–4× reduction
**What:** Switch from affine to Jacobian projective coordinates. Affine ECPointAdd
requires modular inversion (λ = (y2−y1)/(x2−x1)) — extremely expensive (~30× a
multiplication). Jacobian coords use Z field to defer division; only one inversion at
the very end (convert back to affine for output). Jacobian formulas: ~10–12 field
multiplications per add, vs 7 affine + 1 inversion. Net win: eliminate 255 inversions.
Use the Brier-Joye unified addition formula for simplicity.
**Applies when:** `projective == false` in InspectionResult.
**Constraint:** Output must still return affine (x,y) point for correctness check.

## Lever 3: toffoli_ladder_ancilla_reuse
**Impact:** 1.3–2× reduction on qubit component
**What:** Replace fan-out Toffoli trees (one ancilla allocated per Toffoli) with
linear Toffoli ladders that reuse a single ancilla bit via compute-uncompute pattern.
Pattern: instead of `ccx(a, b, anc0); ccx(c, d, anc1); ccx(anc0, anc1, out)`,
use `ccx(a, b, anc); ccx(c, d, out); cx(anc, out); ccx(c, d, out); ccx(a, b, anc)`.
Scan for sequential CCX blocks in field multiplication subcircuits.
**Applies when:** always (local transformation, does not depend on global structure).
**Constraint:** Gate count increases slightly; net product must decrease.

## Lever 4: karatsuba_field_mult
**Impact:** 1.2–1.5× reduction on gate component
**What:** Replace schoolbook O(n²) modular multiplication with Karatsuba O(n^1.58).
For 256-bit fields, Karatsuba splits each operand into two 128-bit halves:
`a = a_hi * 2^128 + a_lo`, compute three 128-bit products, reconstruct.
Gate count: schoolbook ~n² = 65536 Toffoli; Karatsuba ~3*(n/2)^2 = ~49152 (~25% less).
Ancilla cost increases (need temp registers for intermediate products).
**Applies when:** field multiplication uses schoolbook method (inspect for nested loops
of depth ~n over n-bit operands).
**Constraint:** Must preserve modular reduction (mod secp256k1 prime p).
```

- [ ] **Step 1.4: Create empty init files**

```bash
touch /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/tests/__init__.py
touch /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/agents/__init__.py
```

- [ ] **Step 1.5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/ .gitignore
git commit -m "feat(quantum-ecc): scaffold harness directory + levers reference"
```

---

## Task 2: `state.py` — CircuitState persistence

**Files:**
- Create: `cynic-python/lab/quantum-ecc/state.py`
- Create: `cynic-python/lab/quantum-ecc/tests/test_state.py`

- [ ] **Step 2.1: Write failing test**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/tests/test_state.py`:

```python
"""Tests for state.py — CircuitState read/write."""
import json
import tempfile
from pathlib import Path

import pytest

from state import CircuitState, load_state, save_state


def test_load_state_creates_default_when_missing() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        state = load_state(path)
        assert state.best_product == 0
        assert state.tried_levers == []
        assert state.iterations == []
        assert state.baseline_product is None


def test_save_and_load_roundtrip() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        original = CircuitState(
            best_product=5_000_000_000,
            best_gate_count=2_500_000,
            best_qubit_count=2_000,
            tried_levers=["windowed_scalar_mult"],
            iterations=[{"lever": "windowed_scalar_mult", "product": 5_000_000_000}],
            baseline_product=6_500_000_000,
        )
        save_state(original, path)
        loaded = load_state(path)
        assert loaded.best_product == 5_000_000_000
        assert loaded.tried_levers == ["windowed_scalar_mult"]
        assert loaded.baseline_product == 6_500_000_000


def test_load_state_handles_missing_optional_fields() -> None:
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "state.json"
        path.write_text(json.dumps({"best_product": 7_000_000_000}))
        state = load_state(path)
        assert state.best_product == 7_000_000_000
        assert state.baseline_product is None
        assert state.tried_levers == []
```

- [ ] **Step 2.2: Run test to verify it fails**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -m pytest tests/test_state.py -v
```

Expected: `ImportError: No module named 'state'`

- [ ] **Step 2.3: Write state.py**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/state.py`:

```python
"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — circuit state persistence.

Tracks best gate×qubit product, tried levers, iteration history.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted to Tier 2.
"""
import json
import logging
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Optional


@dataclass
class CircuitState:
    best_product: int = 0
    best_gate_count: int = 0
    best_qubit_count: int = 0
    tried_levers: list[str] = field(default_factory=list)
    iterations: list[dict[str, Any]] = field(default_factory=list)
    baseline_product: Optional[int] = None


def load_state(path: Path) -> CircuitState:
    """Load CircuitState from JSON file, returning defaults if missing or partial."""
    if not path.exists():
        return CircuitState()
    try:
        data = json.loads(path.read_text())
        return CircuitState(
            best_product=data.get("best_product", 0),
            best_gate_count=data.get("best_gate_count", 0),
            best_qubit_count=data.get("best_qubit_count", 0),
            tried_levers=data.get("tried_levers", []),
            iterations=data.get("iterations", []),
            baseline_product=data.get("baseline_product"),
        )
    except (json.JSONDecodeError, KeyError) as e:
        logging.warning("state_load_failed", extra={"path": str(path), "error": str(e)})
        return CircuitState()


def save_state(state: CircuitState, path: Path) -> None:
    """Persist CircuitState to JSON file atomically."""
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(asdict(state), indent=2))
    tmp.replace(path)
```

- [ ] **Step 2.4: Run tests to verify they pass**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -m pytest tests/test_state.py -v
```

Expected: 3 tests PASS.

- [ ] **Step 2.5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/state.py cynic-python/lab/quantum-ecc/tests/test_state.py
git commit -m "feat(quantum-ecc): CircuitState persistence (state.py)"
```

---

## Task 3: `evaluator.py` — cargo build+eval runner

**Files:**
- Create: `cynic-python/lab/quantum-ecc/evaluator.py`
- Create: `cynic-python/lab/quantum-ecc/tests/test_evaluator.py`

- [ ] **Step 3.1: Write failing tests**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/tests/test_evaluator.py`:

```python
"""Tests for evaluator.py — cargo build+eval runner."""
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from evaluator import EvalResult, parse_eval_output, run_eval


def test_parse_eval_output_success() -> None:
    output = "gates: 2800000\nqubits: 2300\nproduct: 6440000000\ncorrectness: PASS\n"
    result = parse_eval_output(output)
    assert result.gate_count == 2_800_000
    assert result.qubit_count == 2_300
    assert result.product == 6_440_000_000
    assert result.correctness is True


def test_parse_eval_output_fail() -> None:
    output = "gates: 2800000\nqubits: 2300\nproduct: 6440000000\ncorrectness: FAIL\n"
    result = parse_eval_output(output)
    assert result.correctness is False


def test_parse_eval_output_missing_field_raises() -> None:
    output = "gates: 2800000\nqubits: 2300\n"
    with pytest.raises(ValueError, match="product"):
        parse_eval_output(output)


def test_run_eval_returns_result_on_success() -> None:
    fake_output = "gates: 4000000\nqubits: 2700\nproduct: 10800000000\ncorrectness: PASS\n"
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout=fake_output, stderr="")
        result = run_eval(Path("/fake/repo"))
    assert result.gate_count == 4_000_000
    assert result.product == 10_800_000_000


def test_run_eval_raises_on_cargo_failure() -> None:
    with patch("evaluator.subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=1, stdout="", stderr="error[E0308]: mismatched types"
        )
        with pytest.raises(RuntimeError, match="cargo"):
            run_eval(Path("/fake/repo"))
```

- [ ] **Step 3.2: Run tests to verify they fail**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -m pytest tests/test_evaluator.py -v
```

Expected: `ImportError: No module named 'evaluator'`

- [ ] **Step 3.3: Write evaluator.py**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/evaluator.py`:

```python
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
```

- [ ] **Step 3.4: Run tests to verify they pass**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -m pytest tests/test_evaluator.py -v
```

Expected: 5 tests PASS.

- [ ] **Step 3.5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/evaluator.py cynic-python/lab/quantum-ecc/tests/test_evaluator.py
git commit -m "feat(quantum-ecc): cargo eval runner (evaluator.py)"
```

---

## Task 4: `agents/inspector.py` — circuit characterization

**Files:**
- Create: `cynic-python/lab/quantum-ecc/agents/inspector.py`
- Modify: `cynic-python/pyproject.toml`

- [ ] **Step 4.1: Add anthropic to pyproject.toml**

In `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/pyproject.toml`, add `"anthropic>=0.40"` to the `dependencies` list:

```toml
dependencies = [
    "pydantic>=2.0",
    "pytest>=7.4",
    "pytest-cov>=4.1",
    "mypy>=1.5",
    "ruff>=0.1",
    "numpy>=1.24",
    "anthropic>=0.40",
]
```

Then install:

```bash
pip install "anthropic>=0.40"
```

- [ ] **Step 4.2: Write agents/inspector.py**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/agents/inspector.py`:

```python
"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — circuit structure inspector.

Calls Claude Haiku to characterize src/point_add/mod.rs.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""
import json
import logging
import os
from dataclasses import dataclass
from pathlib import Path

import anthropic

INSPECTOR_MODEL = "claude-haiku-4-5-20251001"

INSPECTOR_SYSTEM = """You are a quantum circuit analyst. You will read Rust source code
implementing a quantum ECDSA point addition circuit using the kickmix ISA (ops: CX, CCX,
CCZ, SWAP, REGISTER, APPEND_TO_REGISTER). Your job is to characterize the circuit structure.

Answer these questions by reading the code:
1. windowed: Is scalar multiplication windowed (grouped bits + QROM lookup)?
   False means it processes one scalar bit at a time (~256 controlled point adds).
2. projective: Does it use Jacobian/projective coordinates (Z field, three-coordinate points)?
   False means it uses affine coordinates and likely calls modular inversion per point add.
3. inversions_per_add: How many modular inversions occur per ECPointAdd call? (0 if projective)
4. recommended_lever: Given the structure, which lever should be tried first?
   Choose from: windowed_scalar_mult, projective_coordinates, toffoli_ladder_ancilla_reuse,
   karatsuba_field_mult

Return ONLY valid JSON with these exact fields:
{
  "windowed": false,
  "projective": false,
  "inversions_per_add": 1,
  "recommended_lever": "windowed_scalar_mult",
  "reasoning": "One sentence explanation."
}"""


@dataclass
class InspectionResult:
    windowed: bool
    projective: bool
    inversions_per_add: int
    recommended_lever: str
    reasoning: str


def inspect_circuit(circuit_content: str) -> InspectionResult:
    """Characterize circuit structure via Claude Haiku. Raises on API error."""
    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])

    message = client.messages.create(
        model=INSPECTOR_MODEL,
        max_tokens=512,
        system=INSPECTOR_SYSTEM,
        messages=[
            {
                "role": "user",
                "content": f"Analyze this quantum circuit implementation:\n\n```rust\n{circuit_content}\n```",
            }
        ],
    )

    raw = message.content[0].text.strip()
    logging.info("inspector_response", extra={"raw": raw[:200]})

    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        json_start = raw.find("{")
        json_end = raw.rfind("}") + 1
        if json_start == -1:
            raise ValueError(f"Inspector returned non-JSON: {raw[:200]}")
        data = json.loads(raw[json_start:json_end])

    return InspectionResult(
        windowed=bool(data["windowed"]),
        projective=bool(data["projective"]),
        inversions_per_add=int(data["inversions_per_add"]),
        recommended_lever=str(data["recommended_lever"]),
        reasoning=str(data.get("reasoning", "")),
    )


def read_circuit_files(repo_dir: Path) -> str:
    """Read all .rs files from src/point_add/ and concatenate."""
    point_add_dir = repo_dir / "src" / "point_add"
    parts: list[str] = []
    for rs_file in sorted(point_add_dir.glob("*.rs")):
        parts.append(f"// === {rs_file.name} ===\n{rs_file.read_text()}")
    if not parts:
        raise FileNotFoundError(f"No .rs files found in {point_add_dir}")
    return "\n\n".join(parts)
```

- [ ] **Step 4.3: Smoke-test inspector (manual, no real repo needed)**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -c "
from agents.inspector import inspect_circuit
sample = '''pub fn build() -> Vec<Op> {
    let mut ops = vec![];
    for bit in 0..256 {
        ops.push(Op::CCX(bit, bit+1, bit+2));
    }
    ops
}'''
result = inspect_circuit(sample)
print(result)
"
```

Expected: `InspectionResult(windowed=False, projective=False, ...)`. Requires `ANTHROPIC_API_KEY` set.

- [ ] **Step 4.4: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/agents/inspector.py cynic-python/pyproject.toml
git commit -m "feat(quantum-ecc): circuit inspector agent (Claude Haiku)"
```

---

## Task 5: `agents/optimizer.py` — Rust code generation

**Files:**
- Create: `cynic-python/lab/quantum-ecc/agents/optimizer.py`

- [ ] **Step 5.1: Write optimizer.py**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/agents/optimizer.py`:

```python
"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — circuit optimizer agent.

Calls Claude Sonnet to generate improved Rust circuit code.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""
import logging
import os
from dataclasses import dataclass

import anthropic

OPTIMIZER_MODEL = "claude-sonnet-4-6"

KICKMIX_ISA_REFERENCE = """
The kickmix ISA (Rust enum you must use):

```rust
pub enum Op {
    X(usize),
    Z(usize),
    CX(usize, usize),           // CNOT: control, target
    CZ(usize, usize),
    CCX(usize, usize, usize),   // Toffoli: c1, c2, target  ← most expensive
    CCZ(usize, usize, usize),
    SWAP(usize, usize),
    Register(String, usize),    // Declare named register of N qubits
    AppendToRegister(String, usize),
    PushCondition(usize),
    PopCondition,
}
```

Rules:
- All ops are reversible. Ancilla qubits MUST return to |0⟩ after use.
- CCX and CCZ dominate cost — minimize them.
- Metric = total_ops_count × total_qubit_count. Both dimensions matter.
- Correctness checked against P-256 test vectors — must not break it.
- Return ONLY valid Rust code. No markdown fences. No prose outside comments.
"""

OPTIMIZER_SYSTEM = f"""You are a quantum circuit optimizer working on the ecdsa.fail challenge.
You will receive the current Rust source for point_add::build() -> Vec<Op> and a specific
optimization lever to apply.

{KICKMIX_ISA_REFERENCE}

Return the COMPLETE modified Rust source. The function signature must remain:
  pub fn build() -> Vec<Op>

Do NOT wrap in markdown code blocks.
If you cannot implement the lever correctly and safely, return the ORIGINAL code unchanged."""


@dataclass
class OptimizationResult:
    modified_content: str
    lever_applied: str
    changed: bool


def generate_optimization(
    circuit_content: str,
    lever_name: str,
    lever_description: str,
) -> OptimizationResult:
    """Call Claude Sonnet to generate improved Rust. Returns OptimizationResult."""
    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])

    user_prompt = (
        f"Apply this optimization to the circuit:\n\n"
        f"## Lever: {lever_name}\n{lever_description}\n\n"
        f"## Current circuit source:\n{circuit_content}\n\n"
        f"Return the complete modified Rust source."
    )

    message = client.messages.create(
        model=OPTIMIZER_MODEL,
        max_tokens=8192,
        system=OPTIMIZER_SYSTEM,
        messages=[{"role": "user", "content": user_prompt}],
    )

    new_content = message.content[0].text.strip()

    if new_content.startswith("```"):
        lines = new_content.split("\n")
        new_content = "\n".join(
            line for line in lines if not line.startswith("```")
        ).strip()

    changed = new_content != circuit_content.strip()
    logging.info(
        "optimizer_result",
        extra={"lever": lever_name, "changed": changed, "content_len": len(new_content)},
    )

    return OptimizationResult(
        modified_content=new_content,
        lever_applied=lever_name,
        changed=changed,
    )
```

- [ ] **Step 5.2: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/agents/optimizer.py
git commit -m "feat(quantum-ecc): circuit optimizer agent (Claude Sonnet)"
```

---

## Task 6: `harness.py` — main optimization loop

**Files:**
- Create: `cynic-python/lab/quantum-ecc/harness.py`

- [ ] **Step 6.1: Write harness.py**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/harness.py`:

```python
"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — main optimization loop.

Usage:
  python harness.py --repo ./ecdsafail-challenge [--max-iters 20]

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

logging.basicConfig(level=logging.INFO, stream=sys.stdout)

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
        f"({result.gate_count:,}g × {result.qubit_count:,}q)"
    )
    subprocess.run(["git", "commit", "-m", msg], cwd=repo_dir, check=True, capture_output=True)
    logging.info("committed_improvement: lever=%s product=%d", lever, result.product)


def run_loop(repo_dir: Path, max_iterations: int) -> None:
    """Main optimization loop."""
    if not os.environ.get("ANTHROPIC_API_KEY"):
        raise RuntimeError("ANTHROPIC_API_KEY not set")

    lever_descriptions = load_lever_descriptions()
    state = load_state(STATE_FILE)

    logging.info("loop_start: best_product=%d tried=%s", state.best_product, state.tried_levers)

    for iteration in range(max_iterations):
        logging.info("--- iteration %d ---", iteration)

        circuit_content = read_circuit_files(repo_dir)
        inspection = inspect_circuit(circuit_content)
        logging.info(
            "inspection: windowed=%s projective=%s recommended=%s",
            inspection.windowed, inspection.projective, inspection.recommended_lever,
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
```

- [ ] **Step 6.2: Smoke-test imports**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python -c "from harness import run_loop, select_lever, load_lever_descriptions; print('imports OK')"
```

Expected: `imports OK`

- [ ] **Step 6.3: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/harness.py
git commit -m "feat(quantum-ecc): main optimization loop (harness.py)"
```

---

## Task 7: `bootstrap.sh` — setup and baseline

**Files:**
- Create: `cynic-python/lab/quantum-ecc/bootstrap.sh`

- [ ] **Step 7.1: Write bootstrap.sh**

Create `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/bootstrap.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$SCRIPT_DIR/ecdsafail-challenge"
REPO_URL="https://github.com/ecdsafail/ecdsafail-challenge"

echo "=== Quantum ECC Harness Bootstrap ==="

if ! command -v cargo &>/dev/null; then
    echo "ERROR: cargo not found. Install Rust: https://rustup.rs/"; exit 1
fi
if ! command -v git &>/dev/null; then
    echo "ERROR: git not found."; exit 1
fi
if [[ -z "${ANTHROPIC_API_KEY:-}" ]]; then
    echo "ERROR: ANTHROPIC_API_KEY not set."; exit 1
fi

echo "✓ cargo $(cargo --version | cut -d' ' -f2), git $(git --version | cut -d' ' -f3)"

if [[ -d "$REPO_DIR/.git" ]]; then
    echo "Repo exists — pulling latest..."
    timeout 30 git -C "$REPO_DIR" pull --ff-only
else
    echo "Cloning $REPO_URL..."
    timeout 120 git clone "$REPO_URL" "$REPO_DIR"
fi

echo "✓ Challenge repo at $REPO_DIR"
echo "Running baseline evaluation (2-5 min)..."
cd "$REPO_DIR"
cargo build --release 2>&1 | tail -3
cargo run --release --bin build 2>&1 | tail -2
EVAL_OUTPUT=$(cargo run --release --bin eval 2>&1)
echo "$EVAL_OUTPUT"

STATE_FILE="$SCRIPT_DIR/state.json"
python3 - <<PYEOF
import json, re
from pathlib import Path

output = """$EVAL_OUTPUT"""

def extract(field):
    m = re.search(rf"^{field}:\s*(\S+)", output, re.MULTILINE)
    return int(m.group(1)) if m else 0

gates  = extract("gates")
qubits = extract("qubits")
product = extract("product")

state_path = Path("$STATE_FILE")
if not state_path.exists() or json.loads(state_path.read_text()).get("baseline_product", 0) == 0:
    state_path.write_text(json.dumps({
        "best_product": product,
        "best_gate_count": gates,
        "best_qubit_count": qubits,
        "tried_levers": [],
        "iterations": [],
        "baseline_product": product,
    }, indent=2))
    print(f"State initialized: product={product:,} ({gates:,} gates x {qubits:,} qubits)")
else:
    print("State file exists — preserving progress.")
PYEOF

echo ""
echo "=== Bootstrap complete ==="
echo "Run the harness with:"
echo "  cd $SCRIPT_DIR && python harness.py --repo $REPO_DIR --max-iters 20"
```

- [ ] **Step 7.2: Make executable**

```bash
chmod +x /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/bootstrap.sh
```

- [ ] **Step 7.3: Run bootstrap.sh**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
./bootstrap.sh
```

Expected: ends with `=== Bootstrap complete ===` and a `state.json` with `baseline_product` set.

- [ ] **Step 7.4: Verify eval output format matches parser (MANDATORY)**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python - <<'EOF'
import subprocess
from evaluator import parse_eval_output
proc = subprocess.run(
    ["cargo", "run", "--release", "--bin", "eval"],
    cwd="ecdsafail-challenge", capture_output=True, text=True
)
print("raw:", repr(proc.stdout[:400]))
result = parse_eval_output(proc.stdout)
print("parsed:", result)
EOF
```

If `parse_eval_output` raises `ValueError`: the actual output format differs from assumed. Update the regex keys in `evaluator.py:parse_eval_output` to match the real field names before proceeding.

- [ ] **Step 7.5: Commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/bootstrap.sh
git commit -m "feat(quantum-ecc): bootstrap.sh — clone, baseline eval, state init"
```

---

## Task 8: End-to-end smoke run

- [ ] **Step 8.1: Run one iteration**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc
python harness.py --repo ./ecdsafail-challenge --max-iters 1
```

Watch for these log lines (in order):
1. `loop_start` — confirms state loaded
2. `inspection:` — confirms inspector agent worked
3. `selected lever:` — confirms lever selection
4. Either `IMPROVED:` or `rejected:` — confirms full loop completed

- [ ] **Step 8.2: Check state.json updated**

```bash
cat /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python/lab/quantum-ecc/state.json
```

Expected: `iterations` list has one entry.

- [ ] **Step 8.3: If correctness FAIL on first try — expected behavior**

The optimizer may generate incorrect Rust on the first attempt. The harness should:
- Log `rejected: ... correctness=False`
- Revert the file
- Mark the lever as tried in `state.json`

Run again with `--max-iters 1` to try the next lever. This is normal — track which levers produce improvements in `state.json`.

- [ ] **Step 8.4: Final commit**

```bash
cd /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
git add cynic-python/lab/quantum-ecc/
git commit -m "chore(quantum-ecc): post-smoke-run checkpoint"
```

---

## Self-Review

**Spec coverage:**
- ✓ bootstrap.sh — Task 7
- ✓ harness.py main loop — Task 6
- ✓ inspector agent (Haiku) — Task 4
- ✓ optimizer agent (Sonnet) — Task 5
- ✓ evaluator (cargo build+eval) — Task 3
- ✓ levers.md reference — Task 1
- ✓ state.json persistence — Task 2
- ✓ Tier 1 docstrings with 2026-07-02 death date — all Python files
- ✓ No shell=True — evaluator.py and harness.py use list args throughout
- ✓ Structured logging — all Python files use `logging.*`
- ✓ Type annotations — present on all functions
- ✓ anthropic dependency — added in Task 4

**Type consistency:** `EvalResult`, `CircuitState`, `InspectionResult`, `OptimizationResult` defined before use. All imports in harness.py match module exports.

**Known risk:** Task 7.4 is mandatory — `parse_eval_output` regex must be verified against real eval output before trusting any improvement delta. If the eval binary outputs different field names, the fix is localized to two lines in `evaluator.py`.
