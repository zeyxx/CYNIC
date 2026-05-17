# Organic Agent — Passive Capture & Human Signal Integration

**Date:** 2026-05-18
**Status:** Draft
**Scope:** Hermes organ-X expansion — capture human interactions passively, use as calibration + trigger + taste profile.

---

## Problem

The organic agent (Hermes) captures tweets from automated searches and passive browsing, but has zero visibility into **what the human actually cares about**. Bookmarks, likes, and viewed tweets from T.'s personal account never reach the pipeline. Without this signal:
- Dogs can't calibrate against human judgment
- The search generator farms blindly (no taste signal)
- Bookmarks on phone are ephemeral — no compounding

## Constraint

**No X API.** All data comes from passive interception of network traffic via mitmproxy. The proxy sees GraphQL responses; it cannot initiate requests to X.

---

## Architecture

### Network Path

```
Phone (iOS/Android)
  └── Tailscale → ubuntu-desktop-titouan (100.74.31.10)
        └── Proxy mitm :8888 (bind Tailscale IP + 127.0.0.1)

Desktop (Chrome profiles)
  └── proxy=127.0.0.1:8888 (existing)

Both → same proxy → same pipeline
```

**Tailscale proxy binding:** The proxy currently binds `127.0.0.1:8888`. Add bind on Tailscale IP to accept phone traffic. Phone configures HTTP proxy to Tailscale IP:8888.

**Security:** Tailscale ACLs ensure only T.'s devices can reach the proxy. No public exposure. mitmproxy cert installed on phone once (standard MITM CA setup).

### Expanded Capture Operations

Current `CAPTURE_OPS`:
```python
{"SearchTimeline", "UserTweets", "TweetDetail",
 "HomeTimeline", "HomeLatestTimeline", "ListLatestTweetsTimeline"}
```

Expanded:
```python
CAPTURE_OPS = {
    # Existing (passive feed — everything T. sees)
    "SearchTimeline", "UserTweets", "TweetDetail",
    "HomeTimeline", "HomeLatestTimeline", "ListLatestTweetsTimeline",
    # Human curation signal (NEW)
    "Bookmarks",              # GET: bulk bookmark page scroll
    "Likes",                  # GET: bulk likes page scroll
    "Followers",              # GET: following list (social graph)
    "Following",              # GET: following list (social graph)
}

# Mutation ops — detected separately (POST requests with these op names)
# ALL mutations captured from Phase 1; consumption is progressive (see below).
MUTATION_OPS = {
    # Positive curation
    "CreateBookmark",         # "I want to revisit this"
    "FavoriteTweet",          # "I approve"
    "CreateRetweet",          # "My network should see this"
    "CreateTweet",            # Reply or quote (check in_reply_to / quoted_tweet_id)
    # Relationship
    "Follow",                 # "Durable interest in this person"
    "Unfollow",               # "Lost interest"
    # Negative curation
    "DeleteBookmark",         # "Not as useful as I thought"
    "UnfavoriteTweet",        # "Changed my mind"
    "BlockUser",              # "Harmful"
    "MuteUser",               # "Too much noise"
    "NotInterested",          # "Algorithm got me wrong"
    "ReportTweet",            # "Violates norms"
    # Reversal
    "DeleteRetweet",          # Undo amplification
    "UnblockUser",            # Reversal
    "UnmuteUser",             # Reversal
}
```

### Account Discrimination

The proxy distinguishes accounts by inspecting the `auth_token` cookie in request headers. Each authenticated session has a unique token.

```python
# Account mapping: auth_token fingerprint → account_id
# Built dynamically: first time a new token is seen, tag as "unknown"
# Manual mapping in accounts.toml:
#   [accounts.personal]
#   auth_fingerprint = "<first-8-chars-of-auth-token>"
```

Fallback: if fingerprint not mapped, tag as `"unknown_account"`. Never discard data for lack of attribution.

### Dataset Schema Extension

Every captured tweet gains optional fields (schema_version 3 → 4):

