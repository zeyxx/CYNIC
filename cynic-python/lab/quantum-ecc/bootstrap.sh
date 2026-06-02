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
    echo "WARNING: ANTHROPIC_API_KEY not set — needed by harness.py, not by bootstrap."
fi

echo "cargo $(cargo --version | cut -d' ' -f2), git $(git --version | cut -d' ' -f3)"

if [[ -d "$REPO_DIR/.git" ]]; then
    echo "Repo exists — pulling latest..."
    timeout 30 git -C "$REPO_DIR" pull --ff-only
else
    echo "Cloning $REPO_URL..."
    timeout 120 git clone "$REPO_URL" "$REPO_DIR"
fi

echo "Challenge repo at $REPO_DIR"
echo "Running baseline evaluation (2-5 min first time)..."
cd "$REPO_DIR"
cargo build --release 2>&1 | tail -3
cargo run --release --bin build_circuit 2>&1 | tail -2
EVAL_OUTPUT=$(cargo run --release --bin eval_circuit 2>&1)
echo "$EVAL_OUTPUT"

STATE_FILE="$SCRIPT_DIR/state.json"
python3 - <<PYEOF
import json, re, sys
from pathlib import Path

output = """$EVAL_OUTPUT"""

def extract_float(pattern):
    m = re.search(pattern, output, re.MULTILINE)
    if not m:
        print(f"WARNING: pattern '{pattern}' not found in eval output", file=sys.stderr)
        return 0
    return float(m.group(1))

toffoli = int(extract_float(r"avg executed Toffoli\s*:\s*([\d.]+)"))
qubits  = int(extract_float(r"qubits\s*:\s*([\d.]+)"))
product = toffoli * qubits
gates   = toffoli

state_path = Path("$STATE_FILE")
if not state_path.exists():
    existing_baseline = 0
else:
    try:
        existing_baseline = json.loads(state_path.read_text()).get("baseline_product", 0)
    except Exception:
        existing_baseline = 0

if existing_baseline == 0:
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
    print(f"State file exists (baseline={existing_baseline:,}) — preserving progress.")
PYEOF

echo ""
echo "=== Bootstrap complete ==="
echo ""
echo "To run the harness:"
echo "  cd $SCRIPT_DIR && python3 harness.py --repo $REPO_DIR --max-iters 20"
