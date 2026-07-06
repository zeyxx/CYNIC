import os
import json
import logging
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from datetime import datetime

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] JARVIS-BRAIN: %(message)s")

# Data Lake setup
STORAGE_DIR = Path(os.path.expanduser("~/.cynic/organs/jarvis/data_lake"))
STORAGE_DIR.mkdir(parents=True, exist_ok=True)
RAW_LOG_FILE = STORAGE_DIR / "raw_signals.jsonl"

class JarvisBrainHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/signal":
            content_length = int(self.headers["Content-Length"])
            post_data = self.rfile.read(content_length)
            
            try:
                signal = json.loads(post_data.decode("utf-8"))
                
                # Append to raw data lake
                with RAW_LOG_FILE.open("a", encoding="utf-8") as f:
                    f.write(json.dumps(signal) + "\n")
                
                logging.info(f"Ingested raw signal from {signal.get("sensor_id", "unknown")}")
                
                self.send_response(201)
                self.send_header("Content-type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"status": "ingested"}).encode("utf-8"))
            except Exception as e:
                logging.error(f"Failed to parse signal: {e}")
                self.send_response(400)
                self.end_headers()
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        # Mute default http.server logging to avoid spam
        pass

def run_brain(port=9988):
    server_address = ("127.0.0.1", port)
    httpd = HTTPServer(server_address, JarvisBrainHandler)
    logging.info(f"Starting Jarvis Brain Ingestion on port {port}...")
    logging.info(f"Raw data lake: {RAW_LOG_FILE}")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        logging.info("Shutting down Jarvis Brain...")
        httpd.server_close()

if __name__ == "__main__":
    run_brain()
