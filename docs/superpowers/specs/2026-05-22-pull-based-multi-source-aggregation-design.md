# Pull-Based Multi-Source Aggregation

> The judgment pulls from every organ. The organ does not push to the judgment.

**Date:** 2026-05-22
**Status:** Design
**Authors:** T. + Claude Opus

---

## Problem

The X organ ingests 21K+ tweets and sends each one individually to `/judge`. The deterministic heuristic abstains on 3/6 axioms ("requires semantic understanding"), producing identical BARK verdicts (q~0.306) for every tweet. 33 req/min saturated the single GPU slot for zero usable intelligence.

A single tweet is noise. The signal is in the convergence — 5 independent authors mentioning the same token in 6 hours. The organism needs forensic aggregation across sources (X, Telegram, on-chain) per item, not per-tweet judgment.

## Design Principles

1. **Organs observe, the kernel judges.** Organs are indexed databases, not judgment pipelines.
2. **The judgment pulls.** When judging $TOKEN, the kernel queries every organ: "what do you know about this?"
3. **Convergence triggers judgment.** Organs detect when signal converges on an item and emit a structured summary.
4. **Organ-agnostic pattern.** The convergence detection + summary building interface is reusable across X, Telegram, and future organs.
5. **Two triggers coexist.** Autonomous (convergence-triggered) + on-demand (user/agent-triggered).

## Architecture

```
Organs (observe + digest)          Kernel (aggregate + judge)
========================          ==========================

X daemon                           convergence consumer (K15)
  ├─ /observe per tweet              polls tool=*-convergence
  ├─ convergence detector              │
  │   (3+ authors / cashtag / 6h)      ├─ resolve cashtag → mint
  └─ POST /observe                     ├─ pull X summary
       tool=x-convergence              ├─ pull Telegram summary
       target=$SOL                     ├─ pull Helius on-chain
                                       ├─ build multi-source stimulus
Telegram daemon                        └─ POST /judge (27B + det)
  ├─ /observe per message                    │
  ├─ convergence detector                    ▼
  └─ POST /observe                     verdict (rich, multi-source)
       tool=tg-convergence

Demand trigger (user/agent)
  POST /judge { mint, domain }
  └─ kernel aggregates on the fly
       (same pull path, no convergence needed)
```

## Components

### 1. X Daemon — Observe Mode

**Current:** every tweet → `/judge` → identical BARK verdict.
**New:** every tweet → `/observe` (store, no Dog evaluation).

```python
# Per tweet:
POST /observe {
    tool: "x-proxy-cynic",
    target: tweet_id,           # traçability
    domain: detected_domain,    # D1, D2, D4...
    tags: [cashtags + narratives + f"tweet:{tweet_id}"],
    context: structured_summary,  # author, metrics, text[:300]
    agent_id: "hermes-x-cynic",
}
```

The ingest daemon stops calling `/judge`. High-signal tweets are stored, not judged individually.

### 2. Convergence Detector (Organ-Agnostic Interface)

Python interface implemented per organ:

```python
class OrganDigester(Protocol):
    def detect_convergence(self, recent_observations: list[dict]) -> list[ConvergenceSignal]:
        """Scan recent observations for item convergence."""
        ...

    def build_summary(self, signal: ConvergenceSignal, observations: list[dict]) -> dict:
        """Build structured summary for a convergent item."""
        ...
```

**X implementation:**
- Group observations by cashtag (from `tags[]`)
- Convergence = 3+ distinct `author_screen_name` mentioning same cashtag within 6h window
- Summary includes: author count, tier distribution, sentiment direction, coordination score, key quotes (max 3, for Dog traçability)

**Telegram implementation:**
- Group by keyword/cashtag extracted from messages
- Convergence = 3+ distinct channels or 5+ messages in same channel within 6h
- Summary includes: channel count, message volume, first-seen timestamp, key excerpts

**Convergence signal output:**

```python
POST /observe {
    tool: "{organ}-convergence",     # x-convergence, tg-convergence
    target: resolved_mint or "$SOL", # mint if resolved, cashtag if ambiguous
    domain: "D1",                    # inferred from item type
    tags: ["convergence", "$SOL", "mint:So1111...", "compound-loop"],
    # ^ compound-loop tag: K21 — nightshift MUST filter this to prevent amplification
    context: json.dumps({            # MUST be json.dumps() — context field is TYPE string
        "item": "$SOL",
        "resolved_mint": "So1111..." | null,
        "author_count": 5,
        "authors": ["@whale1", "@analyst2", ...],
        "tier_distribution": {"whale": 2, "influencer": 1, "unknown": 2},
        "coordination_score": 0.1,   # 0=independent, 1=identical texts
        "sentiment": "bearish",      # majority direction
        "window_hours": 6,
        "key_quotes": [
            {"author": "@whale1", "text": "...", "tweet_id": "123"},
            {"author": "@analyst2", "text": "...", "tweet_id": "456"},
        ],
        "source_organ": "x",
    },
    agent_id: "hermes-x-cynic",
}
```

