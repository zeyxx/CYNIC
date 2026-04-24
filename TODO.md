# CYNIC — TODO

> **Live work ledger, not protocol spec. The organism must metabolize, not just grow.**
> Coordination and session lifecycle live in `AGENTS.md`. Session rows below are historical snapshots.

## Rules

1. **MAX 15 ACTIVE.** Everything else → DEFERRED. Not touched in 3 sessions → deleted.
2. **1 EXPERIMENT PER SESSION.** Start: hypothesis. End: measurement. No measurement = dissipated heat (§V.3).
3. **CLOSE ≥ OPEN.** Discover N items → close or defer N. The TODO never grows.
4. **TRACK COST.** Each session logs tokens, duration, output in the Session Log below.

Last updated: 2026-04-24 | Session: claude-4ef41e8d (opsec + budget + deploy-script + maturity-tier)

---

## Current State (probed 2026-04-23 22:25 CEST)

- **Kernel:** v0.7.7-212-g925099a-dirty, sovereign, 5/5 Dogs registered, DEPLOYED with thinking-aware budget
- **RAM:** 13Gi/27Gi (48%), swap empty — healthy
- **Screener:** `scripts/token_screener.py` live, 4/5 watchlist tokens produce verdicts
- **Watchlist cron:** active (every 4h, 5 tokens), 20+ verdicts in DB
- **Crystals:** 50 forming (25 dev, 25 general), 0 token-analysis — **epistemic gate blocks** (disagreement 0.418 > 0.382)
- **UI:** https://cynic-ui.vercel.app LIVE, Tailscale Funnel active (cloudflared disabled)
- **Calibration:** Δ(HOWL-BARK)=0.260, Δ(HOWL-AMB)=0.023 (still weak)
- **Hackathon:** feature freeze 2026-04-27, submission 2026-05-04

---

## P0 — Done this session

- [x] **DIAG-01: RAM crisis.** Resolved — 48% used, swap empty (was 94.8%).
- [x] **DIAG-02: Tunnel.** Cloudflared disabled (replaced by Tailscale Funnel). Funnel active.
- [x] **DIAG-03: coord RecordId='0'.** Fixed — `mcp_audit` table had corrupted index. Dropped + recreated. 0 errors in logs.
- [x] **Thinking-aware budget (PR #22).** `completion_budget()` now: `content*1.2 + thinking*1.5`, clamped [768, 4096]. Deployed.
- [x] **Opsec audit.** 10 truths crystallized. Memory persisted.

## P0 — Crystal flywheel (needs time, not code)

- [ ] **Crystal token-analysis blocked by epistemic gate.** disagreement=0.418 > 0.382 threshold. Dogs disagree on token scores. **Root cause:** 3 Dogs (det-dog + qwen-7b + qwen35) have different scoring distributions. Gemma was timing out/skipped → fewer voters → higher variance. **Expected resolution:** as gemma recovers (budget fix takes effect over 20+ calls), 4th voter reduces disagreement. **Falsification:** after 5+ watchlist cycles with gemma voting, if disagreement still > 0.382 → prompt engineering needed.

## P1 — HOWL/AMB discrimination

- [x] **Maturity tier in stimulus.** `estimate_maturity_tier()` derives ESTABLISHED/MODERATE/NEW from on-chain signals. JUP 0.35→0.48, USDC 0.30→0.22. Δ(ESTABLISHED-NEW)=0.260.

## P2 — Hackathon demo E2E

- [x] Tailscale Funnel live (`ubuntu-desktop-titouan.tail7ec70e.ts.net/health` → 200)
- [x] UI live (`cynic-ui.vercel.app` → 200)
- [ ] **Pinocchio PDA re-init.** Programme alive (A4QK3jj2...), deployer key lost. Agent=`Aw6PW...`, Guardian=`3JLQS...`. Need client script for `initialize_community` with new keys. ~1-2h focused work.
- [ ] Colosseum registration (4 mai) + submission form (11 mai)

## P3 — Infrastructure (non-blocking)

- [ ] **System llama-server crash-loop** — needs `sudo systemctl disable --now llama-server.service`
- [ ] Nightshift first-cycle delay — add CYNIC_PROJECT_ROOT to env
- [ ] Auth /health full payload (T1 from opsec audit) — split into /live (open) + /health (auth'd)
- [ ] SurrealDB slow query — summarizer 8.9s, needs index

## DEFERRED

- S5-S7 structural refactors
- NaN filter in judge/math.rs
- Two TokenData structs (enrichment.rs vs stimulus.rs) — Python screener bypasses both
- Heartbeat hook PostToolUse
- Full findings tracker: `docs/audit/CYNIC-FINDINGS-TRACKER.md`
