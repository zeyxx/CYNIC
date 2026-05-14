"""
CYNIC Behavior Logger — captures mouse/keyboard events with window context.

Training data for Hermes behavioral ML. Append-only JSONL.
Captures: mouse moves (sampled), clicks, scrolls, key presses + active window + URL.

Privacy: runs locally, never transmitted. Data stays in ~/.cynic/organs/hermes/behavior/.

Usage:
    python behavior_logger.py                    # default output
    python behavior_logger.py --output /path.jsonl
    python behavior_logger.py --mouse-sample 0.5 # sample 50% of mouse moves

Environment:
    DISPLAY must be set (X11 required for window context).
"""

__version__ = "0.1.0"

import argparse
import json
import logging
import os
import re
import signal
import sqlite3
import subprocess
import sys
import threading
import time
from datetime import datetime, timezone, timedelta
from pathlib import Path

from pynput import mouse, keyboard
import requests

from hermes_paths import BEHAVIOR_LOG as DEFAULT_OUTPUT, TWEET_ID_INDEX

logger = logging.getLogger("behavior-logger")
MOUSE_MOVE_INTERVAL = 0.2  # min seconds between mouse move events (rate limit)
FLUSH_INTERVAL = 5.0  # flush to disk every N seconds


# ── Window context (X11) ──

def get_active_window() -> dict:
    """Get active window title via xprop (available on all X11 systems). Returns {} on failure."""
    try:
        # Get active window ID
        raw = subprocess.check_output(
            ["xprop", "-root", "_NET_ACTIVE_WINDOW"],
            timeout=1, stderr=subprocess.DEVNULL
        ).decode().strip()
        # Parse: "_NET_ACTIVE_WINDOW(WINDOW): window id # 0x2a0000a"
        wid = raw.split("#")[-1].strip() if "#" in raw else ""
        if not wid or wid == "0x0":
            return {}
        # Get window name
        name_raw = subprocess.check_output(
            ["xprop", "-id", wid, "WM_NAME"],
            timeout=1, stderr=subprocess.DEVNULL
        ).decode().strip()
        # Parse: 'WM_NAME(STRING) = "Window Title"'
        name = name_raw.split('"')[1] if '"' in name_raw else ""
        return {"window_id": wid, "window_name": name}
    except (subprocess.SubprocessError, ValueError, FileNotFoundError, IndexError):
        return {}


def get_chrome_url() -> str:
    """Get current Chrome tab URL via CDP. Returns '' on failure."""
    try:
        import requests
        resp = requests.get("http://127.0.0.1:40769/json", timeout=1)
        if resp.status_code == 200:
            tabs = resp.json()
            # Find the active/visible tab (type=page, not devtools)
            for tab in tabs:
                if tab.get("type") == "page" and tab.get("url", "").startswith("http"):
                    return tab["url"]
    except Exception:
        pass
    return ""


def get_tweet_id_from_index(timestamp_str: str, url: str = "") -> str:
    """Fallback: look up tweet_id from mitmproxy index using temporal proximity.

    Strategy:
    1. If URL contains /status/{id}, extract directly
    2. Otherwise, query mitmproxy index for tweets returned within ±30s
    3. Return first tweet_id from nearest operation
    """
    # Direct extraction from URL
    if url:
        match = re.search(r'/status/(\d+)', url)
        if match:
            return match.group(1)

    # Query mitmproxy index
    index_db = TWEET_ID_INDEX
    if not index_db.exists():
        return ""

    try:
        dt = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        start_time = (dt - timedelta(seconds=30)).isoformat()
        end_time = (dt + timedelta(seconds=30)).isoformat()

        conn = sqlite3.connect(str(index_db))
        cursor = conn.cursor()

        cursor.execute("""
            SELECT tweet_ids FROM tweet_operations
            WHERE timestamp BETWEEN ? AND ?
            ORDER BY ABS(julianday(timestamp) - julianday(?)) ASC
            LIMIT 1
        """, (start_time, end_time, timestamp_str))

        row = cursor.fetchone()
        conn.close()

        if row:
            tweet_ids = json.loads(row[0])
            if tweet_ids:
                return tweet_ids[0]
    except Exception as e:
        logger.debug(f"Index lookup failed: {e}")

    return ""


