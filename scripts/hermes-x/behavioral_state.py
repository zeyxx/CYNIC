"""
Behavioral state computation for Organ X search executor.
Reads execution logs and produces domain weights, backoff levels, environment state.
"""

import json
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Optional
from datetime import datetime, timedelta

PHI = 1.6180339887

@dataclass
class BehavioralState:
    """Current behavioral state of the search executor."""
    domain_weights: Dict[str, float]      # normalized, sum=1.0
    backoff_level: int                    # 0-4, 0=normal, 4=max backoff
    env_state: str                        # "ok" | "degraded" | "blocked"
    consecutive_zeros: int                # streak of cycles with 0 results
    computed_at: str                      # ISO timestamp


def compute_behavioral_state(organ_dir: str) -> BehavioralState:
    """
    Compute behavioral state from execution and behavior logs.

    Sources:
    - search_execution_log.jsonl: success rate per domain, consecutive_zeros
    - behavior_log.jsonl: backoff_level, env_state from previous cycle
    - search_config.json: prior domain weights for bootstrap
    """
    organ_path = Path(organ_dir)

    # Read the last 200 execution log entries
    exec_log_path = organ_path / "search_execution_log.jsonl"
    exec_entries = []
    if exec_log_path.exists():
        with open(exec_log_path, "r") as f:
            lines = f.readlines()
            for line in lines[-200:]:  # last 200
                try:
                    exec_entries.append(json.loads(line))
                except json.JSONDecodeError:
                    pass

    # Read the last behavior log entry (if exists)
    behavior_log_path = organ_path / "behavior_log.jsonl"
    prev_backoff_level = 0
    prev_env_state = "ok"
    if behavior_log_path.exists():
        with open(behavior_log_path, "r") as f:
            lines = f.readlines()
            if lines:
                try:
                    last_entry = json.loads(lines[-1])
                    prev_backoff_level = last_entry.get("backoff_level_after", 0)
                    prev_env_state = last_entry.get("env_state", "ok")
                except json.JSONDecodeError:
                    pass

    # Read search config for bootstrap priors
    config_path = organ_path / "search_config.json"
    domain_priors = {}
    if config_path.exists():
        try:
            config = json.load(open(config_path))
            domains = config.get("domains", {})
            for domain, cfg in domains.items():
                # Use curated_signals.weight as prior if available
                curated = cfg.get("curated_signals", {})
                domain_priors[domain] = curated.get("weight", 0.2)
        except (json.JSONDecodeError, FileNotFoundError):
            pass

    # Count successful entries per domain (status=="success" only)
    success_by_domain = {}
    total_successes = 0
    zero_count = 0

    for entry in exec_entries:
        if entry.get("status") == "success":
            domain = entry.get("domain", "unknown")
            success_by_domain[domain] = success_by_domain.get(domain, 0) + 1
            total_successes += 1

            # Count results_count==0 (ignore "error" or "executed" status entries)
            if entry.get("results_count", 0) == 0:
                zero_count += 1

    consecutive_zeros = zero_count

    # Compute domain weights: blend 70% real + 30% prior
    all_domains = set(success_by_domain.keys()) | set(domain_priors.keys())
    domain_weights = {}

    if total_successes >= 10:
        # We have enough real data
        for domain in all_domains:
            real_rate = success_by_domain.get(domain, 0) / total_successes if total_successes > 0 else 0
            prior = domain_priors.get(domain, 0.2)
            blended = 0.7 * real_rate + 0.3 * prior
            domain_weights[domain] = max(0.05, blended)  # floor at 0.05
    else:
        # Bootstrap: use priors
        for domain in all_domains:
            domain_weights[domain] = max(0.05, domain_priors.get(domain, 0.2))

    # Normalize
    total = sum(domain_weights.values())
    if total > 0:
        domain_weights = {d: w / total for d, w in domain_weights.items()}

    # Determine backoff level and env_state based on consecutive_zeros
    backoff_level = prev_backoff_level
    env_state = prev_env_state

    if consecutive_zeros >= 10:
        backoff_level = 3
        env_state = "blocked"
    elif consecutive_zeros >= 3:
        env_state = "degraded"
        if backoff_level < 2:
            backoff_level = min(4, backoff_level + 1)
    elif total_successes > 0 and consecutive_zeros == 0:
        # Positive signal: recovery
        backoff_level = max(0, backoff_level - 1)
        env_state = "ok"

    return BehavioralState(
        domain_weights=domain_weights,
        backoff_level=backoff_level,
        env_state=env_state,
        consecutive_zeros=consecutive_zeros,
        computed_at=datetime.utcnow().isoformat() + "Z",
    )
