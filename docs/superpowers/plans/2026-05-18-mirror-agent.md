# Mirror Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Python daemon that continuously learns T.'s behavioral patterns from behavior_log.jsonl, dataset.jsonl, and kernel data — then emits askesis insights and dispatches Hermes tasks when confidence crosses phi^-2.

**Architecture:** Tier 2 Python daemon tailing local JSONL files + polling kernel REST. Online learning via EMA. Profile checkpointed to disk every 1000 events / 15min. Two output channels: `mirror-askesis` observations (Slack delivery) and `mirror-action` agent tasks (Hermes executor).

**Tech Stack:** Python 3.11+, pydantic (data models), pytest (tests), mypy --strict (types). No ML libraries in v0 — pure statistics. Kernel REST for reads/writes. JSONL for all persistence.

**Spec:** `docs/superpowers/specs/2026-05-18-mirror-agent-design.md`

---

## File Map

```
cynic-python/
├── organs/
│   └── mirror/
│       ├── __init__.py              # __version__, module docstring (Tier 2 tag)
│       ├── daemon.py                # Boot, signal handling, shutdown, startup observation
│       ├── coordinator.py           # Event loop: tail sources, route events to learner
│       ├── profile.py               # BehavioralProfile + supporting types + serde
│       ├── checkpoint.py            # ProfileCheckpoint: save/load/replay cursor
│       ├── sources/
│       │   ├── __init__.py
│       │   ├── base.py              # Source protocol (ABC for all sources)
│       │   ├── behavior.py          # Tail behavior_log.jsonl
│       │   └── x_signals.py         # Tail dataset.jsonl
│       ├── learner.py               # OnlineLearner: ingest event → update profile
│       ├── askesis.py               # Insight generation + Slack delivery
│       ├── action.py                # Confidence gate + agent task dispatch
│       ├── predictions.py           # Prediction records + outcome join
│       └── tests/
│           ├── __init__.py
│           ├── test_profile.py
│           ├── test_checkpoint.py
│           ├── test_behavior_source.py
│           ├── test_x_source.py
│           ├── test_learner.py
│           ├── test_askesis.py
│           ├── test_action.py
│           └── test_predictions.py

infra/systemd/
└── mirror-agent.service             # Systemd unit file

scripts/hermes-x/core/
└── hermes_paths.py                  # Add MIRROR_DIR, MIRROR_PROFILE, MIRROR_PREDICTIONS
```

---

## Task 1: Profile Data Model + Serialization

**Files:**
- Create: `cynic-python/organs/mirror/__init__.py`
- Create: `cynic-python/organs/mirror/profile.py`
- Create: `cynic-python/organs/mirror/tests/__init__.py`
- Create: `cynic-python/organs/mirror/tests/test_profile.py`

This task builds the core data structures. Everything else depends on this.

- [ ] **Step 1: Write test for BehavioralProfile creation and serialization**

```python
# tests/test_profile.py
"""Tests for BehavioralProfile data model and serialization."""
import json
from datetime import datetime, timezone

from organs.mirror.profile import (
    BehavioralProfile,
    Distribution,
    Feature,
    Tension,
    TimeWindow,
    feature_confidence,
)

PHI_INV = 0.618034


def test_empty_profile_creation() -> None:
    """A fresh profile has zero observations and empty collections."""
    profile = BehavioralProfile.empty()
    assert profile.observation_count == 0
    assert profile.profile_version == "0.1.0"
    assert len(profile.activity_hours) == 0
    assert len(profile.narrative_affinity) == 0


def test_profile_roundtrip_json() -> None:
    """Profile survives JSON serialize -> deserialize without data loss."""
    profile = BehavioralProfile.empty()
    profile.observation_count = 42
    profile.activity_hours = {21: 0.35, 22: 0.28}
    profile.narrative_affinity = {"agent": 0.7, "defi": 0.3}

    json_str = profile.to_json()
    restored = BehavioralProfile.from_json(json_str)

    assert restored.observation_count == 42
    assert restored.activity_hours == {21: 0.35, 22: 0.28}
    assert restored.narrative_affinity == {"agent": 0.7, "defi": 0.3}


def test_feature_confidence_low_observations() -> None:
    """5 observations with perfect stability gives low confidence."""
    c = feature_confidence(n_observations=5, stability=1.0)
    assert 0.08 < c < 0.10


def test_feature_confidence_high_observations() -> None:
    """200 observations with high stability approaches but never exceeds phi^-1."""
    c = feature_confidence(n_observations=200, stability=0.9)
    assert 0.50 < c < PHI_INV


def test_feature_confidence_never_exceeds_phi_inv() -> None:
    """Even infinite observations cannot exceed phi^-1."""
    c = feature_confidence(n_observations=1_000_000, stability=1.0)
    assert c <= PHI_INV


def test_feature_confidence_zero_stability() -> None:
    """Zero stability means zero confidence regardless of observation count."""
    c = feature_confidence(n_observations=1000, stability=0.0)
    assert c == 0.0


def test_distribution_from_values() -> None:
    """Distribution computes correct statistics from a value list."""
    d = Distribution.from_values([1.0, 2.0, 3.0, 4.0, 5.0])
    assert d.n == 5
    assert d.mean == 3.0
    assert 1.4 < d.std < 1.5  # population std
    assert d.p50 == 3.0
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_profile.py -v`
Expected: ModuleNotFoundError

