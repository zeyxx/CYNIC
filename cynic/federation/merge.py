"""
MergeEngine — Weighted Q-Table Merging for Federation

This module implements the core federation algorithm: merging remote Q-Table
snapshots into local tables with weighted averaging based on visit counts.

Supports the advanced QTable implementation with Thompson sampling arms.
"""
from __future__ import annotations

import logging
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.learning.qlearning import QTable

from cynic.core.phi import PHI_INV

logger = logging.getLogger("cynic.federation.merge")

__all__ = ["merge_q_tables"]


def merge_q_tables(
    local: QTable,
    remote_snapshot: dict,
) -> int:
    """
    Merge remote Q-Table snapshot into local table with weighted averaging.

    Algorithm:
    1. For each key in remote_snapshot (format "state_key:action"):
       - If (state, action) exists in local table:
         - Weighted merge of q_value based on visits.
         - Combine Thompson wins/losses arms.
       - If (state, action) NOT in local table:
         - Adopt the remote entry with a trust discount on wins/losses.
    2. Return count of merged keys

    Args:
        local: Local QTable instance to merge into.
        remote_snapshot: Dict of QEntry data from peer.
                        Keys: "state_key:action"
                        Values: dict matching QEntry.to_dict()
    """
    merged_count = 0

    for composite_key, remote_data in remote_snapshot.items():
        try:
            state_key = remote_data.get("state_key")
            action = remote_data.get("action")
            
            if not state_key or not action:
                continue

            # Get or create local entry
            entry = local._get_or_create(state_key, action)
            
            remote_q = remote_data.get("q_value", 0.5)
            remote_visits = remote_data.get("visits", 0)
            remote_wins = remote_data.get("wins", 0)
            remote_losses = remote_data.get("losses", 0)

            if entry.visits > 0:
                # Key exists: weighted merge of Q-value
                total_visits = entry.visits + remote_visits
                local_weight = entry.visits / total_visits
                remote_weight = remote_visits / total_visits
                
                entry.q_value = (local_weight * entry.q_value) + (remote_weight * remote_q)
                entry.q_value = max(0.0, min(1.0, entry.q_value))
                
                # Combine Thompson arms (direct summation of evidence)
                entry.wins += remote_wins
                entry.losses += remote_losses
                entry.visits = total_visits
            else:
                # New key: adopt with trust discount (20% reduction in evidence)
                entry.q_value = remote_q
                entry.wins = int(remote_wins * 0.8)
                entry.losses = int(remote_losses * 0.8)
                entry.visits = entry.wins + entry.losses
            
            merged_count += 1
            
        except Exception as e:
            logger.warning("Failed to merge remote entry %s: %s", composite_key, e)

    return merged_count
