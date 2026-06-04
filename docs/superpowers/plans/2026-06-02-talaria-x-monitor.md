# Talaria X Monitor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect real-time interactions with @TalariaBuild on X and alert the Telegram Ops group, with a data-centric `pending/processed/` pipeline.

**Architecture:** `notification_poller.py` reads the X notifications page via CDP every 5min → writes `{tweet_id}.json` to `pending/`. `search_executor.py` sweeps MetaDAO/Solana queries → also writes to `pending/`. `talaria_alerter.py` runs at offset +30s → reads `pending/`, sends Telegram, moves files to `processed/`.

**Tech Stack:** Python 3.11+, playwright (async), python-telegram-bot OR raw requests to Bot API, systemd timers, existing `HubClient`, `hermes_paths.py`.

**Spec:** `docs/superpowers/specs/2026-06-02-talaria-x-monitor-design.md`

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `scripts/hermes-x/core/hermes_paths.py` | Add `PENDING_DIR`, `PROCESSED_DIR`, `POLLER_STATE` constants |
| Create | `scripts/hermes-x/core/notification_poller.py` | CDP navigate /notifications → parse → write pending/ |
| Create | `scripts/hermes-x/core/talaria_alerter.py` | Read pending/ → Telegram → move to processed/ |
| Create | `scripts/hermes-x/core/test_notification_poller.py` | Unit tests for poller (mock CDP) |
| Create | `scripts/hermes-x/core/test_talaria_alerter.py` | Unit tests for alerter (mock Telegram) |
| Modify | `scripts/hermes-x/core/search_executor.py` | Route talaria-*/metadao-* results to pending/ |
| Modify | `/home/user/.cynic/organs/hermes/x/search_tasks.jsonl` | Add 4 Talaria sweep queries |
| Create | `infra/systemd/hermes-notification-poller.service` | Systemd service |
| Create | `infra/systemd/hermes-notification-poller.timer` | Every 5min |
| Create | `infra/systemd/hermes-notification-alerter.service` | Systemd service |
| Create | `infra/systemd/hermes-notification-alerter.timer` | Every 5min +30s offset |

---

## Task 1: hermes_paths.py — ajouter les constantes pending/processed

**Files:**
- Modify: `scripts/hermes-x/core/hermes_paths.py`

- [ ] **Step 1.1 : Lire le fichier actuel**

```bash
grep -n "CAPTURES_DIR\|DATASET\|SEARCH_TASKS\|NAVIGATION_LOG" scripts/hermes-x/core/hermes_paths.py
```
Expected: lignes ~75-84 montrant les constantes existantes.

- [ ] **Step 1.2 : Ajouter les constantes après `NAVIGATION_LOG`**

Dans `scripts/hermes-x/core/hermes_paths.py`, après la ligne `NAVIGATION_LOG = ...`, ajouter :

```python
# Talaria monitor — pending/processed pipeline
PENDING_DIR = HERMES_X_DIR / "pending"
PROCESSED_DIR = HERMES_X_DIR / "processed"
POLLER_STATE = HERMES_X_DIR / "poller_state.json"
```

- [ ] **Step 1.3 : Créer les répertoires sur le système de fichiers**

```bash
mkdir -p ~/.cynic/organs/hermes/x/pending
mkdir -p ~/.cynic/organs/hermes/x/processed
```

Expected: exit 0, pas d'erreur.

- [ ] **Step 1.4 : Vérifier que l'import fonctionne**

```bash
cd scripts/hermes-x/core && python3 -c "from hermes_paths import PENDING_DIR, PROCESSED_DIR, POLLER_STATE; print(PENDING_DIR, PROCESSED_DIR)"
```
Expected: affiche les deux chemins sous `~/.cynic/organs/hermes/x/`.

- [ ] **Step 1.5 : Commit**

```bash
git add scripts/hermes-x/core/hermes_paths.py
git commit -m "feat(hermes): add PENDING_DIR/PROCESSED_DIR/POLLER_STATE to hermes_paths"
```

---

## Task 2: Tests unitaires du poller (TDD — écrire avant le code)

**Files:**
- Create: `scripts/hermes-x/core/test_notification_poller.py`

- [ ] **Step 2.1 : Créer le fichier de tests**