- [ ] **Step 3: Implement profile.py**

```python
# organs/mirror/__init__.py
"""
Tier 2 INFRASTRUCTURE: Mirror Agent — behavioral model of T.

K15 Consumer: T. (askesis via Slack), Hermes executor (mirror-action tasks)
Systemd: mirror-agent.service (continuous daemon)
Promotion date: 2026-05-18 (from mirror-agent design spec)

Metrics: observation_count, profile_version, last checkpoint
"""
__version__ = "0.1.0"
```

```python
# organs/mirror/profile.py
"""BehavioralProfile — the core data model for the mirror agent."""
from __future__ import annotations

import json
import statistics
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Any

PHI_INV = 0.618034
PHI_INV_SQ = 0.381966  # phi^-2, action gate threshold
CONFIDENCE_HALF_LIFE = 50  # conjecture — recalibrate after 30 days of real data


def feature_confidence(n_observations: int, stability: float) -> float:
    """Confidence bounded by phi^-1.

    Args:
        n_observations: data points backing this feature.
        stability: EMA 7-day variance normalized to [0, 1].
            1.0 = perfectly stable, 0.0 = maximally volatile.
    """
    raw = 1.0 - (1.0 / (1.0 + n_observations / CONFIDENCE_HALF_LIFE))
    return min(raw * stability, PHI_INV)


@dataclass
class TimeWindow:
    start_hour: int
    end_hour: int
    strength: float


@dataclass
class Distribution:
    mean: float
    std: float
    p25: float
    p50: float
    p75: float
    n: int

    @classmethod
    def empty(cls) -> Distribution:
        return cls(mean=0.0, std=0.0, p25=0.0, p50=0.0, p75=0.0, n=0)

    @classmethod
    def from_values(cls, values: list[float]) -> Distribution:
        if not values:
            return cls.empty()
        sorted_v = sorted(values)
        n = len(sorted_v)
        return cls(
            mean=statistics.mean(sorted_v),
            std=statistics.pstdev(sorted_v),
            p25=sorted_v[max(0, n // 4 - 1)] if n >= 4 else sorted_v[0],
            p50=statistics.median(sorted_v),
            p75=sorted_v[min(n - 1, 3 * n // 4)] if n >= 4 else sorted_v[-1],
            n=n,
        )


@dataclass
class Feature:
    name: str
    weight: float
    confidence: float


@dataclass
class Tension:
    description: str
    mirror_signal: str
    dog_signal: str
    frequency: int
    first_seen: str  # ISO datetime
    last_seen: str   # ISO datetime


@dataclass
class BehavioralProfile:
    # Temporal patterns
    activity_hours: dict[int, float] = field(default_factory=dict)
    peak_windows: list[TimeWindow] = field(default_factory=list)
    session_duration_dist: Distribution = field(default_factory=Distribution.empty)
    context_switch_rate: float = 0.0

    # Content preferences
    narrative_affinity: dict[str, float] = field(default_factory=dict)
    author_affinity: dict[str, float] = field(default_factory=dict)
    content_length_pref: Distribution = field(default_factory=Distribution.empty)
    thread_vs_single: float = 0.0
    media_preference: dict[str, float] = field(default_factory=dict)
    dwell_by_content_type: dict[str, float] = field(default_factory=dict)

    # Machine usage
    app_time_distribution: dict[str, float] = field(default_factory=dict)
    coding_vs_browsing_ratio: float = 0.0

    # Decision patterns
    bookmark_predictors: list[Feature] = field(default_factory=list)
    ignore_predictors: list[Feature] = field(default_factory=list)
    tension_zones: list[Tension] = field(default_factory=list)

    # Meta
    pattern_stability: dict[str, float] = field(default_factory=dict)
    blind_spots: list[str] = field(default_factory=list)
    profile_version: str = "0.1.0"
    updated_at: str = ""
    observation_count: int = 0

    @classmethod
    def empty(cls) -> BehavioralProfile:
        return cls(updated_at=datetime.now(timezone.utc).isoformat())

    def to_json(self) -> str:
        return json.dumps(asdict(self), indent=2, default=str)

    @classmethod
    def from_json(cls, raw: str) -> BehavioralProfile:
        data = json.loads(raw)
        # Reconstruct nested types
        if "peak_windows" in data:
            data["peak_windows"] = [TimeWindow(**w) for w in data["peak_windows"]]
        for dist_field in ("session_duration_dist", "content_length_pref"):
            if dist_field in data and isinstance(data[dist_field], dict):
                data[dist_field] = Distribution(**data[dist_field])
        if "bookmark_predictors" in data:
            data["bookmark_predictors"] = [Feature(**f) for f in data["bookmark_predictors"]]
        if "ignore_predictors" in data:
            data["ignore_predictors"] = [Feature(**f) for f in data["ignore_predictors"]]
        if "tension_zones" in data:
            data["tension_zones"] = [Tension(**t) for t in data["tension_zones"]]
        # activity_hours keys must be int
        if "activity_hours" in data:
            data["activity_hours"] = {int(k): v for k, v in data["activity_hours"].items()}
        return cls(**data)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_profile.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/__init__.py cynic-python/organs/mirror/profile.py \
       cynic-python/organs/mirror/tests/__init__.py cynic-python/organs/mirror/tests/test_profile.py
git commit -m "feat(mirror): profile data model + confidence formula + serde"
```

