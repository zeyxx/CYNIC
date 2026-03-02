"""
CYNIC API Reality Test â€” The final confrontation with the void.

Boots the actual uvicorn server in a separate thread and queries it via HTTP
to prove the Organism is alive and answering to the outside world.
"""
import json
import sys
import threading
import time
import urllib.request
from pathlib import Path

# Setup path
sys.path.append(str(Path(__file__).parent.parent))

PORT = 8080
BASE_URL = f"http://127.0.0.1:{PORT}"

def run_server():
    import uvicorn
    from cynic.interfaces.api.server import app
    # Run with standard config
    uvicorn.run(app, host="127.0.0.1", port=PORT, log_level="warning")

def query(path, method="GET", data=None):
    url = f"{BASE_URL}{path}"
    headers = {'User-Agent': 'CYNIC-Probe/1.0', 'Content-Type': 'application/json'}
    req = urllib.request.Request(url, headers=headers, method=method)
    if data:
        json_data = json.dumps(data).encode('utf-8')
        req.data = json_data
    
    with urllib.request.urlopen(req) as response:
        return json.loads(response.read().decode())

if __name__ == "__main__":
    print("\n--- ðŸ§ª CYNIC LIVE API PROBE ---")
    
    server_thread = threading.Thread(target=run_server, daemon=True)
    server_thread.start()

    print(f"Step 1: Waiting for server to awaken on port {PORT}...")
    time.sleep(8) # Give it time to initialize the 11 dogs

    try:
        # 1. Health Check
        print("Step 2: Probing /health...")
        health = query("/health")
        print(f"   - Status: {health.get('status')}")
        
        # 2. Version
        print("Step 3: Probing /api/observability/version...")
        version = query("/api/observability/version")
        print(f"   - Version: {version.get('version')}")

        # 3. Judge (The real test)
        print("Step 4: Submitting a live judgment request...")
        judge_req = {
            "content": "Verify system integrity via live probe.",
            "reality": "CYNIC",
            "level": "MICRO"
        }
        judge_resp = query("/api/judge", method="POST", data=judge_req)
        jid = judge_resp.get("judgment_id")
        print(f"   - Judgment ID: {jid}")
        print(f"   - Initial Verdict: {judge_resp.get('verdict')}")

        # 4. Poll
        print("Step 5: Polling for result...")
        time.sleep(2)
        status = query(f"/api/judge/{jid}")
        print(f"   - Final Status: {status.get('status')}")
        
        print("\nâœ… LIVE API PROBE SUCCESSFUL: The Organism is responsive and processing.")
        sys.exit(0)

    except Exception as e:
        print(f"\nâŒ LIVE API PROBE FAILED: {e}")
        sys.exit(1)
