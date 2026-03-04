"""
CYNIC KV Cache Manager (LMCache Doctrine).
Manages llama-server slots to reuse precomputed context tensors.
"""
from __future__ import annotations
import logging
from typing import Dict, List, Optional

logger = logging.getLogger("cynic.organism.kv_manager")

class KVManager:
    """
    Allocates and tracks KV slots in llama-server to minimize prefill latency.
    """
    def __init__(self):
        self.system_prompt_slot = 0
        self.agent_slots: Dict[str, int] = {} # AgentID -> SlotID
        self.next_available_slot = 1

    def get_slot_for_task(self, agent_id: str, is_recurring: bool = True) -> int:
        """Determines which KV slot to use for the inference call."""
        if not is_recurring:
            return -1 # Random slot
            
        if agent_id in self.agent_slots:
            return self.agent_slots[agent_id]
            
        # Allocate new slot
        slot = self.next_available_slot
        self.agent_slots[agent_id] = slot
        self.next_available_slot += 1
        logger.info(f"KVManager: Allocated slot {slot} for agent {agent_id}")
        return slot

    def reset_slots(self):
        self.agent_slots = {}
        self.next_available_slot = 1