def get_tweet_id_from_click(x: int, y: int) -> str:
    """Query Chrome DOM via CDP WebSocket to extract tweet_id at click position.

    Uses DevTools Protocol Runtime.evaluate to run JavaScript that finds the tweet element
    at (x, y) and extracts its ID from data attributes or parent containers.
    Returns '' on failure or if not on a tweet.
    """
    try:
        import json as stdlib_json
        import websocket

        # Get the X.com page tab and its WebSocket URL
        resp = requests.get("http://127.0.0.1:40769/json", timeout=1)
        if resp.status_code != 200:
            return ""

        tabs = resp.json()
        page_tab = None
        for tab in tabs:
            if tab.get("type") == "page" and "x.com" in tab.get("url", "").lower():
                page_tab = tab
                break

        if not page_tab or not page_tab.get("webSocketDebuggerUrl"):
            return ""

        ws_url = page_tab["webSocketDebuggerUrl"]

        # Connect to Chrome DevTools WebSocket
        ws = websocket.create_connection(ws_url, timeout=2)

        # JavaScript to find tweet element at (x, y) and extract tweet_id
        js_code = f"""
        (function() {{
            const elem = document.elementFromPoint({x}, {y});
            if (!elem) return "";

            // Walk up tree to find tweet container
            let node = elem;
            for (let i = 0; i < 10; i++) {{
                if (!node) break;

                // Check data attributes
                const testId = node.getAttribute("data-testid");
                if (testId && testId.includes("tweet")) {{
                    // Look for tweet ID in various places
                    const tweetId = node.getAttribute("data-tweet-id") ||
                                   node.getAttribute("data-status-id") ||
                                   node.getAttribute("aria-label")?.match(/\\d{{15,}}/)?.[0];
                    if (tweetId && tweetId.match(/^\\d{{15,}}$/)) return tweetId;
                }}

                // Check aria-label for tweet IDs (common pattern)
                const ariaLabel = node.getAttribute("aria-label");
                if (ariaLabel) {{
                    const match = ariaLabel.match(/\\b(\\d{{15,}})\\b/);
                    if (match) return match[1];
                }}

                // Check for tweet ID in various attributes
                for (const attr of node.attributes || []) {{
                    if (attr.value && attr.value.match(/^\\d{{15,}}$/)) {{
                        return attr.value;
                    }}
                }}

                node = node.parentElement;
            }}
            return "";
        }})()
        """

        # Send Runtime.evaluate command
        msg = {
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": js_code,
                "returnByValue": True,
            }
        }

        ws.send(json.dumps(msg))

        # Read response (should come back quickly)
        response_str = ws.recv()
        response = json.loads(response_str)

        ws.close()

        # Extract result
        if "result" in response and "result" in response["result"]:
            tweet_id = response["result"]["result"].get("value", "")
            if tweet_id and len(tweet_id) > 15 and tweet_id.isdigit():
                return tweet_id

        return ""

    except Exception as e:
        logger.debug(f"CDP tweet extraction failed: {e}")
        return ""


# ── Logger core ──

