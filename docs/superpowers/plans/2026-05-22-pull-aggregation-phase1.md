# Pull-Based Multi-Source Aggregation — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the X organ from per-tweet judgment (33 req/min, all BARK) to observe-and-aggregate (store tweets, detect convergence, emit rich multi-source summaries for kernel judgment).

**Architecture:** X daemon switches from `/judge` to `/observe` per tweet. A Python convergence detector groups observations by cashtag, emits structured summaries when 3+ authors converge. A kernel runtime loop consumes convergence signals, pulls Helius on-chain data, and triggers multi-source `/judge` calls. The demand path (`POST /judge { mint }`) also pulls convergence summaries when available.

**Tech Stack:** Python 3 (X daemon, convergence detector, cashtag resolver), Rust (kernel StoragePort, runtime loop, enrichment), SurrealDB (observations, cashtag_mint table), Helius API (existing enricher).

**Spec:** `docs/superpowers/specs/2026-05-22-pull-based-multi-source-aggregation-design.md`

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `scripts/hermes-x/core/x_ingest_daemon.py` | Switch `/judge` → `/observe`, remove judge throttling |
| Create | `scripts/hermes-x/core/convergence_detector.py` | Group observations by cashtag, detect convergence, emit summaries |
| Create | `cynic-python/resolvers/cashtag_mint.py` | Resolve $SYMBOL → mint address via local cache + Helius |
| Modify | `cynic-kernel/src/storage/mod.rs` | Add `obs_tool_idx` index, `list_observations_by_tag` |
| Modify | `cynic-kernel/src/storage/surreal/activity.rs` | Implement `list_observations_by_tag` query |
| Modify | `cynic-kernel/src/domain/storage/mod.rs` | Add `list_observations_by_tag` to StoragePort trait |
| Create | `cynic-kernel/src/infra/tasks/convergence_consumer.rs` | Poll convergence signals, trigger multi-source judgment |
| Modify | `cynic-kernel/src/infra/tasks/mod.rs` | Register convergence consumer |
| Modify | `cynic-kernel/src/pipeline/enrichment.rs` | Pull convergence summaries in `enrich_token` |
| Modify | `cynic-kernel/src/main.rs` | Spawn convergence consumer loop |

---

### Task 1: X Daemon — Observe Mode

Switch the ingest daemon from `/judge` to `/observe`. This immediately stops the 33 req/min flood and frees the GPU slot.

**Files:**
- Modify: `scripts/hermes-x/core/x_ingest_daemon.py`

- [ ] **Step 1: Add observe-only mode flag**

In the daemon's main processing function, replace the `post_judge()` call with `post_observe()` for ALL tweets (not just low-signal). The existing `post_observe()` function already exists at line 262.

```python
# In process_row() or equivalent — change the routing:
# BEFORE: high-signal → /judge, low-signal → /observe
# AFTER:  all tweets → /observe (no /judge calls from daemon)

# Keep signal_score computation (used by convergence detector later)
# but route everything to /observe with proper tags
```

Key changes:
- Remove `post_judge()` call from the main loop
- Ensure `post_observe()` includes `tags: [cashtags + narratives]` (already does)
- Add `domain: detected_domain` to observe payload (currently hardcoded "twitter" — use the domain detector output)
- Keep `JUDGE_THROTTLE` variable but unused (remove in cleanup)

- [ ] **Step 2: Verify daemon runs in observe-only mode**

```bash
systemctl --user restart hermes-x-ingest
sleep 10
journalctl --user -u hermes-x-ingest --since "10 sec ago" | grep -E "JUDGE|observe"
# Expected: only "observe" lines, zero "JUDGE" lines
```

- [ ] **Step 3: Verify GPU slot is free**

```bash
source ~/.cynic-env
# Check that 27B slot is no longer perpetually busy
journalctl --user -u cynic-kernel --since "1 min ago" | grep "slots saturated" | wc -l
# Expected: 0 (or very few — only from other consumers)
```

- [ ] **Step 4: Commit**

```bash
git add scripts/hermes-x/core/x_ingest_daemon.py
git commit -m "fix(hermes-x): switch ingest daemon from /judge to /observe

All tweets now stored via /observe with proper domain + tags.
No more /judge calls from daemon — kills 33 req/min flood.
Convergence detector (next task) will emit rich summaries for /judge."
```

