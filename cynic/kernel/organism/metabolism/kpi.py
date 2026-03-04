"""
CYNIC KPI Registry - Strategic Performance Monitoring.
Established by the Solutions Architect to measure the emergence quality.
"""
from __future__ import annotations
import json
import time
from pathlib import Path
from dataclasses import dataclass, asdict

@dataclass
class KPIState:
    phi_stability: float = 1.0 # Current avg quality
    metabolic_roi: float = 0.0 # Tokens / Latency
    recovery_rate: float = 0.0 # Success/Total
    total_missions: int = 0
    successful_missions: int = 0
    last_update: float = time.time()

class KPIManager:
    def __init__(self, path: str = "audit/kpis.json"):
        self.path = Path(path)
        self.state = self.load()

    def load(self) -> KPIState:
        if self.path.exists():
            with open(self.path, "r") as f:
                return KPIState(**json.load(f))
        return KPIState()

    def persist(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with open(self.path, "w") as f:
            json.dump(asdict(self.state), f, indent=2)

    def record_mission(self, success: bool, phi: float, tps: float):
        self.state.total_missions += 1
        if success: self.state.successful_missions += 1
        
        # Exponential Moving Average for Phi
        alpha = 0.3
        self.state.phi_stability = (phi * alpha) + (self.state.phi_stability * (1 - alpha))
        self.state.metabolic_roi = tps
        self.state.recovery_rate = self.state.successful_missions / self.state.total_missions
        self.state.last_update = time.time()
        self.persist()
