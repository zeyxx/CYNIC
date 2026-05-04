# Behavioral Analytics Architecture — Data-Centric Design

## Problem

Current state: Write a new analysis script for each question.
- `domain_discovery_v1.py` — clustering
- `qwen3_embedding_validation.py` — embedding quality
- `improved_behavior_detection.py` — session structure
- Next: domain routing analysis, temporal patterns, etc.

**Root cause:** No unified data model. Each script reimplements load/transform/analyze.

## Solution: Data-Centric Pipeline

```
Raw Events (behavior_log.jsonl)
        ↓
[Ingestion Layer]
        ↓
Normalized Event Store (BehaviorEvent + metadata)
        ↓
[Analysis Layer] — Pluggable analyzers
├─ SessionAnalyzer
├─ DomainPreferenceAnalyzer
├─ SequentialPatternAnalyzer
├─ TemporalAnalyzer
└─ (custom analyzers)
        ↓
[Output Store] — Queryable insights
├─ behavioral_profile.json (aggregate)
├─ session_log.jsonl (sessions with metadata)
├─ domain_preferences.json (per-domain signal)
└─ pattern_library.json (detected patterns)
        ↓
[Consumer Layer] — K15 clients
├─ organ-x domain router
├─ Soma layer 1 (adaptive thresholds)
├─ crystal enrichment (behavioral context)
└─ (future: RL agent)
```

## Data Model

### Raw Input: BehaviorEvent
```json
{
  "ts": "2026-05-03T21:30:45.123456",
  "type": "click|scroll|hover|capture|search",
  "tweet_id": "123456789",
  "x": 512,
  "y": 240,
  "duration_ms": 2500,
  "context": {
    "capture_file": "...",
    "timeline_type": "home|search|user|detail",
    "position_in_timeline": 5,
    "visible_tweet_count": 32
  }
}
```

### Normalized Store: BehaviorAggregate (daily)
```json
{
  "date": "2026-05-03",
  "summary": {
    "total_clicks": 1847,
    "total_scroll_distance": 45280,
    "sessions": 12,
    "peak_hour": 21,
    "avg_click_interval_ms": 450
  },
  "sessions": [
    {
      "id": "s_20260503_213000",
      "start_ts": "2026-05-03T21:30:00",
      "end_ts": "2026-05-03T21:47:30",
      "duration_sec": 1050,
      "clicks": 127,
      "domains": {"ai": 62, "token": 38, "general": 27},
      "entropy": 0.88,  # domain diversity (0=pure, 1=uniform)
      "dominant_domain": "ai"
    }
  ],
  "domain_distribution": {
    "ai": {"clicks": 1124, "pct": 60.9},
    "token": {"clicks": 412, "pct": 22.3},
    "general": {"clicks": 311, "pct": 16.8}
  },
  "temporal": {
    "active_hours": [16, 17, 21, 22],
    "click_bursts": 1823,  # clicks < 10s apart
    "burst_pct": 98.7
  }
}
```

### Output: Pluggable Insights
```json
{
  "behavioral_profile.json": {
    "created": "2026-05-03",
    "summary": "Exploratory reader, broad domains, burst-pattern (rapid scrolling)",
    "engagement": {
      "type": "scan_and_sample",
      "avg_time_per_tweet_ms": 3200,
      "re_engagement_rate": 0.12,
      "click_clustering": "dispersed"
    },
    "preferences": {
      "primary_domain": "ai",
      "secondary_domain": "token",
      "domain_switching_pattern": "rapid"
    },
    "temporal": {
      "peak_hours": [19, 20, 21, 22],
      "peak_day": "friday",
      "inactivity_threshold_hours": 6
    }
  }
}
```

## Architecture Components

### 1. Ingestion (autonomous, runs hourly)

**Input:** `~/.cynic/organs/hermes/behavior/behavior_log.jsonl` (append-only)

**Process:**
1. Read new events since last run
2. Parse and normalize (type coercion, timezone, missing data)
3. Enrich with tweet metadata (domain keywords, author, text)
4. Append to `normalized_events.jsonl` with processing timestamp

**Output:** `normalized_events.jsonl` (deduplicated, enriched)

**Cron:** `@hourly` or triggered by behavior_log size > threshold

---

### 2. Analysis Layer (pluggable)

Each analyzer is a class that consumes `BehaviorAggregate` and produces insights.

**Interface:**
```python
class BehaviorAnalyzer(ABC):
    @abstractmethod
    def analyze(self, aggregate: BehaviorAggregate) -> Dict:
        """Consume aggregate, return insights."""
        pass
    
    @property
    def output_key(self) -> str:
        """Unique key for this insight (e.g., 'session_patterns')."""
        pass
```

**Built-in Analyzers:**

1. **SessionAnalyzer** → `session_log.jsonl`
   - Detect session boundaries (time + domain coherence)
   - Compute session entropy, dominant domain, duration
   - Output: list of `SessionRecord`

2. **DomainPreferenceAnalyzer** → `domain_preferences.json`
   - Aggregate clicks by domain (keyword-based classification)
   - Compute preference score per domain
   - Detect domain-switching patterns
   - Output: `{"domain": "ai", "clicks": 1124, "pct": 60.9, "trend": "stable"}`