class BehaviorLogger:
    def __init__(self, output: Path, mouse_sample: float = 1.0):
        self.output = output
        self.output.parent.mkdir(parents=True, exist_ok=True)
        self.mouse_sample = mouse_sample
        self.buffer: list[dict] = []
        self.lock = threading.Lock()
        self.last_mouse_move = 0.0
        self.last_window_check = 0.0
        self.cached_window: dict = {}
        self.cached_url = ""
        self.running = True
        self.events_total = 0

        logger.info("behavior_logger v%s starting — output=%s", __version__, output)

    def _now(self) -> str:
        return datetime.now(timezone.utc).isoformat()

    def _context(self) -> dict:
        """Get current window context (cached, refreshed every 0.5s)."""
        now = time.time()
        if now - self.last_window_check > 0.5:
            self.cached_window = get_active_window()
            # Try to get Chrome URL; always query CDP (it's available or fails gracefully)
            # Remove the window_name check — CDP connection is the gate, not window title
            self.cached_url = get_chrome_url()
            self.last_window_check = now
        ctx = dict(self.cached_window)
        if self.cached_url:
            ctx["url"] = self.cached_url
        return ctx

    def _emit(self, event: dict):
        """Thread-safe append to buffer."""
        event["ts"] = self._now()
        event.update(self._context())
        with self.lock:
            self.buffer.append(event)
            self.events_total += 1

    def flush(self):
        """Write buffered events to disk."""
        with self.lock:
            if not self.buffer:
                return
            batch = self.buffer
            self.buffer = []

        with open(self.output, "a") as f:
            for event in batch:
                f.write(json.dumps(event, separators=(",", ":")) + "\n")

    # ── Mouse callbacks ──

    def on_mouse_move(self, x: int, y: int):
        now = time.time()
        if now - self.last_mouse_move < MOUSE_MOVE_INTERVAL:
            return  # rate limit
        self.last_mouse_move = now
        self._emit({"type": "mouse_move", "x": x, "y": y})

    def on_mouse_click(self, x: int, y: int, button, pressed: bool):
        if pressed:
            event = {
                "type": "click",
                "x": x, "y": y,
                "button": button.name,
            }

            # Try to extract tweet_id: CDP first (real-time), then index fallback (reliable)
            if self.cached_url and "x.com" in self.cached_url:
                tweet_id = ""
                source = ""

                # Approach 1: CDP WebSocket (real-time DOM extraction)
                tweet_id = get_tweet_id_from_click(x, y)
                if tweet_id:
                    source = "cdp"

                # Approach 2: mitmproxy index (temporal proximity lookup)
                if not tweet_id:
                    ts = self._now()
                    tweet_id = get_tweet_id_from_index(ts, self.cached_url)
                    if tweet_id:
                        source = "index"

                if tweet_id:
                    event["tweet_id"] = tweet_id
                    event["tweet_id_source"] = source

            self._emit(event)

    def on_mouse_scroll(self, x: int, y: int, dx: int, dy: int):
        self._emit({"type": "scroll", "x": x, "y": y, "dx": dx, "dy": dy})

    # ── Keyboard callbacks ──

    def on_key_press(self, key):
        try:
            k = key.char if hasattr(key, "char") and key.char else str(key)
        except AttributeError:
            k = str(key)
        self._emit({"type": "key", "key": k})

    # ── Run ──

    def run(self):
        mouse_listener = mouse.Listener(
            on_move=self.on_mouse_move,
            on_click=self.on_mouse_click,
            on_scroll=self.on_mouse_scroll,
        )
        key_listener = keyboard.Listener(on_press=self.on_key_press)

        mouse_listener.start()
        key_listener.start()

        logger.info("Listening for mouse + keyboard events (flush every %.0fs)", FLUSH_INTERVAL)

        last_event_time = time.time()
        last_health_check = time.time()

        while self.running:
            time.sleep(FLUSH_INTERVAL)
            self.flush()

            # Listener health check: if no events for 120s, restart (indicates listener crash)
            now = time.time()
            if now - last_event_time > 120:
                logger.warning("No events for 120s — restarting listeners")
                mouse_listener.stop()
                key_listener.stop()
                mouse_listener = mouse.Listener(
                    on_move=self.on_mouse_move,
                    on_click=self.on_mouse_click,
                    on_scroll=self.on_mouse_scroll,
                )
                key_listener = keyboard.Listener(on_press=self.on_key_press)
                mouse_listener.start()
                key_listener.start()
                last_event_time = now

            # Health checkpoint every 60s (K15 producer signal)
            if now - last_health_check > 60:
                health = {
                    "type": "health_checkpoint",
                    "events_captured": self.events_total,
                    "listeners_alive": mouse_listener.is_alive() and key_listener.is_alive(),
                }
                self._emit(health)
                last_health_check = now

            # Track latest event time (detect listener silence)
            with self.lock:
                if self.buffer:
                    last_event_time = now

            if self.events_total > 0 and self.events_total % 1000 == 0:
                logger.info("Events captured: %d", self.events_total)

        # Final flush
        self.flush()
        mouse_listener.stop()
        key_listener.stop()
        logger.info("Stopped — total events: %d", self.events_total)


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )

    parser = argparse.ArgumentParser(description="CYNIC Behavior Logger")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--mouse-sample", type=float, default=1.0,
                        help="Mouse move sampling rate (0.0-1.0)")
    args = parser.parse_args()

    bl = BehaviorLogger(args.output, mouse_sample=args.mouse_sample)

    def handle_signal(sig, frame):
        bl.running = False
        logger.info("Shutdown signal received")
    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    bl.run()


if __name__ == "__main__":
    main()
