# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-24 | Hermes rework session planned

---

## HACKATHON (freeze Apr 27, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Confirmed devnet: tx `3bToTTx…`, PDA `AYD9xNQ3…`, Status: Ok, 3401 CU, 6 axiom scores on-chain.
- [ ] **Colosseum full submission.** Project created on arena.colosseum.org. Need: description longue, video demo (3min), GitHub link, deployed URL. Deadline: May 10 23:59 PDT.
- [ ] **Video demo.** Screen recording: paste token → Dogs deliberate → verdict → (stretch: on-chain tx). 2-3 min.

## ARCHITECTURE (session dédiée requise)

- [ ] **Hermes rework — session dédiée.** 3 Hermes distincts (Ouroboros/GPU, NousResearch, Antenne-X) à renommer + repositionner dans la topologie. Décisions : noms canoniques, `organs/` directory, scripts/ nettoyé. **Falsify:** `grep -r "Hermes" docs/ scripts/ cynic-kernel/` retourne des résultats non-ambigus.
- [ ] **Topologie organs/.** Créer couche `organs/` (hermes/, askesis/), déplacer `cynic-askesis/` et `scripts/hermes-x/` dedans. Dépend du Hermes rework.

## ORGANISM (no deadline, compound value)

- [ ] **Record thinking_tokens on failure (D2).** InferenceDog has thinking_max AtomicU32 but organ only reads it on Success. Wire reading on ALL update_stats_entry calls. Closes self-calibration loop. **Falsify:** gemma budget adapts after 5 failures without any success.
- [ ] **Domain-aware routing (D4).** Exclude Dogs with <50% json_valid_rate per domain. Gemma on token-analysis = 12% → skip. **Falsify:** token-analysis verdicts use only functional Dogs.
- [ ] **Auth /health (T1/O4).** ~~Split /live (boolean, open) + /health (full topology, auth required).~~ DONE in code: /metrics + /events now require auth, /health public response stripped of version. **Remaining:** deploy new binary + verify. **Falsify:** `curl funnel/metrics` → 401; `curl funnel/health` returns no version field.
- [ ] **Early verdict (O5).** Return response when quorum_count Dogs responded. Don't wait for slow Dogs. **Falsify:** verdict latency < 10s when 3/5 Dogs respond in 5s.
- [ ] **Nightshift first-cycle.** Add CYNIC_PROJECT_ROOT to ~/.config/cynic/env. **Falsify:** nightshift status != "never" within 5min of boot.

## DEBT (fix when touching adjacent code)

- [ ] NaN filter in judge/math.rs (trimmed_mean lets NaN through)
- [ ] Two TokenData structs (enrichment.rs vs stimulus.rs) — Python screener bypasses both. Test `calibration_token.rs` broken (pre-existing).
- [ ] SurrealDB summarizer slow query 8.9s — needs index
- [ ] LUKS full-disk encryption on cynic-core. KC1: physical access = full data. Post-hackathon priority.
- [ ] Headscale self-hosted coordinator. KC2: Tailscale Inc = single point of mesh failure.
- [ ] mitmdump currently running on 0.0.0.0:8888 — kill and restart with `--listen-host 127.0.0.1` (KC4).

## DONE (remove next session)

- [x] PR #22 merged — self-calibrating budget, deploy script, lint-services, maturity tier
- [x] Pinocchio PDA re-init (8DVUKmJa...)
- [x] Colosseum project created
- [x] DIAG-01/02/03 fixed
- [x] System llama-server killed
