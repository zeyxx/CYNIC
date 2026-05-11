# Hermes Shared Browser & Behavioral Engine

> **One Chrome, two windows. The agent learns to browse like T.**

**Date:** 2026-05-11
**Status:** DESIGN APPROVED — pending implementation plan
**Layers:** L0 (browser + attribution) → L1 (behavioral capture) → L2 (replay + simulation)
**Organ:** Hermes X (`~/.cynic/organs/hermes/x/`)

---

## 1. Problem Statement

The Hermes X organ has a broken data pipeline. The dedicated Chrome browser (`hermes-browser.service`) crash-loops due to `DISPLAY=:1` (no X server). Without the browser:
- No traffic flows through the proxy → `dataset.jsonl` empty → curation dead
- No behavioral data captured → `behavior_log.jsonl` has 2 entries
- No CDP for navigator/search-executor → agent actions fail silently
- The engagement extension has never collected data

Beyond fixing the immediate breakage, the behavioral analysis layer is the **foundation for all future organic agents**. An agent must be able to simulate T.'s browsing behavior with measurable accuracy before operating autonomously.

## 2. Design Decisions

| Decision | Choice | Alternatives Rejected |
|---|---|---|
| Browser topology | One Chrome, two windows (human + agent) | Two separate Chrome instances (dual session risk); single window time-sharing (no concurrence) |
| Concurrence | Both windows active simultaneously | Agent only when human idle (limits agent throughput) |
| Attribution | CDP tab tracking via Browser Hub | Time-window heuristic (lossy with concurrence); operation-type heuristic (fragile) |
| Hub architecture | Central service, sole CDP client | State file convention (race conditions, latency); Chrome extension as hub (MV3 lifecycle fragile) |
| Behavioral capture | Two layers: raw input (30Hz) + semantic actions | Raw only (no learning signal); semantic only (no mimicry data) |
| Capture frequency | 30Hz mouse sampling | 10Hz (insufficient for Fitts' Law extraction); 60Hz (overkill, 2x storage) |
| Replay engine | Replay (reproduction) + Generative (autonomous) | Replay only (no autonomy); generative only (no calibration baseline) |
| Implementation | L0→L1→L2, each stable before next | All at once (too risky); L0 only (blocks future layers) |

## 3. Architecture

### 3.1 Component Overview

```
┌──────────────────────────────────────────────────────────┐
│                    Chrome (1 PID)                         │
│  ┌──────────────┐              ┌──────────────────┐      │
│  │ Window Human │              │ Window Agent      │      │
│  │ (tabs T.)    │              │ (tabs nav/search) │      │
│  └──────────────┘              └──────────────────┘      │
└─────────────────────────┬────────────────────────────────┘
                          │ CDP ws://127.0.0.1:40769
                          │
              ┌───────────┴───────────┐
              │    Browser Hub        │
              │  Tab Registry         │
              │  CDP Network listener │
              │  Viewport tracker (L1)│
              │  HTTP :40770          │
              └───┬──────┬──────┬────┘
                  │      │      │
    ┌─────────────┤      │      ├─────────────────┐
    │             │      │      │                 │
┌───┴───┐  ┌─────┴──┐ ┌─┴────┐ ┌┴──────────┐  ┌──┴──────────┐
│Proxy  │  │Nav/    │ │Search│ │Behavior   │  │Replay       │
│mitm   │  │Simul.  │ │Exec. │ │Logger v2  │  │Engine (L2)  │
│:8888  │  │        │ │      │ │           │  │             │
└───────┘  └────────┘ └──────┘ └───────────┘  └─────────────┘
```

### 3.2 Permanent Services (systemd)

| Service | Role | Port | Depends On |
|---|---|---|---|
| `hermes-browser.service` | Chrome with CDP, DISPLAY=:0, two windows | CDP :40769 | — |
| `hermes-browser-hub.service` | Tab registry, CDP listener, HTTP API | :40770 | hermes-browser |
| `hermes-proxy.service` | mitmdump + x_proxy.py, attribution tagging | :8888 | hermes-browser-hub |
| `hermes-behavior.service` (L1) | Behavior Logger v2 (raw + semantic + correlator) | — | hermes-browser-hub |

### 3.3 Ephemeral Consumers (timer-triggered)

Navigator, search-executor, task-runner: request tabs from Hub, execute, release.

## 4. Browser Hub

### 4.1 Role

The **sole CDP client**. No consumer talks directly to Chrome. This guarantees:
- Coherent tab registry (attribution)
- No CDP race conditions (single WebSocket)
- Natural extension point for L1 (viewport tracking) and L2 (replay)

### 4.2 HTTP API (:40770)

```
GET  /status                    → { chrome_pid, uptime, tabs, windows }
POST /tabs                      → { owner, url, window: "agent" } → { tab_id, cdp_target_id }
DELETE /tabs/:tab_id            → close tab, remove from registry
GET  /tabs                      → [{ tab_id, owner, url, created_at, window }]
GET  /tabs/:tab_id/owner        → "agent:navigator" | "human"
POST /tabs/:tab_id/navigate     → { url } (L2: replay pilots via Hub)
GET  /attribution?url=&ts=      → { source, owner_detail, tab_id }
WS   /events                    → stream: tab_created, tab_closed, tab_navigated, focus_changed
```

### 4.3 Tab Registry

```
TabEntry {
    tab_id:         ChromeTabId
    cdp_target_id:  String
    owner:          "human" | "agent:<service_name>"
    window_id:      ChromeWindowId
    created_at:     Timestamp
    last_url:       String
    last_activity:  Timestamp
}
```

**Attribution rule:** Hub creates two windows at boot. All tabs in `agent_window` = agent. All tabs in `human_window` = human. Tabs created manually by T. land in whichever window is active.

**Edge case:** T. drags a tab between windows → Hub receives `Target.targetInfoChanged`, updates registry.

### 4.4 Boot Sequence

1. Chrome starts with `--remote-debugging-port=40769`, DISPLAY=:0
2. Hub connects via CDP WebSocket, subscribes to `Target.*` events
3. Hub creates two windows: `human` (visible, X.com home) and `agent` (can be minimized)
4. Hub scans existing tabs (pre-existing tabs = all `human`)
5. Hub exposes `:40770`, writes `browser-state.json`
6. Proxy and consumers start

### 4.5 Resilience

- **Chrome crash:** Hub detects CDP disconnect → `degraded` mode. Consumers get 503. Chrome restarts (on-failure), Hub reconnects.
- **Hub crash:** Proxy loses tagging → fallback `source: "unknown"`. Chrome survives. Hub restart re-scans existing tabs (all pre-existing = `human`).
- **Proxy crash:** No impact on Hub or Chrome. dataset.jsonl stops growing temporarily.

### 4.6 Language

Python (asyncio + websockets for CDP, aiohttp for HTTP API). Consistent with Hermes organ (all Python). L0 scope: ~300 lines.

## 5. Proxy Attribution

### 5.1 The Problem

mitmdump sees HTTP requests. Chrome makes requests. mitmdump has no native way to know which tab originated a request.

### 5.2 Solution: CDP Network → Tab Correlation

```
Chrome                          Hub                         Proxy
  │                              │                            │
  │─ Network.requestWillBeSent ─▶│                            │
  │  {requestId, url, frameId}   │ map[url+ts] = tab_owner    │
  │                              │                            │
  │─ HTTP request ──────────────────────────────────────────▶│
  │                              │◀── GET /attribution?url=.. │
  │                              │──── { source: "agent" } ──▶│
  │                              │                            │ tag row
```

1. Chrome emits `Network.requestWillBeSent` via CDP → Hub receives `(url, timestamp, frameId)`
2. Hub resolves `frameId → targetId → TabEntry.owner`
3. Hub stores in ring buffer: `{ url_prefix, timestamp, owner }` (TTL 5s, max 1000 entries)
4. mitmdump receives the same HTTP request (ms later)
5. `x_proxy.py` calls `GET /attribution?url=<url>&ts=<timestamp>`
6. Hub matches by `url_prefix + |ts - stored_ts| < 2s` → returns `owner`
7. `x_proxy.py` adds `"source": owner` to enriched row

### 5.3 x_proxy.py Changes

Minimal: add `source` field to `_enrich()`, add HTTP call to Hub with fallback:
- Hub reachable → `source: "agent:navigator"` or `"human"`
- Hub unreachable → `source: "unknown"` (proxy never blocks)

### 5.4 Expected Accuracy

- GraphQL requests: >99% (unique URLs + timestamps)
- Concurrent same-second requests: resolved by requestId + URL matching
- Prefetch/preconnect without tab: `source: "unknown"` (no tweet data in these)

**Falsification:** inject 100 requests from agent tab + 100 from human tab simultaneously. Attribution accuracy must be >95%.

## 6. Behavior Logger v2

### 6.1 Architecture

Two layers, two output files, one service. In-process correlator joins them.

```
┌─────────────────────────────────────────────┐
│           Behavior Logger v2                 │
│  ┌────────────────┐  ┌───────────────────┐  │
│  │ Layer 0: Raw   │  │ Layer 1: Semantic │  │
│  │ pynput 30Hz    │  │ CDP viewport +    │  │
│  │                │  │ extension events   │  │
│  └───────┬────────┘  └────────┬──────────┘  │
│          ▼                    ▼              │
│  behavior_raw.jsonl   behavior_semantic.jsonl│
│          └────────┬───────────┘              │
│                   ▼                          │
│           Correlator → attention_map.jsonl   │
└─────────────────────────────────────────────┘
```

### 6.2 Layer 0 — Raw Input (mimicry training data)

pynput captures mouse/keyboard/scroll, enriched with Hub context:

```jsonl
{
  "ts": "2026-05-11T14:32:01.234Z",
  "type": "scroll",
  "dx": 0, "dy": -340,
  "window": "Google Chrome",
  "tab_id": "3FA2...",
  "tab_owner": "human",
  "viewport_url": "https://x.com/home",
  "screen_xy": [842, 456]
}
```

**Event types:** `mouse_move` (with velocity, acceleration, curvature, segment), `click` (with dwell_before_click_ms), `scroll` (aggregated per burst), `keypress` (modifiers only — ctrl+c, ctrl+v; no content capture for privacy), `selected_text_hash` (blake2b, not cleartext).

**Sampling:** 30Hz for mouse_move (33ms intervals). Scroll aggregated per burst. Keyboard on-event only.

**Feature extraction in-stream:** velocity, acceleration, jerk, curvature computed between samples. Segment classification: `ballistic` (fast toward target), `approach` (deceleration), `correction` (micro-adjust), `idle` (>200ms no movement).

**Compression:** raw 30Hz capture in memory → storage writes key points only (direction changes, click targets, scroll stops, segment boundaries) + feature summary per segment. ~30MB/day effective.

### 6.3 Layer 1 — Semantic Actions (learning signal)

CDP viewport tracker + Chrome extension produce high-level events:

```jsonl
{"ts": "...", "type": "tweet_visible", "tweet_id": "192...", "viewport_position": "center", "tab_id": "...", "tab_owner": "human"}
{"ts": "...", "type": "tweet_engaged", "tweet_id": "192...", "action": "like", "dwell_ms": 1900, "tab_id": "...", "tab_owner": "human"}
{"ts": "...", "type": "thread_expanded", "tweet_id": "192...", "tab_id": "...", "tab_owner": "human"}
{"ts": "...", "type": "cross_domain_exit", "from_url": "https://x.com/...", "to_url": "https://solscan.io/token/...", "trigger_tweet_id": "192...", "tab_id": "...", "tab_owner": "human"}
{"ts": "...", "type": "profile_hover", "author": "...", "hover_ms": 1200, "tab_id": "...", "tab_owner": "human"}
```

**Viewport tracker:** Hub polls `Runtime.evaluate("window.scrollY")` at 2Hz per X.com tab. Combined with tweet positions via `Runtime.evaluate` on `article[data-testid="tweet"]`. Emits `tweet_visible` on viewport enter, computes `dwell_ms` on exit.

**Extension contribution:** DOM mutation observer for engagements (like, bookmark, reply, retweet). `webNavigation` for cross-domain exits. Mouseenter on avatar/username >500ms for profile_hover. Extension POSTs to Hub (`:40770/events/extension`), Hub writes.

### 6.4 Correlator

In-process join of raw + semantic. Produces attention_map.jsonl:

```jsonl
{
  "ts": "2026-05-11T14:32:03.400Z",
  "tweet_id": "1921234567890",
  "funnel_stage": "ACTION",
  "source": "human",
  "exposure_ts": "2026-05-11T14:32:01.500Z",
  "dwell_ms": 1900,
  "scroll_velocity_before": 234.5,
  "dwell_before_click_ms": 1868,
  "action": "like",
  "cross_domain_exit": "solscan.io",
  "signal_score": 4,
  "killchain": ["proxy:HomeTimeline", "viewport:visible", "raw:scroll_stop", "semantic:like", "semantic:cross_domain"]
}
```

**Funnel stages:** CHARGÉ (proxy) → VU (viewport) → ATTENTION (dwell) → ACTION (engage) → CROSS_DOMAIN (exit URL). Score = max stage reached.

### 6.5 Emergent Capabilities

These arise from combining the layers:

| Capability | Sources | Consumer |
|---|---|---|
| **Attention Map** | proxy × viewport × behavior | stochastic scheduler, agent effectiveness |
| **Discovery Attribution** | proxy × attribution × engagement | killchain tracer |
| **Agent Effectiveness Score** | attribution × engagement × verdicts | briefing consumer |
| **Implicit Feedback Loop** | attention × behavioral model → scheduler | search-generator (closed loop) |

## 7. L2 Replay & Simulation Engine

### 7.1 Mode 1 — Replay (reproduction)

Replays a recorded T. session in the agent browser window.

```
behavior_raw.jsonl → Sequence Parser → Action Queue → Browser Hub → Chrome (agent window)
```

- **Sequence Parser:** segments by session (gap >5min), filters `tab_owner: "human"` + X.com URLs
- **Action Queue:** preserves original timing (delay_ms between actions)
- **Humanization layer:** Gaussian noise (sigma=15ms timing, sigma=3px position) so no two replays are bit-identical

### 7.2 Mode 2 — Generative (autonomous behavior)

The agent generates behavior resembling T. based on a trained model.

```
Behavioral Model → Decision Engine → Action Generator → Browser Hub → Chrome
```

- **Decision Engine:** for each visible tweet, computes `P(action)` for each possible action (scroll-past, dwell, like, bookmark, reply, profile-visit, cross-domain). Samples according to learned probabilities.
- **Action Generator:** produces physical actions via Fitts' law mouse movement, human typing rhythm, natural scroll patterns.
- **Behavioral Model:** trained exclusively on T.'s data. Architecture TBD post-L1 (data-driven choice: logistic regression baseline, upgrade if justified).

### 7.3 Accuracy Measurement

**Mimicry divergence** via Dynamic Time Warping (DTW):

1. Record 10 sessions of T. (1h each)
2. Replay same stimuli (same tweets, same order) for the agent
3. Compare action sequences
4. Score = `1 - normalized_DTW`
5. Target: > phi-inverse (0.618)

**Dimensions measured:**
- Mouse trajectory (DTW on xy curves)
- Timing (distribution of inter-event delays)
- Decisions (Cohen's kappa between T. actions and agent actions on same tweets)
- Funnel distribution (% EXPOSURE/ATTENTION/ACTION/CROSS_DOMAIN similarity)

### 7.4 Guardrails

- **Rate limiting:** agent cannot engage more than T.'s P95 rate
- **No write actions without approval:** like/bookmark = autonomous OK. Reply/retweet = approval required (L2+ only)
- **Kill switch:** `POST /hub/agent/pause` → agent ceases all browser actions, tabs stay open but idle
- **Divergence alert:** if mimicry score drops below 0.382 (phi^-2) in real-time → auto-pause + alert
- **KPI:** `captcha_triggered_count` must remain 0

## 8. Data Flow

```
T. browse X ────────────────────────────────────────────┐
Agent browse X ─────────────────────────────────────────┤
                                                        │
                    Chrome (1 PID, 2 windows)            │
                         │          │                    │
              HTTP traffic│          │CDP events          │pynput
                         │          │                    │
                         ▼          ▼                    ▼
                    ┌─────────┐ ┌──────────┐    ┌──────────────┐
                    │  Proxy  │ │   Hub    │    │ Behavior     │
                    │ mitm    │ │ registry │    │ Logger v2    │
                    │ :8888   │ │ :40770   │    │              │
                    └────┬────┘ └──┬───┬───┘    └──┬────┬──────┘
                         │        │   │            │    │
              GET /attrib│◀───────┘   │            │    │
                         │            │            │    │
                         ▼            ▼            ▼    ▼
                   dataset.jsonl  viewport_    raw   semantic
                   (source tagged) events     .jsonl  .jsonl
                         │            │         │      │
                         │            │         └──┬───┘
                         │            │            │
                         │            │     attention_map.jsonl
                         │            │            │
              ┌──────────┼────────────┼────────────┤
              ▼          ▼            ▼            ▼
         Curation    Ingest      Scheduler    Effectiveness
                     →kernel     (adjust       Score
                                 searches)
                        │
                        ▼
                     /observe → kernel → Dogs → verdicts
                                                    │
                                              gemini briefing
                                              (+ attention data)
                                                    │
                                                 SKILL.md
                                                    │
                                              search-generator
                                              (closed loop)
```

## 9. ORGAN_MANIFEST v2 Artifacts

### Perception Layer (browser + proxy + behavioral)

| Artifact | Path | Writer | Readers | Contract |
|---|---|---|---|---|
| browser-state | `browser-state.json` | browser-hub | nav, search-exec, behavior | CDP port, window IDs, PID |
| tab-registry | in-memory (Hub API) | browser-hub | proxy, behavior, replay | SSOT for agent vs human |
| dataset.jsonl | `dataset.jsonl` | x-proxy | curation, ingest, keyword-discovery | Append-only, `source` field tagged |
| behavior_raw | `behavior/raw.jsonl` | behavior-logger-v2 L0 | correlator, replay-engine, model-trainer | 30Hz mouse, tab-correlated. 30-day retention. |
| behavior_semantic | `behavior/semantic.jsonl` | behavior-logger-v2 L1 | correlator, scheduler, briefing | tweet_visible, tweet_engaged, cross_domain |
| attention_map | `behavior/attention_map.jsonl` | correlator | effectiveness, briefing, scheduler | One row per tweet, funnel stage |
| viewport_events | `behavior/viewport_events.jsonl` | browser-hub CDP | behavior-logger-v2 | 2Hz poll, tweet enter/leave viewport |
| navigation_context | `behavior/navigation_context.jsonl` | browser-hub CDP | killchain, behavior-logger | tab_switch, url_transition, focus |
| cross_domain_exits | `behavior/cross_domain_exits.jsonl` | chrome-extension | killchain, correlator | from_url, to_url, trigger_tweet_id |
| engagement | `behavior/engagement.jsonl` | chrome-extension | engagement-correlator, correlator | like, bookmark, reply via DOM mutation |
| session_boundaries | `behavior/sessions.jsonl` | behavior-logger-v2 | temporal-analysis, replay | session_start/end, active/idle duration |

### Cognition Layer (existing, unchanged)

| Artifact | Path | Writer | Readers |
|---|---|---|---|
| skill.md | `agent/SKILL.md` | gemini-cli | hermes-9b, claude-code |
| domains.yaml | `cynic-python/lab/config/domains.yaml` | claude-code | lab.py, gemini-cli |
| agent-tasks | `agent-tasks/` | gemini-cli | hermes-9b |
| lab_briefing | `analysis/lab_briefing_latest.json` | lab.py | gemini-cli, claude-code |
| verdicts | `verdicts/` | ingest-daemon | lab.py, gemini-cli |
| observations | `observations/` | hermes-9b | lab.py, kernel, gemini-cli |

### Action Layer (L2)

| Artifact | Path | Writer | Readers |
|---|---|---|---|
| replay_sequences | `replay/sequences/` | replay-engine | mimicry-benchmark |
| behavioral_model | `replay/model/` | model-trainer | replay-engine decision engine |

### Coordination Rules

```
Rule 6: BROWSER HUB IS SSOT FOR TABS. No consumer creates Chrome tabs
        directly via CDP. All tab operations go through Hub HTTP API.

Rule 7: ATTRIBUTION IS APPEND-ONLY. Once source is tagged in dataset.jsonl,
        it is never retroactively changed. Unknown stays unknown.

Rule 8: BEHAVIORAL DATA IS T.'s PROPERTY. Raw behavior never leaves the
        machine. No POST to kernel, no sync to remote. Local only.
        Aggregates (attention_map) can be consumed by kernel.
```

## 10. KPIs

| KPI | L0 Target | L1 Target | L2 Target | Measurement |
|---|---|---|---|---|
| Capture rate | >0 tweets/h | >50 tweets/h | >50 tweets/h | `wc -l dataset.jsonl` / uptime |
| Attribution accuracy | >95% | >99% | >99% | Compare tab registry vs request log |
| Behavioral coverage | n/a | >80% sessions | >80% | behavior_log entries / chrome sessions |
| Killchain completeness | n/a | >60% | >80% | killchain entries with source != unknown |
| Agent effectiveness | n/a | measurable | ratio >0.8 | signal_score mean by source |
| Mimicry score (DTW) | n/a | n/a | >0.618 | benchmark suite |
| Funnel correlation | n/a | measurable | >0.7 Spearman | agent vs human funnel distribution |
| Captcha triggered | 0 | 0 | 0 | monitor count |

## 11. Implementation Layers

### L0 — Shared Browser + Attribution

**Scope:** make data flow. Zero intelligence, zero analysis.

| Component | File | New/Modified | ~Lines |
|---|---|---|---|
| Chrome service fix | `infra/systemd/hermes-browser.service` | Modified | 10 |
| Browser Hub | `scripts/hermes-x/core/browser_hub.py` | New | 300 |
| Hub service | `infra/systemd/hermes-browser-hub.service` | New | 20 |
| Proxy tagging | `scripts/hermes-x/core/x_proxy.py` | Modified | +30 |
| Hub client lib | `scripts/hermes-x/core/hub_client.py` | New | 50 |
| Nav/Search via Hub | `organic_navigator.py`, `search_executor.py` | Modified | +20 each |

**Exit criteria (all required for L1):**
- 7 days without crash of Chrome + Hub + Proxy trio
- Attribution accuracy >95% (1-day manual spot check)
- dataset.jsonl >500 rows with correct source
- Curation service green

### L1 — Enhanced Behavioral Capture

**Scope:** observe. Capture the full funnel. Zero autonomous action.

| Component | File | New/Modified | ~Lines |
|---|---|---|---|
| Behavior Logger v2 | `scripts/hermes-x/core/behavior_logger_v2.py` | New | 500 |
| Viewport tracker | `scripts/hermes-x/core/viewport_tracker.py` | New | 200 |
| Chrome extension v2 | `scripts/hermes-x/x-engagement-extension/` | Modified | +100 |
| Behavior service | `infra/systemd/hermes-behavior.service` | Modified | 10 |
| Scheduler wiring | `stochastic_scheduler.py` | Modified | +30 |
| Killchain tracer v2 | `hermes_killchain_tracer.py` | Modified | +50 |

**Exit criteria (all required for L2):**
- 14 days of behavioral data without >4h gap during active hours
- Attention map >1000 tweets with funnel stage >= ATTENTION
- Killchain completeness >60%
- Agent effectiveness ratio measurable
- behavior_raw sufficient for model training (>10 sessions of 1h+)

### L2 — Replay & Simulation Engine

**Scope:** act. Reproduce and generate human-like behavior.

| Component | File | New/Modified | ~Lines |
|---|---|---|---|
| Sequence parser | `scripts/hermes-x/core/replay/sequence_parser.py` | New | 200 |
| Action generator | `scripts/hermes-x/core/replay/action_generator.py` | New | 400 |
| Replay engine | `scripts/hermes-x/core/replay/engine.py` | New | 300 |
| Decision engine | `scripts/hermes-x/core/replay/decision_engine.py` | New | 300 |
| Model trainer | `scripts/hermes-x/core/replay/train_mimicry.py` | New | 400 |
| Mimicry benchmark | `scripts/hermes-x/core/replay/benchmark.py` | New | 200 |
| Replay service | `infra/systemd/hermes-replay.service` | New | 20 |

**Production criteria:**
- Mimicry score >0.618 on 10 benchmark sessions
- Zero X rate limits triggered in 7 days
- Agent effectiveness ratio >0.8
- Kill switch tested and functional

## 12. Identified Risks

| Risk | Impact | Mitigation | Status |
|---|---|---|---|
| **Resource budget** | Chrome + Hub + Proxy + Logger 30Hz on machine already running kernel + llama-server (~32GB RAM) | Monitor RSS. Chrome 2 windows ~500MB-1GB. Logger ~50MB. Hub ~30MB. | Monitor in L0 |
| **X bot detection** | Agent window automated actions may trigger Arkose Labs captcha or session ban | Humanization layer, rate limit = T.'s P95, captcha KPI = 0 | Monitor from L0, critical for L2 |
| **Chrome extension MV3** | Service workers timeout after 5 minutes — continuous DOM observation may break | Verify extension manifest version. Use `chrome.alarms` keepalive or event-driven approach. | Check before L1 |
| **Chrome auto-update** | CDP protocol changes can break Hub | Pin Chrome version or test Hub CDP compatibility at boot. Log Chrome version. | Monitor in L0 |
| **Data retention** | behavior_raw at ~30MB/day = ~900MB/month | 30-day rolling retention. Cron logrotate to `behavior/archive/`. Archive keeps feature summaries only, not raw events. | Implement in L1 |
| **30Hz sufficiency** | Micro-corrections (20-30ms) may fall between samples | Start 30Hz. If L2 mimicry score shows gap, upgrade to 50Hz. Format allows frequency change without breaking consumers. | Evaluate in L2 |

## 13. What This Spec Does NOT Cover

- **Model architecture for L2** — data-driven decision after L1 data collection
- **Multi-platform** — X/Twitter only. Other platforms = separate spec
- **Remote behavioral data** — Rule 8: raw behavior never leaves the machine
- **Full agent autonomy** — L2 keeps guardrails. Unsupervised operation = separate spec post-L2
- **Existing broken links** — hermes-task-runner polling kernel instead of organ-local (documented in ORGAN_MANIFEST, orthogonal)