```json
{
  "human_signals": ["bookmark", "tweet_click", "like"],
  "human_signals_ts": {
    "bookmark": "2026-05-18T10:30:00Z",
    "tweet_click": "2026-05-18T10:29:55Z"
  },
  "interaction_account": "personal | cynic | unknown",
  "capture_context": "bulk_scroll | realtime_action | passive_feed",
  "feed_position": 7
}
```

**`human_signals`** is an **append-only list** per tweet (deduplicated by tweet_id). A single tweet can accumulate multiple signals over time: seen in feed → clicked → liked → bookmarked. Each signal appends; nothing is overwritten.

**Signal types captured (complete graph):**

| Signal | Source | Strength | Consumed from |
|--------|--------|----------|---------------|
| `view` | HomeTimeline position | 0 | Phase 1 |
| `bookmark` | CreateBookmark mutation | +4 | Phase 1 |
| `like` | FavoriteTweet mutation | +3 | Phase 1 |
| `tweet_click` | TweetDetail request | +2 | Phase 3 |
| `profile_visit` | UserTweets request (on author) | +2 | Phase 3 |
| `reply` | CreateTweet (in_reply_to) | +5 | Phase 2 |
| `quote` | CreateTweet (quoted_tweet_id) | +5 | Phase 2 |
| `retweet` | CreateRetweet | +3 | Phase 2 |
| `follow_author` | Follow (on tweet author) | +4 | Phase 2 |
| `link_click` | External URL request via proxy | +3 | Phase 3 |
| `photo_expand` | Media URL request | +1 | Phase 3 |
| `dwell` | Inter-request gap > 3s | +1 | Phase 5 |
| `search_query` | SearchTimeline variables | +4 (intent) | Phase 5 |
| `not_interested` | NotInterested mutation | -2 | Phase 2 |
| `mute` | MuteUser mutation | -3 | Phase 2 |
| `block` | BlockUser mutation | -5 | Phase 2 |
| `unfollow` | Unfollow mutation | -3 | Phase 2 |
| `unbookmark` | DeleteBookmark mutation | -1 | Phase 2 |
| `unlike` | UnfavoriteTweet mutation | -1 | Phase 2 |

**Capture context:**
- `passive_feed` — tweets seen in HomeTimeline/search (T. scrolled past them)
- `realtime_action` — mutation detected (any signal from the graph above)
- `bulk_scroll` — browsing the Bookmarks/Likes page
- `implicit_navigation` — inferred from request patterns (tweet_click, profile_visit)

**`feed_position`**: index of the tweet in the HomeTimeline GraphQL response (0 = top). Captures what the algorithm pushed highest.

### Design Principle: Capture Wide, Consume Narrow

**Phase 1 captures ALL signals** from day one. The proxy detects and stores every mutation and navigation pattern into the dataset. But consumption is progressive:

| Phase | Signals consumed (acted upon) | Signals captured but dormant |
|-------|------------------------------|------------------------------|
| **1** | bookmark, like, view | Everything else accumulates |
| **2** | + reply, quote, RT, follow, block, mute, unfollow | Navigation signals dormant |
| **3** | + tweet_click, profile_visit, link_click, photo_expand | Temporal signals dormant |
| **5** | + feed_position, dwell, search_query | Full graph active |

This ensures no signal is lost while we build consumption paths incrementally. When Phase 3 activates, it has 2+ weeks of dormant data to bootstrap from.

---

## Pipeline Integration

### Ingest Daemon (existing, modified)

The ingest daemon already tails `dataset.jsonl` and POSTs to `/judge` or `/observe`. Changes:

1. Tweets with `human_signals` containing any **Phase 1 consumed signal** (bookmark, like) get **priority routing** to `/judge` regardless of signal_score threshold
2. Additionally POST to `/observe` with `domain=human-signal` and tags from signals list
3. Real-time mutations (bookmark, like in Phase 1; expanded in Phase 2) additionally emit to `domain=human-trigger`
4. Passive feed tweets (`human_signals: ["view"]` only) follow normal routing (signal_score threshold applies)
5. **Signal dedup:** If a tweet_id already exists in the dataset, the new signal is appended to its `human_signals` list (not a new row). The ingest daemon maintains a lightweight tweet_id → row_offset index for append operations.

### K15 Consumer: Calibration Path

New consumer on `domain=human-signal`:

