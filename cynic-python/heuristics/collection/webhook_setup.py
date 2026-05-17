#!/usr/bin/env python3
"""Helius Webhook Setup — subscribe to token transfer events.

Registers an enhanced webhook for our tracked tokens.
Each transaction involving these mints triggers an HTTP POST to our receiver.

Payload: parsed tokenTransfers with mint, fromUserAccount, toUserAccount, amount.
Cost: 1 credit per event delivered.

Usage:
    python3 webhook_setup.py --create          # create webhook for all tracked tokens
    python3 webhook_setup.py --list            # list existing webhooks
    python3 webhook_setup.py --delete <id>     # delete a webhook
    python3 webhook_setup.py --update <id>     # update addresses on existing webhook

Requires: HELIUS_WEBHOOK_URL env var (your public receiver endpoint).
"""

import json
import os
import sys
import requests
from typing import Optional, List, Dict

# ── ENV ──

def _load_env() -> None:
    for env_file in [
        os.path.join(os.path.dirname(__file__), '.env'),
        os.path.expanduser('~/.cynic-env'),
    ]:
        if os.path.exists(env_file):
            with open(env_file) as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        key = key.replace('export ', '').strip()
                        if key not in os.environ:
                            os.environ[key] = value.strip()

_load_env()

HELIUS_API_KEY = os.getenv("HELIUS_API_KEY")
WEBHOOK_URL = os.getenv("HELIUS_WEBHOOK_URL")  # e.g. https://cynic-core.tail1234.ts.net:3031/webhook/helius

if not HELIUS_API_KEY:
    print("ERROR: HELIUS_API_KEY not set")
    sys.exit(1)

HELIUS_WEBHOOKS_API = "https://api-mainnet.helius-rpc.com/v0/webhooks"
HEADERS = {"Authorization": f"Bearer {HELIUS_API_KEY}", "Content-Type": "application/json"}

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CALIB_PATH = os.path.join(SCRIPT_DIR, "calibration_results_real.json")
WATCHLIST_PATH = os.path.join(SCRIPT_DIR, "watchlist.json")


def get_tracked_mints() -> List[str]:
    """Get all mints we want to track."""
    mints = []
    if os.path.exists(CALIB_PATH):
        with open(CALIB_PATH) as f:
            calib = json.load(f)
        mints.extend(r["mint"] for r in calib.get("results", []))
    if os.path.exists(WATCHLIST_PATH):
        with open(WATCHLIST_PATH) as f:
            watchlist = json.load(f)
        mints.extend(t["mint"] for t in watchlist if t["mint"] not in mints)
    return mints


def list_webhooks() -> List[Dict]:
    """List all existing webhooks."""
    resp = requests.get(HELIUS_WEBHOOKS_API, headers=HEADERS, timeout=15)
    if resp.status_code != 200:
        print(f"Error listing webhooks: {resp.status_code} {resp.text}")
        return []
    return resp.json()


def create_webhook(webhook_url: str, mints: List[str]) -> Optional[Dict]:
    """Create an enhanced webhook for token mints."""
    payload = {
        "webhookURL": webhook_url,
        "transactionTypes": ["Any"],  # capture all tx types involving our mints
        "accountAddresses": mints,
        "webhookType": "enhanced",
    }

    resp = requests.post(HELIUS_WEBHOOKS_API, headers=HEADERS, json=payload, timeout=15)
    if resp.status_code not in (200, 201):
        print(f"Error creating webhook: {resp.status_code} {resp.text}")
        return None
    return resp.json()


def delete_webhook(webhook_id: str) -> bool:
    """Delete a webhook by ID."""
    resp = requests.delete(f"{HELIUS_WEBHOOKS_API}/{webhook_id}", headers=HEADERS, timeout=15)
    return resp.status_code == 200


def update_webhook(webhook_id: str, mints: List[str]) -> Optional[Dict]:
    """Update addresses on existing webhook."""
    payload = {
        "accountAddresses": mints,
    }
    resp = requests.put(f"{HELIUS_WEBHOOKS_API}/{webhook_id}",
                        headers=HEADERS, json=payload, timeout=15)
    if resp.status_code != 200:
        print(f"Error updating: {resp.status_code} {resp.text}")
        return None
    return resp.json()


def main() -> None:
    if "--list" in sys.argv:
        webhooks = list_webhooks()
        print(f"Webhooks: {len(webhooks)}")
        for wh in webhooks:
            wid = wh.get("webhookID", "?")
            url = wh.get("webhookURL", "?")
            addrs = wh.get("accountAddresses", [])
            wtype = wh.get("webhookType", "?")
            print(f"  [{wid}] {wtype} → {url} ({len(addrs)} addresses)")
        return

    if "--delete" in sys.argv:
        idx = sys.argv.index("--delete")
        if idx + 1 < len(sys.argv):
            wid = sys.argv[idx + 1]
            if delete_webhook(wid):
                print(f"Deleted: {wid}")
            else:
                print(f"Failed to delete: {wid}")
        return

    if "--create" in sys.argv:
        if not WEBHOOK_URL:
            print("ERROR: HELIUS_WEBHOOK_URL not set")
            print("Set it to your public receiver endpoint, e.g.:")
            print("  export HELIUS_WEBHOOK_URL=https://your-server.example.com/webhook/helius")
            sys.exit(1)

        mints = get_tracked_mints()
        print(f"Creating enhanced webhook for {len(mints)} mints")
        print(f"  URL: {WEBHOOK_URL}")

        # Helius limit: split into chunks if needed (API may accept many but docs say 25 via dashboard)
        # Via API, larger batches should work
        result = create_webhook(WEBHOOK_URL, mints)
        if result:
            wid = result.get("webhookID", "?")
            print(f"  Created! ID: {wid}")
            # Save webhook ID for reference
            with open(os.path.join(SCRIPT_DIR, "webhook_id.txt"), "w") as f:
                f.write(wid)
        return

    if "--update" in sys.argv:
        idx = sys.argv.index("--update")
        if idx + 1 < len(sys.argv):
            wid = sys.argv[idx + 1]
            mints = get_tracked_mints()
            result = update_webhook(wid, mints)
            if result:
                print(f"Updated webhook {wid} with {len(mints)} addresses")
        return

    # Default: show status
    print("Usage:")
    print("  --create   Create webhook (requires HELIUS_WEBHOOK_URL env)")
    print("  --list     List existing webhooks")
    print("  --delete   Delete a webhook by ID")
    print("  --update   Update addresses on webhook")
    print()
    print(f"Tracked mints: {len(get_tracked_mints())}")
    print(f"HELIUS_WEBHOOK_URL: {WEBHOOK_URL or 'NOT SET'}")


if __name__ == "__main__":
    main()
