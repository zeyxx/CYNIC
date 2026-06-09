# Hermes Next Steps — Unblocking Phase 3 Measurement

## Current State (2026-05-04 02:30 UTC)

**Foundation is complete:**
- ✓ Kill-chain tracer (hermes_killchain_tracer.py): Maps 29K clicks to tweets
- ✓ Learned weights (learned_weights.json): T.'s behavior patterns extracted
- ✓ Organic agent (hermes_organic_agent.py): Ready to browse and reason
- ✓ Coverage analysis (KILLCHAIN_ANALYSIS.md): Explains why temporal correlation is weak

**Bottleneck identified:**
- ✗ T.'s actual X.com engagement data (likes, bookmarks, replies)
- ✗ Cannot measure agent learning quality without ground truth

---

## The Measurement Problem

### What We Want to Test

**Hypothesis:** "Agent learned from T.'s CYNIC behavior predicts X.com engagement"

**Setup:**
1. Agent browses X.com, reasons about tweets, predicts engagement probability
2. T. naturally browses X.com, engages with some tweets (likes, bookmarks, etc.)
3. Compare: Agent's predictions vs T.'s actual engagement
4. Measure precision: (correct predictions) / (total predictions)

**Target:** Precision > 60% (baseline random: ~15%)

### The Missing Data

**We have:**
- Agent predictions (hermes_observations.jsonl)
- T.'s behavioral profile (learned_weights.json)

**We don't have:**
- T.'s actual likes on X.com
- T.'s actual bookmarks on X.com
- T.'s actual replies on X.com
- T.'s scroll patterns on X.com (which tweets T. actually saw)

**Why this matters:**
Without ground truth, we can't measure if learning works. The agent could predict
perfectly and we'd still fail Phase 3 if we can't validate against T.'s real behavior.

---

## Three Options to Unblock

### Option A: Browser Extension (Recommended, 4h setup)

**What it does:**
- Intercepts X.com engagement (like clicks, bookmark clicks, reply opens)
- Logs to local JSON file with tweet_id, timestamp, action_type

**Advantages:**
- Real engagement data, no API limits
- Works with T.'s existing browsing (no changes to workflow)
- Can log additional signals (hover time, scroll position)

**Disadvantages:**
- Requires Chrome/Firefox extension development
- Need to handle edge cases (retweets, quote tweets)

**Implementation:**
```javascript
// Add to content script for x.com domain
document.addEventListener('click', (e) => {
  if (e.target.matches('[aria-label*="Like"]')) {
    // Log engagement
    const tweetId = getTweetIdFromDOM(e.target);
    fetch('http://localhost:8888/log-engagement', {
      method: 'POST',
      body: JSON.stringify({
        tweet_id: tweetId,
        action: 'like',
        timestamp: new Date().toISOString()
      })
    });
  }
  // Similar for bookmark, reply, retweet
});
```

**Timeline:** 3-4h to build, 1h testing

### Option B: X API (Moderate complexity, 2-3h setup)

**What it does:**
- Fetch T.'s like/bookmark/reply history via Twitter API
- Correlate with captured tweets by tweet_id

**Advantages:**
- Official data source, complete history
- Can fetch additional metadata (engagement counts over time)

**Disadvantages:**
- Requires T.'s OAuth consent
- Rate-limited API calls
- Historical data only (not real-time during browsing)
- Can't capture scroll patterns

**Implementation:**
```python
# Fetch T.'s likes from last 7 days
liked_tweets = await client.get_liked_tweets(
    id=user_id,
    max_results=100,
    tweet_fields=['created_at', 'public_metrics']
)

# Match against killchain.jsonl
for link in killchain:
    for tweet in link['top_tweets']:
        if tweet['tweet_id'] in [t.id for t in liked_tweets]:
            # Agent predicted this, T. actually liked it — TRUE POSITIVE
            record_tp(tweet['tweet_id'])
```

**Timeline:** 1-2h to authenticate, 1h to correlate

