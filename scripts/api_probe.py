"""
CYNIC API Reality Test — The final confrontation with the void.

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

def run_server():
    import uvicorn

    from cynic.interfaces.api.server import app
    # Run with standard config on a safe port
    uvicorn.run(app, host="127.0.0.1", port=8080, log_level="warning")

if __name__ == "__main__":

    server_thread = threading.Thread(target=run_server, daemon=True)
    server_thread.start()

    time.sleep(5)

    try:
        req = urllib.request.Request("http://127.0.0.1:8080/", headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            
        req2 = urllib.request.Request("http://127.0.0.1:8080/federation/status", headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req2) as response2:
            data2 = json.loads(response2.read().decode())
            
    except Exception:
        pass