---

## Task 2: Checkpoint Persistence + Crash Recovery

**Files:**
- Create: `cynic-python/organs/mirror/checkpoint.py`
- Create: `cynic-python/organs/mirror/tests/test_checkpoint.py`

- [ ] **Step 1: Write test for checkpoint save/load/recovery**

```python
# tests/test_checkpoint.py
"""Tests for ProfileCheckpoint persistence and crash recovery."""
import json
import tempfile
from datetime import datetime, timezone
from pathlib import Path

from organs.mirror.checkpoint import ProfileCheckpoint
from organs.mirror.profile import BehavioralProfile


def test_checkpoint_save_and_load(tmp_path: Path) -> None:
    """Checkpoint roundtrips through disk without data loss."""
    profile = BehavioralProfile.empty()
    profile.observation_count = 100
    profile.activity_hours = {21: 0.4}

    cp = ProfileCheckpoint(
        profile=profile,
        cursors={"behavior": 5000, "x_signals": 1200},
        askesis_sent_today=2,
        askesis_date="2026-05-18",
    )

    path = tmp_path / "profile.json"
    cp.save(path)

    loaded = ProfileCheckpoint.load(path)
    assert loaded.profile.observation_count == 100
    assert loaded.cursors["behavior"] == 5000
    assert loaded.askesis_sent_today == 2


def test_checkpoint_load_missing_file(tmp_path: Path) -> None:
    """Missing checkpoint file returns fresh checkpoint."""
    path = tmp_path / "nonexistent.json"
    loaded = ProfileCheckpoint.load(path)
    assert loaded.profile.observation_count == 0
    assert loaded.cursors == {}


def test_checkpoint_askesis_throttle() -> None:
    """Throttle respects 3/day limit and resets on date change."""
    cp = ProfileCheckpoint(
        profile=BehavioralProfile.empty(),
        cursors={},
        askesis_sent_today=3,
        askesis_date="2026-05-18",
    )
    assert not cp.can_send_askesis("2026-05-18")
    assert cp.can_send_askesis("2026-05-19")  # new day resets


def test_checkpoint_increment_askesis() -> None:
    """Incrementing askesis counter updates date and count."""
    cp = ProfileCheckpoint(
        profile=BehavioralProfile.empty(),
        cursors={},
        askesis_sent_today=0,
        askesis_date="2026-05-17",
    )
    cp.record_askesis_sent("2026-05-18")
    assert cp.askesis_sent_today == 1
    assert cp.askesis_date == "2026-05-18"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_checkpoint.py -v`
