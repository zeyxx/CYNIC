#!/usr/bin/env python3
"""Validate infra/verticals/*.json against cynic.vertical_intent.v1 convention.

Lightweight, stdlib-only. Proves the vertical restructuring is coherent:
- every intent.json carries the required schema fields
- read_back_proof declares PoH (offchain) + DNS external_proof + thresholds
- remediation is REQUIRED when bounded_action.mutates == true, and ABSENT
  otherwise (absence is not a model error per manifest convention)
- the manifest lists every vertical and each has a matching intent.json
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VERTICALS_DIR = os.path.join(ROOT, "infra", "verticals")

REQUIRED_TOP = [
    "schema", "vertical", "layer", "owner_crate", "desired_state",
    "code_contract", "observe", "reconcile", "bounded_action", "read_back_proof",
]
REQUIRED_PROOF = ["poh_checkpoint", "dns_proof", "maturity_percent_min", "known_coverage_percent_min"]


def load(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def err(msg):
    print("  ERROR:", msg)
    return False


def validate_intent(path):
    ok = True
    data = load(path)
    schema = data.get("schema")
    if schema != "cynic.vertical_intent.v1":
        ok &= err(f"{path}: schema must be cynic.vertical_intent.v1, got {schema!r}")
    for field in REQUIRED_TOP:
        if field not in data:
            ok &= err(f"{path}: missing top-level field {field!r}")
    # read_back_proof shape
    rb = data.get("read_back_proof", {})
    for field in REQUIRED_PROOF:
        if field not in rb:
            ok &= err(f"{path}: read_back_proof missing {field!r}")
    if "poh_checkpoint" in rb and "schema" not in rb["poh_checkpoint"]:
        ok &= err(f"{path}: read_back_proof.poh_checkpoint missing schema")
    if "dns_proof" in rb:
        dns = rb["dns_proof"]
        if "schema" not in dns:
            ok &= err(f"{path}: read_back_proof.dns_proof missing schema")
        if not dns.get("external_proof"):
            ok &= err(f"{path}: read_back_proof.dns_proof.external_proof must be true")
    # remediation conditional rule
    mutates = data.get("bounded_action", {}).get("mutates", False)
    has_remediation = "remediation" in data
    if mutates and not has_remediation:
        ok &= err(f"{path}: bounded_action.mutates==true but remediation ABSENT (must be present)")
    if not mutates and has_remediation:
        ok &= err(f"{path}: bounded_action.mutates==false but remediation PRESENT (should be absent per convention)")
    return ok


def main():
    manifest_path = os.path.join(VERTICALS_DIR, "manifest.json")
    if not os.path.exists(manifest_path):
        print("FATAL: manifest.json missing")
        return 1
    manifest = load(manifest_path)
    if manifest.get("schema") != "cynic.verticals_manifest.v1":
        print("FATAL: manifest schema mismatch")
        return 1

    listed = {v["id"] for v in manifest.get("verticals", [])}
    present = set()
    all_ok = True

    for root, _dirs, files in os.walk(VERTICALS_DIR):
        for fn in files:
            if fn != "intent.json":
                continue
            path = os.path.join(root, fn)
            rel = os.path.relpath(path, VERTICALS_DIR)
            vertical = os.path.dirname(rel)
            present.add(vertical)
            print(f"check {vertical}")
            all_ok &= validate_intent(path)

    # manifest <-> intent.json symmetry
    missing_intent = listed - present
    extra_intent = present - listed
    if missing_intent:
        all_ok &= err(f"manifest lists but no intent.json: {sorted(missing_intent)}")
    if extra_intent:
        all_ok &= err(f"intent.json present but not in manifest: {sorted(extra_intent)}")

    print()
    if all_ok:
        print(f"OK: {len(present)} verticales conform to cynic.vertical_intent.v1")
        return 0
    print("FAIL: convention violations above")
    return 1


if __name__ == "__main__":
    sys.exit(main())
