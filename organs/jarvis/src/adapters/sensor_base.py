
import abc
import time
from typing import Dict, Any, List
from datetime import datetime, timezone

class JarvisSensor(abc.ABC):
    def __init__(self, sensor_id: str, modality: str):
        self.sensor_id = sensor_id
        self.modality = modality
        self.running = False
    
    def _now(self) -> str:
        return datetime.now(timezone.utc).isoformat()

    def create_signal(self, payload: Dict[str, Any], context: Dict[str, Any] = None) -> Dict[str, Any]:
        return {
            "schema": "cynic.jarvis.cognitive_signal.v1",
            "sensor_id": self.sensor_id,
            "timestamp": self._now(),
            "modality": self.modality,
            "payload": payload,
            "context": context or {}
        }

    @abc.abstractmethod
    def start(self, emit_callback):
        pass

    @abc.abstractmethod
    def stop(self):
        pass
