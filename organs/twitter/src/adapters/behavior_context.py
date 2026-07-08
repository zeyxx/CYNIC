"""Runtime context adapters for the behavior logger."""

from __future__ import annotations

import json
import logging
import re
import sqlite3
import subprocess
from datetime import datetime, timedelta

import requests
from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.contracts.src.surface_contracts import json_object_or_default

from organs.twitter.src.adapters.hermes_paths import TWEET_ID_INDEX


logger = logging.getLogger("behavior-context")


def get_active_window() -> dict:
    """Get active window context via xprop: WM_NAME, WM_CLASS, workspace."""
    try:
        raw = subprocess.check_output(
            ["xprop", "-root", "_NET_ACTIVE_WINDOW"],
            timeout=1,
            stderr=subprocess.DEVNULL,
        ).decode().strip()
        wid = raw.split("#")[-1].strip() if "#" in raw else ""
        if not wid or wid == "0x0":
            return {}

        props_raw = subprocess.check_output(
            ["xprop", "-id", wid, "WM_NAME", "WM_CLASS"],
            timeout=1,
            stderr=subprocess.DEVNULL,
        ).decode().strip()

        name = ""
        wm_class = ""
        for line in props_raw.splitlines():
            if line.startswith("WM_NAME"):
                parts = line.split('"')
                name = parts[1] if len(parts) >= 2 else ""
            elif line.startswith("WM_CLASS"):
                parts = line.split('"')
                wm_class = parts[3] if len(parts) >= 4 else ""

        workspace = -1
        try:
            desk_raw = subprocess.check_output(
                ["xprop", "-root", "_NET_CURRENT_DESKTOP"],
                timeout=1,
                stderr=subprocess.DEVNULL,
            ).decode().strip()
            workspace = int(desk_raw.split("=")[-1].strip())
        except (subprocess.SubprocessError, ValueError):
            pass

        result: dict = {"window_id": wid, "window_name": name}
        if wm_class:
            result["wm_class"] = wm_class
        if workspace >= 0:
            result["workspace"] = workspace
        return result
    except (subprocess.SubprocessError, ValueError, FileNotFoundError, IndexError):
        return {}


def get_chrome_url() -> str:
    """Get current Chrome tab URL via CDP. Returns '' on failure."""
    try:
        resp = requests.get("http://127.0.0.1:40769/json", timeout=1)
        if resp.status_code == 200:
            for tab in resp.json():
                if tab.get("type") == "page" and tab.get("url", "").startswith("http"):
                    return tab["url"]
    except Exception:
        pass
    return ""


def get_tweet_id_from_index(timestamp_str: str, url: str = "") -> str:
    """Look up tweet_id from URL or mitmproxy index using temporal proximity."""
    if url:
        match = re.search(r"/status/(\d+)", url)
        if match:
            return match.group(1)

    index_db = TWEET_ID_INDEX
    if not index_db.exists():
        return ""

    try:
        dt = datetime.fromisoformat(timestamp_str.replace("Z", "+00:00"))
        start_time = (dt - timedelta(seconds=30)).isoformat()
        end_time = (dt + timedelta(seconds=30)).isoformat()

        conn = sqlite3.connect(str(index_db))
        cursor = conn.cursor()
        cursor.execute(
            """
            SELECT tweet_ids FROM tweet_operations
            WHERE timestamp BETWEEN ? AND ?
            ORDER BY ABS(julianday(timestamp) - julianday(?)) ASC
            LIMIT 1
        """,
            (start_time, end_time, timestamp_str),
        )
        row = cursor.fetchone()
        conn.close()

        if row:
            tweet_ids = json_loads_or_default(row[0], [])
            if tweet_ids:
                return tweet_ids[0]
    except Exception as exc:
        logger.debug("Index lookup failed: %s", exc)

    return ""


def get_tweet_id_from_click(x: int, y: int) -> str:
    """Query Chrome DOM via CDP WebSocket to extract tweet_id at click position."""
    try:
        import websocket

        resp = requests.get("http://127.0.0.1:40769/json", timeout=1)
        if resp.status_code != 200:
            return ""

        page_tab = None
        for tab in resp.json():
            if tab.get("type") == "page" and "x.com" in tab.get("url", "").lower():
                page_tab = tab
                break

        if not page_tab or not page_tab.get("webSocketDebuggerUrl"):
            return ""

        ws = websocket.create_connection(page_tab["webSocketDebuggerUrl"], timeout=2)
        js_code = f"""
        (function() {{
            const elem = document.elementFromPoint({x}, {y});
            if (!elem) return "";
            let node = elem;
            for (let i = 0; i < 10; i++) {{
                if (!node) break;
                const testId = node.getAttribute("data-testid");
                if (testId && testId.includes("tweet")) {{
                    const tweetId = node.getAttribute("data-tweet-id") ||
                                   node.getAttribute("data-status-id") ||
                                   node.getAttribute("aria-label")?.match(/\\d{{15,}}/)?.[0];
                    if (tweetId && tweetId.match(/^\\d{{15,}}$/)) return tweetId;
                }}
                const ariaLabel = node.getAttribute("aria-label");
                if (ariaLabel) {{
                    const match = ariaLabel.match(/\\b(\\d{{15,}})\\b/);
                    if (match) return match[1];
                }}
                for (const attr of node.attributes || []) {{
                    if (attr.value && attr.value.match(/^\\d{{15,}}$/)) return attr.value;
                }}
                node = node.parentElement;
            }}
            return "";
        }})()
        """
        ws.send(json.dumps({"id": 1, "method": "Runtime.evaluate", "params": {"expression": js_code, "returnByValue": True}}))
        response = json_object_or_default(ws.recv())
        ws.close()

        if "result" in response and "result" in response["result"]:
            tweet_id = response["result"]["result"].get("value", "")
            if tweet_id and len(tweet_id) > 15 and tweet_id.isdigit():
                return tweet_id
        return ""
    except Exception as exc:
        logger.debug("CDP tweet extraction failed: %s", exc)
        return ""
