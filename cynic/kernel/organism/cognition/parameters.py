"""
CYNIC Synaptic Parameter Governor.
Determines the optimal LLM parameters based on the Axiomatic Task.
Inspired by Unsloth optimization tutorials.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, Any

@dataclass
class SynapticParams:
    temperature: float = 0.0
    top_p: float = 0.95
    min_p: float = 0.05 # Unsloth recommendation for better quality
    repeat_penalty: float = 1.1
    max_tokens: int = 4096 # Higher for Qwen 3.5 context
    stop_sequences: list[str] = field(default_factory=list)
    cache_prompt: bool = True # Essential for APU to avoid re-evaluating context
    # Unsloth/Llama.cpp specifics
    rope_freq_base: float = 0.0 
    rope_freq_scale: float = 0.0

class ParameterGovernor:
    """
    Adjusts the 'Synaptic Tension' of the LLM based on the required Axiom.
    """
    @staticmethod
    def get_params_for_axiom(axiom: str) -> SynapticParams:
        if axiom == "BACKEND":
            # Maximum precision, zero drift
            return SynapticParams(temperature=0.0, top_p=0.1, repeat_penalty=1.2)
        
        if axiom == "ARCHITECTURE":
            # Higher context potential, slightly more creative
            return SynapticParams(temperature=0.3, top_p=0.9, max_tokens=4096)
        
        if axiom == "PHI" or axiom == "CULTURE":
            # Philosophical judgment requires broader sampling
            return SynapticParams(temperature=0.7, top_p=0.95)
            
        if axiom == "BURN":
            # Extreme pruning/optimization requires radical focus
            return SynapticParams(temperature=0.0, repeat_penalty=1.5)

        return SynapticParams() # Default