```python
# scripts/hermes-x/core/test_notification_poller.py
"""Unit tests for notification_poller — all CDP calls mocked."""
import json
import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from pathlib import Path
from unittest.mock import MagicMock, patch, AsyncMock
import pytest
import tempfile


# ── helpers ────────────────────────────────────────────────────────────────

def _make_notif_html_item(tweet_id: str, author: str, text: str, notif_type: str = "mention") -> MagicMock:
    """Fake Playwright locator item representing one notification DOM element."""
    item = MagicMock()
    # inner_text returns combined text
    item.inner_text = AsyncMock(return_value=f"@{author} {notif_type} your Tweet\n{text}")
    # locator for tweet link
    link = MagicMock()
    link.get_attribute = AsyncMock(return_value=f"/{author}/status/{tweet_id}")
    item.locator = MagicMock(return_value=link)
    return item


# ── parse_notification ──────────────────────────────────────────────────────

def test_parse_notification_mention():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@alice mentioned you in a Tweet\nHey $TALARIA is launching!")
    assert result["type"] == "mention"
    assert "TALARIA" in result["text"]


def test_parse_notification_reply():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@bob replied to your Tweet\nWhen is the ICO?")
    assert result["type"] == "reply"


def test_parse_notification_retweet():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@carol retweeted your Tweet")
    assert result["type"] == "retweet"


def test_parse_notification_like():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@dave liked your Tweet")
    assert result["type"] == "like"


def test_parse_notification_unknown_defaults_to_notification():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@eve followed you")
    assert result["type"] == "notification"


# ── build_interaction_entry ─────────────────────────────────────────────────

def test_build_entry_has_required_fields():
    from notification_poller import build_interaction_entry
    entry = build_interaction_entry(
        tweet_id="999",
        notif_type="mention",
        author="alice",
        text="Hello Talaria",
        source="notification_poller",
    )
    assert entry["schema_version"] == 1
    assert entry["tweet_id"] == "999"
    assert entry["type"] == "mention"
    assert entry["author"] == "@alice"
    assert entry["source"] == "notification_poller"
    assert "detected_at" in entry
    assert entry["url"] == "https://x.com/alice/status/999"


def test_build_entry_normalises_author_handle():
    from notification_poller import build_interaction_entry
    # Author with @ prefix should not be doubled
    entry = build_interaction_entry("1", "mention", "@alice", "text", "notification_poller")
    assert entry["author"] == "@alice"


# ── dedup logic ──────────────────────────────────────────────────────────────

def test_is_already_seen_pending(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (pending / "tweet_123.json").write_text("{}")
    assert is_already_seen("123", pending, processed) is True


def test_is_already_seen_processed(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (processed / "tweet_456.json").write_text("{}")
    assert is_already_seen("456", pending, processed) is True


def test_is_not_seen(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    assert is_already_seen("789", pending, processed) is False


# ── write_pending ────────────────────────────────────────────────────────────

def test_write_pending_creates_file(tmp_path):
    from notification_poller import write_pending
    pending = tmp_path / "pending"
    pending.mkdir()
    entry = {"schema_version": 1, "tweet_id": "abc", "type": "mention"}
    write_pending(entry, pending)
    out = pending / "tweet_abc.json"
    assert out.exists()
    data = json.loads(out.read_text())
    assert data["tweet_id"] == "abc"


# ── monitor_blind logic ───────────────────────────────────────────────────────

def test_monitor_blind_resets_on_nonzero(tmp_path):
    from notification_poller import update_poller_state, load_poller_state
    state_file = tmp_path / "poller_state.json"
    update_poller_state(state_file, found_count=3)
    state = load_poller_state(state_file)
    assert state["consecutive_empty_cycles"] == 0


def test_monitor_blind_increments_on_zero(tmp_path):
    from notification_poller import update_poller_state, load_poller_state
    state_file = tmp_path / "poller_state.json"
    update_poller_state(state_file, found_count=0)
    update_poller_state(state_file, found_count=0)
    state = load_poller_state(state_file)
    assert state["consecutive_empty_cycles"] == 2


def test_monitor_blind_triggers_at_3(tmp_path):
    from notification_poller import update_poller_state, should_emit_blind_alert
    state_file = tmp_path / "poller_state.json"
    for _ in range(3):
        update_poller_state(state_file, found_count=0)
    assert should_emit_blind_alert(state_file) is True


def test_monitor_blind_does_not_trigger_at_2(tmp_path):
    from notification_poller import update_poller_state, should_emit_blind_alert
    state_file = tmp_path / "poller_state.json"
    for _ in range(2):
        update_poller_state(state_file, found_count=0)
    assert should_emit_blind_alert(state_file) is False
```

- [ ] **Step 2.2 : Vérifier que les tests échouent (module absent)**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_notification_poller.py -v 2>&1 | head -20
```
Expected: `ModuleNotFoundError: No module named 'notification_poller'` — FAIL confirmé.

- [ ] **Step 2.3 : Commit les tests**

```bash
git add scripts/hermes-x/core/test_notification_poller.py
git commit -m "test(hermes): notification_poller unit tests (TDD — red)"
```

---

## Task 3: Implémenter `notification_poller.py`

**Files:**
- Create: `scripts/hermes-x/core/notification_poller.py`

- [ ] **Step 3.1 : Créer notification_poller.py**

```python
#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Notification Poller — reads @TalariaBuild X notifications via CDP.

Navigates to x.com/notifications in hermes-browser (real Chrome, already logged in
as @TalariaBuild). Writes each new interaction to pending/{tweet_id}.json.
Tracks consecutive empty cycles for monitor-blind detection.