Expected: ModuleNotFoundError

- [ ] **Step 3: Implement checkpoint.py**

Atomic write via tmp+rename. Load with corrupt-file recovery (returns fresh). Askesis throttle counter with date-aware reset.

- [ ] **Step 4: Run tests**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_checkpoint.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/checkpoint.py cynic-python/organs/mirror/tests/test_checkpoint.py
git commit -m "feat(mirror): checkpoint persistence with crash recovery + askesis throttle"
```

---

## Task 3: Behavior Log Source (Tail + Parse)

**Files:**
- Create: `cynic-python/organs/mirror/sources/__init__.py`
- Create: `cynic-python/organs/mirror/sources/base.py`
- Create: `cynic-python/organs/mirror/sources/behavior.py`
- Create: `cynic-python/organs/mirror/tests/test_behavior_source.py`

- [ ] **Step 1: Write test for behavior source parsing + tailing**

Tests cover: parse click, parse key, skip malformed lines, skip health_checkpoint, read from offset, track current_offset. See spec for event format: `{"type":"click","x":500,"y":300,"button":"left","ts":"...","window_id":"0x1","window_name":"Chrome","url":"https://x.com"}`.

Real event types to consume: `click`, `key`, `mouse_move`, `scroll`. Skip: `health_checkpoint`, and any entries without a `type` field (stale analysis entries exist in the log).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_behavior_source.py -v`
Expected: ModuleNotFoundError

- [ ] **Step 3: Implement base.py (Event dataclass + Source protocol) and behavior.py**

`Event(source, event_type, timestamp, data)`. `BehaviorSource` reads JSONL line-by-line from offset, parses JSON, filters by CONSUMABLE_TYPES, tracks error_count and current_offset.

- [ ] **Step 4: Run tests**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_behavior_source.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/sources/
git commit -m "feat(mirror): behavior_log source — parse, tail, offset tracking"
```

---

## Task 4: X Signals Source (dataset.jsonl)

**Files:**
- Create: `cynic-python/organs/mirror/sources/x_signals.py`
- Create: `cynic-python/organs/mirror/tests/test_x_source.py`

- [ ] **Step 1: Write test for dataset.jsonl parsing**

Tests cover: parse standard tweet, detect bookmarked tweets (`viewer_bookmarked: true` → event_type `tweet_bookmarked`), skip malformed rows, skip rows without `tweet_id`.

Real dataset fields (observed): `tweet_id`, `text`, `author_screen_name`, `author_followers_count`, `capture_ts`, `signal_score`, `narratives`, `likes`, `retweets`, `views`, `engagement_rate`, `viewer_bookmarked`, `has_media`, `is_self_thread`, plus 30+ other fields.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_x_source.py -v`
Expected: ModuleNotFoundError

- [ ] **Step 3: Implement x_signals.py**

Same pattern as behavior source: JSONL reader, offset tracking, error counting. Event type classification based on `viewer_bookmarked` field.

- [ ] **Step 4: Run tests**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_x_source.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/sources/x_signals.py cynic-python/organs/mirror/tests/test_x_source.py
git commit -m "feat(mirror): X signals source — parse tweets + bookmark detection"
```

---

## Task 5: Online Learner (EMA + Profile Updates)

**Files:**
- Create: `cynic-python/organs/mirror/learner.py`
- Create: `cynic-python/organs/mirror/tests/test_learner.py`

- [ ] **Step 1: Write test for learner ingestion**

Tests cover: activity_hours update from click events, narrative_affinity from tweets, bookmarked tweets boost affinity, app_time from window_name, delta reports changed fields, askesis trigger when confidence crosses phi^-2.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_learner.py -v`
Expected: ModuleNotFoundError

