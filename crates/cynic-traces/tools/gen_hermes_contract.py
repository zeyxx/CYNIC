#!/usr/bin/env python3
"""Regenerate the machine-transcribed Hermes conservation contract.

Reads FIELD_FATE maps + reasoning concept from the Python normalizer
contract and emits src/adapters/hermes/contract.rs. The generated file
is committed; this script is the replayable gesture (review finding:
an inline /tmp script is not a gesture).

Usage:
  PYTHONPATH=apps/cynic-data/src python3 crates/cynic-traces/tools/gen_hermes_contract.py

The script refuses to change the contract silently: it prints the counts
(messages/sessions/usage/total fates, out-of-scope, concepts) so the
operator sees what moved.
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(HERE, "src", "adapters", "hermes", "contract.rs")


def esc(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def main() -> int:
    from cynic_data.canonical.adapters.normalizers.hermes_contract import CONTRACT

    lines: list[str] = []
    lines.append("//! Hermes conservation contract — MACHINE-TRANSCRIBED.")
    lines.append("//!")
    lines.append("//! Do not hand-edit. Regenerate with:")
    lines.append("//!   PYTHONPATH=apps/cynic-data/src python3 crates/cynic-traces/tools/gen_hermes_contract.py")
    lines.append("//! Prose rationale lives in the Python source")
    lines.append("//! (apps/cynic-data/.../hermes_contract.py).")
    lines.append("")
    lines.append("use std::collections::HashMap;")
    lines.append("")
    lines.append("use crate::conservation::{ConceptResolution, ConservationContract};")
    lines.append("")
    lines.append('pub const PRODUCER: &str = "hermes-agent";')
    lines.append(f"pub const PRODUCER_SCHEMA_VERSION: i64 = {CONTRACT.producer_schema_version};")
    lines.append(
        f'pub const PRODUCER_SCHEMA_SOURCE: &str = "{esc(CONTRACT.producer_schema_source)}";'
    )
    lines.append("")
    lines.append("pub fn contract() -> ConservationContract {")
    lines.append("    let mut field_fate = HashMap::new();")
    for k in sorted(CONTRACT.field_fate):
        lines.append(
            f'    field_fate.insert("{esc(k)}".to_owned(), "{esc(CONTRACT.field_fate[k])}".to_owned());'
        )
    lines.append("    let mut out_of_scope = HashMap::new();")
    for k in sorted(CONTRACT.out_of_scope):
        lines.append(
            f'    out_of_scope.insert("{esc(k)}".to_owned(), "{esc(CONTRACT.out_of_scope[k])}".to_owned());'
        )
    concepts = list(CONTRACT.concepts)
    if len(concepts) != 1 or concepts[0].concept != "reasoning":
        print(
            f"contract shape changed (concepts={[c.concept for c in concepts]}): "
            "update the generator AND the Rust consumer",
            file=sys.stderr,
        )
        return 1
    c = concepts[0]
    lines.append("    let reasoning = ConceptResolution {")
    lines.append(f'        concept: "{c.concept}".to_owned(),')
    lines.append(
        "        homes: vec![%s]," % ", ".join(f'"{h}".to_owned()' for h in c.homes)
    )
    lines.append("        model_profiles: HashMap::new(),")
    lines.append("    };")
    lines.append("    let mut profiles = reasoning.model_profiles.clone();")
    for m in sorted(c.model_profiles):
        lines.append(
            f'    profiles.insert("{esc(m)}".to_owned(), "{esc(c.model_profiles[m])}".to_owned());'
        )
    lines.append("    let reasoning = ConceptResolution { model_profiles: profiles, ..reasoning };")
    lines.append("    let mut transforms = HashMap::new();")
    for k in sorted(CONTRACT.transforms):
        lines.append(
            f'    transforms.insert("{esc(k)}".to_owned(), "{esc(CONTRACT.transforms[k])}".to_owned());'
        )
    lines.append("    ConservationContract {")
    lines.append("        producer: PRODUCER.to_owned(),")
    lines.append("        producer_schema_version: PRODUCER_SCHEMA_VERSION,")
    lines.append("        producer_schema_source: PRODUCER_SCHEMA_SOURCE.to_owned(),")
    lines.append("        field_fate,")
    lines.append("        concepts: vec![reasoning],")
    lines.append("        transforms,")
    lines.append("        out_of_scope,")
    lines.append("    }")
    lines.append("}")
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    n_msg = sum(1 for k in CONTRACT.field_fate if k.startswith("messages."))
    n_ses = sum(1 for k in CONTRACT.field_fate if k.startswith("sessions."))
    print(f"wrote {OUT}")
    print(
        f"fates: messages={n_msg} sessions={n_ses} total={len(CONTRACT.field_fate)} "
        f"out_of_scope={len(CONTRACT.out_of_scope)} concepts={[c.concept for c in concepts]}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