K15 Consumer: talaria_alerter.py (reads pending/)
Systemd: hermes-notification-poller.timer (every 5min)
Failure mode: CDP unavailable → exit 0 (timer retries)
"""

__version__ = "0.1.0"

import asyncio
import json
import logging
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    from playwright.async_api import async_playwright, TimeoutError as PwTimeout
except ImportError:
    print("ERROR: playwright not installed. Run: pip install playwright && playwright install chromium")
    sys.exit(1)

from hermes_paths import PENDING_DIR, PROCESSED_DIR, POLLER_STATE, HERMES_X_DIR

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("notification-poller")

BROWSER_STATE = HERMES_X_DIR.parent / "browser-state.json"
CDP_TOTAL_TIMEOUT_MS = 9000   # P18: hard cap
DOM_WAIT_TIMEOUT_MS  = 6000   # 6s for notifications selector


# ── Pure helpers (testable without Playwright) ──────────────────────────────

def parse_notification_text(text: str) -> dict:
    """Classify notification type from inner text. Returns {type, text}."""
    lower = text.lower()
    if "retweeted" in lower:
        notif_type = "retweet"
    elif "liked" in lower:
        notif_type = "like"
    elif "replied" in lower:
        notif_type = "reply"
    elif "mentioned" in lower:
        notif_type = "mention"
    else:
        notif_type = "notification"
    # Strip the social boilerplate — keep only the tweet body (after first \n)
    lines = text.strip().splitlines()
    body = lines[-1].strip() if len(lines) > 1 else text.strip()
    return {"type": notif_type, "text": body[:280]}


def build_interaction_entry(
    tweet_id: str,
    notif_type: str,
    author: str,
    text: str,
    source: str,
    keywords: list[str] | None = None,
) -> dict:
    """Build a canonical interaction entry dict."""
    author_clean = author if author.startswith("@") else f"@{author}"
    handle = author_clean.lstrip("@")
    return {
        "schema_version": 1,
        "tweet_id": tweet_id,
        "type": notif_type,
        "author": author_clean,
        "author_followers": None,
        "text": text[:280],
        "url": f"https://x.com/{handle}/status/{tweet_id}",
        "detected_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": source,
        "keywords_matched": keywords or [],
    }


def is_already_seen(tweet_id: str, pending: Path, processed: Path) -> bool:
    return (pending / f"tweet_{tweet_id}.json").exists() or \
           (processed / f"tweet_{tweet_id}.json").exists()


def write_pending(entry: dict, pending: Path) -> None:
    pending.mkdir(parents=True, exist_ok=True)
    out = pending / f"tweet_{entry['tweet_id']}.json"
    out.write_text(json.dumps(entry, ensure_ascii=False))


def load_poller_state(state_file: Path) -> dict:
    if state_file.exists():
        try:
            return json.loads(state_file.read_text())
        except (json.JSONDecodeError, OSError):
            pass
    return {"consecutive_empty_cycles": 0}


def update_poller_state(state_file: Path, found_count: int) -> dict:
    state = load_poller_state(state_file)
    if found_count > 0:
        state["consecutive_empty_cycles"] = 0
    else:
        state["consecutive_empty_cycles"] = state.get("consecutive_empty_cycles", 0) + 1
    state_file.write_text(json.dumps(state))
    return state


def should_emit_blind_alert(state_file: Path) -> bool:
    state = load_poller_state(state_file)
    return state.get("consecutive_empty_cycles", 0) >= 3


def write_blind_alert(pending: Path) -> None:
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    entry = {
        "schema_version": 1,
        "tweet_id": f"MONITOR_BLIND_{ts}",
        "type": "monitor_blind",
        "author": "@system",
        "author_followers": None,
        "text": "3 consecutive empty notification cycles. Check: hermes-browser active? X session valid? DOM selector changed?",
        "url": "",
        "detected_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": "notification_poller",
        "keywords_matched": [],
    }
    fname = f"MONITOR_BLIND_{ts}.json"
    (pending / fname).write_text(json.dumps(entry, ensure_ascii=False))
    logger.warning("MONITOR BLIND emitted: %s", fname)


# ── CDP logic ────────────────────────────────────────────────────────────────

def _get_cdp_url() -> str | None:
    if not BROWSER_STATE.exists():
        logger.warning("browser-state.json not found at %s", BROWSER_STATE)
        return None
    try:
        state = json.loads(BROWSER_STATE.read_text())
        return state.get("cdp_url")
    except Exception as e:
        logger.error("failed to read browser-state.json: %s", e)
        return None


async def _extract_notifications(page) -> list[dict]:
    """Navigate to /notifications and extract all visible items."""
    await page.goto("https://x.com/notifications", wait_until="domcontentloaded", timeout=CDP_TOTAL_TIMEOUT_MS)
    try:
        await page.wait_for_selector('[data-testid="notification"]', timeout=DOM_WAIT_TIMEOUT_MS)
    except PwTimeout:
        logger.info("No notification elements found within %dms", DOM_WAIT_TIMEOUT_MS)
        return []

    items = await page.locator('[data-testid="notification"]').all()
    results = []
    for item in items:
        try:
            # Extract tweet URL for tweet_id
            link_el = item.locator('a[href*="/status/"]').first
            href = await link_el.get_attribute("href", timeout=500)
            if not href:
                continue
            # href: /author/status/tweet_id
            parts = href.strip("/").split("/")
            if len(parts) < 3 or parts[1] != "status":
                continue
            author = parts[0]
            tweet_id = parts[2]

            # Inner text for type detection
            inner = await item.inner_text(timeout=500)
            parsed = parse_notification_text(inner)
            results.append({
                "tweet_id": tweet_id,
                "author": author,
                "type": parsed["type"],
                "text": parsed["text"],
            })
        except Exception as e:
            logger.debug("skipping notification item: %s", e)
            continue
    return results


async def run() -> int:
    cdp_url = _get_cdp_url()
    if not cdp_url:
        logger.warning("CDP unavailable, exiting cleanly")
        return 0

    http_endpoint = cdp_url.replace("ws://", "http://")
    found_count = 0

    try:
        async with async_playwright() as p:
            browser = await p.chromium.connect_over_cdp(http_endpoint)
            contexts = browser.contexts
            context = contexts[0] if contexts else await browser.new_context()
            page = await context.new_page()

            raw_items = await _extract_notifications(page)
            await page.close()

            PENDING_DIR.mkdir(parents=True, exist_ok=True)
            PROCESSED_DIR.mkdir(parents=True, exist_ok=True)

            for raw in raw_items:
                if is_already_seen(raw["tweet_id"], PENDING_DIR, PROCESSED_DIR):
                    logger.debug("skip seen: %s", raw["tweet_id"])
                    continue
                entry = build_interaction_entry(
                    tweet_id=raw["tweet_id"],
                    notif_type=raw["type"],
                    author=raw["author"],
                    text=raw["text"],
                    source="notification_poller",
                )
                write_pending(entry, PENDING_DIR)
                found_count += 1
                logger.info("new: %s by @%s (%s)", raw["tweet_id"], raw["author"], raw["type"])

    except Exception as e:
        logger.error("CDP error: %s", str(e)[:120])
        return 0  # non-fatal — timer retries

    state = update_poller_state(POLLER_STATE, found_count)
    logger.info("done: %d new, %d consecutive_empty=%d",
                found_count, 0, state["consecutive_empty_cycles"])

    if should_emit_blind_alert(POLLER_STATE):
        write_blind_alert(PENDING_DIR)
        # Reset counter after emitting so we don't spam
        update_poller_state(POLLER_STATE, found_count=1)

    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(run()))
```

- [ ] **Step 3.2 : Lancer les tests unitaires**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_notification_poller.py -v
```
Expected: tous les tests passent (PASS).

- [ ] **Step 3.3 : Commit**

```bash
git add scripts/hermes-x/core/notification_poller.py
git commit -m "feat(hermes): notification_poller — CDP /notifications → pending/ pipeline"
```

---

## Task 4: Tests unitaires de l'alerter (TDD)

**Files:**
- Create: `scripts/hermes-x/core/test_talaria_alerter.py`

- [ ] **Step 4.1 : Créer test_talaria_alerter.py**

```python
# scripts/hermes-x/core/test_talaria_alerter.py
"""Unit tests for talaria_alerter — Telegram calls mocked."""
import json
import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from pathlib import Path
from unittest.mock import patch, MagicMock
import pytest


# ── format_telegram_message ────────────────────────────────────────────────

def test_format_normal_interaction():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "mention",
        "author": "@alice",
        "author_followers": 12400,
        "text": "MetaDAO ICO is live!",
        "url": "https://x.com/alice/status/123",
        "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z",
    }
    msg = format_telegram_message(entry)
    assert "🔔" in msg
    assert "@TalariaBuild" in msg
    assert "mention" in msg
    assert "@alice" in msg
    assert "12.4K" in msg
    assert "MetaDAO ICO is live!" in msg
    assert "https://x.com/alice/status/123" in msg


def test_format_unknown_followers():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "reply",
        "author": "@bob",
        "author_followers": None,
        "text": "When launch?",
        "url": "https://x.com/bob/status/456",
        "source": "search_sweep",
        "detected_at": "2026-06-02T18:01:00Z",
    }
    msg = format_telegram_message(entry)
    assert "@bob" in msg
    assert "?" not in msg or "K followers" not in msg  # no follower count displayed


def test_format_monitor_blind():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "monitor_blind",
        "author": "@system",
        "author_followers": None,
        "text": "3 consecutive empty cycles.",
        "url": "",
        "source": "notification_poller",
        "detected_at": "2026-06-02T18:05:00Z",
    }
    msg = format_telegram_message(entry)
    assert "⚠️" in msg
    assert "MONITOR BLIND" in msg


# ── send_telegram ────────────────────────────────────────────────────────────

def test_send_telegram_returns_true_on_200():
    from talaria_alerter import send_telegram
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is True


def test_send_telegram_returns_false_on_429():
    from talaria_alerter import send_telegram
    mock_resp = MagicMock()
    mock_resp.status_code = 429
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is False


def test_send_telegram_returns_false_on_exception():
    from talaria_alerter import send_telegram
    with patch("talaria_alerter.requests.post", side_effect=Exception("network down")):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is False


# ── process_pending ──────────────────────────────────────────────────────────

def test_process_pending_moves_on_success(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()

    entry = {
        "schema_version": 1,
        "tweet_id": "abc",
        "type": "mention",
        "author": "@alice",
        "author_followers": None,
        "text": "test",
        "url": "https://x.com/alice/status/abc",
        "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z",
        "keywords_matched": [],
    }
    (pending / "tweet_abc.json").write_text(json.dumps(entry))

    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")

    assert sent == 1
    assert kept == 0
    assert not (pending / "tweet_abc.json").exists()
    assert (processed / "tweet_abc.json").exists()


def test_process_pending_keeps_on_429(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()

    entry = {
        "schema_version": 1, "tweet_id": "xyz", "type": "mention",
        "author": "@bob", "author_followers": None, "text": "test",
        "url": "https://x.com/bob/status/xyz", "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z", "keywords_matched": [],
    }
    (pending / "tweet_xyz.json").write_text(json.dumps(entry))

    mock_resp = MagicMock()
    mock_resp.status_code = 429
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")

    assert sent == 0
    assert kept == 1
    assert (pending / "tweet_xyz.json").exists()  # not moved


def test_process_pending_respects_batch_cap(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()

    # Create 25 pending files — batch cap is 20
    for i in range(25):
        entry = {
            "schema_version": 1, "tweet_id": str(i), "type": "mention",
            "author": "@user", "author_followers": None, "text": "t",
            "url": f"https://x.com/u/status/{i}", "source": "notification_poller",
            "detected_at": "2026-06-02T18:00:00Z", "keywords_matched": [],
        }
        (pending / f"tweet_{i}.json").write_text(json.dumps(entry))

    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")

    assert sent == 20
    assert kept == 5  # remaining — not an error, next cycle handles them


def test_process_pending_skips_invalid_json(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (pending / "tweet_bad.json").write_text("not json {{")

    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")

    assert sent == 0
    assert kept == 0  # invalid file is skipped (not moved, not sent)
```

- [ ] **Step 4.2 : Vérifier que les tests échouent**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_talaria_alerter.py -v 2>&1 | head -10
```
Expected: `ModuleNotFoundError: No module named 'talaria_alerter'` — FAIL confirmé.

- [ ] **Step 4.3 : Commit les tests**

```bash
git add scripts/hermes-x/core/test_talaria_alerter.py
git commit -m "test(hermes): talaria_alerter unit tests (TDD — red)"
```

---

## Task 5: Implémenter `talaria_alerter.py`

**Files:**
- Create: `scripts/hermes-x/core/talaria_alerter.py`

- [ ] **Step 5.1 : Créer talaria_alerter.py**

```python
#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Talaria Alerter — reads pending/, sends Telegram, moves to processed/.

Reads all *.json files in pending/. Sends each to TALARIA_OPS_CHAT_ID via Telegram Bot API.
On success → move to processed/. On 429 → stop batch (retry next cycle). Other errors → skip.
Batch cap: 20 messages per cycle (Telegram rate limit protection).

K15 Consumer: TALARIA_OPS_CHAT_ID (Telegram Ops group)
Systemd: hermes-notification-alerter.timer (every 5min, offset +30s)
Env required: CYNIC_TELEGRAM_BOT_TOKEN, TALARIA_OPS_CHAT_ID
"""