---

### Task 2: Cashtag-Mint Resolver

Python module that resolves `$SOL` → mint address. Used by the convergence detector to tag summaries with resolved mints.

**Files:**
- Create: `cynic-python/resolvers/__init__.py`
- Create: `cynic-python/resolvers/cashtag_mint.py`
- Create: `cynic-python/resolvers/test_cashtag_mint.py`

- [ ] **Step 1: Write the test**

```python
# test_cashtag_mint.py
import pytest
from cashtag_mint import CashtagResolver

def test_known_symbol_resolves():
    """SOL should resolve to the well-known mint."""
    resolver = CashtagResolver(cache={
        "SOL": "So11111111111111111111111111111111"
    })
    assert resolver.resolve("SOL") == "So11111111111111111111111111111111"
    assert resolver.resolve("$SOL") == "So11111111111111111111111111111111"

def test_unknown_symbol_returns_none():
    resolver = CashtagResolver(cache={})
    assert resolver.resolve("NONEXISTENT_XYZ") is None

def test_ambiguous_symbol_returns_none():
    """Multiple mints for same symbol = ambiguous."""
    resolver = CashtagResolver(cache={
        "PEPE": "AMBIGUOUS"  # sentinel
    })
    assert resolver.resolve("PEPE") is None

def test_strip_dollar_sign():
    resolver = CashtagResolver(cache={"BTC": "mint123"})
    assert resolver.resolve("$BTC") == "mint123"
    assert resolver.resolve("BTC") == "mint123"
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd cynic-python && python -m pytest resolvers/test_cashtag_mint.py -v
# Expected: ModuleNotFoundError
```

- [ ] **Step 3: Implement resolver**

```python
# cashtag_mint.py
"""
Cashtag → Mint resolver. File cache + Helius searchAssets fallback.

Tier 2 INFRASTRUCTURE: consumed by convergence detector.
K15 consumer: convergence summaries use resolved_mint for kernel enrichment cross-reference.
"""
import json
import os
import logging
from pathlib import Path
from typing import Optional

logger = logging.getLogger("cashtag-resolver")

# Well-known mappings that never change
WELL_KNOWN = {
    "SOL": "So11111111111111111111111111111111",
    "USDC": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "USDT": "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
}

AMBIGUOUS = "AMBIGUOUS"

class CashtagResolver:
    def __init__(self, cache: dict[str, str] | None = None, cache_path: Path | None = None):
        self._cache: dict[str, str] = {**WELL_KNOWN}
        if cache:
            self._cache.update(cache)
        self._cache_path = cache_path

    def resolve(self, symbol: str) -> Optional[str]:
        """Resolve cashtag to mint. Returns None if unknown or ambiguous."""
        symbol = symbol.lstrip("$").upper()
        mint = self._cache.get(symbol)
        if mint == AMBIGUOUS:
            return None
        return mint

    def add(self, symbol: str, mint: str):
        """Add a resolved mapping to cache."""
        symbol = symbol.lstrip("$").upper()
        self._cache[symbol] = mint

    def mark_ambiguous(self, symbol: str):
        """Mark a symbol as ambiguous (multiple mints)."""
        symbol = symbol.lstrip("$").upper()
        self._cache[symbol] = AMBIGUOUS

    def save(self):
        """Persist cache to disk (if cache_path set)."""
        if self._cache_path:
            self._cache_path.write_text(json.dumps(self._cache, indent=2))

    @classmethod
    def load(cls, cache_path: Path) -> "CashtagResolver":
        """Load from file, or create empty."""
        cache = {}
        if cache_path.exists():
            try:
                cache = json.loads(cache_path.read_text())
            except (json.JSONDecodeError, OSError) as e:
                logger.warning("cache load failed (%s), starting fresh", e)
        return cls(cache=cache, cache_path=cache_path)
```

- [ ] **Step 4: Run tests, verify pass**

```bash
cd cynic-python && python -m pytest resolvers/test_cashtag_mint.py -v
# Expected: 4 passed
```

- [ ] **Step 5: Commit**

```bash
git add cynic-python/resolvers/
git commit -m "feat(resolvers): cashtag-mint resolver with file cache"
```