### Option C: Manual Logging (Simple, 1h setup)

**What it does:**
- T. manually logs tweets engaged with in a simple form or paste URL

**Advantages:**
- Zero technical complexity
- T. can filter/reflect on what was truly engaging

**Disadvantages:**
- Incomplete (T. will forget some)
- Biased (only deliberate engagement, not accidental scrolls)
- Limited to one test session

**Implementation:**
- Simple text file or spreadsheet
- T. adds tweet URLs when engaging
- Script extracts tweet IDs and matches against killchain

**Timeline:** <1h setup

---

## Recommendation: Start with Option A

**Why:** Engagement tracking without changing T.'s workflow; captures real behavior.

**Immediate steps:**
1. **Build minimal extension** (toggle X.com engagement logging)
   - Content script intercepts like/bookmark/reply clicks
   - Sends to local server at localhost:8888 (mitmproxy addon)
   - Logs to ~/.cynic/organs/hermes/x/engagement.jsonl

2. **Run test for 1 day:**
   - T. browses X.com normally
   - Extension logs engagement silently
   - Agent makes predictions on same tweets (or parallel session)

3. **Correlate results:**
   - Match agent predictions vs actual engagement
   - Compute precision, recall, F1
   - Document findings

4. **Decide**: If precision > 60%, learning works. If precision < 50%, revisit agent design.

---

## Fallback: Use T.'s Actual X.com Window Clicks

**Discovery from kill-chain analysis:**
- 6 clicks in X.com windows (April 29 - May 4)
- These are the only ground truth we have

**Approach:**
1. Find captures within ±30 min of X.com clicks
2. Extract visible tweets
3. Assume: Tweets visible during click are candidates for engagement
4. Score agent on precision (did it predict the visible high-signal tweets?)

**Quality:** Poor (assumption that visible ≠ engaged)
**Timeline:** 1h to implement, but low confidence in results

---

## Git Status

Three branches with Hermes work:
- `fix/ci-auto-delete-branch-2026-05-02` (original, stale)
- `docs/hermes-roadmap-2026-05-04` (latest, ready to merge)
- Check with `git branch -a`

**Status to push:**
1. Push `docs/hermes-roadmap-2026-05-04` → create PR
2. Keep `killchain.jsonl` in artifacts (don't commit, large file)
3. Next session: Create `feat/hermes-engagement-tracking-*` for extension work

---

## Success Criteria for Phase 3

| Metric | Target | Success |
|--------|--------|---------|
| Engagement data collected | 50+ samples | TBD (depends on option) |
| Agent predictions | 100+ observations | Ready (organic_agent.py works) |
| Precision (agent vs T.'s actual engagement) | > 60% | Falsifiable |
| Recall (T.'s engaged tweets / agent found) | > 50% | Bonus (hard to measure) |
| vs baseline (random tweets) | Agent > baseline by >20% | Strong evidence of learning |

---

## Timeline to Unblock

**Option A (extension):** 4-5h work
- Build extension (3h)
- Test and refine (1h)
- Run 1-day measurement (passive)
- Analyze results (1h)

**Option B (X API):** 2-3h work
- OAuth setup (1h)
- Correlation script (1h)
- Analysis (1h)

**Option C (manual):** <1h
- But incomplete data

**Recommendation:** Option A + Option B in parallel. Start extension, begin API auth concurrently.

**Earliest Phase 3 results:** 2026-05-05 (tomorrow, if started today)

---

## Final Note

The organic agent and kill-chain foundation are **solid**. The kill-chain analysis explains
why temporal correlation is weak (CYNIC work ≠ X.com browsing), which is **not** a bug — it's
the correct diagnosis. The agent uses learned domain patterns instead of temporal proximity,
which is the right approach.

The measurement bottleneck is not architectural; it's operational. We just need T.'s engagement
data to validate the hypothesis. Once we have that, Phase 3 is straightforward.

Next session: Pick engagement tracking method and implement it.
