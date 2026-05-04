# Hermes Organic Agent — Learning Roadmap

## What We Have (✓ DONE)
- Behavioral profiler: Extracts T.'s patterns from 761K events
- Organic agent: Navigates X.com feed, reasons about tweets
- Real Chrome: Connects via CDP (not headless)
- Observation storage: Records decisions + confidence + signals

## What's Needed (48-72h work)

### Phase 1: Learn from Behavior (24h)
**Goal:** Extract reasoning weights from behavior_log.jsonl

Current naive heuristic:
```
if "code" in text: score += 0.15
if len(text) > 200: score += 0.10
decision = "engage" if score > 0.65 else "scroll"
```

**What we should learn:**
1. **Content signals** — What keywords appear in tweets T. clicks on vs scrolls past?
   - Extract tweets from kill-chain (clicks → captured URLs → tweet IDs)
   - Measure: "Which keywords → T. engagement?"
   - Output: weight_dict = {keyword: engagement_rate}

2. **Temporal signals** — When does T. browse each domain?
   - Peak hours for general content: 21h (T. does personal browsing then)
   - Peak hours for tech: might be different (work breaks)
   - Output: domain_hours = {domain: [peak_hours]}

3. **Depth patterns** — How much does T. read before deciding?
   - 11.1 keystrokes/click = T. reads responses/threads
   - Scroll-to-click = 2.7 = selective, not reactive
   - Output: engagement_threshold = measure how long before T. clicks

4. **Author signals** — Does T. follow/engage with specific authors?
   - Track which authors T. clicks from behavior_log
   - Output: followed_authors = [list]

### Phase 2: Implement Learned Reasoning (24h)
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

## Implementation Sequence

**Day 1 (Today):**
- [x] Build agent foundation + behavioral profiler
- [ ] Extract learning data from behavior_log + kill-chain
- [ ] Implement keyword weight learning

**Day 2:**
- [ ] Add temporal signal learning (peak hours per domain)
- [ ] Add author signal learning (followed authors)
- [ ] Refactor `reason_about_tweet()` to use learned weights
- [ ] Run agent for 10 cycles, collect observations

**Day 3:**
- [ ] Compare agent observations vs T.'s actual engagement
- [ ] Measure precision/recall
- [ ] vs baseline (keyword search) comparison
- [ ] Document findings

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
