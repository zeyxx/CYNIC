"""
CYNIC API Reality Test — The final confrontation with the void.

Boots the actual uvicorn server in a separate thread and queries it via HTTP
to prove the Organism is alive and answering to the outside world.
"""
import sys
import threading
import time
import urllib.request
import json
from pathlib import Path

# Setup path
sys.path.append(str(Path(__file__).parent.parent))

def run_server():
    import uvicorn
    from cynic.interfaces.api.server import app
    # Run with standard config on a safe port
    uvicorn.run(app, host="127.0.0.1", port=8080, log_level="warning")

if __name__ == "__main__":
    print("\n" + "="*60)
    print("  CYNIC API REALITY PROBE")
    print("="*60 + "\n")

    print("[1] Igniting the API Server on port 8080...")
    server_thread = threading.Thread(target=run_server, daemon=True)
    server_thread.start()

    print("[2] Waiting for the Organism to Awaken (5 seconds)...")
    time.sleep(5)

    print("[3] Sending HTTP GET to http://127.0.0.1:8080/")
    try:
        req = urllib.request.Request("http://127.0.0.1:8080/", headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            print("\nSUCCESS: The Organism responded from the Void:")
            print(json.dumps(data, indent=2))
            
        print("\n[4] Probing the Federation Interface (/federation/status)...")
        req2 = urllib.request.Request("http://127.0.0.1:8080/federation/status", headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req2) as response2:
            data2 = json.loads(response2.read().decode())
            print("\nSUCCESS: Federation is online:")
            print(json.dumps(data2, indent=2))
            
    except Exception as e:
        print(f"\nTHE VOID WON. Connection failed: {e}")

    print("\n" + "="*60)
    print("  PROBE COMPLETE")
    print("="*60 + "\n")
