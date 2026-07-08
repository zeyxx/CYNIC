#!/usr/bin/env python3
"""Probe live Pump/PumpSwap evidence for the CYNIC Solana library."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[4]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from organs.solana.src.adapters.kernel_observe import post_observation
from organs.solana.src.adapters.solana_rpc import SolanaRpcClient
from organs.solana.src.core.pump_live_probe import MintProbe, classify_mint_probe


BASE58_RE = re.compile(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mints", nargs="*", help="Solana mint addresses to probe")
    parser.add_argument("--mint-file", help="File containing one mint per line")
    parser.add_argument("--rpc-url", help="Solana RPC endpoint; defaults to SOLANA_RPC_URL/Helius/public RPC")
    parser.add_argument("--signature-limit", type=int, default=20)
    parser.add_argument("--transaction-limit", type=int, default=8)
    parser.add_argument("--observe", action="store_true", help="POST each result to Kernel /observe")
    parser.add_argument("--pretty", action="store_true", help="Print one pretty JSON array instead of JSONL")
    args = parser.parse_args(argv)

    mints = list(_iter_mints(args.mints, args.mint_file))
    if not mints:
        parser.error("provide at least one mint or --mint-file")

    client = SolanaRpcClient(endpoint=args.rpc_url)
    results = [
        _probe_one(client, mint, args.signature_limit, args.transaction_limit)
        for mint in mints
    ]

    if args.observe:
        for result in results:
            result["kernel_observe"] = _observe_result(result)

    if args.pretty:
        print(json.dumps(results, indent=2, sort_keys=True))
    else:
        for result in results:
            print(json.dumps(result, sort_keys=True))
    return 0


def _iter_mints(cli_mints: list[str], mint_file: str | None) -> Iterable[str]:
    seen: set[str] = set()
    for mint in cli_mints:
        normalized = mint.strip()
        if normalized and normalized not in seen:
            seen.add(normalized)
            yield normalized
    if mint_file:
        for line in Path(mint_file).read_text().splitlines():
            normalized = line.strip()
            if not normalized or normalized.startswith("#"):
                continue
            if normalized not in seen:
                seen.add(normalized)
                yield normalized


def _probe_one(
    client: SolanaRpcClient,
    mint: str,
    signature_limit: int,
    transaction_limit: int,
) -> dict[str, object]:
    if not BASE58_RE.match(mint):
        return _error_result(mint, "invalid_base58_shape")
    try:
        account = client.get_mint_account(mint)
        signatures = client.get_signatures(mint, max(1, signature_limit))
        transactions = []
        for signature_info in signatures[: max(0, transaction_limit)]:
            signature = signature_info.get("signature")
            if not isinstance(signature, str):
                continue
            try:
                transactions.append(client.get_transaction(signature))
            except RuntimeError as exc:
                transactions.append({"error": str(exc), "signature": signature})
        return classify_mint_probe(
            MintProbe(
                mint=mint,
                account=account,
                signatures=signatures,
                transactions=transactions,
            )
        )
    except RuntimeError as exc:
        return _error_result(mint, str(exc))


def _error_result(mint: str, error: str) -> dict[str, object]:
    return {
        "schema": "cynic.solana.pump_live_probe.v1",
        "domain": "solana",
        "kind": "mechanism_observation",
        "target": f"solana:mint:{mint}",
        "mint": mint,
        "classification": "probe_error",
        "confidence": "none",
        "error": error,
    }


def _observe_result(result: dict[str, object]) -> dict[str, object]:
    try:
        return post_observation(result)
    except RuntimeError as exc:
        return {"status": "failed", "error": str(exc)}


if __name__ == "__main__":
    raise SystemExit(main())
