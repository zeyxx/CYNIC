# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-24 | Merged: PR #22 (opsec + budget + deploy + maturity + PDA)

---

## HACKATHON (freeze Apr 27, submission May 11)

- [ ] **submit_verdict on-chain.** Write client script that calls Pinocchio `submit_verdict` with a real kernel verdict. Without this, "on-chain settlement" claim is aspirational. PDA ready: `8DVUKmJabj5gzQXE6u6DpnQxsDMGy8Be5aHzjqxttHow`. **Falsify:** `solana confirm <tx>` shows verdict data on-chain.
- [ ] **Colosseum full submission.** Project created on arena.colosseum.org. Need: description longue, video demo (3min), GitHub link, deployed URL. Deadline: May 10 23:59 PDT.
- [ ] **Video demo.** Screen recording: paste token → Dogs deliberate → verdict → (stretch: on-chain tx). 2-3 min.

## ORGANISM (no deadline, compound value)

- [ ] **Record thinking_tokens on failure (D2).** InferenceDog has thinking_max AtomicU32 but organ only reads it on Success. Wire reading on ALL update_stats_entry calls. Closes self-calibration loop. **Falsify:** gemma budget adapts after 5 failures without any success.
- [ ] **Domain-aware routing (D4).** Exclude Dogs with <50% json_valid_rate per domain. Gemma on token-analysis = 12% → skip. **Falsify:** token-analysis verdicts use only functional Dogs.
- [ ] **Auth /health (T1/O4).** Split /live (boolean, open) + /health (full topology, auth required). Funnel exposes /health to internet. **Falsify:** `curl funnel/health` → 401.
- [ ] **Early verdict (O5).** Return response when quorum_count Dogs responded. Don't wait for slow Dogs. **Falsify:** verdict latency < 10s when 3/5 Dogs respond in 5s.
- [ ] **Nightshift first-cycle.** Add CYNIC_PROJECT_ROOT to ~/.config/cynic/env. **Falsify:** nightshift status != "never" within 5min of boot.

## DEBT (fix when touching adjacent code)

- [ ] NaN filter in judge/math.rs (trimmed_mean lets NaN through)
- [ ] Two TokenData structs (enrichment.rs vs stimulus.rs) — Python screener bypasses both
- [ ] SurrealDB summarizer slow query 8.9s — needs index

## DONE (remove next session)

- [x] PR #22 merged — self-calibrating budget, deploy script, lint-services, maturity tier
- [x] Pinocchio PDA re-init (8DVUKmJa...)
- [x] Colosseum project created
- [x] DIAG-01/02/03 fixed
- [x] System llama-server killed
