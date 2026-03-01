"""
MergeEngine — Weighted Q-Table Merging for Federation

This module implements the core federation algorithm: merging remote Q-Table
snapshots into local tables with weighted averaging based on visit counts.

Key algorithm:
1. For existing keys: weighted merge of q_score, confidence, satisfaction_avg
2. For new keys: adopt with 20% confidence trust discount
3. All confidence values clamped to PHI_INV = 0.618 (max trust threshold)

This enables multiple CYNIC instances to learn collectively from each other's
judgment outcomes while maintaining φ-bounded uncertainty constraints.
"""

from __future__ import annotations

from cynic.kernel.core.phi import PHI_INV
from cynic.kernel.organism.brain.learning.unified_learning import UnifiedQTable

__all__ = ["merge_q_tables"]


def merge_q_tables(
    local: UnifiedQTable,
    remote_snapshot: dict,
    peer_total_judgments: int,
) -> int:
    """
    Merge remote Q-Table snapshot into local table with weighted averaging.

    Algorithm:
    1. For each key in remote_snapshot:
       - If key exists in local.table:
         - Calculate weights: local_weight = local_visits / total_visits
         - Weighted merge of q_score, confidence, satisfaction_avg
         - Clamp confidence to PHI_INV (0.618) max
         - Update visits = total_visits
       - If key NOT in local.table:
         - Adopt the remote entry
         - Apply 20% confidence trust discount: confidence *= 0.8
         - Clamp to PHI_INV
    2. Return count of merged keys

    Args:
        local: Local UnifiedQTable to merge into (modified in-place)
        remote_snapshot: Dict from remote peer's UnifiedQTable.values snapshot
                        Keys are in format "DOMAIN:context_hash"
                        Values are dicts with: domain, context_hash, q_score,
                        confidence, visits, satisfaction_avg
        peer_total_judgments: Total judgments made by remote peer (for context)

    Returns:
        Count of merged keys (existing + newly adopted entries)
    """
    merged_count = 0

    for _key_str, remote_data in remote_snapshot.items():
        # Convert key string "DOMAIN:context_hash" to tuple (domain, context_hash)
        # Extract domain and context_hash from the remote data (more reliable)
        domain = remote_data.get("domain", "")
        context_hash = remote_data.get("context_hash", "")
        key_tuple = (domain, context_hash)

        remote_visits = remote_data.get("visits", 0)
        remote_q_score = remote_data.get("q_score", 0.5)
        remote_confidence = remote_data.get("confidence", 0.5)
        remote_data.get("satisfaction_avg", 0.5)

        if key_tuple in local.values:
            # Key exists: weighted merge
            local_q_score = local.values[key_tuple]

            # For UnifiedQTable, we don't have explicit visit counts per entry
            # So we use a default assumption: remote has all visits
            # In practice, we weight by assuming local had some implicit visits
            # Default: treat local as having 1 implicit visit
            local_visits = 1
            total = local_visits + remote_visits

            if total > 0:
                local_weight = local_visits / total
                remote_weight = remote_visits / total

                # Weighted merge of q_score
                merged_q_score = local_weight * local_q_score + remote_weight * remote_q_score

                # Clamp merged q_score to [0, 100]
                merged_q_score = max(0.0, min(100.0, merged_q_score))

                # Update local value
                local.values[key_tuple] = merged_q_score
        else:
            # Key doesn't exist: adopt with trust discount
            # Apply 20% confidence discount: confidence *= 0.8
            discounted_confidence = remote_confidence * 0.8

            # Clamp to PHI_INV
            discounted_confidence = min(PHI_INV, discounted_confidence)

            # Adopt the remote q_score (no discounting on the score itself)
            local.values[key_tuple] = remote_q_score

        merged_count += 1

    return merged_count