3. **TemporalAnalyzer** → `temporal_patterns.json`
   - Hour-of-day distribution
   - Day-of-week patterns
   - Burst detection (inter-click intervals)
   - Output: `{"peak_hours": [19,20,21,22], "burst_pct": 98.7}`

4. **SequentialPatternAnalyzer** → `click_sequences.json`
   - Measure consecutive click similarity (same domain, keywords)
   - Detect chains (A→B→C patterns)
   - Output: transition matrix + chain frequency

5. **CoherenceAnalyzer** → `session_coherence.json`
   - For each session, measure semantic coherence
   - Compare to random baseline
   - Output: coherence scores per session

---

### 3. Output Store

**Location:** `~/.cynic/organs/hermes/behavioral_insights/`

**Files:**
- `behavioral_profile.json` — Summary of current user state (human-readable)
- `session_log.jsonl` — All sessions with metadata
- `domain_preferences.json` — Current domain distribution + trends
- `temporal_patterns.json` — When user is active, what dominates
- `pattern_library.json` — Detected recurring patterns
- `analytics_metadata.json` — Timestamps, data freshness, analyzer versions

---

### 4. Consumer Interface (K15)

**Query API:**
```python
class BehaviorAnalytics:
    def get_current_profile(self) -> BehaviorProfile: ...
    def get_domain_preference(self, domain: str) -> DomainStats: ...
    def get_sessions(self, since: datetime) -> List[SessionRecord]: ...
    def get_temporal_signal(self, hour: int) -> TemporalStats: ...
    def detect_anomaly(self, current_event: BehaviorEvent) -> float: ...
    
    # For organ-x
    def predict_next_domain(self, context: SessionContext) -> str: ...
    def get_coherence_threshold(self) -> float: ...
```

**Consumers:**
- **organ-x domain router:** `predict_next_domain()` → routes observations
- **soma layer 1:** `get_temporal_signal()` → adapt GPU thresholds at peak hours
- **crystal enrichment:** `get_current_profile()` → inject behavioral context
- **learning loop:** Subscribe to `analytics_metadata.updated` → trigger retraining

---

## Autonomy & Flexibility

### Autonomy
- **Ingestion runs hourly** (cron or systemd timer)
- **Analyzers run daily** on aggregated data (cheaper than per-event)
- **New analyzers added without touching core pipeline** — just drop class in `analyzers/`

### Flexibility
- **Custom analyzers:** Subclass `BehaviorAnalyzer`, implement `analyze()`, drop in
- **Example:** `SeasonalAnalyzer` (detect multi-week patterns), `AnomalyDetector` (flag unusual sessions)
- **No script rewrite:** Analyzer runs automatically in daily job

### Data-Centric
- **Single source of truth:** `normalized_events.jsonl`
- **Immutable audit trail:** All aggregates timestamped
- **Version control:** Analyzer code in git, outputs in data store (queryable history)

---

## Implementation Phases

### Phase 1: Core Pipeline (2h)
1. **Ingestion:** `BehaviorIngestion` class (read behavior_log → normalize → enrich)
2. **Aggregation:** Daily roll-up to `BehaviorAggregate`
3. **Base analyzers:** Session, Domain, Temporal
4. **Output store:** Write insights to JSON files

**Deployed as:** `systemd timer` (`behavioral_analytics.timer`) that runs daily

### Phase 2: Consumer Wiring (1h)
1. **BehaviorAnalytics API:** Read-only interface to insights
2. **Organ-x integration:** `domain_router` queries `get_domain_preference()`
3. **Test:** Verify organic-x uses behavioral signal, not embeddings

### Phase 3: Advanced Analyzers (3d, optional)
1. **SequentialPatternAnalyzer** → Markov chains for domain prediction
2. **CoherenceAnalyzer** → Embedding-based coherence (uses llama-embed)
3. **AnomalyDetector** → Detect unusual sessions (learning loop signal)

---

## Why This Solves the Problem

| Before | After |
|--------|-------|
| New question = new script | New question = new analyzer class |
| Scripts load/transform data independently | Single normalization pipeline |
| Results scattered (files, stdout) | Queryable insight store (K15 consumer) |
| Hard to version | Immutable data + version-controlled code |
| Manual runs | Autonomous (cron + systemd) |
| No audit trail | Full history in `behavior_*.jsonl` |

---

## Start Here

1. **Build ingestion + aggregation** → `BehaviorIngestion`, `BehaviorAggregate`
2. **Implement 3 base analyzers** → Session, Domain, Temporal
3. **Deploy as systemd timer** → Runs daily, populates output store
4. **Integrate with organ-x** → Router queries insights
5. **Measure:** Does behavioral signal beat embeddings for routing?

---

## Open Questions

- **Real-time vs batch?** Current design is daily batch (cheap, simple). Real-time requires event stream.
- **Storage backend?** Currently JSON files. Scale to >1M sessions = consider SQLite or DB.
- **Feedback loop?** What does organ-x learning look like? (RL? IRL? Supervised on outcomes?)

**Recommended:** Start batch, measure signal quality. If good, consider real-time streaming.
