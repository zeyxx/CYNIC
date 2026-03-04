"""
CYNIC Synaptic Pipeline - Industrial Information Processing.
Decomposes task execution into a series of resilient, typed stages (Synapses).
Inspired by the 9 Engineering Lenses.
"""
from __future__ import annotations
import time
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

logger = logging.getLogger("cynic.organism.pipeline")

@dataclass
class SynapticContext:
    """The 'Blood' of the pipeline, carrying data between synapses."""
    task_id: str
    axiom: str
    raw_input: str
    filtered_input: str = ""
    relevant_facts: List[str] = field(default_factory=list)
    selected_muscle: str = ""
    parameters: Dict[str, Any] = field(default_factory=dict)
    response_content: str = ""
    latency_ms: float = 0.0
    success: bool = False
    metadata: Dict[str, Any] = field(default_factory=dict)

class Synapse(ABC):
    """A single stage in the synaptic pipeline."""
    @abstractmethod
    async def fire(self, context: SynapticContext) -> bool:
        """Processes the context. Returns True to continue, False to abort."""
        pass

class SynapticPipeline:
    """Orchestrates the firing of multiple synapses."""
    def __init__(self):
        self.synapses: List[Synapse] = []

    def add_synapse(self, synapse: Synapse):
        self.synapses.append(synapse)

    async def execute(self, context: SynapticContext) -> SynapticContext:
        start_time = time.time()
        for synapse in self.synapses:
            synapse_name = synapse.__class__.__name__
            try:
                logger.debug(f"Pipeline: Firing {synapse_name}...")
                if not await synapse.fire(context):
                    logger.warning(f"Pipeline: {synapse_name} aborted the sequence.")
                    break
            except Exception as e:
                logger.error(f"Pipeline: Critical failure in {synapse_name}: {e}")
                context.metadata["error"] = str(e)
                break
        
        context.latency_ms = (time.time() - start_time) * 1000
        return context
