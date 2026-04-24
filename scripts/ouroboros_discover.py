#!/usr/bin/env python3
"""Ouroboros Discovery — ingest real market data, optionally score with Dogs.

Two-phase pipeline:
  1. INGEST: discover tokens → enrich via Helius → store raw features (JSONL)
  2. SCORE (optional): submit to kernel /judge → append verdict to row

The raw features dataset is the primary asset. Dog verdicts are a column, not the product.

Usage:
    python3 scripts/ouroboros_discover.py                             # ingest only (default)
    python3 scripts/ouroboros_discover.py --score                     # ingest + judge
    python3 scripts/ouroboros_discover.py --source all --limit 10     # all sources
    python3 scripts/ouroboros_discover.py --resnapshot                # re-enrich existing mints for T+Nd ground truth
    python3 scripts/ouroboros_discover.py --dry-run                   # discover but don't enrich
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATASET_PATH = ROOT / "data" / "token_discovery.jsonl"
WATCHLIST_PATH = ROOT / "scripts" / "watchlist.txt"
SCREENER_PATH = ROOT / "scripts" / "token_screener.py"

PUMPFUN_PROGRAM = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
CULTSCREENER_API = "https://cultscreener-api.onrender.com"


# ── Env ──

def load_env():
    for path in [Path.home() / ".cynic-env", Path.home() / ".config" / "cynic" / "env"]:
        if path.exists():
            for line in path.read_text().splitlines():
                line = line.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue
                k, v = line.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def load_helius_key() -> str:
    key = os.environ.get("HELIUS_API_KEY")
    if key:
        return key
    config_path = Path.home() / ".helius" / "config.json"
    if config_path.exists():
        with open(config_path) as f:
            return json.load(f).get("apiKey", "")
    return ""


# ── Helius API ──

def helius_rpc(api_key: str, method: str, params: list) -> dict | list | None:
    url = f"https://mainnet.helius-rpc.com/?api-key={api_key}"
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.load(resp)
            if "error" in data:
                print(f"RPC error ({method}): {data['error']}", file=sys.stderr)
                return None
            return data.get("result")
    except Exception as e:
        print(f"RPC failed ({method}): {e}", file=sys.stderr)
        return None


def helius_das(api_key: str, method: str, params: dict) -> dict | None:
    url = f"https://mainnet.helius-rpc.com/?api-key={api_key}"
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.load(resp)
            if "error" in data:
                print(f"DAS error ({method}): {data['error']}", file=sys.stderr)
                return None
            return data.get("result")
    except Exception as e:
        print(f"DAS failed ({method}): {e}", file=sys.stderr)
        return None


# ── Discovery sources ──

def discover_dexscreener(limit: int = 20) -> list[str]:
    """DexScreener /token-profiles/latest/v1 — free, no auth."""
    url = "https://api.dexscreener.com/token-profiles/latest/v1"
    req = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "CYNIC/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            tokens = json.load(resp)
    except Exception as e:
        print(f"DexScreener failed: {e}", file=sys.stderr)
        return []
    mints = [t["tokenAddress"] for t in tokens
             if t.get("chainId") == "solana" and t.get("tokenAddress")]
    print(f"Discovery [dexscreener]: {len(mints)} Solana tokens", file=sys.stderr)
    return mints[:limit]


def discover_pumpfun_creates(api_key: str, limit: int = 10, scan_depth: int = 200) -> list[str]:
    """Scan recent pump.fun txs for 'Instruction: Create' in logs."""
    if not api_key:
        print("Skipping pumpfun: no HELIUS_API_KEY", file=sys.stderr)
        return []
    sigs = helius_rpc(api_key, "getSignaturesForAddress",
                      [PUMPFUN_PROGRAM, {"limit": scan_depth}])
    if not sigs:
        return []
    mints = []
    scanned = 0
    for sig_info in sigs:
        sig = sig_info.get("signature", "")
        if not sig:
            continue
        tx = helius_rpc(api_key, "getTransaction",
                        [sig, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}])
        scanned += 1
        if not tx:
            continue
        logs = tx.get("meta", {}).get("logMessages", [])
        if not any("Instruction: Create" in log for log in logs):
            continue
        msg = tx.get("transaction", {}).get("message", {})
        for ix in msg.get("instructions", []):
            if ix.get("programId") == PUMPFUN_PROGRAM:
                accts = ix.get("accounts", [])
                if len(accts) >= 3:
                    mint = accts[2]
                    if mint not in mints:
                        mints.append(mint)
                    break
        if len(mints) >= limit:
            break
    print(f"Discovery [pumpfun]: {len(mints)} creates (scanned {scanned}/{len(sigs)} txs)", file=sys.stderr)
    return mints


def discover_cultscreener(limit: int = 30) -> list[str]:
    """CultScreener curated tokens — community-vetted projects."""
    url = f"{CULTSCREENER_API}/api/curated"
    req = urllib.request.Request(url, headers={"Accept": "application/json", "User-Agent": "CYNIC/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.load(resp)
    except Exception as e:
        print(f"CultScreener failed: {e}", file=sys.stderr)
        return []
    tokens = data.get("tokens", data) if isinstance(data, dict) else data
    if not isinstance(tokens, list):
        return []
    mints = [t.get("mintAddress", t.get("mint_address", ""))
             for t in tokens if t.get("mintAddress") or t.get("mint_address")]
    print(f"Discovery [cultscreener]: {len(mints)} curated tokens", file=sys.stderr)
    return mints[:limit]


# ── Enrichment (raw features from Helius — reuses token_screener.py logic) ──

# DEX programs for pool filtering (from token_screener.py)
DEX_PROGRAMS = {
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
    "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
    "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb",
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
    "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg",
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
}

BURN_ADDRESSES = {
    "11111111111111111111111111111111",
    "1nc1nerator11111111111111111111111111111111",
}


def enrich_token(api_key: str, mint: str) -> dict | None:
    """Fetch raw on-chain features for a token. Returns structured dict or None."""
    # 1. getAsset — metadata, authorities, price
    asset_result = helius_das(api_key, "getAsset", {"id": mint})
    if not asset_result:
        return None

    content = asset_result.get("content", {})
    metadata = content.get("metadata", {})
    token_info = asset_result.get("token_info", {})
    price_info = token_info.get("price_info", {})

    supply_raw = token_info.get("supply")
    decimals = token_info.get("decimals")
    human_supply = supply_raw / (10 ** decimals) if supply_raw and decimals else None
    price_usd = price_info.get("price_per_token")
    market_cap = human_supply * price_usd if human_supply and price_usd else None

    features = {
        "mint": mint,
        "name": metadata.get("name"),
        "symbol": metadata.get("symbol"),
        "token_standard": metadata.get("token_standard"),
        "supply_raw": supply_raw,
        "supply_human": human_supply,
        "decimals": decimals,
        "price_usd": price_usd,
        "market_cap": market_cap,
        "mint_authority": token_info.get("mint_authority"),
        "freeze_authority": token_info.get("freeze_authority"),
        "mutable": asset_result.get("mutable"),
        "description": metadata.get("description", "")[:200] if metadata.get("description") else None,
    }

    # 2. getTokenLargestAccounts — top-20 holder distribution
    largest = helius_rpc(api_key, "getTokenLargestAccounts", [mint])
    holders = largest.get("value", []) if largest and isinstance(largest, dict) else []

    if holders and supply_raw and supply_raw > 0:
        # Resolve pool owners for filtering
        pool_addrs = _resolve_pools(api_key, holders)
        real_balances = []
        pool_count = len(pool_addrs)
        for acc in holders:
            addr = acc.get("address", "")
            if addr in pool_addrs:
                continue
            amount = int(acc.get("amount", "0"))
            if amount > 0:
                real_balances.append(amount)
        real_balances.sort(reverse=True)

        features["holder_top20_count"] = len(holders)
        features["holder_real_count"] = len(real_balances)
        features["pool_count"] = pool_count
        if real_balances:
            features["top1_pct"] = round((real_balances[0] / supply_raw) * 100, 2)
            features["top10_pct"] = round((sum(real_balances[:10]) / supply_raw) * 100, 2)
            features["herfindahl"] = round(sum((b / supply_raw) ** 2 for b in real_balances), 6)
        # Raw balances for future analysis (top-5 only to keep rows manageable)
        features["top5_balances"] = [int(b) for b in real_balances[:5]]
    else:
        features["holder_top20_count"] = 0
        features["holder_real_count"] = 0
        features["pool_count"] = 0

    return features


def _resolve_pools(api_key: str, accounts: list[dict]) -> set[str]:
    """Resolve which token accounts are DEX pools (from token_screener.py pattern)."""
    if not accounts:
        return set()
    token_addrs = [a.get("address", "") for a in accounts]
    result = helius_rpc(api_key, "getMultipleAccounts", [token_addrs, {"encoding": "jsonParsed"}])
    if not result or "value" not in result:
        return set()

    wallet_to_token = {}
    for i, info in enumerate(result["value"]):
        if not info:
            continue
        parsed = info.get("data", {}).get("parsed", {}).get("info", {})
        wallet_owner = parsed.get("owner", "")
        if wallet_owner:
            wallet_to_token[wallet_owner] = token_addrs[i]

    pool_addrs = set()
    wallets_to_check = []
    for wallet, token_addr in wallet_to_token.items():
        if wallet in BURN_ADDRESSES:
            pool_addrs.add(token_addr)
        else:
            wallets_to_check.append((wallet, token_addr))

    if not wallets_to_check:
        return pool_addrs

    wallet_addrs = [w for w, _ in wallets_to_check]
    result2 = helius_rpc(api_key, "getMultipleAccounts", [wallet_addrs, {"encoding": "base64"}])
    if not result2 or "value" not in result2:
        return pool_addrs

    for i, info in enumerate(result2["value"]):
        if info and info.get("owner", "") in DEX_PROGRAMS:
            pool_addrs.add(wallets_to_check[i][1])
    return pool_addrs


# ── Scoring (optional — calls kernel /judge) ──

def score_token(mint: str) -> dict | None:
    """Run token_screener.py → kernel /judge. Returns verdict dict or None."""
    try:
        result = subprocess.run(
            [sys.executable, str(SCREENER_PATH), mint],
            capture_output=True, text=True, timeout=180,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        return None


# ── Dedup ──

def load_seen_mints() -> set[str]:
    seen = set()
    if DATASET_PATH.exists():
        for line in DATASET_PATH.read_text().splitlines():
            if not line.strip():
                continue
            try:
                seen.add(json.loads(line).get("mint", ""))
            except json.JSONDecodeError:
                pass
    if WATCHLIST_PATH.exists():
        for line in WATCHLIST_PATH.read_text().splitlines():
            mint = line.strip().split("#")[0].strip()
            if mint:
                seen.add(mint)
    return seen


def load_existing_mints() -> list[str]:
    """Load all mints from the dataset (for resnapshot mode)."""
    mints = []
    if DATASET_PATH.exists():
        for line in DATASET_PATH.read_text().splitlines():
            if not line.strip():
                continue
            try:
                mints.append(json.loads(line)["mint"])
            except (json.JSONDecodeError, KeyError):
                pass
    return mints


# ── Dataset ──

def append_row(row: dict):
    DATASET_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(DATASET_PATH, "a") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")


def build_row(mint: str, source: str, features: dict | None, verdict: dict | None, snapshot_n: int = 0) -> dict:
    """Build a dataset row from raw features and optional verdict."""
    row = {
        "mint": mint,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "source": source,
        "snapshot": snapshot_n,  # 0 = initial discovery, 1+ = re-snapshots
    }

    if features:
        row["features"] = {
            "name": features.get("name"),
            "symbol": features.get("symbol"),
            "supply_human": features.get("supply_human"),
            "price_usd": features.get("price_usd"),
            "market_cap": features.get("market_cap"),
            "mint_authority": features.get("mint_authority") is not None,
            "freeze_authority": features.get("freeze_authority") is not None,
            "mutable": features.get("mutable"),
            "top1_pct": features.get("top1_pct"),
            "top10_pct": features.get("top10_pct"),
            "herfindahl": features.get("herfindahl"),
            "pool_count": features.get("pool_count"),
            "holder_real_count": features.get("holder_real_count"),
            "top5_balances": features.get("top5_balances"),
        }
    else:
        row["features"] = None

    if verdict:
        row["verdict"] = {
            "kind": verdict.get("verdict"),
            "q_score": verdict.get("q_score", {}).get("total"),
            "voter_count": verdict.get("voter_count"),
            "axioms": {
                k: verdict.get("q_score", {}).get(k)
                for k in ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"]
                if verdict.get("q_score", {}).get(k) is not None
            },
            "dogs": [
                {"id": d.get("dog_id"), "s": d.get("sovereignty")}
                for d in verdict.get("dog_scores", [])
            ],
            "failed_dogs": verdict.get("failed_dogs", []),
            "failed_dog_errors": verdict.get("failed_dog_errors", {}),
        }

    return row


# ── Main ──

def main():
    parser = argparse.ArgumentParser(description="Ouroboros Discovery — ingest market data")
    parser.add_argument("--mints-file", help="File with mint addresses (one per line)")
    parser.add_argument("--limit", type=int, default=10, help="Max tokens to discover (default 10)")
    parser.add_argument("--source", default="dexscreener",
                        choices=["dexscreener", "pumpfun", "cultscreener", "all", "file"],
                        help="Discovery source (default: dexscreener)")
    parser.add_argument("--scan-depth", type=int, default=200, help="Pump.fun tx scan depth")
    parser.add_argument("--score", action="store_true", help="Also submit to /judge after enrichment")
    parser.add_argument("--resnapshot", action="store_true",
                        help="Re-enrich all existing mints (ground truth snapshots)")
    parser.add_argument("--dry-run", action="store_true", help="Discover but don't enrich")
    parser.add_argument("--include-seen", action="store_true", help="Don't skip already-ingested mints")
    args = parser.parse_args()

    load_env()
    api_key = load_helius_key()
    if not api_key:
        print("ERROR: No HELIUS_API_KEY", file=sys.stderr)
        sys.exit(1)

    # ── Resnapshot mode: re-enrich existing mints ──
    if args.resnapshot:
        mints = load_existing_mints()
        if not mints:
            print("No existing mints to resnapshot.", file=sys.stderr)
            sys.exit(0)
        # Deduplicate
        seen = set()
        unique_mints = []
        for m in mints:
            if m not in seen:
                seen.add(m)
                unique_mints.append(m)
        mints = unique_mints

        # Count existing snapshots per mint to determine snapshot_n
        snapshot_counts: dict[str, int] = {}
        if DATASET_PATH.exists():
            for line in DATASET_PATH.read_text().splitlines():
                if not line.strip():
                    continue
                try:
                    row = json.loads(line)
                    m = row.get("mint", "")
                    snapshot_counts[m] = snapshot_counts.get(m, 0) + 1
                except json.JSONDecodeError:
                    pass

        print(f"\n=== Re-snapshotting {len(mints)} mints ===\n", file=sys.stderr)
        for i, mint in enumerate(mints, 1):
            snap_n = snapshot_counts.get(mint, 0)
            print(f"[{i}/{len(mints)}] {mint[:16]}... (snapshot #{snap_n})", file=sys.stderr)
            features = enrich_token(api_key, mint)
            if features:
                row = build_row(mint, "resnapshot", features, None, snapshot_n=snap_n)
                append_row(row)
                sym = features.get("symbol", "?")
                price = features.get("price_usd")
                price_str = f"${price:.6f}" if price else "no price"
                print(f"  {sym} {price_str}", file=sys.stderr)
            else:
                print(f"  FAILED to enrich", file=sys.stderr)
            if i < len(mints):
                time.sleep(0.5)
        print(f"\nResnapshot complete.", file=sys.stderr)
        sys.exit(0)

    # ── Discovery mode ──
    if args.mints_file or args.source == "file":
        path = Path(args.mints_file) if args.mints_file else None
        if not path or not path.exists():
            print(f"ERROR: mints file not found", file=sys.stderr)
            sys.exit(1)
        mints = [line.strip().split("#")[0].strip()
                 for line in path.read_text().splitlines()
                 if line.strip() and not line.strip().startswith("#")]
        source = "file"
    elif args.source == "dexscreener":
        mints = discover_dexscreener(limit=args.limit)
        source = "dexscreener"
    elif args.source == "pumpfun":
        mints = discover_pumpfun_creates(api_key, limit=args.limit, scan_depth=args.scan_depth)
        source = "pumpfun"
    elif args.source == "cultscreener":
        mints = discover_cultscreener(limit=args.limit)
        source = "cultscreener"
    elif args.source == "all":
        mints = []
        seen_set = set()
        cs = discover_cultscreener(limit=args.limit)
        mints.extend(cs)
        seen_set.update(cs)
        dx = discover_dexscreener(limit=args.limit)
        mints.extend(m for m in dx if m not in seen_set)
        seen_set.update(dx)
        pf = discover_pumpfun_creates(api_key, limit=args.limit, scan_depth=args.scan_depth) if api_key else []
        mints.extend(m for m in pf if m not in seen_set)
        source = "all"
    else:
        mints = []
        source = "none"

    if not mints:
        print("No tokens discovered.", file=sys.stderr)
        sys.exit(0)

    if not args.include_seen:
        seen = load_seen_mints()
        before = len(mints)
        mints = [m for m in mints if m not in seen]
        skipped = before - len(mints)
        if skipped:
            print(f"Skipped {skipped} already-ingested mints", file=sys.stderr)

    if not mints:
        print("All discovered tokens already in dataset.", file=sys.stderr)
        sys.exit(0)

    print(f"\n=== Ingesting {len(mints)} tokens (source={source}, score={args.score}) ===\n", file=sys.stderr)

    if args.dry_run:
        for m in mints:
            print(m)
        sys.exit(0)

    # ── Enrich + optionally score ──
    t_start = time.time()
    stats = {"ingested": 0, "enrichment_failed": 0, "scored": 0, "score_failed": 0}

    for i, mint in enumerate(mints, 1):
        print(f"[{i}/{len(mints)}] {mint[:16]}...", file=sys.stderr)

        # Phase 1: enrich (always)
        features = enrich_token(api_key, mint)
        if not features:
            print(f"  ENRICH FAILED", file=sys.stderr)
            append_row(build_row(mint, source, None, None))
            stats["enrichment_failed"] += 1
            continue

        sym = features.get("symbol", "?")
        price = features.get("price_usd")
        top1 = features.get("top1_pct")
        price_str = f"${price:.6f}" if price else "no price"
        top1_str = f"top1={top1}%" if top1 is not None else ""
        print(f"  {sym} {price_str} {top1_str}", file=sys.stderr)
        stats["ingested"] += 1

        # Phase 2: score (optional)
        verdict = None
        if args.score:
            verdict = score_token(mint)
            if verdict:
                v = verdict.get("verdict", "?")
                q = verdict.get("q_score", {}).get("total", 0)
                print(f"  → {v} Q={q:.3f}", file=sys.stderr)
                stats["scored"] += 1
            else:
                print(f"  → SCORE FAILED", file=sys.stderr)
                stats["score_failed"] += 1

        row = build_row(mint, source, features, verdict)
        append_row(row)

        if i < len(mints):
            time.sleep(0.5)

    elapsed = time.time() - t_start
    rows_total = sum(1 for _ in open(DATASET_PATH)) if DATASET_PATH.exists() else 0

    print(f"\n=== Ingestion complete ({elapsed:.0f}s) ===", file=sys.stderr)
    print(f"Ingested: {stats['ingested']}/{len(mints)}", file=sys.stderr)
    if stats["enrichment_failed"]:
        print(f"Enrichment failed: {stats['enrichment_failed']}", file=sys.stderr)
    if args.score:
        print(f"Scored: {stats['scored']}, score failed: {stats['score_failed']}", file=sys.stderr)
    print(f"Dataset: {DATASET_PATH} ({rows_total} rows total)", file=sys.stderr)


if __name__ == "__main__":
    main()
