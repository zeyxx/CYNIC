
import time
import json
import logging
from organs.jarvis.src.adapters.sensor_input import InputSensor
from organs.jarvis.src.adapters.sensor_vision import VisionSensor
from organs.jarvis.src.adapters.sensor_context import ContextSensor

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")

class JarvisConstellation:
    def __init__(self):
        self.sensors = [
            InputSensor(),
            VisionSensor(),
            ContextSensor()
        ]

    def _on_signal(self, signal: dict):
        import urllib.request
        import urllib.error
        
        req = urllib.request.Request(
            "http://127.0.0.1:9988/signal",
            data=json.dumps(signal).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        try:
            with urllib.request.urlopen(req, timeout=2) as response:
                if response.status == 201:
                    logging.debug(f"Signal sent: {signal[modality]}")
        except urllib.error.URLError as e:
            logging.warning(f"Failed to send signal (is Jarvis Brain running?): {e}")")

    def start(self):
        logging.info("Starting Jarvis Sensor Constellation...")
        for sensor in self.sensors:
            sensor.start(self._on_signal)
            logging.info(f"Started sensor: {sensor.sensor_id} ({sensor.modality})")

    def stop(self):
        logging.info("Stopping Jarvis Sensor Constellation...")
        for sensor in self.sensors:
            sensor.stop()

if __name__ == "__main__":
    constellation = JarvisConstellation()
    try:
        constellation.start()
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        constellation.stop()
