#!/usr/bin/env python3
"""GASdf invocation membrane."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start.parent


ROOT = _bootstrap_repo_path()
ORGAN_ROOT = ROOT / "organs" / "gasdf"
ORGAN_SRC = ORGAN_ROOT / "src"
if str(ORGAN_SRC) not in sys.path:
    sys.path.insert(0, str(ORGAN_SRC))

from adapters.runtime_membrane import OrganOwnedRuntimeMembrane
from core.policy import evaluate_policy


def status(lifecycle_approved: bool = False) -> dict[str, object]:
    runtime = OrganOwnedRuntimeMembrane(ORGAN_ROOT)
    evidence = runtime.evidence()
    policy = evaluate_policy(
        has_fee_payer_env=bool(os.environ.get("FEE_PAYER_PRIVATE_KEY")),
        runtime_owned_by_organ=True,
        signer_adapter_present=evidence.signer_adapter_present,
        lifecycle_approved=lifecycle_approved,
    )
    return {
        "schema": "cynic.gasdf.status.v1",
        "mutates": False,
        "runtime": evidence.to_dict(),
        "runtime_owned_by_organ": policy.runtime_owned_by_organ,
        "can_sign": policy.can_sign,
        "required_gates": list(policy.required_gates),
        "reason": policy.reason,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-gasdf control surface")
    parser.add_argument("--lifecycle-approved", action="store_true")
    parser.add_argument("--loop", action="store_true")
    parser.add_argument("--interval", type=float, default=30.0)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.loop:
        while True:
            print(json.dumps(status(args.lifecycle_approved), sort_keys=True), flush=True)
            time.sleep(args.interval)
    print(json.dumps(status(args.lifecycle_approved), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