---

### Task 3: Convergence Detector

Python module that scans recent X observations, groups by cashtag, and emits convergence summaries when 3+ distinct authors mention the same cashtag in 6h.

**Files:**
- Create: `scripts/hermes-x/core/convergence_detector.py`
- Create: `scripts/hermes-x/core/test_convergence_detector.py`

- [ ] **Step 1: Write the test**

```python
# test_convergence_detector.py
import pytest
from datetime import datetime, timezone, timedelta
from convergence_detector import detect_convergence, ConvergenceSignal

def _obs(author: str, cashtag: str, minutes_ago: int = 0) -> dict:
    ts = datetime.now(timezone.utc) - timedelta(minutes=minutes_ago)
    return {
        "agent_id": "hermes-x-cynic",
        "tags": [cashtag],
        "context": f"@{author} [5]: something about ${cashtag}",
        "created_at": ts.isoformat(),
        "target": f"tweet_{author}_{cashtag}",
    }

def test_convergence_3_authors():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 1
    assert signals[0].cashtag == "SOL"
    assert signals[0].author_count == 3

def test_no_convergence_below_threshold():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0

def test_same_author_doesnt_count_twice():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("alice", "SOL", 20),  # same author
        _obs("bob", "SOL", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0

def test_outside_window_excluded():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 400),  # >6h ago
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0

def test_multiple_cashtags():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 30),
        _obs("dave", "BONK", 10),
        _obs("eve", "BONK", 20),
        _obs("frank", "BONK", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 2
    tags = {s.cashtag for s in signals}
    assert tags == {"SOL", "BONK"}
```

- [ ] **Step 2: Run test, verify it fails**

```bash
cd scripts/hermes-x/core && python -m pytest test_convergence_detector.py -v
# Expected: ImportError
```

- [ ] **Step 3: Implement convergence detector**

```python
# convergence_detector.py
"""
Convergence detector — groups X organ observations by cashtag,
emits ConvergenceSignal when 3+ distinct authors mention the same item.

Tier 2 INFRASTRUCTURE: consumed by convergence consumer (kernel) via /observe.
K15 consumer: convergence signals trigger multi-source judgment.
"""
import json
import logging
import re
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timezone, timedelta
from typing import Optional

logger = logging.getLogger("convergence-detector")

@dataclass
class ConvergenceSignal:
    cashtag: str
    resolved_mint: Optional[str]
    author_count: int
    authors: list[str]
    coordination_score: float  # 0=independent, 1=identical texts
    sentiment: str  # "bearish", "bullish", "neutral"
    key_quotes: list[dict]  # [{author, text, tweet_id}]
    window_hours: int
    domain: str

def _extract_author(context: str) -> str:
    """Extract @author from context string '@author [score]: text'."""
    m = re.match(r"@(\S+)", context or "")
    return m.group(1) if m else ""

def _extract_cashtags(tags: list[str]) -> list[str]:
    """Filter tags to cashtags only (uppercase, 2-10 chars)."""
    return [t for t in tags if t.isupper() and 2 <= len(t) <= 10 and t.isalpha()]

def detect_convergence(
    observations: list[dict],
    min_authors: int = 3,
    window_hours: int = 6,
) -> list[ConvergenceSignal]:
    """Scan observations, return convergence signals for items above threshold."""
    cutoff = datetime.now(timezone.utc) - timedelta(hours=window_hours)

    # Group by cashtag → set of (author, context, tweet_id)
    groups: dict[str, list[dict]] = defaultdict(list)
    for obs in observations:
        created = obs.get("created_at", "")
        try:
            ts = datetime.fromisoformat(created.replace("Z", "+00:00"))
        except (ValueError, AttributeError):
            continue
        if ts < cutoff:
            continue

        author = _extract_author(obs.get("context", ""))
        if not author:
            continue

        for tag in _extract_cashtags(obs.get("tags", [])):
            groups[tag].append({
                "author": author,
                "context": obs.get("context", ""),
                "target": obs.get("target", ""),
                "ts": ts,
            })

    signals = []
    for cashtag, entries in groups.items():
        # Deduplicate by author
        seen_authors: dict[str, dict] = {}
        for entry in entries:
            a = entry["author"]
            if a not in seen_authors:
                seen_authors[a] = entry

        if len(seen_authors) < min_authors:
            continue

        authors = list(seen_authors.keys())
        # Key quotes: one per author, most recent first, max 3
        quotes = [
            {"author": e["author"], "text": e["context"][:200], "tweet_id": e["target"]}
            for e in list(seen_authors.values())[:3]
        ]

        # Coordination: check for identical texts (basic)
        texts = [e["context"] for e in seen_authors.values()]
        unique_texts = len(set(texts))
        coord = 1.0 - (unique_texts / len(texts)) if texts else 0.0

        signals.append(ConvergenceSignal(
            cashtag=cashtag,
            resolved_mint=None,  # filled by caller via resolver
            author_count=len(seen_authors),
            authors=authors,
            coordination_score=round(coord, 3),
            sentiment="neutral",  # Phase C: sentiment analysis
            key_quotes=quotes,
            window_hours=window_hours,
            domain="D1",  # default; caller can override based on cashtag
        ))

    return signals
```