```
For each human-bookmarked tweet that also has a Dog verdict:
  - Compare: Dog verdict vs human action
  - If bookmark + BARK → "tension" (human likes what Dogs reject)
  - If bookmark + HOWL → "alignment" (human and Dogs agree)
  - Store tension/alignment ratio as calibration metric
  - Surface tensions to T. periodically
```

**Consumer output:** calibration data feeds CCM crystal generation. Tensions are high-value crystal material.

### K15 Consumer: Trigger Path

On `domain=human-trigger` (real-time mutations only):

```
When CreateBookmark detected:
  1. Extract tweet content
  2. If contains $CASHTAG → dispatch Helius enrichment task
  3. If contains URL → dispatch browse+extract task
  4. If is thread head → dispatch thread capture task
  5. If author is new (not in dataset) → dispatch author profile capture
```

Tasks dispatched via kernel `/agent-tasks` to `hermes-agent-executor`.

### Search Generator: Taste Profile

After 50+ bookmarks with narratives accumulated:

1. Extract narrative frequency distribution from bookmarked tweets
2. Extract author frequency (who does T. bookmark most)
3. Weight search query generation toward these narratives/authors
4. Score search results against taste profile before presenting
5. Proactive flagging: "similar to your recent bookmarks"

Implementation: cosine similarity on narrative vectors. No ML needed initially.

Taste profile artifact (regenerated each `hermes-feedback-loop` cycle):
```json
{
  "narrative_weights": {"agent": 0.3, "sovereignty": 0.25, "exploit": 0.15},
  "preferred_authors": ["@alice", "@bob"],
  "engagement_pattern": {"min_length": 150, "prefers_threads": true},
  "bookmark_count": 73,
  "updated_at": "2026-05-18T..."
}
```

---

## Proxy Changes (Detail)

### Bind Address

mitmdump doesn't support multiple `--listen-host`. Solution: bind `0.0.0.0` with firewall:

```bash
# Only allow Tailscale subnet (100.64.0.0/10) + localhost
iptables -A INPUT -p tcp --dport 8888 -s 127.0.0.1 -j ACCEPT
iptables -A INPUT -p tcp --dport 8888 -s 100.64.0.0/10 -j ACCEPT
iptables -A INPUT -p tcp --dport 8888 -j DROP
```

Service change:
```ini
ExecStart=... mitmdump -s core/x_proxy.py \
  --listen-host 0.0.0.0 -p 8888 \
  --set "allow_hosts=(?:.*\\.)?x\\.com|(?:.*\\.)?twitter\\.com" ...
```

### Mutation Detection

```python
def request(self, flow: http.HTTPFlow) -> None:
    # Existing: read_only enforcement
    # ...

    # NEW: Detect mutations for human signal capture
    if flow.request.method == "POST" and "/i/api/graphql/" in flow.request.url:
        try:
            body = json.loads(flow.request.content)
            variables = body.get("variables", {})
            # The response will contain the bookmarked/liked tweet data
            flow.metadata["mutation_op"] = self._extract_op_from_body(body)
            flow.metadata["mutation_account"] = self._identify_account(flow)
        except (json.JSONDecodeError, TypeError):
            pass
```

### Account Fingerprinting

```python
def _identify_account(self, flow: http.HTTPFlow) -> str:
    """Identify account from request cookies."""
    cookies = flow.request.cookies
    auth_token = cookies.get("auth_token", "")
    fingerprint = auth_token[:8] if auth_token else ""

    for account_id, config in self._account_config.get("accounts", {}).items():
        if config.get("auth_fingerprint") == fingerprint:
            return account_id

    if fingerprint:
        logger.info("unknown account fingerprint: %s...", fingerprint)
    return "unknown"
```

---

## Phone Setup (One-Time)

1. Install Tailscale on phone (already in tailnet)
2. Install mitmproxy CA cert:
   - Temporarily configure proxy on phone
   - Navigate to `http://mitm.it` → download cert
   - iOS: Settings → General → VPN & Device Management → Install → Trust
   - Android: Settings → Security → Install cert from storage
