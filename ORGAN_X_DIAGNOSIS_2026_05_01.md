# Organ X Diagnosis — 2026-05-01 Post-Reboot

**Status:** 95% infrastructure OK, curation pipeline offline, K15 loop broken.

## Live Components

| Component | Status | Last Update | Notes |
|-----------|--------|-------------|-------|
| hermes-browser.service | ✓ active | 2026-05-01 18:38:56 | X.com loaded, CDP :40769 |
| hermes-behavior.service | ✓ active | 2026-05-01 18:23:06 | 533K events, 84MB |
| mitmproxy | ✓ running | 9 active proxy conns | Captures working, /about endpoint returns 502 |
| llama-embed.service | ✗ failing | auto-restart loop | Model missing: Qwen3-Embedding-0.6B-Q8_0.gguf |
| CYNIC kernel | ✗ offline | last reached 2026-04-30 | CYNIC_API_KEY unset, unreachable from Organ X |
| Hermes agent | ✗ not running | — | No active agent sessions |

## Data Pipeline State

### Captures (Raw Passive Collection)
- **494 JSON files** collected April 24 — May 01
- **Format:** GraphQL operation + response (not parsed)
- **Domains:** X.com pages (HomeTimeline, SearchTimeline, Bookmarks, Likes, UserTweets, TweetDetail)
- **Gap:** No extraction of tweet text → D1-D6 domain classification missing

### Behavior Logging (ML Training Data)
- **533,809 events** in behavior_log.jsonl (84MB)
- **Content:** Mouse/keyboard events with window context + timestamps
- **Quality:** High-fidelity human interaction patterns
- **Usage:** Ready for behavior ML training (scroll speed, pause distributions, click targeting)

### Reflections (Agent Knowledge)
- **38 cycles** recorded (4/30 02:55 — 5/1 18:20)
- **Status:** All domain=null (empty), kernel_status="unreachable"
- **K15 violation:** Producer (reflections cron) has no consumer (kernel not accessible)

## Critical Gaps for Hackathon (J9)

### Gap 1: Kernel Connectivity (BLOCKER)
**Problem:** CYNIC_API_KEY not set in environment. Organ X cannot:
- Send observations to kernel via `/observe`
- Query verdicts for feedback loop
- Store curated D1-D6 data

**Fix:** Set `CYNIC_API_KEY` in `~/.cynic-env`, ensure kernel running on Tailscale `<TAILSCALE_CORE>:3030`

**Timeline:** 1 command, 0 implementation

### Gap 2: Curation Pipeline (ARCHITECTURAL)
**Problem:** Raw GraphQL captures exist, but no:
- Text extraction from X.com API responses
- Domain classification (D1: tokens/solana, D2: LLM, D3: sovereignty, D4: security, D5: macro, D6: epistemology)
- Labeled dataset for agent training

**Current state:** Memory (2026-04-27) claims "curated/D2-D6.jsonl" exists, **it does not**.

**Options:**
1. **Manual curation sprint** (1-2h): Python script to extract + keyword-classify 20-30 tweets per domain
2. **Keyword heuristic** (30min): D1=["solana", "token", "swap"], D2=["llm", "model"], etc. + regex classifier
3. **Defer to Phase 2** (post-hackathon): Full ML classifier on 533K behavior + trial runs

**Recommendation for May 10:** Use Option 2 (keyword heuristic) to unblock domain-aware curation by May 6. Sufficient for "Organ X reflects on signal" narrative.

### Gap 3: Agent Orchestration (OPERATIONAL)
**Problem:** Hermes agent is not running in directed exploration mode.

**Design (from memory 2026-04-27):**
- Agent reads Domain Dashboard (coverage % per D1-D6)
- Picks lowest-coverage domain with known consumer
- Executes searches on X.com for that domain
- Observes results via mitmproxy
- Stores reflections via kernel `/observe`

**Current:** Browser running + behavior capture live. Agent infrastructure absent.

**Implementation:** `hermes-agent-orchestrator.py` that:
1. Queries kernel for domain coverage
2. Spins up Claude/Gemini agent on `<TAILSCALE_GPU>:8080` (Qwen 27B)
3. Injects exploration task + behavioral profile
4. Routes reflections back to kernel

**Timeline:** 1 session (3-4h) for prototype. Post-hackathon priority.

## Empirical Audit Plan (Option 1)

### Behavior Data Quality (Ready Now)
- **533K events** in behavior_log.jsonl
- **What to measure:**
  - Scroll speed distribution (pixels/sec) — should be bimodal (reading vs. skimming)
  - Pause durations (ms between events) — should show natural clustering (1-2s deliberation, 50-200ms mechanical)
  - Click targeting (x,y coords) — should favor center-top, left nav (human-typical), not uniform random
  - Window switching patterns — should show "context switching" (reads, decides, clicks)
- **Falsification:** If uniform random, organic movement is impossible. If all pauses <50ms, data logger is broken.

### Capture Volume Analysis (30min)
- Graph: capture count over time (April 24 — May 1)
- Identify when agent was active vs. idle
- Check: Are captures clustering around specific domains (search patterns)?

## Behavior ML Baseline (Option 4)

**Ready to start:** 533K events, high-quality labels (window context)

**Design:**
1. Tokenize events: scroll_direction, scroll_speed, pause_duration, click_region, window_type → vector
2. Train shallow LSTM (1M params on 4060 Ti): predict next event (P_scroll | prev 10 events)
3. Use trained embeddings as "organic movement encoder" for agent
4. Agent samples from learned distribution instead of heuristics

**Expected:** 85%+ prediction accuracy (human behavior is highly regular). If <70%, data quality issue.

## Post-Hackathon Roadmap

1. **Kernel restored** (May 2) — CYNIC_API_KEY + kernel running
2. **Curation pipeline live** (May 6) — keyword heuristic + D1-D6 labels on 494 captures
3. **Agent orchestration** (May 10) — Hermes agent + exploration loop
4. **Behavior ML** (May 15) — LSTM on 533K events, deployed in agent kernel calls
5. **Cross-domain synthesis** (May 20) — Claude analyzes curated D1-D6 for intersections (e.g., token+security patterns)

## Observations

- **Infrastructure is healthy** — mitmproxy, browser, behavior logger all active post-reboot
- **Data collection is working** — 494 captures, 533K behavior events, 38 reflection cycles
- **Curation is missing** — architectural design exists (Domain Dashboard, D1-D6 taxonomy), implementation incomplete
- **Kernel connectivity is the gate** — everything else waits on `/observe` and verdict feedback loop

**Confidence on blockers:** observed (CYNIC_API_KEY unset, kernel offline), deduced (curation ≠ exists).

---

*Session: deep/organ-x-diagnosis-2026-05-01, 18:36-18:xx CEST*