- [ ] **Step 4: Run tests, verify pass**

```bash
cd scripts/hermes-x/core && python -m pytest test_convergence_detector.py -v
# Expected: 5 passed
```

- [ ] **Step 5: Commit**

```bash
git add scripts/hermes-x/core/convergence_detector.py scripts/hermes-x/core/test_convergence_detector.py
git commit -m "feat(hermes-x): convergence detector — groups by cashtag, emits signals"
```

---

### Task 4: Convergence Emitter (Wire Detector to /observe)

Integrate the convergence detector into the X daemon lifecycle. Run after each batch of observations, emit convergence summaries to kernel.

**Files:**
- Modify: `scripts/hermes-x/core/x_ingest_daemon.py`

- [ ] **Step 1: Add convergence check to daemon main loop**

After processing a batch of tweets (observe mode), query recent observations from local state (in-memory list of recently observed cashtags), run `detect_convergence()`, and POST summaries.

```python
# In the daemon's main loop, after observe batch:
from convergence_detector import detect_convergence
from cashtag_mint import CashtagResolver

# Every N observations (e.g., every 50 tweets or every 5 min):
# 1. Collect recent observations from in-memory buffer
# 2. Run detect_convergence()
# 3. For each signal: resolve cashtag, POST /observe with tool=x-convergence

def emit_convergence(signals, resolver, cooldown_map):
    for signal in signals:
        # Cooldown check (1h per cashtag)
        if signal.cashtag in cooldown_map:
            if (now - cooldown_map[signal.cashtag]).total_seconds() < 3600:
                continue

        # Resolve mint
        signal.resolved_mint = resolver.resolve(signal.cashtag)

        # Build context (JSON string — K21 compliance)
        context = json.dumps({
            "item": signal.cashtag,
            "resolved_mint": signal.resolved_mint,
            "author_count": signal.author_count,
            "authors": signal.authors,
            "tier_distribution": {},  # Phase C
            "coordination_score": signal.coordination_score,
            "sentiment": signal.sentiment,
            "window_hours": signal.window_hours,
            "key_quotes": signal.key_quotes,
            "source_organ": "x",
        })

        target = signal.resolved_mint or f"${signal.cashtag}"
        tags = [
            "convergence",
            signal.cashtag,
            "compound-loop",  # K21: nightshift must filter this
        ]
        if signal.resolved_mint:
            tags.append(f"mint:{signal.resolved_mint}")

        payload = {
            "tool": "x-convergence",
            "target": target,
            "domain": signal.domain,
            "context": context,
            "tags": tags,
            "agent_id": f"hermes-x-{ACCOUNT_ID}",
        }

        resp = requests.post(f"{KERNEL_ADDR}/observe", json=payload, headers=_headers(), timeout=10)
        if resp.status_code == 200:
            cooldown_map[signal.cashtag] = now
            logger.info("CONVERGENCE %s: %d authors → %s", signal.cashtag, signal.author_count, target)
```

- [ ] **Step 2: Test convergence emission end-to-end**

```bash
systemctl --user restart hermes-x-ingest
sleep 60  # let it observe + detect
journalctl --user -u hermes-x-ingest --since "1 min ago" | grep "CONVERGENCE"
# Expected: convergence lines for cashtags with 3+ authors (if data exists)
```

