#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Epistemic Strategy Consumer.
Consumes 'epistemic-telemetry' observations and dispatches tasks to Hermes.

Thresholds (Initial Axiomatic):
- SOVEREIGNTY: Priority Fee > 10,000 µlamports (Network Stress)
- FIDELITY: Jito Intensity < 10 tx/5m (Infra bypass/failure)
- BURN: Significant Burn balance increase (TBD, currently logging)

This consumer is the first step towards data-driven axiomatic adjustment.
"""

import json
import logging
import os
import sys
import time
from pathlib import Path

import requests

# Add parent dir to path to import Config
sys.path.append(str(Path(__file__).parent.parent))
from config import get_config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
logger = logging.getLogger("epistemic-consumer")

AGENT_ID = "epistemic-strategy-consumer"
DOMAIN = "epistemic-telemetry"
JITO_PROGRAM = "JitoTiPevE9D2x6pkA7P6CYnATWzjtneQLhbK7Yv3uS"

# Initial Thresholds
FEE_THRESHOLD = 10000
JITO_INTENSITY_FLOOR = 10


class EpistemicStrategyConsumer:
    def __init__(self):
        self.config = get_config()
        self.cynic_addr = self.config.cynic_rest_addr
        self.cynic_key = self.config.cynic_api_key

    def headers(self):
        return {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.cynic_key}",
        }

    def fetch_observations(self, limit: int = 10) -> list:
        try:
            url = f"{self.cynic_addr}/observations?domain={DOMAIN}&limit={limit}"
            resp = requests.get(url, headers=self.headers(), timeout=10)
            if resp.status_code == 200:
                return resp.json() or []
            return []
        except Exception as e:
            logger.error("Failed to fetch observations: %s", e)
            return []

    def evaluate(self, obs: dict):
        value = obs.get("value", {})
        if not value:
            return

        intensity = value.get("intensity", {})
        jito_intensity = intensity.get(JITO_PROGRAM, 0)
        fee = value.get("priority_fee_microlamports", 0)

        signals = []

        if fee > FEE_THRESHOLD:
            signals.append(f"Network Stress: Priority Fee at {fee} µlamports (Threshold: {FEE_THRESHOLD})")
        
        if jito_intensity < JITO_INTENSITY_FLOOR:
            signals.append(f"Infra Redline: Jito intensity dropped to {jito_intensity} tx/5m (Floor: {JITO_INTENSITY_FLOOR})")

        if signals:
            self.dispatch_task(obs, signals)

    def dispatch_task(self, obs: dict, signals: list):
        context = "\n".join(signals)
        value = obs.get("value", {})
        
        # Structured task for Hermes Agent Task Executor
        task_data = {
            "objective": f"Axiomatic Alert: Epistemic Anchor Shift detected on Solana.\nSignals: {context}",
            "actions": [
                "Analyze the provided raw telemetry (intensity, fees, burns).",
                "Verify network status using internal tools if necessary.",
                "Synthesize a 1-tweet technical observation for @TalariaBuild.",
                "Focus on Verticalité Native and hardware/finality tension.",
                "Ensure the tone is cold, technical, and free of 'AI slop'.",
                "Post the tweet using the x_poster tool."
            ],
            "telemetry": value
        }

        try:
            url = f"{self.cynic_addr}/agent-tasks"
            payload = {
                "kind": "hermes",
                "domain": "axiomatic-comms",
                "content": json.dumps(task_data),
                "agent_id": AGENT_ID,
            }
            resp = requests.post(url, json=payload, headers=self.headers(), timeout=10)
            if resp.status_code in (200, 201):
                logger.info("✓ Dispatched structured task to Hermes for signals: %s", ", ".join(signals))
            else:
                logger.error("Failed to dispatch task: %d %s", resp.status_code, resp.text)
        except Exception as e:
            logger.error("Task dispatch failed: %s", e)

    def run(self):
        logger.info("Epistemic Strategy Consumer starting...")
        seen_ids = set()
        while True:
            observations = self.fetch_observations()
            for obs in observations:
                obs_id = obs.get("_id")
                if obs_id not in seen_ids:
                    self.evaluate(obs)
                    seen_ids.add(obs_id)
            
            # Limit seen_ids size
            if len(seen_ids) > 1000:
                seen_ids = set(list(seen_ids)[-500:])

            time.sleep(60)


if __name__ == "__main__":
    consumer = EpistemicStrategyConsumer()
    consumer.run()