__version__ = "0.1.0"

import json
import logging
import os
import sys
from pathlib import Path

import requests

from hermes_paths import PENDING_DIR, PROCESSED_DIR

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("talaria-alerter")

BATCH_CAP = 20
TELEGRAM_TIMEOUT = 10  # seconds — R24

BOT_TOKEN = os.environ.get("CYNIC_TELEGRAM_BOT_TOKEN", "")
OPS_CHAT_ID = os.environ.get("TALARIA_OPS_CHAT_ID", "")


# ── Pure helpers (testable) ─────────────────────────────────────────────────

def _fmt_followers(count) -> str:
    if count is None:
        return ""
    if count >= 1000:
        return f"{count / 1000:.1f}K followers"
    return f"{count} followers"


def format_telegram_message(entry: dict) -> str:
    notif_type = entry.get("type", "notification")

    if notif_type == "monitor_blind":
        return (
            f"⚠️ MONITOR BLIND — notification_poller\n"
            f"{entry.get('text', '')}\n"
            f"📡 {entry.get('detected_at', '')} UTC"
        )

    author = entry.get("author", "@?")
    followers = _fmt_followers(entry.get("author_followers"))
    followers_str = f" ({followers})" if followers else ""
    text = entry.get("text", "")
    url = entry.get("url", "")
    source = entry.get("source", "?")
    detected_at = entry.get("detected_at", "")[:16].replace("T", " ")

    return (
        f"🔔 @TalariaBuild — {notif_type}\n"
        f"👤 {author}{followers_str}\n"
        f'💬 "{text}"\n'
        f"🔗 {url}\n"
        f"📡 {source} | {detected_at} UTC"
    )


