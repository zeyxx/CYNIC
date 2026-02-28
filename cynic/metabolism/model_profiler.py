"""
ModelProfiler — Metabolic Resource Awareness.

Analyzes hardware (CPU/RAM) and estimates cycle capacity.
Inspired by llmfit, optimized for Ryzen 5700G.
"""
from __future__ import annotations

import logging
import psutil
import platform
import time
from dataclasses import dataclass, field
from typing import Dict, Optional

logger = logging.getLogger("cynic.metabolism.model_profiler")

@dataclass(frozen=True)
class HardwareProfile:
    cpu_cores: int
    total_ram_gb: float
    available_ram_gb: float
    is_apu: bool = True # Ryzen 5700G is an APU (integrated Vega 8)
    speed_k: float = 70.0 # x86 CPU constant from llmfit

@dataclass
class MetabolicCapacity:
    cycles_per_second: float
    max_context_window: int
    memory_fit_score: float # [0, 1]
    bottleneck: str # CPU | RAM | VRAM

class ModelProfiler:
    """Estimates the organism's ability to 'breathe' (process cycles)."""
    
    def __init__(self):
        self.profile = self._probe_hardware()
        
    def _probe_hardware(self) -> HardwareProfile:
        mem = psutil.virtual_memory()
        return HardwareProfile(
            cpu_cores=psutil.cpu_count(logical=True) or 8,
            total_ram_gb=round(mem.total / (1024**3), 2),
            available_ram_gb=round(mem.available / (1024**3), 2),
            is_apu="AMD" in platform.processor() and "Vega" in platform.processor() or True
        )

    def estimate_capacity(self, model_size_gb: float = 5.0) -> MetabolicCapacity:
        """
        Calculates how many cycles CYNIC can run.
        On a 5700G, RAM is shared. If model_size exceeds available_ram, 
        bottleneck is RAM.
        """
        fit_score = min(1.0, self.profile.available_ram_gb / (model_size_gb * 1.5))
        
        # Simple capacity model: K * Cores * Fit / Complexity
        cycles_sec = (self.profile.speed_k * self.profile.cpu_cores * fit_score) / 100
        
        bottleneck = "CPU"
        if fit_score < 0.5:
            bottleneck = "RAM"
            
        return MetabolicCapacity(
            cycles_per_second=round(cycles_sec, 2),
            max_context_window=4096 if bottleneck == "CPU" else 2048,
            memory_fit_score=fit_score,
            bottleneck=bottleneck
        )

    def announce_limits(self) -> str:
        cap = self.estimate_capacity()
        return (f"METABOLIC LIMITS: Running on {platform.processor()} | "
                f"Cap: {cap.cycles_per_second} cycles/s | "
                f"Fit: {cap.memory_fit_score:.1%} | "
                f"Bottleneck: {cap.bottleneck}")