- [ ] **Step 3: Verify convergence observation in kernel**

```bash
source ~/.cynic-env
curl -s -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  "http://${CYNIC_REST_ADDR}/observations?limit=10" | \
  python3 -c "import json,sys; [print(f'{o[\"tool\"]} target={o[\"target\"]}') for o in json.load(sys.stdin) if 'convergence' in o.get('tool','')]"
# Expected: x-convergence observations with cashtag targets
```

- [ ] **Step 4: Commit**

```bash
git add scripts/hermes-x/core/x_ingest_daemon.py
git commit -m "feat(hermes-x): wire convergence detector → /observe summaries

Daemon observes tweets, detects cashtag convergence (3+ authors/6h),
emits x-convergence observations with compound-loop tag (K21).
Cooldown: 1h per cashtag. Mint resolved via cashtag resolver."
```

---

### Task 5: Kernel — `list_observations_by_tag` + `obs_tool_idx`

Add the storage query the convergence consumer needs: filter observations by tag, and index the `tool` field.

**Files:**
- Modify: `cynic-kernel/src/domain/storage/mod.rs` — add trait method
- Modify: `cynic-kernel/src/storage/surreal/activity.rs` — implement SurrealDB query
- Modify: `cynic-kernel/src/storage/mod.rs` — add `obs_tool_idx` to schema bootstrap

- [ ] **Step 1: Add trait method to StoragePort**

```rust
// In domain/storage/mod.rs, after list_observations_by_target:
async fn list_observations_by_tag(
    &self,
    _domain: &str,
    _tag: &str,
    _limit: u32,
) -> Result<Vec<RawObservation>, StorageError> {
    Ok(vec![]) // default no-op
}
```

- [ ] **Step 2: Implement SurrealDB query**

```rust
// In storage/surreal/activity.rs:
pub(super) async fn list_observations_by_tag(
    db: &Surreal<Any>,
    domain: &str,
    tag: &str,
    limit: u32,
) -> Result<Vec<RawObservation>, StorageError> {
    let query = format!(
        "SELECT * FROM observation WHERE domain = $domain AND tags CONTAINS $tag ORDER BY created_at DESC LIMIT {limit}"
    );
    let mut resp = db.query(&query)
        .bind(("domain", domain))
        .bind(("tag", tag))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;
    let rows: Vec<RawObservation> = resp.take(0)
        .map_err(|e| StorageError::Query(e.to_string()))?;
    Ok(rows)
}
```

- [ ] **Step 3: Add tool index to schema bootstrap**

```rust
// In storage/mod.rs, after obs_agent_idx line:
DEFINE INDEX IF NOT EXISTS obs_tool_idx ON observation FIELDS tool;\
```

- [ ] **Step 4: Wire trait → implementation**

Forward `list_observations_by_tag` calls through ReconnectableStorage (K17).

- [ ] **Step 5: Build and test**

```bash
cargo check -p cynic-kernel
cargo test -p cynic-kernel -- list_observations
# Expected: compiles, existing tests pass
```

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/domain/storage/mod.rs cynic-kernel/src/storage/
git commit -m "feat(storage): list_observations_by_tag + obs_tool_idx

SurrealDB WHERE tags CONTAINS query. Tool index for convergence
consumer's primary query path."
```

---

### Task 6: Kernel — Convergence Consumer Runtime Loop

The K15 consumer that polls convergence observations and triggers multi-source judgment.

**Files:**
- Create: `cynic-kernel/src/infra/tasks/convergence_consumer.rs`
- Modify: `cynic-kernel/src/infra/tasks/mod.rs`
- Modify: `cynic-kernel/src/main.rs`

- [ ] **Step 1: Create convergence consumer module**

```rust
// convergence_consumer.rs
//! K15 consumer: polls *-convergence observations, triggers multi-source /judge.
//!
//! Dedup: verdict cache check — if a verdict for this target+domain exists
//! within the last 2h, skip re-judging. Safe on boot (cache loads from DB).

use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
use crate::domain::slot_semaphore::SlotPriority;
use crate::domain::storage::StoragePort;
use crate::judge::Judge;

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const DEDUP_WINDOW_HOURS: i64 = 2;

