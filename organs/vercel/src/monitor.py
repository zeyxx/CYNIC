#!/usr/bin/env python3
# Tier 3
"""
Vercel Edge Manager (organ-vercel)
Monitors CYNIC frontend deployments on Vercel via REST API.
"""

import json
import os
import sys
import urllib.request
import urllib.error
from pathlib import Path
from datetime import datetime, timezone

ORGANS_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = ORGANS_DIR / "state" / "state.json"
ENV_FILE = Path("/home/user/.config/cynic/env")

def get_vercel_token():
    if not ENV_FILE.exists():
        return os.environ.get("VERCEL_TOKEN")
    
    for line in ENV_FILE.read_text().splitlines():
        if line.startswith("VERCEL_TOKEN="):
            return line.split("=", 1)[1].strip().strip('"').strip("'")
    return os.environ.get("VERCEL_TOKEN")

def fetch_deployments(auth_tok):
    url = "https://api.vercel.com/v6/deployments?limit=10"
    req = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {auth_tok}",
        "Content-Type": "application/json"
    })
    
    try:
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode())
    except urllib.error.URLError as e:
        print(f"Error fetching Vercel deployments: {e}")
        return None

def analyze_deployments(data):
    if not data or "deployments" not in data:
        return []
        
    deployments = []
    for d in data["deployments"]:
        deployments.append({
            "id": d.get("uid"),
            "name": d.get("name"),
            "url": d.get("url"),
            "state": d.get("state"),
            "created": d.get("created")
        })
    return deployments

def main():
    auth_tok = get_vercel_token()
    if not auth_tok:
        print("VERCEL_TOKEN not found. Skipping Vercel monitor.")
        return 0
        
    data = fetch_deployments(auth_tok)
    deployments = analyze_deployments(data)
    
    state = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "total_fetched": len(deployments),
        "failing": len([d for d in deployments if d["state"] == "ERROR"]),
        "ready": len([d for d in deployments if d["state"] == "READY"]),
        "deployments": deployments
    }
    
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(state, indent=2))
    
    dataset_file = STATE_FILE.parent / "dataset.jsonl"
    with open(dataset_file, "a") as f:
        f.write(json.dumps(state) + "\n")
    
    print(f"Vercel surface audited. {state['total_fetched']} deployments found. {state['failing']} failing.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
