# Hermes Organic Agent — Learning Roadmap

## Progress (2026-05-04)

**✓ Phase 1: COMPLETE** (extracted learned_weights.json)
- Keywords: code (14.7%), architecture (10.3%), python (8.8%), rust (7.4%), api (5.9%), algorithm (5.2%)
- Temporal: Peak hours at 7h (5.99%), 22-23h (4.5%)
- Depth: 11.1 keystrokes/click (deep reader), 2.7 scrolls/click (selective)

**✓ Phase 2: COMPLETE** (hermes_organic_agent.py uses learned_weights.json)
- Navigates X.com feed with scrolling
- Scores tweets using learned keyword weights + temporal peaks + author tier
- Acts on decisions (read_thread, like, visit_author)
- Stores observations to hermes_observations.jsonl

**✓ Kill-Chain Foundation: COMPLETE** (hermes_killchain_tracer.py maps clicks to tweets)
- 29,327 clicks analyzed
- 1,293 matches found (4.4% coverage with ±5s window)
- Coverage is low but expected: behavior_log is CYNIC work, not X.com browsing
- Only 6 clicks (0.02%) in X.com windows — organic agent doesn't rely on temporal correlation

## What's Needed (Phase 3 Blocker)

### Phase 3: Measure Learning Quality (BLOCKED)
**Blocker:** T.'s actual X.com engagement data (likes, bookmarks, replies)

**What we need:**
1. **Ground truth for agent predictions** — When agent predicts "engage", does T. actually engage?
   - Required: T.'s actual likes/bookmarks/replies on X.com
   - Source: Browser extension, X API, or manual logging
   - Cost: Enable engagement tracking (2h setup)

2. **Test setup** — Run agent and measure against baseline
   - Agent observes: "predict engagement on N tweets"
   - Compare: precision (% predicted tweets T. engages with)
   - Target: > 60% precision (baseline: ~15% random)

### Phase 2 Details (Reference Only)
Update `HermesOrganicAgent.reason_about_tweet()` to use learned weights:

```python
def reason_about_tweet(self, tweet: dict) -> BehavioralDecision:
    signals = []
    score = 0.5

    # Use LEARNED weights instead of hardcoded
    for keyword, weight in self.profile["learned_keywords"].items():
        if keyword in tweet["text"].lower():
            score += weight
            signals.append(f"keyword:{keyword}")

    # LEARNED temporal signal
    hour = tweet["hour"]
    if hour in self.profile["learned_peak_hours"]:
        score += 0.2
        signals.append(f"peak_hour:{hour}")

    # LEARNED author preference
    if tweet["author"] in self.profile["followed_authors"]:
        score += 0.3
        signals.append("known_author")

    # Confidence = how well profile predicts this decision
    confidence = self.calculate_confidence(tweet)
    
    # Decision
    if score > 0.7: decision = "read_thread"
    elif score > 0.55: decision = "engage"
    else: decision = "scroll"

    return BehavioralDecision(...)
```

### Phase 3: Measure Learning Quality (18h)
**Hypothesis:** Agent learned from T.'s behavior will find higher-signal tweets than random search.

Metrics:
1. **Precision:** Of tweets agent said "engage", how many actually match T.'s interests?
   - Measure: Ask T. "Did you find this interesting?" (ground truth)
   - Target: > 60% precision on engaged tweets

2. **Recall:** Of tweets T. would engage with, how many did agent catch?
   - Measure: Compare agent observations vs T.'s actual behavior during test
   - Target: > 50% recall

3. **Contrast:** Agent vs baseline (keyword search)
   - Baseline: search_executor.py returns 6-10 tweets per search
   - Agent: navigates organically, reasons about each
   - Measure: Signal quality (how many observations lead to learning?)

### Phase 4: Feedback Loop (closed loop learning, 48h+)
Agent observes T.'s actual behavior during test window:
- Agent: "This tweet should engage" → stores decision
- T. (human): Naturally browses, may or may not engage
- Compare: Agent prediction vs reality
- Learn: Update weights based on feedback

Pattern: `observation → T's actual behavior → match/mismatch → refine weights`

## Implementation Sequence (Updated 2026-05-04)

**Day 1 (May 4):**
- [x] Build agent foundation + behavioral profiler
- [x] Extract learning data from behavior_log + kill-chain
- [x] Implement keyword weight learning
- [x] Kill-chain tracer: Map clicks to tweets, measure coverage
- [x] Document gap: Temporal correlation is weak, but domain correlation works

**Day 2 (Ready when blocked issue resolved):**
- [x] Add temporal signal learning (peak hours)
- [x] Refactor `reason_about_tweet()` to use learned weights
- [ ] Run agent for 10 cycles, collect observations
- [ ] **BLOCKED:** Need T.'s actual X.com engagement data to measure quality

**Phase 3 (Measurement):**
- [ ] Enable engagement tracking on X.com (browser extension or API)
- [ ] Run agent live, compare predictions vs actual engagement
- [ ] Measure precision/recall
- [ ] vs baseline (keyword search) comparison
- [ ] Document findings

**Why we're blocked:**
The agent is ready to run. The kill-chain analysis shows why temporal correlation doesn't work
(CYNIC clicks ≠ X.com browsing). But we can't measure learning quality without ground truth:
T.'s actual engagement on X.com (likes, bookmarks, etc.). This requires enabling engagement
tracking, which is a separate project (browser extension or X API integration).

## Falsifiability

**H1:** "Agent learning from behavior finds higher-signal content than keyword search"
- **Test:** Run agent 3 cycles (organic) vs search_executor 3 cycles (keyword)
- **Measure:** Signal quality = observations T. finds useful / total observations
- **Threshold:** Agent > baseline by >20%

**H2:** "T.'s typing patterns (11.1 keystrokes/click) predict deep engagement"
- **Test:** Tweets where agent predicts engagement should have >5 responses from T.
- **Measure:** Engagement depth = responses + likes / observations
- **Threshold:** >50% of engaged observations have follow-up action

**H3:** "Temporal signals improve reasoning"
- **Test:** Remove temporal weighting, compare precision
- **Measure:** Precision with temporal / without temporal
- **Threshold:** >10% improvement

## Why This Matters

Current broken approach: Mechanical search loop.
- X.com blocks headless → 0 results
- Searches are random, not driven by T.'s interests
- No learning, no feedback
- **Outcome:** Hollow A/B test measuring nothing

New approach: Behavioral reasoning agent.
- Real browsing (T.'s behavior guides navigation)
- Learning from patterns (keystrokes, scrolls, clicks = signals)
- Feedback loop (agent prediction vs reality)
- **Outcome:** Real measurement of "can an agent learn to find signal?"

This is NOT automating T. This is building something that **thinks** about what T. cares about, then amplifies signal by navigating with that intent.
