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
import signal
import subprocess
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path

from pynput import mouse, keyboard

logger = logging.getLogger("behavior-logger")

# ── Config ──

DEFAULT_OUTPUT = Path(os.environ.get(
    "BEHAVIOR_LOG_PATH",
    Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl"
))
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
            # Only query Chrome URL if active window looks like Chrome
            if "chrome" in self.cached_window.get("window_name", "").lower():
                self.cached_url = get_chrome_url()
            else:
                self.cached_url = ""
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
            self._emit({
                "type": "click",
                "x": x, "y": y,
                "button": button.name,
            })

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

        while self.running:
            time.sleep(FLUSH_INTERVAL)
            self.flush()
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
