# X Ingest Audit — 2026-05-04

## Status: OBSERVED (φ⁻¹ limited confidence)

We capture **what T. could have seen** on X, but not **what T. actually did**.

---

## What We Capture ✓

**Mechanism:** mitmproxy addon (`hermes-proxy.service` + `x_proxy.py`)
- Passive interception of X GraphQL responses
- Runs 24/7 on localhost:8888
- Filters: only X/Twitter traffic decrypted (SoC)

**Data Points (670 captures, 11 days):**

| Page Type | Count | Sample Tweets |
|-----------|-------|---------------|
| HomeTimeline | 156 | 34 per page |
| TweetDetail | 236 | 1-8 (thread context) |
| UserTweets | 116 | 20 per page |
| SearchTimeline | 100 | 6-10 per search |
| Likes | 49 | 20 per page |
| Bookmarks | 13 | 20 per page |

**Per-Tweet Fields:**
```
✓ ID, text, created_at, author handle + name
✓ Author metadata: followers, statuses, verified status, bio
✓ Engagement metrics: likes, retweets, replies, bookmarks, quote count, view count
✓ Entities: cashtags ($TOKEN), hashtags, mentions
✓ Content flags: is_retweet, is_reply, is_self_thread, language
✓ Enrichment: signal_score (-5 to +7), author_tier, narratives, coordination_count
```

**Example enrichment (x_proxy.py):**
```python
signal_score():
  - Spam detection (-3): "100x moon", "airdrop", etc
  - Quality signals (+2): "on-chain data", "smart contract", "analysis"
  - Author tier (+2 whale, -1 bot): followers > 100K = whale
  - Engagement ratio (+2 if >5%, +1 if >2%)
  - Absolute engagement (+1 if >1000 likes)
  - Narratives (+1): "warning" / "hype" / "analysis" / "community" / "launch"
```

---

## What We DON'T Capture ✗

**T.'s Actual Engagement:**
```
✗ viewer_favorited (hardcoded False in x_proxy.py line 320)
✗ viewer_retweeted (hardcoded False)
✗ viewer_bookmarked (hardcoded False)
✗ How long T. spent viewing each tweet
✗ Which tweets T. scrolled past (vs clicked)
✗ T.'s replies/quote tweets (only visible if T. posts them)
```

**T.'s X Activity:**
```
✗ Notifications T. received ("you have 3 new likes")
✗ T.'s own posts (only appears if T. posts during proxy window)
✗ T.'s follows/unfollows
✗ T.'s DMs, conversations
✗ T.'s list operations, mutes, blocks
✗ T.'s settings changes
```

**Behavioral Correlation:**
```
✗ Kill-chain link: Click in CYNIC window → Specific X tweet
  (We have: behavior_log.jsonl timestamps + captures timestamps
   But: No automated correlation which tweet T. clicked on)
```

---

## The Kill-Chain Gap

**Current state (broken):**

```
16:30:45 — behavior_log.jsonl
  ├─ type: "click"
  ├─ window_name: "Ready (CYNIC)"
  └─ [T. clicked in CYNIC, likely reading something]

16:30:50 — HomeTimeline captured
  ├─ operation: "HomeTimeline"
  ├─ 34 tweets extracted
  └─ signal_scores: [-2, 0, 1, 3, 1, ...] (enriched)

17:15:22 — behavior_log.jsonl
  ├─ type: "click"
  ├─ window_name: "Ready (CYNIC)"
  └─ [Another CYNIC window click]

??? — [Which of the 34 tweets did T. actually engage with?]
      [Which captured timeline did T. scroll?]
      [Did T. like any? Bookmark? Click the thread?]
```

**Inference problem:** If we see:
- CYNIC window active 16:30-16:40
- HomeTimeline captured at 16:30:50
- Low signal tweets (scores < 0) in that timeline

Do we assume T. didn't engage? Or that T. was doing something else in CYNIC?

**Answer:** We cannot assume. The correlation is weak (φ⁻¹ = 0.618 max confidence).

---

## What Would Close the Loop

**Option A: X API side (Bidirectional)**
- Fetch T.'s own timeline (what T. interacted with)
- Fetch T.'s like/retweet/bookmark history
- Fetch notifications log
- Requires X Developer API access + OAuth with T.'s account
- **Problem:** Passive (hermes-proxy) doesn't authenticate; would need separate authenticated client
- **Cost:** API quota limits

**Option B: Browser instrumentation (Click tracking)**
- Inject JavaScript into X pages to track which tweets T. clicks
- Log: tweet_id, timestamp, action (like/bookmark/open_thread)
- Send to local endpoint for capture
- **Problem:** X's CSP blocks inline scripts; would need content script (browser extension)
- **Advantage:** Real engagement data, no API quota

**Option C: Behavioral inference (Temporal proximity + ML)**
- Correlate behavior_log clicks/keystrokes with visible tweets
- Train classifier: "Which of these 34 tweets did T. engage with?"
- Features: keyword match (from learned_weights.json), temporal proximity, author tier
- **Problem:** Still inferred, not ground truth
- **Advantage:** Works with current data

---

## Recommendation

**For learning behavioral patterns** (what you asked):

The current capture + learned_weights combo is **sufficient but biased**:

- ✓ We learned: T. clicks on code/architecture/python keywords (14.7% CYNIC clicks)
- ✓ We learned: T. engages 2.7x more selectively than average (scroll/click ratio)
- ✓ We learned: T. is a deep reader (11.1 keystrokes/click)

- ✗ But this is learned from **CYNIC window only**, not from X behavior
- ✗ We assumed: If T. was in CYNIC window during timeline capture, T. cares about similar topics
- ✗ We cannot verify: Did T. actually like/retweet any of the 34 captured tweets?

**For Phase 3 measurement** (test agent learning):

The kill-chain gap means:
- Agent predicts "engage" on 12 tweets
- We run the agent
- We observe: Did T. actually engage on those 12 tweets?
- **Problem:** We don't know unless we add engagement tracking

---

## Architectural Fix (For option 3)

Move learned profile to kernel + close the loop:

```
Behavioral Learning Engine (kernel + Python hybrid)
├── SurrealDB schema
│   ├── behavior_events (from behavior_logger)
│   ├── x_captures (from x_proxy)
│   ├── x_engagement (YOUR X actions — MISSING)
│   └── observation_feedback (agent predictions vs reality)
├── Learning pipeline
│   ├── Feature extraction (keywords, temporal, author)
│   ├── Model training (logistic regression: observed engagement)
│   └── Confidence calibration
└── REST API
    ├── GET /behavior/profile → current model
    ├── POST /behavior/observe → feedback loop
    └── GET /behavior/predict?tweet={...} → engagement probability
```

**Still need to solve:** How to capture T.'s actual X engagement (Option A/B/C above).

---

## Falsifiable Hypothesis

**Current claim (φ⁻¹ confidence):**
"A logistic regression trained on 29K clicks in CYNIC window can predict T.'s engagement with X tweets"

**Falsification:**
If AUC < 0.65 on test set, the keyword-based learning doesn't transfer to X behavior.
(T.'s interests in CYNIC ≠ T.'s interests on X)

**Test required:** 
Run agent, measure actual engagement, compare vs predictions.
Without engagement tracking, we cannot falsify this hypothesis.

