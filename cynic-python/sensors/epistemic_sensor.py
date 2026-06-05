#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Epistemic Anchor Sensor — Solana technical pillars telemetry.

Tracks:
- Oracle Staleness (Pyth/Switchboard)
- Infra Intensity (Jito/Compute Budget)
- Burn Rate (SIMD-547 anchors)
- Network Congestion (Priority Fees)

POSTs to /observe domain=epistemic-telemetry.
"""

import json
import logging
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import requests

# Add parent dir to path to import Config
sys.path.append(str(Path(__file__).parent.parent))
from config import get_config

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
logger = logging.getLogger("epistemic-sensor")

AGENT_ID = "epistemic-sensor"
DOMAIN = "epistemic-telemetry"

ORACLE_PROGRAMS = [
    "rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ",  # Pyth Receiver
    "SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv",  # Switchboard V3
]

INFRA_PROGRAMS = [
    "ComputeBudget111111111111111111111111111111",
    "JitoTiPevE9D2x6pkA7P6CYnATWzjtneQLhbK7Yv3uS",
]

BURN_ADDRESSES = [
    "1nc1nerator11111111111111111111111111111111",
    "1111111111111111111111111111111111111111111",
]


class EpistemicSensor:
    def __init__(self):
        self.config = get_config()
        self.helius_url = f"https://mainnet.helius-rpc.com/?api-key={self.config.helius_api_key}"
        self.cynic_addr = self.config.cynic_rest_addr
        self.cynic_key = self.config.cynic_api_key

    def _rpc_call(self, method: str, params: list) -> dict:
        try:
            resp = requests.post(
                self.helius_url,
                json={"jsonrpc": "2.0", "id": 1, "method": method, "params": params},
                timeout=15,
            )
            return resp.json()
        except Exception as e:
            logger.error("RPC call %s failed: %s", method, e)
            return {}

    def get_program_intensity(self, program_id: str) -> int:
        """Count signatures in the last minute."""
        res = self._rpc_call("getSignaturesForAddress", [program_id, {"limit": 100}])
        if not res or "result" not in res:
            return 0
        
        now = time.time()
        count = 0
        for sig in res["result"]:
            ts = sig.get("blockTime")
            if ts and now - ts < 300:  # 5 minutes
                count += 1
        return count

    def get_priority_fees(self) -> int:
        """Get estimate of recent priority fees."""
        res = self._rpc_call("getPriorityFeeEstimate", [{"accountKeys": INFRA_PROGRAMS, "options": {"includeAllPriorityFeeLevels": True}}])
        if not res or "result" not in res:
            return 0
        return int(res["result"].get("priorityFeeLevels", {}).get("medium", 0))

    def get_burn_balances(self) -> dict:
        """Track balance of primary burn addresses."""
        balances = {}
        for addr in BURN_ADDRESSES:
            res = self._rpc_call("getBalance", [addr])
            if res and "result" in res:
                balances[addr] = res["result"].get("value", 0) / 1e9
        return balances

    def collect(self) -> dict:
        metrics = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "intensity": {p: self.get_program_intensity(p) for p in ORACLE_PROGRAMS + INFRA_PROGRAMS},
            "priority_fee_microlamports": self.get_priority_fees(),
            "burn_balances_sol": self.get_burn_balances(),
        }
        return metrics

    def report(self, metrics: dict):
        if not self.cynic_key:
            logger.error("CYNIC_API_KEY not set, skipping report")
            return

        payload = {
            "tool": AGENT_ID,
            "domain": DOMAIN,
            "status": "active",
            "context": f"Network Intensity: {sum(metrics['intensity'].values())} txs/5m | Fee: {metrics['priority_fee_microlamports']} | Burns: {sum(metrics['burn_balances_sol'].values()):.2f} SOL",
            "agent_id": AGENT_ID,
            "value": metrics,
        }

        try:
            resp = requests.post(
                f"{self.cynic_addr}/observe",
                json=payload,
                headers={"Authorization": f"Bearer {self.cynic_key}"},
                timeout=10,
            )
            if resp.status_code == 200:
                logger.info("Reported epistemic telemetry to kernel")
            else:
                logger.error("Failed to report: %d %s", resp.status_code, resp.text)
        except Exception as e:
            logger.error("Report failed: %s", e)


def main():
    if not os.environ.get("HELIUS_API_KEY"):
        # Attempt to load from config if not in env
        cfg = get_config()
        if not cfg.helius_api_key:
            logger.error("HELIUS_API_KEY not found in env or config")
            sys.exit(1)

    sensor = EpistemicSensor()
    metrics = sensor.collect()
    sensor.report(metrics)


if __name__ == "__main__":
    main()
