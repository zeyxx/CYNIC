# Token screening — use the kernel, not a Python duplicate

`scripts/token_screener.py` was **deleted on 2026-05-30** (R12: dual sensing).

It re-implemented on-chain fetch + holder concentration + LP detection in Python,
duplicating what the kernel already does *better* in Rust:

- `cynic-kernel/src/backends/helius/mod.rs` — full enrichment (holders, age, K-Score, identities)
- `cynic-kernel/src/backends/helius/lp_detection.rs` — LP burned/locked/unsecured + burn/locker
  program detection. The Python version had **no lock detection** and defaulted missing LP
  data to `"unsecured"` instead of `"unknown"` (the BONK false-negative class).

The kernel's `/judge` endpoint enriches a bare mint itself — no Python needed.

## Screen a token (replaces `python3 scripts/token_screener.py <MINT>`)

```bash
set -a; source ~/.cynic-env; set +a
ADDR="${CYNIC_REST_ADDR}"; [[ "$ADDR" != http* ]] && ADDR="http://$ADDR"

curl -s --max-time 120 -X POST "$ADDR/judge" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"content":"<MINT_ADDRESS>","domain":"token-analysis"}' | python3 -m json.tool
```

The response includes the full `token_data` block (lp_status, supply_burned_pct,
holder_count, age_hours, k-score, …) plus the per-axiom verdict. The kernel is the
single source of truth for on-chain enrichment.