### 3. Cashtag-Mint Resolver

Maps `$SOL` → `So11111111111111111111111111111111`.

**Sources (in priority order):**
1. Local cache (file-based, `cashtag_mint_map.json` — grows over time)
2. Helius `searchAssets` by symbol (API call, cached)
3. DexScreener search by symbol (fallback)

**Ambiguity handling:**
- 1 result → resolved, include `resolved_mint` in convergence signal
- 0 results → `resolved_mint: null`, signal stored but kernel skip on-chain enrichment
- 2+ results → `resolved_mint: null`, tag with `ambiguous_symbol`. Ambiguity = signal (token squatting)

**Storage:** SurrealDB table `cashtag_mint` (not a file — avoids concurrent write corruption between X and Telegram daemons). Schema: `{ symbol: string, mint: string, resolved_at: datetime, source: string }`. Shared by all organs via kernel REST or direct DB.

**Location:** Python module in `cynic-python/`, shared by X and Telegram digesters. NOT in the kernel — resolution happens at organ level before posting the convergence signal.

### 4. K15 Convergence Consumer (Kernel)

A runtime loop in the kernel that consumes convergence signals and triggers multi-source judgment.

```rust
// In runtime_loops.rs — new loop alongside nightshift, state_log, etc.
async fn convergence_consumer_loop(judge, storage, enricher, ...) {
    loop {
        // Poll for recent convergence observations not yet judged
        let signals = storage.list_observations_raw(
            Some("D1"),          // or any domain
            None,                // any agent
            20,
        ).await?
        .into_iter()
        .filter(|o| o.tool.ends_with("-convergence"))
        .collect();

        for signal in signals {
            // Dedup: check verdict cache for this target+domain in last 2h.
            // Uses existing VerdictCache infrastructure — no new storage needed.
            // If a verdict exists for this target within the window, skip.
            // Safe on boot: cache loads from DB, so restart doesn't re-judge.
            if verdict_cache.has_recent(signal.domain, signal.target, hours=2) {
                continue;
            }

            // 1. Parse context (json string → struct)
            // 2. Extract resolved_mint from context or target
            // 3. Pull Helius enrichment (existing enrich_token path)
            // 4. Pull X summary (from signal itself)
            // 5. Pull Telegram summary (list_observations_by_target for tg-convergence)
            // 6. Build multi-source stimulus
            // 7. POST /judge with priority=Hermes (queued, not background)
        }

        sleep(60s).await;  // poll interval — 60s not 30s (GPU slot is precious)
    }
}
```

**Dedup:** Tag judged convergence signals with `judged:verdict_id` to avoid re-processing. Or maintain an in-memory set of processed observation IDs (reset at boot — re-judging is safe, just wasteful).

### 5. Enriched Multi-Source Stimulus

The stimulus sent to `/judge` combines all sources:

```
[DOMAIN: D1 — Token Analysis]

[ON-CHAIN DATA — Helius]
mint: So1111...
symbol: $SOL
holders: 1,234,567
top1_pct: 2.3% (type: Wallet)
HHI: 0.008
lp_status: burned
age: 892 days
price: $168.42
volume_24h: $2.1B

[SOCIAL SIGNAL — X Organ]
convergence: 5 authors in 6h
authors: @whale1 (whale), @analyst2 (influencer), @user3, @user4, @user5
coordination_score: 0.1 (independent)
sentiment: bearish
key_quotes:
  - @whale1: "SOL validators are seeing massive unstaking..."
  - @analyst2: "On-chain data shows large transfers to exchanges"

[SOCIAL SIGNAL — Telegram Organ]
convergence: 3 channels, 12 messages
channels: #solana-trading, #crypto-alerts, #whale-moves
sentiment: bearish

[BASELINES]
...

[QUESTION]
Evaluate this token's current risk profile using all available sources.
```

### 6. Demand Trigger (On-Demand Aggregation)

When a user or agent calls `POST /judge { content: "mint_address", domain: "token-analysis" }`:

1. `enrich_token` fires (existing path)
2. NEW: query convergence signals by mint: `list_observations_by_target("D1", mint_address)`
   - Works because convergence observations store `target = resolved_mint` when known
   - Fallback: query by tag `mint:{address}` via `list_observations_by_tag` for cashtag-targeted signals
3. If convergence summary exists → parse `context` JSON string → inject into stimulus
4. If no convergence → judge with on-chain only (current behavior, no regression)

This means the demand path and the autonomous path share the same enrichment logic. The only difference is the trigger.

## What Changes