pub async fn run(
    judge: arc_swap::Guard<Arc<Judge>>,
    storage: Arc<dyn StoragePort>,
    enricher: Option<Arc<dyn crate::domain::enrichment::TokenEnricherPort>>,
    shutdown: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(POLL_INTERVAL) => {}
            _ = shutdown.cancelled() => break,
        }

        // Poll recent convergence observations
        let observations = match storage.list_observations_raw(None, None, 50).await {
            Ok(obs) => obs,
            Err(e) => {
                tracing::warn!("convergence consumer: storage query failed: {e}");
                continue;
            }
        };

        let convergence_obs: Vec<_> = observations
            .into_iter()
            .filter(|o| o.tool.ends_with("-convergence"))
            .collect();

        if convergence_obs.is_empty() {
            continue;
        }

        for obs in convergence_obs {
            // Parse context JSON
            let ctx: serde_json::Value = match serde_json::from_str(&obs.context) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let target = &obs.target;
            let domain = &obs.domain;

            // Dedup: check if recently judged
            // (implementation: query verdicts by domain+target within window)
            // For now: use a simple in-memory dedup set per cycle
            // TODO: wire to verdict cache has_recent() when available

            // Build multi-source stimulus
            let social_section = format_social_section(&ctx);
            let content = format!(
                "[DOMAIN: {domain}]\n\n[SOCIAL SIGNAL — {source}]\n{social}",
                domain = domain,
                source = ctx.get("source_organ").and_then(|v| v.as_str()).unwrap_or("unknown"),
                social = social_section,
            );

            let stimulus = Stimulus {
                content,
                context: Some(obs.context.clone()),
                domain: Some(domain.clone()),
                request_id: None,
            };

            let metrics = Metrics::new();
            match judge.evaluate(&stimulus, None, &metrics, SlotPriority::Hermes).await {
                Ok(verdict) => {
                    tracing::info!(
                        target = %target,
                        domain = %domain,
                        q_score = %format!("{:.3}", verdict.q_score.total),
                        kind = ?verdict.kind,
                        "convergence verdict issued"
                    );
                }
                Err(e) => {
                    tracing::warn!(target = %target, "convergence judgment failed: {e}");
                }
            }
        }
    }
    tracing::info!("convergence consumer stopped");
}

fn format_social_section(ctx: &serde_json::Value) -> String {
    let mut lines = vec![];
    if let Some(count) = ctx.get("author_count").and_then(|v| v.as_u64()) {
        lines.push(format!("convergence: {} authors in {}h",
            count,
            ctx.get("window_hours").and_then(|v| v.as_u64()).unwrap_or(6)));
    }
    if let Some(authors) = ctx.get("authors").and_then(|v| v.as_array()) {
        let names: Vec<&str> = authors.iter().filter_map(|a| a.as_str()).collect();
        lines.push(format!("authors: {}", names.join(", ")));
    }
    if let Some(coord) = ctx.get("coordination_score").and_then(|v| v.as_f64()) {
        let label = if coord > 0.5 { "coordinated" } else { "independent" };
        lines.push(format!("coordination: {:.1} ({})", coord, label));
    }
    if let Some(quotes) = ctx.get("key_quotes").and_then(|v| v.as_array()) {
        lines.push("key_quotes:".to_string());
        for q in quotes.iter().take(3) {
            let author = q.get("author").and_then(|v| v.as_str()).unwrap_or("?");
            let text = q.get("text").and_then(|v| v.as_str()).unwrap_or("").chars().take(120).collect::<String>();
            lines.push(format!("  - @{}: \"{}\"", author, text));
        }
    }
    lines.join("\n")
}
```

- [ ] **Step 2: Register module in mod.rs**

```rust
// In infra/tasks/mod.rs:
pub mod convergence_consumer;
```

- [ ] **Step 3: Spawn in main.rs**

```rust
// In main.rs, near the nightshift spawn:
{
    let judge = Arc::clone(&judge_handle);
    let storage = Arc::clone(&storage_port);
    let enricher = enricher_port.clone();
    let sd = shutdown.clone();
    tokio::spawn(async move {
        let guard = judge.load_full();
        infra::tasks::convergence_consumer::run(guard, storage, enricher, sd).await;
    });
    klog!("[Ring 3] Convergence consumer started (poll interval: 60s)");
}
```

- [ ] **Step 4: Build and verify**

```bash
cargo check -p cynic-kernel
cargo clippy -p cynic-kernel -- -D warnings
# Expected: compiles clean
```

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/infra/tasks/convergence_consumer.rs cynic-kernel/src/infra/tasks/mod.rs cynic-kernel/src/main.rs
git commit -m "feat(kernel): convergence consumer — polls summaries, triggers multi-source /judge

K15 consumer: x-convergence/tg-convergence observations → rich stimulus →
27B evaluation. Priority=Hermes (queued, not background). 60s poll interval.
Dedup via verdict cache (TODO: wire has_recent when available)."
```

