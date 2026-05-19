# Outcome Feedback Loop — Design Spec

> "A verdict without outcome measurement is an opinion. A verdict with outcome measurement is data."

## Problem

CYNIC judges tokens but never learns if the judgment was correct. The pipeline ends at:
```
token → enrich → judge → verdict → crystal
```

The crystal says "this token was scored Growl because X." It never says "...and 30 days later the token pumped 80%" or "...and 30 days later it died."

Without outcome feedback, the compound loop (CCM) cannot learn to predict. It accumulates judgment patterns but not judgment ACCURACY.

## Design

### New pipeline stage: Outcome Collection

```
Existing:  token → enrich → judge → verdict → crystal
                                                  ↓
New:                                        wait 7/14/30d
                                                  ↓
                                        re-query market data
                                                  ↓
                                        compute outcome metrics
                                                  ↓
                                        enrich crystal with outcome
                                                  ↓
                                        Dogs see outcome-enriched crystals
```

### Outcome Metrics (per token, per time window)

Collected at T+7, T+14, T+30 after judgment:

| Metric | Source | What it measures |
|---|---|---|
| price_change | GeckoTerminal OHLCV | Market performance |
| volume_change | GeckoTerminal OHLCV | Trading activity trend |
| holder_change | Sovereign holder snapshots | Community growth/decay |
| conviction_change | Sovereign conviction computation | Loyalty trend |
| alive | GeckoTerminal (volume > $100 in last 7d) | Survivorship |

### Outcome Label (derived)

```
THRIVING:  alive AND price_change > +20% AND holder_change > 0
STABLE:    alive AND abs(price_change) <= 20%
DECLINING: alive AND price_change < -20%
DEAD:      NOT alive (volume < $100 for 7 consecutive days)
```

### Implementation: `outcome_collector.py`

**Tier 2 INFRASTRUCTURE** (not experimental — this is the feedback loop that makes CCM work).

```
cynic-python/heuristics/collection/outcome_collector.py
```

**Triggered by:** daily cron (systemd timer), same schedule as token-snapshot.

**Logic:**
```python
1. Query kernel: GET /verdicts?min_age_days=7&max_age_days=60&no_outcome=true
   → tokens judged 7-60 days ago that don't have outcome data yet

2. For each token needing outcome at T+7 (judged 7-9 days ago):
   a. GeckoTerminal: get current price, volume, liquidity
   b. Compare with price_at_judgment (stored in verdict or market_snapshot)
   c. Sovereign snapshots: compute holder_change, conviction_change
   d. Classify outcome: THRIVING / STABLE / DECLINING / DEAD

3. Same for T+14 and T+30 windows

4. POST enriched observation to kernel:
   POST /observe {
     tool: "outcome_collector",
     target: verdict_id,
     domain: "token-analysis",
     context: {
       "verdict_id": "...",
       "mint": "...",
       "judgment_date": "2026-05-19",
       "outcome_window": "30d",
       "verdict_at_judgment": "Growl",
       "q_score_at_judgment": 0.45,
       "outcome_label": "THRIVING",
       "price_change": +0.80,
       "volume_change": -0.15,
       "holder_change": +0.05,
       "conviction_change": -0.02,
       "alive": true
     },
     tags: ["outcome", "feedback-loop", "30d"]
   }
```

### Crystal Enrichment

When the nightshift processes outcome observations, it should create or update crystals:

```
Domain: token-analysis
Key: "outcome pattern"
Content: "Tokens judged Growl with supply_burned>20% and holders>1000 
         had outcome: 60% THRIVING, 30% STABLE, 10% DECLINING at T+30"
```

This is the signal that turns individual judgments into accumulated wisdom.

### Stimulus Enhancement

When building token stimulus, if outcome-enriched crystals exist for similar tokens:

```
[HISTORICAL OUTCOMES]
Similar tokens judged in the last 90 days:
  5 similar profiles → 3 THRIVING, 1 STABLE, 1 DEAD
  Average price change: +45% at T+30
  Note: historical pattern, not prediction.
```

This gives Dogs empirical context — not "this axiom should be high because X" but "tokens like this historically did Y."

### Dependencies

1. **Market snapshot at judgment time** — already solved (GeckoTerminal snapshot today)
2. **Verdict storage with token_data** — already exists in kernel (verdict includes token_data)
3. **GeckoTerminal re-query** — same API, just at T+30 instead of T0
4. **Sovereign holder snapshots** — daily cron already accumulating
5. **Crystal enrichment in nightshift** — needs a new handler for "outcome" tagged observations

### What Needs to Be Built

| Component | Effort | Priority |
|---|---|---|
| `outcome_collector.py` — daily cron, re-queries market data | Medium (1-2h) | P0 |
| `outcome_collector.service` + `.timer` — systemd wiring | Small (15min) | P0 |
| Kernel: store `price_at_judgment` in verdict (from DexScreener/GeckoTerminal) | Small — already in token_data | P0 |
| Nightshift handler for outcome observations → crystal enrichment | Medium (1h) | P1 |
| Stimulus builder: [HISTORICAL OUTCOMES] section from outcome crystals | Medium (1h) | P1 |
| GeckoTerminal daily cron for market snapshots (not just holder snapshots) | Small (30min) | P0 |

### Measurement

After 30 days of outcome collection (n~30 tokens with T+30 outcomes):
- Correlate verdict q_score with outcome_label ordinal
- Correlate per-axiom scores with each outcome axis
- If rho(q_score, outcome) > 0.3 → feedback loop adds value
- If rho < 0.1 → Dogs don't discriminate outcomes at current maturity

After 6 months (n~500):
- Train decision tree on (enriched_features → outcome_label)
- Compare: tree with outcome crystals vs tree without
- Delta = compound loop value

### Timeline

- **Now:** Market snapshot T0 posed (done). Daily cron for holder snapshots (running).
- **This week:** Add GeckoTerminal to daily cron (market snapshot daily).
- **This week:** Build outcome_collector.py (queries tokens judged 30+ days ago).
- **T+30 (2026-06-19):** First outcome measurements for today's tokens.
- **T+60 (2026-07-19):** Enough data for first compound loop measurement.

### Falsification

If after 6 months:
- rho(Dogs with outcome crystals, future outcome) ≤ rho(Dogs without outcome crystals, future outcome) → feedback loop doesn't help
- Dogs predict price no better than random regardless of accumulated crystals → compound loop thesis refuted for price
- Dogs predict vitalité at rho > 0.4 but not price → CYNIC is a health detector, not a price oracle (which is fine)
