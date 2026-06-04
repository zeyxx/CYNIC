"""
Tier 1 EXPERIMENTAL: Quantum ECC harness — circuit optimizer agent.

Calls Gemini 2.5 Pro to generate improved Rust circuit code.
Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""
import logging
import os
from dataclasses import dataclass

from google import genai
from google.genai import types as genai_types

OPTIMIZER_MODEL = "gemini-2.5-pro"

KICKMIX_ISA_REFERENCE = """
The kickmix ISA (Rust enum you must use):

```rust
pub enum Op {
    X(usize),
    Z(usize),
    CX(usize, usize),           // CNOT: control, target
    CZ(usize, usize),
    CCX(usize, usize, usize),   // Toffoli: c1, c2, target  <- most expensive
    CCZ(usize, usize, usize),
    SWAP(usize, usize),
    Register(String, usize),    // Declare named register of N qubits
    AppendToRegister(String, usize),
    PushCondition(usize),
    PopCondition,
}
```

Rules:
- All ops are reversible. Ancilla qubits MUST return to |0> after use.
- CCX and CCZ dominate cost — minimize them.
- Score = avg_toffoli x peak_qubits. Both dimensions matter.
- Correctness checked against secp256k1 test vectors — must not break it.
- Return ONLY valid Rust code. No markdown fences. No prose outside comments.
"""

OPTIMIZER_SYSTEM = f"""You are a quantum circuit optimizer working on the ecdsa.fail challenge.
You will receive the current Rust source implementing point_add::build() -> Vec<Op> and a
specific optimization lever to apply.

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
    """Call Gemini 2.5 Pro to generate improved Rust. Returns OptimizationResult."""
    client = genai.Client(api_key=os.environ["GEMINI_API_KEY"])

    user_prompt = (
        f"Apply this optimization to the circuit:\n\n"
        f"## Lever: {lever_name}\n{lever_description}\n\n"
        f"## Current circuit source:\n{circuit_content}\n\n"
        f"Return the complete modified Rust source."
    )

    response = client.models.generate_content(
        model=OPTIMIZER_MODEL,
        contents=user_prompt,
        config=genai_types.GenerateContentConfig(
            system_instruction=OPTIMIZER_SYSTEM,
            max_output_tokens=32768,
            temperature=0.2,
        ),
    )

    new_content = response.text.strip()

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
