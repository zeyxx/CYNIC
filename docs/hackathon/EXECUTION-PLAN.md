# Colosseum Frontier — Execution Plan

> Crystallized 2026-04-18. Feature freeze: 2026-04-27. Submission: 2026-05-04.
> Budget: ~51h (8.5 days x 6h). Buffer: 7h.

## Proposition

**Dual surface, one kernel:**
- Surface A (screener): Paste Solana address -> Dogs deliberate in real-time -> verdict -> on-chain proof
- Surface B (governance): Submit proposal -> deliberation -> on-chain verdict -> optional execute
- Both hit the same `/judge/async` endpoint, same Dogs, same phi-bounded scoring

**1-sentence pitch (DRAFT):**
"CYNIC is an epistemic oracle infrastructure for Solana that evaluates tokens and governance proposals with 5 independent AI validators under phi-bounded doubt (max 61.8% confidence), settling immutable verdicts on-chain via Pinocchio."

## Reality Matrix (probed 2026-04-18)

| Prerequisite | Status | Action |
|---|---|---|
| cloudflared | v2026.3.0 installed | GO |
| Vercel CLI | NOT installed (npx available) | `npm i -g vercel` or use npx |
| cynic-ui build | Passes (304ms, 640KB bundle) | GO |
| vercel.json | Does not exist | Create (SPA rewrite) |
| CORS kernel | Old Replit URL only | Add Vercel + tunnel origins, restart |
| Solana CLI | v3.1.12, devnet | GO |
| Deployer keypair | LOST (/tmp gone) | Keygen + re-init PDA |
| Fee-payer | Exists, 0 SOL devnet | Airdrop |
| Pinocchio program | ALIVE on devnet (executable) | No redeploy needed |
| Community PDAs | Both exist, agent keypair lost | Re-init with new keypair |
| Helius API key | EXISTS in ~/.helius/config.json, LIVE | Move to ~/.cynic-env for kernel |
| Helius in kernel | ZERO code | 10h: port trait + adapter + pipeline hook |
| Kernel | Sovereign, 5/5 circuits, 4/5 Dogs functional | GO |
| /judge/async | WORKS (tested: BARK Q=0.11, 4 Dogs in 16s) | GO — demo backbone |
| Wallet adapter | ZERO in cynic-ui | Stretch goal (Phase 6) |

## Phases

### Phase 0 — Foundations (4h) | Day 1

| Task | Time | Falsification | Unblocks |
|---|---|---|---|
| Keygen + re-init Community PDA (agent != guardian) | 30min | `solana account <PDA>` shows correct fields | Phase 4 |
| Cloudflare named tunnel -> stable URL for kernel | 1h | `curl https://<tunnel>/health` returns `sovereign` from external network | Phase 2, 3 |
| Vercel deploy cynic-ui (vercel.json + env vars) | 1.5h | Page loads on `cynic-xxx.vercel.app`, health indicator green | Phase 2, 3 |
| CORS: add Vercel origin + tunnel origin in kernel | 30min | Fetch from Vercel -> no CORS error | Phase 2, 3 |

**Gate 0:** Vercel UI loads AND talks to kernel via tunnel.

### Phase 1 — Progressive Dog UI Component (8h) | Days 1-2

| Task | Time | Falsification |
|---|---|---|
| `DeliberationView` component: POST /judge/async, poll /judge/status/{id} every 2s, Dogs arrive one-by-one with animation | 5h | Dogs appear progressively, individual scores visible, final verdict emerges |
| Integration: new "Screen" tab with text input -> DeliberationView | 2h | Paste text -> see Dogs deliberate -> final verdict |
| E2E test: rug text vs legit text, verify discrimination | 1h | BARK on rug, WAG/GROWL on legit |

**Gate 1:** Paste text -> see Dogs arrive -> verdict. UX feels responsive.

### Phase 2 — Kernel Enrichment (10h) | Days 2-4

| Task | Time | Falsification |
|---|---|---|
| Port trait `TokenEnricher` in domain/ | 1h | Compiles, no infra types in domain (K5) |
| Helius adapter in backends/: getAsset + getTokenHolders + searchAssets(creator) | 4h | Unit test: mock responses -> structured stimulus |
| Pipeline hook: when domain=token-analysis and content matches base58 address -> enrich before Dogs | 2h | POST /judge/async with mint -> Dogs receive enriched stimulus |
| Calibration: test 5 known addresses (JUP, $ASDF, rug, SOL, BONK) | 2h | JUP=WAG/GROWL, rug=BARK, SOL=HOWL/WAG |
| Config Helius API key in ~/.cynic-env | 1h | Not hardcoded |

**Gate 2:** `curl -X POST /judge/async -d '{"content":"JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN","domain":"token-analysis"}'` -> Dogs receive rich data -> discriminant verdict.

### Phase 3 — Token Screener Surface (6h) | Days 4-5

| Task | Time | Falsification |
|---|---|---|
| UI screener: mint address input + "Screen" button -> /judge/async with domain token-analysis -> DeliberationView | 3h | User pastes address -> Dogs deliberate -> verdict appears |
| Enriched display: show token metadata (name, symbol, holders, HHI) above verdict | 2h | Helius data visible in UI |
| Responsive + polish | 1h | Mobile-friendly |

**Gate 3:** Give URL to S., ask them to paste an address. Confusion = iterate.

### Phase 4 — On-Chain Settlement (6h) | Days 5-6

