# CYNIC — TODO

> ≤15 active items. Actionable, time-bounded, falsifiable. History → memory/. Design → docs/. Rules → .claude/rules/.

Last updated: 2026-04-25 11:45 | Video demo script ready, kernel stable, verdicts live

---

## HACKATHON (freeze May 4, submission May 11)

- [x] **submit_verdict on-chain.** `scripts/submit-verdict.ts` ships. Confirmed devnet: tx `3bToTTx…`, PDA `AYD9xNQ3…`, Status: Ok, 3401 CU, 6 axiom scores on-chain.
- [x] **Rust 1.95.0 upgrade + deterministic-dog fix.** d0dd481: removed forced consensus inclusion (was causing false bias). Built successfully 2026-04-25 11:25. Kernel v0.7.7-218-gd0dd481 deployed. Kernel remains stable, all 5 Dogs operational (gemma timeout observed but not blocking), verdicts crystallizing at 10-15s latency.
- [ ] **Colosseum full submission.** Project created on arena.colosseum.org. Need: description longue, video demo (3min), GitHub link, deployed URL. Deadline: May 10 23:59 PDT.
- [x] **B&C integration analysis.** Reports created: B2C-SOLANA-CONVERGENCE-AUDIT.md (T.'s code patterns), B2C-CYNIC-INTEGRATION-SPECIFICATION.md (scope clarification pending from S.). Decision: stimulus architecture is sound. Blocking: S.'s J6-7 (nonce/permit/Arweave/mint), scope clarification (crypto vs reputation validation).
- [x] **Personality card stimulus + integration stub.** New builder: build_personality_card_stimulus(archetype, confidence, 6 signals). Tests: 7/7 passing. Integration doc added showing B&C flow: client sign → POST /mint-permit → CYNIC /judge → Dogs evaluate chess domain → HOWL/WAG/BARK. Waiting on S. for scope clarification (pre vs post mint, fallback behavior).
- [ ] **Video demo.** Script ready at `/tmp/video-demo-script.md`. Scenes: (1) Token input (15s), (2) Dogs deliberate + axiom scoring (45s), (3) Crystal verdict (40s), (4) On-chain tx (30s stretch). Latencies confirmed: deterministic 0ms, qwen7b ~1.5s, qwen35-gpu ~8.8s. Use latest GROWL/BARK verdicts from `/verdicts` endpoint. **Falsify:** 2-3 min narration + kernel logs visible, q_score + dog_scores visible.

## ARCHITECTURE (session dédiée requise)

- [ ] **Hermes rework — session dédiée.** 3 Hermes distincts (Ouroboros/GPU, NousResearch, Antenne-X) à renommer + repositionner dans la topologie. Décisions : noms canoniques, `organs/` directory, scripts/ nettoyé. **Falsify:** `grep -r "Hermes" docs/ scripts/ cynic-kernel/` retourne des résultats non-ambigus.
- [ ] **Topologie organs/.** Créer couche `organs/` (hermes/, askesis/), déplacer `cynic-askesis/` et `scripts/hermes-x/` dedans. Dépend du Hermes rework.

## AGENT LIFECYCLE (K15 consumer, ops/opsec validation)

- [ ] **Hermes polling daemon.** Add loop to hermes-agent: poll cynic_list_pending_agent_tasks every 5s, stub executor (mark complete immediately), report via cynic_update_agent_task_result. Validates dispatch→poll→execute→complete cycle. Estimated: 30 min code + 15 min test. **Falsify:** Full cycle runs; task moves pending→processing→completed.
- [x] **K15 consumer for tasks.** Add observation listener: task completion triggers observe_crystal with task result as content. Closes audit→observation gap. **Falsify:** Task completion logged; crystal appears 5s later.
- [ ] **Dead letter queue.** Retry failed tasks up to 3x with exponential backoff. Timeouts auto-fail tasks after 30 min. **Falsify:** Failed task retried; timeout task marked failed.
- [ ] **Agent health monitoring.** Require heartbeat on poll (cynic_list_pending_agent_tasks). Track agent uptime per kind. Alert if agent silent >5min. **Falsify:** No pending tasks assigned to silent agent.

## ORGANISM (no deadline, compound value)

- [ ] **CCM gate threshold tuning (K15 blocker).** Current: max_disagreement > φ⁻² (0.382) quarantines verdicts. Observed: USDC (0.568), rug-pull (0.468) — all legitimate tokens exceed threshold. Conjecture: axiom divergence between qwen7b and qwen35 is structural, not calibration drift. **Options:** (a) lower gate to φ⁻³ (0.236), (b) per-axiom disagreement thresholds, (c) accept 80%+ quarantine rate. **Falsify:** (a) new verdicts crystallize & improve over 48h.
- [ ] **Record thinking_tokens on failure (D2).** InferenceDog has thinking_max AtomicU32 but organ only reads it on Success. Wire reading on ALL update_stats_entry calls. Closes self-calibration loop. **Falsify:** gemma budget adapts after 5 failures without any success.
- [ ] **Domain-aware routing (D4).** Exclude Dogs with <50% json_valid_rate per domain. Gemma on token-analysis = 12% → skip. **Falsify:** token-analysis verdicts use only functional Dogs.
- [ ] **Auth /health (T1/O4).** ~~Split /live (boolean, open) + /health (full topology, auth required).~~ DONE in code: /metrics + /events now require auth, /health public response stripped of version. **Remaining:** deploy new binary + verify. **Falsify:** `curl funnel/metrics` → 401; `curl funnel/health` returns no version field.
- [x] **Early verdict (O5).** Return response when quorum_count Dogs responded. Don't wait for slow Dogs. **Falsify:** verdict latency < 10s when 3/5 Dogs respond in 5s.
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
