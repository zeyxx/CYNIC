# GASdf Scaling Plan 0 → 1

Status: draft v0.1, 2026-11-04. Observed facts only. KPIs are measured, not guessed.

## 1. Objective

Move GASdf from **Phase 1 (core foundation, current)** to **Phase 1 complete + Phase 2 entry**:
a self-sustaining gasless relay on Solana mainnet with paying consumers,
positive margin, and a proven burn loop. No outside capital. Fees fund the
network; surplus funds reserves and $ASDF burn.

## 2. Current baseline (observed)

| Item | Value | Source |
|------|-------|--------|
| Relay endpoint | `https://gasdf.talaria.build` | ARCHITECTURE.md |
| SDK tests | 70/70 passing | README.md |
| Relay tests | 38/38 passing | README.md |
| Token basket | SOL, USDC, USDT, $ASDF ($ASDFASDFA on-chain) | ROADMAP.md Phase 1 |
| Kora signer | LXC 107, port 14001, 24/7 | ARCHITECTURE.md |
| Fee formula | 50000 + priority lamports base | TOKENOMICS.md §2 |
| Margin waterfall | hot reserve → cold reserve → burn | TOKENOMICS.md §3 |
| Burn threshold | $5 minimum per batch | ROADMAP.md Phase 1 |
| Token gate (HolDex) | K-score min 40 (deferred, see INGESTION_PLAN.md) | INGESTION_PLAN.md |
| MTU limit | 1232 bytes | TODO.md (legacy) |
| Helius plan | Free (1M credits, 10 req/s) | TOKENOMICS.md §2 |

## 3. KPI (measured, not targeted by wish)

| KPI | Definition | Phase 1 target | Phase 2 trigger |
|-----|-----------|----------------|-----------------|
| `margin_24h` | trailing sponsored lamports (24h sliding window) | > 0 for 7 consecutive days | stable |
| `land_rate` | % txs landed on first attempt | ≥ 95% | ≥ 98% |
| `hot_reserve_sol` | native SOL in hot reserve | ≥ 0.5 SOL (HOT_FLOOR) | ≥ HOT_TARGET (3× daily usage) |
| `cold_reserve_months` | USDC runway in months | ≥ 1 month | ≥ 6 months |
| `burn_lots` | number of completed weekly burn lots with proof | ≥ 1 | ≥ 4 consecutive |
| `consumer_integrations` | dApps using SDK in production | ≥ 1 (TrenchScanner) | ≥ 2 |
| `quote_latency_p95` | p95 quote endpoint latency | < 500 ms | < 300 ms |
| `tx_size_fail_rate` | % txs rejected for MTU overflow | < 5% | < 1% |
| `sol_refill_autonomy` | auto-refill loop operational | manual OK | automated |

## 4. Phases (0 → 1)

### Phase 1 (current) — Core Foundation & Strict Token Basket
Goal: absolute stability, zero-leak economic safety, proven profitability.

Exit criteria (all must hold for 7 consecutive days):
- [ ] `margin_24h` > 0
- [ ] `land_rate` ≥ 95%
- [ ] `hot_reserve_sol` ≥ 0.5 SOL
- [ ] ≥ 1 completed burn lot with on-chain proof
- [ ] ≥ 1 consumer integration (TrenchScanner pay-per-scan)
- [ ] SOL auto-refill loop operational (auto-replenish below 0.5 SOL)
- [ ] external integration spec published for @gcrtrd

### Phase 2 (entry) — Dynamic Community Expansion
Trigger: all Phase 1 exit criteria met.

Scope (in):
- CultScreener adapter (periodic verified token ingestion)
- Conviction gating (auto-accept tokens meeting threshold + liquidity)
- Dynamic slippage guard (real-time Jupiter route eval per token)
- Onchain program gate (Pinocchio) — only when treasury justifies audit cost

Scope (out, forbidden until yield gate):
- Tiers, scores, staking, revenue-sharing, auto-compounding, discounts
- Holding memecoins or $ASDF as inventory
- Operator top-up of reserves

## 5. Cahier des charges évolutif (scope evolution)

### Always in scope
- Fee formula tuning via operator params (HOT_FLOOR, ALPHA, RUNWAY_MONTHS)
- Calibration matrix refit (monthly)
- Security gates (adversarial wire tx inspection, session TTL, rate limiting)
- Append-only ledger (every micro-fee recorded)

### Evolves with data (not by vote)
- CU price mults per bracket (from calibration matrix)
- Congestion multiplier rule
- Token basket expansion (Phase 2 only, conviction-gated)
- Onchain program gate (when treasury justifies audit cost)

### Forbidden until yield gate opens
- Rewards, tiers, staking, revenue-sharing, auto-compounding
- Discounts on fees
- Holding inventory beyond hot/cold reserve

## 6. Risks & mitigations

| Risk | Mitigation | Owner |
|------|-----------|-------|
| MTU overflow on complex swaps | maxAccounts=25, pre-flight size check, ALT support Phase 2 | relay-gateway |
| HolDex 403 (anti-sybil) | whitelist GASdf API key, HolDex whitelist impl | HolDex |
| Helius rate limit (free plan) | fallback RPC, upgrade to Developer ($49) when needed | infra |
| Negative margin on volatile tokens | token cache rule (swap if amount < swap_cost + rent + margin_min) | relay-gateway |
| Reserve depletion | phase-gated waterfall, no operator top-up in Phase 0 | tokenomics |
| Consumer integration blocker | publish concrete SDK snippet for TrenchScanner | GASdf |

## 7. Convergence checkpoints

- Weekly: review KPI table, update phase exit criteria checkboxes.
- Monthly: calibration matrix refit, burn lot proof audit, reserve runway recalc.
- On Phase 1 exit: open Phase 2 scope, publish integration spec.

## 8. Open questions

- Production SHA running (verify `/var/lib/cynic/projects/kora/relay-gateway`)
- HolDex whitelist status
- TrenchScanner integration ETA
- Kora tag pinned (read from deployed unit)
EOF
echo "written"