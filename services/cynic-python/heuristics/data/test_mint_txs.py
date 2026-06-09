#!/usr/bin/env python3
"""Test: does getTransactionsForAddress on a MINT address return token transfers?

One call. 10 cr. Answers the question definitively.
"""

import json
import os
import sys
import requests

def _load_env():
    for f in [os.path.expanduser('~/.cynic-env')]:
        if os.path.exists(f):
            with open(f) as fh:
                for line in fh:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        k, v = line.split('=', 1)
                        k = k.replace('export ', '').strip()
                        if k not in os.environ:
                            os.environ[k] = v.strip()

_load_env()

KEY = os.getenv("HELIUS_API_KEY")
RPC = "https://mainnet.helius-rpc.com/"
RPC_HEADERS = {"Content-Type": "application/json", "Authorization": f"Bearer {KEY}"}

# Fartcoin mint
MINT = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump"

# Exact format from Helius docs page (verified via browser)
resp = requests.post(RPC, headers=RPC_HEADERS, json={
    "jsonrpc": "2.0", "id": 1,
    "method": "getTransactionsForAddress",
    "params": [MINT, {
        "transactionDetails": "full",
        "sortOrder": "desc",
        "limit": 10,
        "filters": {
            "status": "succeeded",
            "tokenAccounts": "balanceChanged",
        },
        "maxSupportedTransactionVersion": 0,
    }]
}, timeout=30)

data = resp.json()

if "error" in data:
    print(f"ERROR: {data['error']}")
    sys.exit(1)

result = data.get("result", {})
txs = result.get("data", [])
print(f"Transactions returned: {len(txs)}")

for i, tx in enumerate(txs):
    print(f"\n{'='*60}")
    print(f"TX #{i+1}")
    bt = tx.get("blockTime")
    slot = tx.get("slot")
    print(f"  blockTime={bt}  slot={slot}")

    meta = tx.get("meta", {})
    pre = meta.get("preTokenBalances", [])
    post = meta.get("postTokenBalances", [])

    print(f"  preTokenBalances: {len(pre)} entries")
    print(f"  postTokenBalances: {len(post)} entries")

    # Show token balance changes for our mint
    mints_seen = set()
    for b in pre + post:
        m = b.get("mint", "?")
        mints_seen.add(m)

    our_mint_found = MINT in mints_seen
    print(f"  Mints in balances: {len(mints_seen)}")
    print(f"  OUR MINT present: {'YES' if our_mint_found else 'NO'}")

    if our_mint_found:
        print(f"\n  TOKEN FLOW:")
        # Build pre/post maps by accountIndex
        pre_map = {}
        for b in pre:
            if b.get("mint") == MINT:
                idx = b.get("accountIndex")
                owner = b.get("owner", "?")
                amt = b.get("uiTokenAmount", {}).get("uiAmount", 0)
                pre_map[idx] = {"owner": owner, "amount": float(amt or 0)}

        post_map = {}
        for b in post:
            if b.get("mint") == MINT:
                idx = b.get("accountIndex")
                owner = b.get("owner", "?")
                amt = b.get("uiTokenAmount", {}).get("uiAmount", 0)
                post_map[idx] = {"owner": owner, "amount": float(amt or 0)}

        all_idx = set(list(pre_map.keys()) + list(post_map.keys()))
        for idx in sorted(all_idx):
            p = pre_map.get(idx, {"owner": "?", "amount": 0})
            q = post_map.get(idx, {"owner": "?", "amount": 0})
            delta = q["amount"] - p["amount"]
            direction = "BUY" if delta > 0 else ("SELL" if delta < 0 else "---")
            owner = q.get("owner") or p.get("owner")
            print(f"    [{idx}] {owner[:12]}...  pre={p['amount']:.2f}  post={q['amount']:.2f}  "
                  f"delta={delta:+.2f}  [{direction}]")
    else:
        # Show what mints ARE present
        for m in list(mints_seen)[:3]:
            print(f"    mint: {m[:12]}...")

    # Also check instructions for transfer types
    tx_data = tx.get("transaction", {})
    msg = tx_data.get("message", {})
    instructions = msg.get("instructions", [])
    inner = meta.get("innerInstructions", [])
    total_inner = sum(len(ii.get("instructions", [])) for ii in inner)
    print(f"  Instructions: {len(instructions)} outer + {total_inner} inner")

print(f"\n{'='*60}")
print("CONCLUSION:")
if any(MINT in {b.get("mint") for b in tx.get("meta", {}).get("preTokenBalances", []) +
       tx.get("meta", {}).get("postTokenBalances", [])} for tx in txs):
    print("  getTransactionsForAddress(MINT) DOES return token transfers")
    print("  Strategy B/C viable — can use mint-level queries")
else:
    print("  getTransactionsForAddress(MINT) does NOT return token transfers")
    print("  Must use per-wallet queries (Strategy A only)")
