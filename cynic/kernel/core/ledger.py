"""
CYNIC Event Ledger - Immutable Audit Trail.
Ensures every axiomatic decision is recorded and cryptographically auditable.
"""
from __future__ import annotations
import json
import time
import hashlib
from pathlib import Path
from dataclasses import dataclass, asdict

@dataclass
class LedgerEntry:
    index: int
    timestamp: float
    event_type: str
    data: dict
    previous_hash: str
    hash: str = ""

    def compute_hash(self) -> str:
        payload = f"{self.index}{self.timestamp}{self.event_type}{json.dumps(self.data, sort_keys=True)}{self.previous_hash}"
        return hashlib.sha256(payload.encode()).hexdigest()

class EventLedger:
    def __init__(self, path: str = "audit/ledger.jsonl"):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._last_hash = "0"
        self._last_index = -1
        self._rebuild_chain()

    def _rebuild_chain(self):
        if self.path.exists():
            with open(self.path, "r") as f:
                for line in f:
                    entry = json.loads(line)
                    self._last_hash = entry["hash"]
                    self._last_index = entry["index"]

    def record(self, event_type: str, data: dict):
        entry = LedgerEntry(
            index=self._last_index + 1,
            timestamp=time.time(),
            event_type=event_type,
            data=data,
            previous_hash=self._last_hash
        )
        entry.hash = entry.compute_hash()
        
        with open(self.path, "a") as f:
            f.write(json.dumps(asdict(entry)) + "\n")
        
        self._last_hash = entry.hash
        self._last_index = entry.index
        return entry.hash