| Task | Time | Falsification |
|---|---|---|
| Fix bridge for async path (or high timeout) | 2h | `node agent-bridge.js judge "..."` works E2E |
| Auto-submit: after token-analysis verdict, bridge auto-submits submit_verdict on-chain | 2h | Verdict PDA created, `solana account <pda>` shows axiom scores |
| UI tx link: after verdict, show "Settled on Solana -> [Explorer link]" | 1h | Click -> Verdict PDA on Solana Explorer |
| Graceful fallback: if devnet down, verdict UI still works (degraded) | 1h | Kill devnet -> verdict displays, "settlement unavailable" message |

**Gate 4:** Screener -> verdict -> tx link -> Solana Explorer shows scores.

### Phase 5 — Governance Surface (6h) | Days 6-7

| Task | Time | Falsification |
|---|---|---|
| UI governance: textarea for proposal + DeliberationView (reuses Phase 1) | 3h | Submit text -> Dogs deliberate -> verdict |
| On-chain: governance verdict -> submit_verdict -> Verdict PDA | 2h | PDA contains proposal_hash + q_score |
| Threshold display: show community threshold + "Action available" if Q > threshold | 1h | If Q > threshold -> green badge |

**Gate 5:** Both surfaces work. Switch between screener and governance in 1 click.

### Phase 6 — Wallet Connect (STRETCH, 8h) | Days 7-8

| Task | Time | Falsification |
|---|---|---|
| Wallet adapter: @solana/wallet-adapter-react + Phantom | 3h | "Connect Wallet" button -> Phantom popup -> connected |
| User-signed judgment: user signs request, pubkey in verdict context | 3h | Verdict PDA references user wallet |
| "My Verdicts": page listing verdicts requested by this wallet | 2h | Connect wallet -> see history |

**GO/NO-GO:** If Phase 5 finishes after Day 7 -> SKIP Phase 6 entirely. Auto-submit is sufficient.

### Phase 7 — Video + Submission (4h) | Days 8-9

| Task | Time |
|---|---|
| 1-sentence pitch + 200-word description | 1h |
| Business plan slide: API credits, protocol fees, CultScreener integration | 1h |
| Demo video: screen record full flow | 1.5h |
| Colosseum submission | 0.5h |

## Budget Summary

| Phase | Hours | Cumulative | Target Day |
|---|---|---|---|
| 0. Foundations | 4h | 4h | D1 (Apr 18) |
| 1. Progressive UI | 8h | 12h | D1-2 |
| 2. Kernel enrichment | 10h | 22h | D2-4 |
| 3. Token screener UI | 6h | 28h | D4-5 |
| 4. On-chain settlement | 6h | 34h | D5-6 |
| 5. Governance surface | 6h | 40h | D6-7 |
| 6. Wallet (STRETCH) | 8h | 48h | D7-8 |
| 7. Video + submission | 4h | 44-52h | D8-9 |
| **Total** | **44h base + 8h stretch** | | |

## Demo Day Failure Modes

| Failure | P(inferred) | Mitigation |
|---|---|---|
| Gemma timeout slows verdict | 80% | Exclude gemma from demo config. 4 Dogs in 16s. |
| Google quota (gemini-cli) | 30% | Backup, not critical. 3 sovereign Dogs suffice. |
| Kernel restart mid-demo | 10% | systemd auto-restart. Health indicator in UI. |
| Solana devnet down | 15% | Graceful fallback (Phase 4): verdict displays, "settlement unavailable". |
| Wallet connection fail | 25% (if Phase 6) | Stretch goal. Without it: zero wallet risk. |
| Cloudflare tunnel drop | 10% | Named tunnel + auto-reconnect. Backup: Tailscale Funnel. |
| Dog regression | 20% | Feature freeze = code freeze from Apr 27. No prompt changes after Phase 2. |
| Helius rate limit | 5% | Free tier 10 req/s. Demo = 1-2 requests. |

## Decisions (crystallized 2026-04-18)

- Token screener = primary demo surface, governance = secondary
- Kernel-side Helius enrichment (not browser-side) — key stays server-side
- Cloudflare named tunnel + Vercel for UI — no IP leak
- Async /judge path exclusively — sync too slow (60s gemma timeout)
- Auto-submit on-chain first, wallet connect as stretch goal
- No Playwright X scraping — on-chain data from Helius suffices for L1
- No CultScreener integration for hackathon — sovereign, independent
- L1 on-demand enrichment only. L2 (batch) and L3 (streaming) are roadmap slides.

## Truth Table (from crystallize-truth)

| T# | Truth | Confidence |
|---|---|---|
| T1 | Token screener is primary demo surface | 58% |
| T1a | On-chain settlement non-negotiable but thin (submit_verdict only) | 55% |
| T1b | Progressive Dog arrival animation is highest-ROI demo feature | 52% |
| T2 | Sync /judge dead for demos — async only | 60% |
| T3 | Lost keypair is 30min mechanical task, not design risk | 60% |
| T4 | Bridge expects sync /judge — must be patched for async | 55% |
| T5 | Real addresses need Helius enrichment in kernel | 50% |
| T6 | Business plan criterion has no answer yet | 45% |
| T7 | Deployment (Vercel+tunnel) blocks all external testing | 58% |
| T8 | 0 users, 0 external validation | 55% |
| T9 | 1-sentence pitch doesn't exist yet | 52% |