def send_telegram(bot_token: str, chat_id: str, text: str) -> bool:
    """Send a message. Returns True on 200, False on any error or non-200."""
    try:
        resp = requests.post(
            f"https://api.telegram.org/bot{bot_token}/sendMessage",
            json={"chat_id": chat_id, "text": text},
            timeout=TELEGRAM_TIMEOUT,
        )
        if resp.status_code == 200:
            return True
        logger.warning("Telegram returned %d", resp.status_code)
        return False
    except Exception as e:
        logger.error("Telegram request failed: %s", e)
        return False


def process_pending(
    pending: Path,
    processed: Path,
    bot_token: str,
    chat_id: str,
) -> tuple[int, int]:
    """
    Process pending/*.json up to BATCH_CAP.
    Returns (sent_count, kept_count).
    Stops batch on 429 (rate limit).
    """
    processed.mkdir(parents=True, exist_ok=True)
    files = sorted(pending.glob("*.json"))[:BATCH_CAP]
    total = len(sorted(pending.glob("*.json")))
    sent = 0
    kept = total - len(files)  # files beyond cap stay for next cycle

    for f in files:
        try:
            entry = json.loads(f.read_text())
        except (json.JSONDecodeError, OSError) as e:
            logger.warning("skipping invalid file %s: %s", f.name, e)
            continue

        msg = format_telegram_message(entry)
        try:
            resp = requests.post(
                f"https://api.telegram.org/bot{bot_token}/sendMessage",
                json={"chat_id": chat_id, "text": msg},
                timeout=TELEGRAM_TIMEOUT,
            )
        except Exception as e:
            logger.error("Telegram request failed for %s: %s", f.name, e)
            kept += 1
            continue

        if resp.status_code == 200:
            dest = processed / f.name
            f.rename(dest)
            sent += 1
        elif resp.status_code == 429:
            logger.warning("429 rate limit — stopping batch, %d files kept", kept + (len(files) - sent))
            kept += len(files) - sent
            break
        else:
            logger.warning("Telegram %d for %s — keeping", resp.status_code, f.name)
            kept += 1

    return sent, kept