- [ ] **Step 3: Implement learner.py**

EMA updates (alpha=0.01) for each profile field. Methods: `ingest(event) -> ProfileDelta`, `should_emit_askesis(delta) -> bool`. Internal: `_update_activity_hours`, `_update_narrative_affinity`, `_update_author_affinity`, `_update_app_time`. Bookmark events get 5x weight; seen tweets get 1x. `ProfileDelta(changed_fields: set[str], max_confidence_change: float)`.

- [ ] **Step 4: Run tests**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/test_learner.py -v`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/learner.py cynic-python/organs/mirror/tests/test_learner.py
git commit -m "feat(mirror): online learner — EMA profile updates from events"
```

---

## Task 6: Askesis Insight Generator

**Files:**
- Create: `cynic-python/organs/mirror/askesis.py`
- Create: `cynic-python/organs/mirror/tests/test_askesis.py`

- [ ] **Step 1: Write test for askesis insight generation**

Tests cover: temporal insight (strong activity peak), content preference insight (skewed narrative affinity), no insight on small delta.

- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement askesis.py**

`generate_insight(profile, delta) -> Insight | None`. Insight types: TEMPORAL, CONTENT_PREFERENCE, APP_USAGE, BLIND_SPOT. Each insight has `message: str`, `insight_type: InsightType`, `confidence: float`, `observation_count: int`. Returns None when delta is insignificant.

- [ ] **Step 4: Run tests**
- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/askesis.py cynic-python/organs/mirror/tests/test_askesis.py
git commit -m "feat(mirror): askesis insight generator — temporal, content, blind spots"
```

---

## Task 7: Action Gate + Agent Task Dispatch

**Files:**
- Create: `cynic-python/organs/mirror/action.py`
- Create: `cynic-python/organs/mirror/tests/test_action.py`

- [ ] **Step 1: Write test for action gating**

Tests cover: below phi^-2 → OBSERVE, above phi^-2 → ACT, task content includes top narratives/authors.

- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement action.py**

`ActionGate.evaluate(profile, domain) -> ActionDecision`. `ActionDecision.OBSERVE | ACT`. `build_task_content(profile) -> str` generates Hermes-readable task prompt from profile.

- [ ] **Step 4: Run tests**
- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/action.py cynic-python/organs/mirror/tests/test_action.py
git commit -m "feat(mirror): action gate — phi^-2 threshold + Hermes task builder"
```

---

## Task 8: Prediction Records + Outcome Join

**Files:**
- Create: `cynic-python/organs/mirror/predictions.py`
- Create: `cynic-python/organs/mirror/tests/test_predictions.py`

- [ ] **Step 1: Write test for prediction lifecycle**

Tests cover: record + retrieve, confirm via matching event, expire after 24h.

- [ ] **Step 2: Run test to verify it fails**
- [ ] **Step 3: Implement predictions.py**

`PredictionStore(path)`. `record(type, target_id, confidence, features) -> Prediction`. `check_outcomes(events) -> list[Prediction]`. Append-only JSONL. 24h expiry.

- [ ] **Step 4: Run tests**
- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/mirror/predictions.py cynic-python/organs/mirror/tests/test_predictions.py
git commit -m "feat(mirror): prediction records — store, join, expire"
```

---

## Task 9: Coordinator (Event Loop)

**Files:**
- Create: `cynic-python/organs/mirror/coordinator.py`

- [ ] **Step 1: Implement coordinator.py**

Event loop: load checkpoint → init sources with cursors → tail each source → route events to learner → check askesis/action/predictions → checkpoint every 1000 events / 15min. `run_once()` for testing. `run_forever()` for daemon. K21 guard: filter own observations from kernel polls.

- [ ] **Step 2: Write integration test**

Test that coordinator processes events end-to-end with temp files: events ingested → profile updated → checkpoint saved → cursors advanced.

- [ ] **Step 3: Run tests**

Run: `cd cynic-python && python -m pytest organs/mirror/tests/ -v`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add cynic-python/organs/mirror/coordinator.py
git commit -m "feat(mirror): coordinator event loop — tail, learn, emit, checkpoint"
```

