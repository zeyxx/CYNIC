# Domain Discovery Strategy — REVISED (2026-05-03)

## Executive Summary (REVISED)

**Previous approach**: Keyword matching on isolated tweets → bimodal clustering (91.5% / 8.5%)

**Problem identified**: Keywords alone can't capture domain semantics. "gpt" in "general market thread" ≠ "gpt" in "LLM technical discussion."

**Reframe**: Tweets exist in **conversation graphs**. Domains emerge from:
1. **Thread context** (conversation_id + reply chains)
2. **Quote relationships** (who references whom, why)
3. **Semantic content** (what's discussed across the thread)
4. **Author voices** (who drives the discussion)
5. **Narrative patterns** (what mood/intent)

## Data Available (Probed)

```
Data sources (operation types):
├─ HomeTimeline (28.0%) — your actual browsing
├─ TweetDetail (28.0%) — full thread context
├─ UserTweets (22.2%) — author profiles + history
├─ SearchTimeline (11.1%) — keyword searches
└─ Likes (10.7%) — liked tweets

Conversation structure (1,384 unique conversation_ids):
├─ Single-tweet "conversations": 91.6% (isolated observations)
├─ Multi-tweet threads: 8.4%
│   ├─ 2 tweets: 3.1%
│   ├─ 3-5 tweets: 4.6%
│   └─ 6+ tweets: 0.7%
├─ Multi-author threads: 1.4%
├─ Quote connectivity: 19.1% (sparse)
└─ conversation_id coverage: 82.5%

Metadata fields:
├─ interaction_type (100% coverage) ✓
├─ in_reply_to_tweet_id (14.2% coverage) ✓
├─ quoted_tweet (13.3% coverage) ✓
└─ narratives (exists but empty — needs extraction)
```

**Interpretation**: Dataset captures **individual tweets with semantic context** (TweetDetail operations), not full conversation threads. 91.6% of tweets are standalone observations. Hypergraph emerges from **semantic similarity between tweets**, not conversation graphs.

---

## The Real CHAOS→MATRIX Path

### Current (Wrong) Approach
```
6,352 tweets
  ↓
Binary keyword matching (41 keywords)
  ↓
K-means clustering → 2 clusters (91.5% / 8.5%)
  ↓
Problem: Clusters are topic-blind (gpt + btc in same cluster)
```

### Correct Approach (Semantic Clustering on Tweets)
```
6,352 individual tweets
  ↓
TF-IDF vectorization (500D sparse vectors)
  ├─ Captures term frequency (how often appears)
  ├─ Weights by inverse document frequency (rare words matter more)
  ├─ Solves: "gpt mentioned once" ≠ "gpt discussed 50×"
  └─ Output: rich semantic representation per tweet
  ↓
K-means clustering on 6,352 × 500D matrix
  ├─ Reveal natural semantic basins
  ├─ Expected: cleaner separation (general, trading, llm technical, security, etc.)
  └─ Hypergraph: tweets with high similarity to multiple clusters = bridges
  ↓
Domain emerges from cluster properties:
  ├─ Top keywords (semantic fingerprint)
  ├─ Top authors (who drives discussion)
  ├─ Narrative patterns (hype, analysis, warning, launch)
  ├─ Quote targets (what gets referenced)
  └─ Interaction patterns (replies, quotes, retweets)
  ↓
Assign domain to each tweet (via cluster membership + bridge signals)
```

---

## Why TF-IDF Matters (vs Binary Keywords)

### Example: The "token" Ambiguity

**Binary keyword approach** (current):
```
Tweet A: "Tokenization in transformers is fundamental..."
Tweet B: "Token price pumped 50% today"
    ↓
Keyword match: "token" present in both
    ↓
Both get: token_keyword = 1.0
    ↓
Same vector representation (ignores context & frequency)
    ↓
Both assigned to "token domain" (wrong)
```

**TF-IDF approach** (proposed):
```
Tweet A: "Tokenization in transformers attention mechanism..."
  ↓
TF-IDF vector: high weight on {tokenization, transformers, attention, mechanism, training}
                low/zero weight on {price, pump, liquidity, trading}
  ↓
Clusters with other LLM technical tweets
  ↓
Domain: "LLM fundamentals"

Tweet B: "Token price pumped 50% today, whale buy-in?"
  ↓
TF-IDF vector: high weight on {price, pump, trading, whale, liquidity}
                low/zero weight on {tokenization, attention, mechanism}
  ↓
Clusters with other trading/market tweets
  ↓
Domain: "Token market dynamics"
```

**Result**: Same keyword, different semantic representations, correctly disambiguated by term frequency & context.

---

## Implementation Path (Data-Centric, NumPy-first)

### Phase 0: TF-IDF Vectorization (1h)

**Manual implementation (no sklearn needed):**
```python
# 1. Build vocabulary from all tweets (tokenize by word)
vocabulary = {}
for tweet in tweets:
    text_lower = tweet['text'].lower()
    words = text_lower.split()  # naive tokenization
    for word in words:
        if word not in vocabulary:
            vocabulary[word] = len(vocabulary)

# 2. Compute IDF (inverse document frequency)
idf = {}
num_docs = len(tweets)
for word in vocabulary:
    docs_with_word = sum(1 for t in tweets if word in t['text'].lower())
    idf[word] = math.log(num_docs / max(1, docs_with_word))

# 3. Vectorize each tweet
vectors = {}
for tweet_id, tweet in tweets.items():
    text_lower = tweet['text'].lower()
    vec = {}
    for word in text_lower.split():
        if word in vocabulary:
            word_idx = vocabulary[word]
            tf = text_lower.count(word)  # term frequency
            vec[word_idx] = tf * idf[word]  # TF-IDF
    vectors[tweet_id] = vec
```

**Or use CountVectorizer logic manually** (faster approach):
- Count word frequency per tweet (TF)
- Weight by log(N / doc_frequency) (IDF)
- Normalize by L2 (cosine similarity ready)

### Phase A: K-means Clustering (1h)
```python
# Use existing cosine_similarity + simple_kmeans from numpy version
# But now on TF-IDF vectors (500D+) instead of binary keywords (41D)

tweet_ids = list(vectors.keys())
clusters, assignments = simple_kmeans(
    tweet_ids=tweet_ids,
    vectors=vectors,
    click_weights=click_weights,
    n_clusters=7,
    max_iter=15
)

# Measure silhouette score (cluster quality)
silhouette = measure_silhouette(vectors, assignments)
```

### Phase B: Domain Definition (1h)
For each cluster, extract:
```python
domain = {
    "cluster_id": 0,
    "size": 1200,  # tweets in this cluster
    "silhouette_score": 0.52,
    "semantic_keywords": ["token", "price", "liquidity", "exchange"],
    "top_authors": [
        {"name": "gcrtrd", "tweets": 145, "narratives": ["analysis", "warning"]},
        {"name": "tradingbot", "tweets": 92, "narratives": ["hype"]},
    ],
    "narrative_fingerprint": {
        "hype": 0.12,
        "analysis": 0.38,
        "warning": 0.25,
        "launch": 0.18,
        "other": 0.07
    },
    "bridge_tweets": 87  # tweets with high sim to other clusters
}
```

### Phase C: Bridge Detection (1h)
Identify tweets at cluster boundaries:
```python
# For each tweet, measure similarity to ALL cluster centroids
# If max_sim_to_own_cluster < threshold AND sim_to_other > threshold:
#   → Tweet is a bridge (multi-domain)

bridges = {}
for tweet_id, vec in vectors.items():
    own_cluster = assignments[tweet_id]
    own_sim = cosine_similarity(vec, centroids[own_cluster])
    
    for other_cluster in centroids:
        if other_cluster != own_cluster:
            other_sim = cosine_similarity(vec, centroids[other_cluster])
            if other_sim > 0.3 and own_sim < 0.7:  # tunable threshold
                bridges[tweet_id] = (own_cluster, other_cluster, other_sim)
```

### Phase D: Validation (1h)
1. **Silhouette score > 0.5** (cluster cohesion)
2. **Manual inspection** (read top keywords + example tweets per cluster)
3. **Cross-ref with your clicks** (do clicked tweets cluster together?)

### Phase 1: K15 Consumer (2h) — After validation
```rust
// Route observations by conversation domain
match observation.conversation_domain {
    Domain::TradingMarkets => route_to_human(),
    Domain::LLMTechnical => route_to_organ_x("specialist_branch"),
    Domain::Security => route_to_human_with_alert(),
    Domain::Bridge => route_to_feedback_loop(),
}
```

---

## Falsification Tests (Revised)

**F0: TF-IDF improvement over binary keywords** ✓
- Compare silhouette on TF-IDF vectors vs binary keyword vectors
- Target: silhouette(TF-IDF) > silhouette(binary) by ≥0.15
- Falsify if: TF-IDF performs worse (abandons approach, use sklearn TF-IDF)

**F1: Cluster quality**
- Silhouette score > 0.5 (good cluster cohesion)
- Manual inspection: can you describe each cluster in 1-2 sentences?
- Falsify if: silhouette < 0.4 OR clusters still mix (gpt + btc together)

**F2: Semantic coherence (keywords per cluster)**
- Do top keywords within a cluster relate to each other?
- Example: Cluster 0 should NOT have {token, price, gpt, attention} mixed
- Falsify if: top 10 keywords are random/incoherent across cluster

**F3: Author-narrative alignment**
- For tweets you click on: do they cluster with other liked tweets?
- Example: if you click @gcrtrd's analysis tweets, do they share a cluster?
- Falsify if: your clicks scattered across 4+ different clusters

**F4: Bridge signal strength**
- 5-15% of tweets are bridges (multi-domain connections)
- Example: 300-900 bridge tweets between clusters
- Falsify if: <1% bridges (domains are too separate) OR >30% (no clear boundaries)

**F5: Signal yield (Phase 3, 7 cycles)**
- Cluster-routed observations yield > baseline (4.2 avg signal)
- Target: > 5.0 avg signal per cluster
- Falsify if: improvement < 2% (clustering isn't the bottleneck)

---

## Why This Works

| Old | New |
|-----|-----|
| Isolated tweets | Conversation context |
| 41 keywords | Full semantic content |
| Binary vectors | 384D embeddings |
| "gpt appears" | "LLM discussion arc" |
| 6K clusters → 2 clusters | 2K conversations → 5-7 coherent domains |

**Key insight**: Authors don't discuss in isolation. They build on each other's ideas, quote sources, evolve arguments. The **domain emerges from the graph, not keywords**.

---

## Timeline

- **Now (1h)**: Implement TF-IDF vectorization (NumPy-only, no deps)
- **+1h**: Re-cluster with TF-IDF vectors (vs binary keywords)
- **+1h**: Measure silhouette improvement (validate F0)
- **+1h**: Extract domains + bridge detection
- **+1h**: Manual validation (inspect clusters, cross-ref clicks)
- **Day 2 onwards**: K15 Phase 1 implementation (if F0-F3 pass)

---

## Next Action (Now)

Implement TF-IDF vectorization in NumPy:

```python
# 1. Build vocabulary from all tweets
# 2. Compute IDF for each word
# 3. Vectorize each tweet (TF × IDF)
# 4. Replace binary vectors in k-means
# 5. Measure silhouette: does TF-IDF improve over binary?
```

Then re-run clustering pipeline with enriched vectors.