def main() -> int:
    if not BOT_TOKEN:
        logger.error("CYNIC_TELEGRAM_BOT_TOKEN not set")
        return 1
    if not OPS_CHAT_ID:
        logger.error("TALARIA_OPS_CHAT_ID not set")
        return 1

    PENDING_DIR.mkdir(parents=True, exist_ok=True)
    PROCESSED_DIR.mkdir(parents=True, exist_ok=True)

    pending_files = list(PENDING_DIR.glob("*.json"))
    if not pending_files:
        logger.info("pending/ empty, nothing to send")
        return 0

    sent, kept = process_pending(PENDING_DIR, PROCESSED_DIR, BOT_TOKEN, OPS_CHAT_ID)
    logger.info("done: %d sent, %d kept for next cycle", sent, kept)
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 5.2 : Lancer les tests**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_talaria_alerter.py -v
```
Expected: tous les tests PASS.

- [ ] **Step 5.3 : Lancer aussi les tests poller pour régression**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_notification_poller.py test_talaria_alerter.py -v
```
Expected: tous les tests PASS.

- [ ] **Step 5.4 : Commit**

```bash
git add scripts/hermes-x/core/talaria_alerter.py
git commit -m "feat(hermes): talaria_alerter — pending/ → Telegram → processed/ pipeline"
```

---

## Task 6: search_executor.py — router les résultats Talaria vers pending/

**Files:**
- Modify: `scripts/hermes-x/core/search_executor.py`

- [ ] **Step 6.1 : Lire la méthode execute_search**

```bash
sed -n '165,230p' scripts/hermes-x/core/search_executor.py
```
Repérer où les résultats de recherche sont écrits (SEARCH_RESULTS_LOG).

- [ ] **Step 6.2 : Ajouter l'import et la fonction de routing**

En haut de `search_executor.py`, dans le bloc d'imports, ajouter :

```python
from hermes_paths import PENDING_DIR, PROCESSED_DIR
from notification_poller import build_interaction_entry, is_already_seen, write_pending
```

Puis ajouter cette fonction après les imports :