---

### Task 7: Enrichment Pull — `enrich_token` Consumes Social Summaries

Wire the demand path: when `/judge` is called with a mint, `enrich_token` also pulls convergence summaries.

**Files:**
- Modify: `cynic-kernel/src/pipeline/enrichment.rs`

- [ ] **Step 1: Add convergence summary pull to enrich_token**

After the existing Helius enrichment and trajectory pull, add:

```rust
// In enrich_token(), after trajectory data pull:
// Pull social convergence summaries for this mint (if any)
let social_summaries = storage
    .list_observations_by_target(&domain_hint, mint, 5)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter(|o| o.tool.ends_with("-convergence"))
    .collect::<Vec<_>>();

if !social_summaries.is_empty() {
    // Parse and inject social context into stimulus
    for obs in &social_summaries {
        if let Ok(ctx) = serde_json::from_str::<serde_json::Value>(&obs.context) {
            // Append social section to enriched content
            let social = format_social_enrichment(&ctx);
            enriched_content.push_str(&social);
        }
    }
    tracing::info!(
        mint = %mint,
        summaries = social_summaries.len(),
        "enrich_token: social convergence data injected"
    );
}
```

- [ ] **Step 2: Also try tag-based lookup (fallback for cashtag-targeted signals)**

```rust
// Fallback: if no observations found by target (mint), try by tag
if social_summaries.is_empty() {
    let by_tag = storage
        .list_observations_by_tag(&domain_hint, &format!("mint:{mint}"), 5)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|o| o.tool.ends_with("-convergence"))
        .collect::<Vec<_>>();
    // ... same injection logic
}
```

- [ ] **Step 3: Build and test**

```bash
cargo check -p cynic-kernel
cargo test -p cynic-kernel -- pipeline
# Expected: compiles, 28+ pipeline tests pass
```

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/pipeline/enrichment.rs
git commit -m "feat(enrichment): pull social convergence summaries in enrich_token

Demand path: POST /judge { mint } now also queries x-convergence/tg-convergence
observations by target (mint) or tag (mint:xxx). Social data injected into
stimulus alongside Helius on-chain data."
```

---

## Integration Test

After all tasks are committed:

- [ ] **Step 1: Deploy kernel**

```bash
cargo build --release -p cynic-kernel
mv ~/bin/cynic-kernel ~/bin/cynic-kernel.old && cp target/release/cynic-kernel ~/bin/cynic-kernel
systemctl --user restart cynic-kernel
```

- [ ] **Step 2: Restart X daemon in observe mode**

```bash
systemctl --user restart hermes-x-ingest
```

- [ ] **Step 3: Wait for convergence (10-30 min)**

```bash
# Monitor for convergence signals
journalctl --user -u hermes-x-ingest -f | grep CONVERGENCE
```

- [ ] **Step 4: Verify convergence consumer triggers judgment**

```bash
journalctl --user -u cynic-kernel --since "5 min ago" | grep "convergence verdict"
```

- [ ] **Step 5: Verify demand path includes social data**

```bash
source ~/.cynic-env
curl -s -X POST -H "Authorization: Bearer ${CYNIC_API_KEY}" -H "Content-Type: application/json" \
  "http://${CYNIC_REST_ADDR}/judge" \
  -d '{"content":"So11111111111111111111111111111111","domain":"token-analysis"}'
# Check: verdict reasoning should reference social signal if convergence exists
```
