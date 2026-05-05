#!/usr/bin/env python3
"""
Chrome launcher with DevTools Protocol remote origin support.

Launches Chrome with flags needed for behavior_logger CDP WebSocket connections.
Used by hermes-behavior systemd service to ensure Chrome accepts CDP from 127.0.0.1:40769.
"""

import subprocess
import sys
import time
from pathlib import Path

# Chrome flags for remote debugging
CHROME_FLAGS = [
    "--enable-automation",
    "--remote-debugging-port=40769",
    "--remote-allow-origins=http://127.0.0.1:40769,http://localhost:40769",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-default-apps",
    "--disable-popup-blocking",
    "--disable-extensions",
]


def launch_chrome():
    """Launch Chrome with DevTools Protocol support."""
    # Try to find Chrome executable
    chrome_paths = [
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    ]

    chrome_bin = None
    for path in chrome_paths:
        if Path(path).exists():
            chrome_bin = path
            break

    if not chrome_bin:
        print(f"ERROR: Chrome not found. Tried: {chrome_paths}", file=sys.stderr)
        return 1

    cmd = [chrome_bin] + CHROME_FLAGS
    print(f"Launching: {' '.join(cmd)}")

    try:
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        print(f"Chrome launched with PID {proc.pid}")
        print(f"DevTools available at: http://127.0.0.1:40769")

        # Wait for Chrome to be ready
        time.sleep(3)

        # Return to let parent process continue
        return 0
    except Exception as e:
        print(f"ERROR: Failed to launch Chrome: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(launch_chrome())