```python
_TALARIA_LABELS = {"talaria-mention", "talaria-token", "talaria-cashtag", "metadao-ico"}

def _route_to_pending_if_talaria(task: dict, tweet_id: str, author: str, text: str, url: str) -> None:
    """If the search task has a talaria/metadao label, also write to pending/."""
    label = task.get("label", "")
    if label not in _TALARIA_LABELS:
        return
    if is_already_seen(tweet_id, PENDING_DIR, PROCESSED_DIR):
        return
    PENDING_DIR.mkdir(parents=True, exist_ok=True)
    PROCESSED_DIR.mkdir(parents=True, exist_ok=True)
    entry = build_interaction_entry(
        tweet_id=tweet_id,
        notif_type="mention",
        author=author,
        text=text,
        source="search_sweep",
        keywords=[task.get("query", "")],
    )
    entry["url"] = url
    write_pending(entry, PENDING_DIR)
```

- [ ] **Step 6.3 : Appeler `_route_to_pending_if_talaria` dans `execute_search`**

Dans `execute_search`, après avoir extrait les résultats (locator articles), pour chaque article extrait, appeler :

```python
# Après extraction du tweet_id, author, text, url :
_route_to_pending_if_talaria(task, tweet_id, author, text, url)
```

Note: chercher dans `execute_search` où `tweet_id` est extrait du href/article pour placer l'appel au bon endroit.

- [ ] **Step 6.4 : Vérifier que tous les tests passent encore**

```bash
cd scripts/hermes-x/core && python3 -m pytest test_notification_poller.py test_talaria_alerter.py -v
```
Expected: PASS.

- [ ] **Step 6.5 : Commit**

```bash
git add scripts/hermes-x/core/search_executor.py
git commit -m "feat(hermes): route talaria-*/metadao-* search results to pending/ pipeline"
```

---

## Task 7: search_tasks.jsonl — ajouter les 4 sweeps Talaria

**Files:**
- Modify: `/home/user/.cynic/organs/hermes/x/search_tasks.jsonl`

- [ ] **Step 7.1 : Vérifier les queries existantes**

```bash
cat ~/.cynic/organs/hermes/x/search_tasks.jsonl | python3 -c "import sys,json; [print(json.loads(l).get('query','')) for l in sys.stdin]"
```
Expected: liste des queries existantes (MetaDAO, Talaria etc. déjà ajoutées le 2026-06-01).

- [ ] **Step 7.2 : Vérifier si les nouvelles queries sont déjà présentes**

```bash
grep -c "talaria-mention\|talaria-cashtag\|metadao-ico\|talaria-token" ~/.cynic/organs/hermes/x/search_tasks.jsonl
```
Expected: 0 (pas encore présentes). Si > 0, skip ce step.

- [ ] **Step 7.3 : Ajouter les 4 nouvelles queries**

```bash
cat >> ~/.cynic/organs/hermes/x/search_tasks.jsonl << 'EOF'
{"query": "@TalariaBuild", "domain": "token_launch", "weight": 1.0, "label": "talaria-mention"}
{"query": "MetaDAO ICO Solana", "domain": "token_launch", "weight": 0.9, "label": "metadao-ico"}
{"query": "Talaria token Solana", "domain": "token_launch", "weight": 0.9, "label": "talaria-token"}
{"query": "$TALARIA", "domain": "token_launch", "weight": 1.0, "label": "talaria-cashtag"}
EOF
```

- [ ] **Step 7.4 : Vérifier le format**

```bash
cat ~/.cynic/organs/hermes/x/search_tasks.jsonl | python3 -c "import sys,json; tasks=[json.loads(l) for l in sys.stdin]; print(f'{len(tasks)} total tasks'); [print(t['query']) for t in tasks if t.get('label','').startswith('talaria') or t.get('label','').startswith('metadao')]"
```
Expected: affiche les 4 nouvelles queries.

- [ ] **Step 7.5 : Commit**

```bash
git add -f ~/.cynic/organs/hermes/x/search_tasks.jsonl 2>/dev/null || echo "file not tracked (expected, it's outside repo)"
# Ce fichier est hors repo — pas de commit git nécessaire pour ce step
```

---

## Task 8: Services et timers systemd

**Files:**
- Create: `infra/systemd/hermes-notification-poller.service`
- Create: `infra/systemd/hermes-notification-poller.timer`
- Create: `infra/systemd/hermes-notification-alerter.service`
- Create: `infra/systemd/hermes-notification-alerter.timer`

- [ ] **Step 8.1 : Créer hermes-notification-poller.service**

```ini
# infra/systemd/hermes-notification-poller.service
[Unit]
Description=CYNIC Hermes — Notification Poller (CDP /notifications → pending/)
After=network.target

[Service]
Type=oneshot
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 core/notification_poller.py
EnvironmentFile=%h/.config/cynic/env
StandardOutput=journal
StandardError=journal
```

- [ ] **Step 8.2 : Créer hermes-notification-poller.timer**

```ini
# infra/systemd/hermes-notification-poller.timer
[Unit]
Description=CYNIC Hermes — Notification Poller timer (every 5min)

[Timer]
OnCalendar=*:0/5
AccuracySec=10s
Persistent=true

[Install]
WantedBy=timers.target
```

