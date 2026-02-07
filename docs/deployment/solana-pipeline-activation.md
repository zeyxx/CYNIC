# Solana Pipeline Activation Guide

> Generated: 2026-02-07
> Confidence: 58% (phi-1 limit)

## Pipeline Overview

```
JUDGMENT --> BLOCK --> FINALIZATION --> SOLANA ANCHOR --> BURN VERIFICATION
   [OK]       [OK]      [92.5%]         [READY]          [NOT DEPLOYED]
```

## 1. Current Pipeline Status

| Step | Status | Notes |
|------|--------|-------|
| Judgment --> Block | WORKING | BlockProducer collects judgments, produces blocks |
| Block Finalization | WORKING (92.5%) | phi-BFT consensus, slot duration fix applied |
| Finalized --> Anchor | READY (dry-run) | SolanaAnchoringManager wired, no keypair set |
| Burn Relay | NOT DEPLOYED | gasdf-relayer complete, never deployed to Render |
| Token Burns | MISSING | No SPL burn instruction on-chain |

## 2. Anchoring Activation

### Environment Variables

Set on **cynic-node-daemon** first (proposer-only anchoring enforced at `solana-anchoring.js:318`).

| Env Var | Value | Source |
|---------|-------|--------|
| `CYNIC_ANCHORING_ENABLED` | `true` | Master switch (default: effectively ON) |
| `CYNIC_ANCHORING_DRY_RUN` | `false` | Must be explicitly `false` to disable simulation |
| `CYNIC_SOLANA_KEY` | `[1,2,...,64]` | 64-byte keypair as JSON array or base58 |
| `SOLANA_CLUSTER` | `devnet` or `mainnet-beta` | Default: `devnet` |
| `CYNIC_ANCHOR_INTERVAL` | `500` | Slots between anchors (default) |
| `HELIUS_API_KEY` | optional | Rate-limit-free RPC |

Source: `packages/node/src/network-singleton.js` lines 99-102

### Activation Order

1. Generate dedicated Solana keypair for anchoring (NOT the main wallet)
2. Fund with minimal SOL (~0.5 SOL for devnet testing)
3. Set env vars on `cynic-node-daemon` (Render: `srv-d5o3aoumcj7s73aiqh80`)
4. Redeploy daemon
5. Monitor logs for: `[ANCHOR] Anchoring merkle root` (INFO level)
6. Verify on-chain: `solana confirm <tx_sig>` or Solana Explorer

### Frequency & Cost

```
500 slots x 400ms = anchor every ~3.3 minutes
~432 anchors/day
~0.005 SOL per anchor (estimate)
~2.16 SOL/day at default interval
```

### Verification

Look for in logs:
```
[ANCHOR] Anchoring merkle root <hash> (block <height>, <count> judgments)
[ANCHOR] Anchor confirmed: <tx_signature>
```

Failure case (Fibonacci retry):
```
[ANCHOR] Anchor failed, retry 1/8 in 8000ms
[ANCHOR] Anchor failed, retry 2/8 in 13000ms  (8s, 13s, 21s, 34s, 55s)
```

## 3. gasdf-relayer Deployment

### Required Environment Variables

Source: `packages/gasdf-relayer/src/index.js`, `solana.js`

| Env Var | Required | Description |
|---------|----------|-------------|
| `SOLANA_RPC_URL` | YES | Solana RPC endpoint |
| `RELAYER_PRIVATE_KEY` | YES | Base58 fee payer keypair |
| `TREASURY_ADDRESS` | YES | Solana pubkey for treasury split |
| `PORT` | NO | Default: 3000 |
| `DATABASE_URL` | NO | For burn history persistence |

### Burn Split (phi-aligned)

Source: `packages/gasdf-relayer/src/burns.js` lines 18-29

```
76.39% --> BURN (null address 1111...)
23.61% --> TREASURY
```

Split derived from: `1 - PHI_INV = 1 - 0.618 = 0.382` then `0.382 / 0.5 = 0.7639`

### Render Deployment

Config exists at `packages/gasdf-relayer/render.yaml`:
- Docker deployment or Node.js
- Health check: `GET /health`
- Endpoints:
  - `POST /v1/quote` -- fee quote (accepts eScore for discounts)
  - `POST /v1/submit` -- submit signed transaction
  - `GET /v1/stats` -- burn statistics
  - `GET /v1/burns` -- recent burns (transparency)
  - `GET /v1/burns/:sig/verify` -- on-chain burn verification

### Connection to Network

gasdf-relayer needs to know about CYNIC network for E-Score validation.
Currently: standalone (no direct connection to network nodes).
Future: Wire `CYNIC_NETWORK_NODES` env var for E-Score verification.

