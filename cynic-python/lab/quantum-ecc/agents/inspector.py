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