---

## Task 10: Daemon Entry Point + Systemd

**Files:**
- Create: `cynic-python/organs/mirror/daemon.py`
- Create: `infra/systemd/mirror-agent.service`
- Modify: `scripts/hermes-x/core/hermes_paths.py` (add mirror paths)

- [ ] **Step 1: Implement daemon.py**

Thin boot wrapper: parse env vars (`CYNIC_REST_ADDR`, `CYNIC_API_KEY`, `X_DATASET_PATH`), setup structured JSON logging (P5), register SIGTERM/SIGINT → graceful shutdown, emit startup observation to kernel, launch coordinator.

- [ ] **Step 2: Create systemd service**

```ini
# infra/systemd/mirror-agent.service
[Unit]
Description=CYNIC Mirror Agent v0.1.0 — behavioral model daemon
After=network-online.target

[Service]
Type=simple
ExecStart=%h/.cynic/organs/mirror/.venv/bin/python -m organs.mirror.daemon
WorkingDirectory=%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/cynic-python
Environment=CYNIC_REST_ADDR=<TAILSCALE_CORE>:3030
EnvironmentFile=%h/.cynic-env
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

No `User=` / `Group=` directives (SYS1). ExecStart uses absolute venv path.

- [ ] **Step 3: Add mirror paths to hermes_paths.py**

Append to `scripts/hermes-x/core/hermes_paths.py`:
```python
MIRROR_DIR = HERMES_DIR.parent / "mirror"
MIRROR_PROFILE = MIRROR_DIR / "behavioral_profile.json"
MIRROR_PREDICTIONS = MIRROR_DIR / "predictions.jsonl"
```

- [ ] **Step 4: Commit**

```bash
git add cynic-python/organs/mirror/daemon.py infra/systemd/mirror-agent.service \
       scripts/hermes-x/core/hermes_paths.py
git commit -m "feat(mirror): daemon entry point + systemd service + path wiring"
```

---

## Task 11: Venv Setup + Smoke Test

- [ ] **Step 1: Create mirror venv**

```bash
python3 -m venv ~/.cynic/organs/mirror/.venv
~/.cynic/organs/mirror/.venv/bin/pip install pydantic requests
```

- [ ] **Step 2: Run full test suite**

```bash
cd cynic-python && python -m pytest organs/mirror/tests/ -v --tb=short
```

Expected: All tests pass.

- [ ] **Step 3: Smoke test with real data**

Run coordinator.run_once() against real behavior_log.jsonl (306K+ events) and dataset.jsonl. Verify: events processed, profile populated, activity_hours non-empty, narrative_affinity non-empty. This is a manual verification, not a committed test.

- [ ] **Step 4: Install and start systemd service**

```bash
cp infra/systemd/mirror-agent.service ~/.config/systemd/user/
systemctl --user daemon-reload && systemctl --user enable mirror-agent
systemctl --user start mirror-agent
systemctl --user status mirror-agent
journalctl --user -xeu mirror-agent | tail -20
```

Verify: Active (running), no errors in journal, structured JSON log output.

- [ ] **Step 5: Verify K15 pipe end-to-end**

After daemon runs ~5 minutes: checkpoint file exists at `~/.cynic/organs/mirror/behavioral_profile.json`, observation_count > 0, mirror-lifecycle observation in kernel (if kernel is up).

- [ ] **Step 6: Final commit**

```bash
git add -A && git commit -m "feat(mirror): smoke test pass — daemon operational"
```

---

## Task Dependency Graph

```
Task 1 (Profile)
    ├── Task 2 (Checkpoint)
    ├── Task 3 (Behavior Source)
    ├── Task 4 (X Signals Source)
    ├── Task 5 (Learner)
    ├── Task 6 (Askesis)
    ├── Task 7 (Action Gate)
    └── Task 8 (Predictions)
Task 9 (Coordinator) ← depends on ALL above
Task 10 (Daemon + Systemd) ← depends on coordinator
Task 11 (Smoke Test) ← depends on everything
```

Tasks 2-8 can be parallelized after Task 1 (all depend only on profile types). Tasks 9-11 are sequential.