3. Configure HTTP proxy in Wi-Fi/Tailscale settings: `100.74.31.10:8888`
4. Verify: browse X → check `proxy_metrics.jsonl` shows `responses_seen` incrementing

---

## Phases

### Phase 1 — Passive Capture (~1 session)
- [ ] Proxy bind `0.0.0.0` + iptables Tailscale-only
- [ ] Add `Bookmarks`, `Likes` to `CAPTURE_OPS`
- [ ] Add `MUTATION_OPS` detection in `request()` hook
- [ ] Account fingerprinting (`_identify_account`)
- [ ] Dataset schema v4: `human_interaction`, `interaction_account`, `capture_context`
- [ ] Ingest daemon: priority routing for human-interaction tweets
- [ ] Phone setup + verification
- [ ] Update `proxy_metrics.jsonl` to track human_interaction counts

### Phase 2 — Trigger (~1 session)
- [ ] K15 consumer handles `domain=human-trigger`
- [ ] Dispatch rules: cashtag → Helius, URL → browse, thread → expand
- [ ] Test: bookmark a tweet with $SOL → verify enrichment task dispatched

### Phase 3 — Taste Profile (~after 50+ bookmarks accumulated)
- [ ] Narrative extraction from bookmarked tweets
- [ ] Taste profile artifact generation (JSON, in feedback-loop cycle)
- [ ] Search generator weighted by taste profile
- [ ] Proactive flagging: "similar to your bookmarks"

### Phase 4 — Calibration Loop (~ongoing, after Phase 1 data accumulates)
- [ ] Dog verdict vs human bookmark comparison
- [ ] Tension/alignment metric in `/health`
- [ ] Surface tensions to T. (Slack or dashboard)
- [ ] Feed alignments to CCM as crystal material

---

## Phase 5 — Algorithm Reverse-Engineering

### Source Material

X's open-source algorithm (`xai-org/x-algorithm`, cloned at `/tmp/x-algorithm`) reveals the full scoring architecture but NOT the weight values. The weights are in an external config (`xai_feature_switches::Params`), dynamically tunable.

### What We Know (from code)

**Scoring formula:**
```
Final = normalize(Σ(weight_i × P(action_i))) + offset
```

**19 engagement signals predicted by Phoenix transformer:**

| Signal | Type | Notes |
|--------|------|-------|
| favorite | + | Like probability |
| reply | + | Reply probability |
| retweet | + | RT probability |
| photo_expand | + | Image click probability |
| click | + | Link/tweet click |
| profile_click | + | Author profile visit |
| vqv (video quality view) | + | Conditional on video duration |
| share | + | Generic share |
| share_via_dm | + | Share to DM |
| share_via_copy_link | + | Copy link |
| dwell (binary) | + | Did user pause on tweet? |
| dwell_time (continuous) | + | How long user paused |
| quote | + | Quote tweet probability |
| quoted_click | + | Click on quoted tweet |
| follow_author | + | Follow after seeing tweet |
| not_interested | **-** | "Not interested" flag |
| block_author | **-** | Block action |
| mute_author | **-** | Mute action |
| report | **-** | Report action |

**Post-scoring adjustments:**
- Author diversity: `score × ((1-floor) × decay^position + floor)` — penalizes repeated authors
- OON (out-of-network): multiplicative factor on non-followed content
- Age bucketing: 60-min granularity, max 80h window, overflow bucket beyond

**Model architecture (Phoenix):**
- Transformer with RoPE, GQA (grouped query attention)
- Input: user (1 token) + history (128 tokens) + candidates (32 tokens)
- Candidates CANNOT attend to each other (independent scoring)
- Hash-based embeddings (no explicit feature engineering)
- Two-tower retrieval narrows millions → hundreds, then transformer ranks

### What We Can Reverse-Engineer

Since the architecture is known but weights are hidden, we can **infer approximate weights empirically** from captured data:

**Data we capture passively:**
1. **Feed position** of each tweet in HomeTimeline response (GraphQL returns ordered list)
2. **Tweet features** we already extract (engagement metrics, author stats, content signals)
3. **Post age** (created_at vs impression time)
4. **Author relationship** (in-network vs OON — detectable from following list)
5. **Human reaction** (did T. bookmark/like/ignore this tweet?)