| Component | Before | After |
|-----------|--------|-------|
| X daemon | `/judge` per tweet (33/min, all BARK) | `/observe` per tweet + convergence summaries |
| Kernel enrichment | On-chain only (Helius) | On-chain + X summary + Telegram summary |
| Convergence detection | Does not exist | Organ-agnostic Python interface, per-organ impl |
| Cashtag resolver | Does not exist | Python module, file cache + Helius fallback |
| Convergence consumer | Does not exist | Kernel runtime loop, polls convergence signals |
| `/judge` demand path | On-chain enrichment | On-chain + pull convergence summaries if available |

## What Does NOT Change

- Helius enricher (same API, same TokenData struct)
- Crystal pipeline (verdicts still crystallize)
- Dog prompts (structured stimulus format unchanged)
- Verdict format (same Verdict struct)
- SlotSemaphore / InferenceQueue (same priority queuing)
- Deterministic Dog (still participates in every verdict)

## Data Model

**Observation `target` convention (reinforced):**
- Phone-number organ: `target` = phone number (the item)
- X organ (per-tweet): `target` = tweet_id (the source) — for traçability
- X organ (convergence): `target` = cashtag (the item) — for kernel query
- Telegram organ (convergence): `target` = cashtag or keyword (the item)

**New query needed:** `list_observations_by_tag(domain, tag, limit)` — SurrealDB `WHERE tags CONTAINS $tag`. Not indexed initially; viable at current scale (21K obs). Add index if >100K.

## Convergence Parameters (Phase B → C)

**Phase B (launch):** Static thresholds.
- X: 3+ distinct authors / same cashtag / 6h window
- Telegram: 3+ channels OR 5+ messages / same keyword / 6h window
- Cooldown: 1h per item after convergence emitted (in-memory dict per daemon; lost on restart — acceptable, convergence consumer dedup catches duplicates)

**Phase C (adaptive):** Thresholds adjust based on verdict feedback.
- Convergence that produced HOWL/WAG → lower threshold (more sensitive)
- Convergence that produced BARK → raise threshold (less noise)
- Coordination detected (3 bots same text) → exclude from author count
- Implemented via crystal feedback loop (existing CCM infrastructure)

## Falsification

1. **Convergence produces richer verdicts:** After deployment, D1 verdicts from convergence signals should have q_score variance > 0.1 (vs current ~0 variance from deterministic-only). Falsify: if q_score remains flat, the multi-source stimulus isn't helping Dogs discriminate.

2. **GPU slot no longer saturated:** X daemon stops flooding `/judge`. Convergence consumer sends ~5-10 rich judgments/day (vs 33/min noise). Falsify: if `/judge` rate from X organ is still >1/min, the daemon isn't in observe mode.

3. **On-demand enrichment works:** `POST /judge { mint }` returns a verdict that includes X social signal when convergence exists. Falsify: check verdict reasoning for social source citations.

## Implementation Order

1. **X daemon: observe mode** — stop `/judge`, start `/observe`. Immediate: kills the flood, frees GPU slot.
2. **Convergence detector (X)** — Python, groups by cashtag, emits summaries.
3. **Cashtag resolver** — Python module, SurrealDB `cashtag_mint` table + Helius lookup.
4. **`list_observations_by_tag`** — Kernel StoragePort + SurrealDB `WHERE tags CONTAINS`. Add `DEFINE INDEX obs_tool_idx ON observation FIELDS tool` to schema bootstrap.
5. **Convergence consumer** — Kernel runtime loop, triggers multi-source `/judge`.
6. **Enrichment pull** — `enrich_token` consumes X + Telegram summaries.
7. **Telegram digester** — Same interface, different grouping logic.
8. **Phase C adaptive thresholds** — after enough verdicts to measure.

## K21 Compliance

Convergence observations MUST carry `compound-loop` tag. Nightshift already filters this tag — no Rust change needed. Without this tag, nightshift re-judges every convergence summary as a "twitter" crystal, creating amplification (K21 violation). The tag is included in the convergence signal output above.

## Known Constraints

1. **`context` is TYPE string** — Python daemons MUST `json.dumps()` the context dict. Kernel parses with `serde_json::from_str()`. Existing pattern in `trajectory_cron` path.
2. **`list_observations_by_tag` is a full table scan** — viable at 21K observations (<100ms). Add SurrealDB index on `tags` if >100K. The `obs_tool_idx` index on `tool` field handles the convergence consumer's primary query path.
3. **Cooldown state is per-process** — daemon restart resets cooldown. Convergence consumer dedup (verdict cache check) is the safety net. Double-emission costs one GPU inference, not a cascade.
4. **Cashtag resolution is best-effort** — ambiguous symbols ($PEPE = 50 tokens) produce `resolved_mint: null`. The kernel judges with on-chain data only when mint is unknown. Ambiguity is tagged for future analysis.