## 4. E-Score Integration

### Current State

Source: `packages/node/src/network/escore-provider.js` (172 lines)

E-Score is calculated locally and shared via gossip heartbeats.

From `network-node.js` lines 462-465, heartbeats include:
```javascript
{
  eScore: compositeScore,      // single number
  eScoreDimensions: {          // 7D breakdown
    BURN:   score,  // weight: phi^3  = 4.236 (HIGHEST)
    BUILD:  score,  // weight: phi^2  = 2.618
    JUDGE:  score,  // weight: phi    = 1.618
    RUN:    score,  // weight: 1.0    (CENTER)
    SOCIAL: score,  // weight: phi^-1 = 0.618
    GRAPH:  score,  // weight: phi^-2 = 0.382
    HOLD:   score,  // weight: phi^-3 = 0.236 (LOWEST)
  }
}
```

Total weight: 3*sqrt(5) + 4 = 10.708

### Active Dimensions

| Dimension | Status | Source |
|-----------|--------|--------|
| JUDGE | ACTIVE | Fed by JUDGMENT_CREATED, BLOCK_FINALIZED events |
| RUN | ACTIVE | Fed by METRICS_REPORTED (uptime) |
| BUILD | PARTIAL | Fed by BLOCK_PROPOSED (if published) |
| BURN | STUB | Needs gasdf-relayer integration |
| SOCIAL | STUB | Needs X/Twitter integration |
| GRAPH | STUB | Needs trust graph implementation |
| HOLD | STUB | Needs token balance checking |

### E-Score --> Burn Pricing

Source: `packages/gasdf-relayer/src/quotes.js` (391 lines)

Higher E-Score = lower burn cost (discount for good actors):
```
discount = eScore * PHI_INV  // max 61.8% discount
finalFee = baseFee * (1 - discount)
```

## 5. On-Chain Program (cynic-anchor)

### Deployed

Program ID: `G3Yana4ukbevyoVNSWrXgRQtQqHYMnPEMi1xvpp9CqBY`
Deployed on: devnet/localnet (from Anchor.toml)

### 15 Instructions

Source: `programs/cynic-anchor/src/lib.rs` (1463 lines)

- `anchor_root` -- anchor merkle root of judgments
- `verify_root` -- verify root exists
- `verify_inclusion` -- merkle inclusion proof (max depth 32)
- `record_burn` -- record burn event
- `update_escore` -- update validator E-Score
- `register_validator` -- register with stake
- `slash_validator` -- slash for misbehavior
- `unregister_validator` -- exit with unstake
- Plus governance, config, treasury instructions

### Missing: SPL Token Integration

No instruction exists to:
- Burn $asdfasdfa tokens (`9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump`)
- Check token balance for HOLD dimension
- Gate API access via burn verification

### Token Program Roadmap

1. Add `burn_asdfasdfa` instruction to cynic-anchor
2. Integrate SPL token program for transfer/burn
3. Create oracle: E-Score --> required burn amount
4. Add `verify_burn` instruction for access gating
5. Wire gasdf-relayer to call `burn_asdfasdfa` on-chain

## 6. Risk Assessment

### Anchoring Failure

- **Mitigation**: Fibonacci retry (8s, 13s, 21s, 34s, 55s), max 8 retries
- **Impact**: PoJ chain continues regardless, anchor catches up on next success
- **Monitoring**: Log `[ANCHOR] Anchor failed` at WARN level

### Burn Relay Failure

- **Risk**: Pending burns in-memory only, lost on crash
- **Mitigation**: Graceful shutdown calls `forceExecuteBurns()`
- **Impact**: Minimal (burns can be re-queued)

### Solana RPC Rate Limits

- **Anchoring**: ~1 TX per 200s (negligible)
- **Relayer**: Depends on usage, could hit limits
- **Mitigation**: Helius API key for higher limits

### Key Security

- Separate wallets for anchoring vs relaying
- Minimal SOL balances (refill via cron)
- Keys in Render secrets (not in code)
- NEVER commit keypairs to git

### Deploy Overlap

- Render zero-downtime deploys run old+new containers (~2 min)
- Anchoring is leader-only, so no duplicate anchors
- Burn relay should use database locks for pending burns

---

## Render Service IDs (Reference)

| Service | ID |
|---------|-----|
| cynic-mcp | `srv-d5kgqsshg0os739k341g` |
| cynic-node-daemon | `srv-d5o3aoumcj7s73aiqh80` |
| cynic-node-alpha | `srv-d5o3c5eid0rc7390nm60` |
| cynic-node-beta | `srv-d5sdbrhr0fns73b46i1g` |
| gasdf-relayer | NOT YET DEPLOYED |