**Regression target:** Position in feed ≈ inverse of algorithm score. Higher position = higher score.

**Method:**
```python
# Pseudo-code for weight inference
for each HomeTimeline response:
    for each tweet at position i:
        features[i] = extract(engagement, author, age, content)
        target[i] = 1.0 / (position + 1)  # Higher position = higher algo score

# After 2+ weeks of data:
model = LinearRegression().fit(features, targets)
# model.coef_ ≈ relative weight of each feature in X's scoring
```

**What this reveals:**
- Which signals X weighs most heavily (favorite vs dwell vs reply)
- How much OON penalty is applied (compare in-network vs OON positions)
- Author diversity decay factor (from repeated-author position drops)
- Age decay curve (from timestamp vs position correlation)

### Integration with Organic Agent

The reverse-engineered weights serve three purposes:

1. **Understanding what X shows T.** — "your feed is 70% optimized for likes, 20% for dwell, 10% for replies" → T. knows the bias

2. **Counter-algorithm farming** — if X deprioritizes content that Dogs score HOWL, the search generator can specifically seek content X would bury. "X won't show you this, but Dogs think it's high signal."

3. **Calibration baseline** — compare algorithm score vs Dog score vs human bookmark. Three-way tension:
   - Algo HIGH + Dog HIGH + Bookmark → perfect alignment (rare)
   - Algo HIGH + Dog LOW + No bookmark → algorithmic noise T. ignores
   - Algo LOW + Dog HIGH + Bookmark → signal X buries, Dogs catch, T. validates

### Phase 5 Tasks

- [ ] Capture feed position (index in `instructions[].entries[]`) per tweet in dataset
- [ ] Detect in-network vs OON from proxy (check if author is in T.'s following)
- [ ] After 2 weeks of data: run linear regression on position ~ features
- [ ] Publish inferred weights as `artifacts/x_algo_weights_inferred.json`
- [ ] Compare: algo ranking vs Dog Q-score → divergence metric
- [ ] Surface counter-algorithmic content in search generator

### Falsification

- **Fails if:** Position in feed is not correlated with features (X uses pure RL/exploration that defies linear approximation). Test: R² < 0.3 after 2 weeks → model is useless, try non-linear.
- **Partially fails if:** X A/B tests aggressively and weights shift weekly → model needs continuous retraining (acceptable, just automate the regression).

---

## Success Criteria

1. **Phase 1:** Bookmarking a tweet on phone → appears in `dataset.jsonl` within 5s, tagged correctly
2. **Phase 2:** Bookmarking `$TOKEN` tweet → Helius enrichment task in `/agent-tasks` within 30s
3. **Phase 3:** Search results ranked by taste > 30% overlap with future bookmarks (2-week test)
4. **Phase 4:** Tension ratio visible; at least 1 crystal/week from calibration data
5. **Phase 5:** Inferred algo weights with R² > 0.3; at least 1 counter-algorithmic signal surfaced per day

## Falsification

- **Phase 1 fails if:** X implements cert pinning on mobile (not current as of 2026-05)
- **Phase 3 fails if:** Bookmark patterns too diverse for similarity (no narrative clusters)
- **Phase 5 fails if:** Feed position uncorrelated with features (R² < 0.3) — try non-linear
- **Proxy approach fails if:** Phone traffic volume overwhelms pipeline (mitigate: rate-limit passive_feed)

---

## Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| X cert pinning on mobile | Low | Desktop-only fallback |
| Phone battery drain | Medium | Monitor; Tailscale is lightweight |
| Auth fingerprint rotation | Low | Re-map on 401; alert T. |
| HomeTimeline volume flood | Medium | Rate-limit `passive_feed` (cap N/min) |
| Personal data in dataset | N/A | Local-only, never in git |

---

## Dependencies

- Tailscale on phone (already in tailnet)
- mitmproxy CA cert on phone (one-time)
- `hermes-proxy.service` bind change + iptables
- `accounts.toml` auth_fingerprint field
- Ingest daemon priority routing
- K15 consumer expansion (Phase 2+)
- `xai-org/x-algorithm` repo (reference for Phoenix/home-mixer architecture, Phase 5)