- [ ] **Step 8.3 : Créer hermes-notification-alerter.service**

```ini
# infra/systemd/hermes-notification-alerter.service
[Unit]
Description=CYNIC Hermes — Talaria Alerter (pending/ → Telegram → processed/)
After=network.target

[Service]
Type=oneshot
WorkingDirectory=/home/user/.cynic/organs/hermes/x
ExecStart=/usr/bin/python3 core/talaria_alerter.py
EnvironmentFile=%h/.config/cynic/env
StandardOutput=journal
StandardError=journal
```

- [ ] **Step 8.4 : Créer hermes-notification-alerter.timer**

```ini
# infra/systemd/hermes-notification-alerter.timer
[Unit]
Description=CYNIC Hermes — Talaria Alerter timer (every 5min +30s offset)

[Timer]
OnCalendar=*:0/5:30
AccuracySec=10s
Persistent=true

[Install]
WantedBy=timers.target
```

- [ ] **Step 8.5 : Déployer et activer les 4 unités**

```bash
cp infra/systemd/hermes-notification-poller.service ~/.config/systemd/user/
cp infra/systemd/hermes-notification-poller.timer ~/.config/systemd/user/
cp infra/systemd/hermes-notification-alerter.service ~/.config/systemd/user/
cp infra/systemd/hermes-notification-alerter.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hermes-notification-poller.timer
systemctl --user enable --now hermes-notification-alerter.timer
```

- [ ] **Step 8.6 : Vérifier le statut**

```bash
systemctl --user status hermes-notification-poller.timer hermes-notification-alerter.timer
```
Expected: `Active: active (waiting)` pour les deux timers.

- [ ] **Step 8.7 : Commit les fichiers systemd**

```bash
git add infra/systemd/hermes-notification-poller.service \
        infra/systemd/hermes-notification-poller.timer \
        infra/systemd/hermes-notification-alerter.service \
        infra/systemd/hermes-notification-alerter.timer
git commit -m "feat(infra): systemd timers for notification_poller + talaria_alerter"
```

---

## Task 9: K15 Falsification — test end-to-end

**But:** Prouver que le pipeline fonctionne : pending/ → Telegram Ops reçoit le message en < 60s.

- [ ] **Step 9.1 : S'assurer que hermes-browser est actif**

```bash
systemctl --user status hermes-browser
```
Si `inactive`: `systemctl --user start hermes-browser && sleep 5`.

- [ ] **Step 9.2 : Injecter une entrée synthétique dans pending/**

```bash
cat > ~/.cynic/organs/hermes/x/pending/tweet_SYNTHETIC_K15_TEST.json << 'EOF'
{
  "schema_version": 1,
  "tweet_id": "SYNTHETIC_K15_TEST",
  "type": "mention",
  "author": "@k15_test",
  "author_followers": 999,
  "text": "K15 falsification test — if you see this in Telegram, the pipeline works",
  "url": "https://x.com/k15_test/status/SYNTHETIC_K15_TEST",
  "detected_at": "2026-06-02T00:00:00Z",
  "source": "k15_falsification",
  "keywords_matched": ["K15"]
}
EOF
```

- [ ] **Step 9.3 : Lancer l'alerter manuellement**

```bash
cd ~/.cynic/organs/hermes/x && python3 core/talaria_alerter.py
```
Expected: `done: 1 sent, 0 kept for next cycle`.

- [ ] **Step 9.4 : Vérifier réception Telegram**

Ouvrir le groupe Ops Telegram. Vérifier que le message K15 test est reçu dans les 60s.

Expected:
```
🔔 @TalariaBuild — mention
👤 @k15_test (999 followers)
💬 "K15 falsification test — if you see this in Telegram, the pipeline works"
🔗 https://x.com/k15_test/status/SYNTHETIC_K15_TEST
📡 k15_falsification | 2026-06-02 00:00 UTC
```

- [ ] **Step 9.5 : Vérifier que le fichier est passé dans processed/**

```bash
ls ~/.cynic/organs/hermes/x/processed/tweet_SYNTHETIC_K15_TEST.json
```
Expected: fichier présent.

- [ ] **Step 9.6 : Lancer le poller manuellement (test CDP live)**

```bash
cd ~/.cynic/organs/hermes/x && python3 core/notification_poller.py
```
Expected: log avec nombre de notifications trouvées + nouvelles entrées dans `pending/`.

---

## Task 10: Commit final + push

- [ ] **Step 10.1 : Vérifier l'état git**

```bash
git status --short
```

- [ ] **Step 10.2 : Commit le plan**

```bash
git add docs/superpowers/plans/2026-06-02-talaria-x-monitor.md
git commit -m "docs(plan): Talaria X monitor implementation plan"
```

- [ ] **Step 10.3 : Faire tourner make gate**

```bash
make gate
```
Expected: exit 0.

- [ ] **Step 10.4 : Push**

```bash
git push
```
