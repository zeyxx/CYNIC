"""
ModelProfiler â€" Metabolic Resource Awareness.

Analyzes hardware (CPU/RAM) and estimates cycle capacity.
Inspired by llmfit, optimized for Ryzen 5700G.
"""

from __future__ import annotations

import logging
import platform
from dataclasses import dataclass

import psutil

logger = logging.getLogger("cynic.kernel.organism.metabolism.model_profiler")


@dataclass(frozen=True)
class HardwareProfile:
    cpu_cores: int
    total_ram_gb: float
    available_ram_gb: float
    is_apu: bool = True
    gpu_name: str = "Radeon Vega 8"
    speed_k: float = 70.0


@dataclass
class MetabolicCapacity:
    cycles_per_second: float
    max_context_window: int
    memory_fit_score: float
    bottleneck: str
    compute_mode: str # CPU | iGPU | GPU


class ModelProfiler:
    """Estimates the organism's ability to 'breathe' (process cycles)."""

    def __init__(self):
        self.profile = self._probe_hardware()

    def _probe_hardware(self) -> HardwareProfile:
        mem = psutil.virtual_memory()
        cpu_brand = platform.processor()
        
        # Detection logic for Ryzen APUs
        is_amd = "AMD" in cpu_brand or "Ryzen" in cpu_brand
        gpu_detected = "Radeon" # Default for 5700G
        
        return HardwareProfile(
            cpu_cores=psutil.cpu_count(logical=True) or 8,
            total_ram_gb=round(mem.total / (1024**3), 2),
            available_ram_gb=round(mem.available / (1024**3), 2),
            is_apu=is_amd,
            gpu_name=gpu_detected
        )

    def estimate_capacity(self, model_size_gb: float = 5.0) -> MetabolicCapacity:
        """
        Calculates capacity based on APU architecture (Shared RAM).
        """
        # On a 5700G, we must reserve RAM for both CPU and iGPU
        reserved_ram = 4.0 # OS + Background
        effective_ram = self.profile.available_ram_gb - reserved_ram
        
        fit_score = min(1.0, effective_ram / (model_size_gb * 1.2))

        # Capacity: If we can use the iGPU, throughput increases for tensor tasks
        compute_mode = "iGPU" if self.profile.is_apu else "CPU"
        
        # Multiplier: iGPU (Vega 8) handles parallel tasks better than raw CPU
        acceleration = 2.5 if compute_mode == "iGPU" else 1.0
        
        cycles_sec = (self.profile.speed_k * self.profile.cpu_cores * fit_score * acceleration) / 100

        bottleneck = "CPU"
        if fit_score < 0.6:
            bottleneck = "RAM"
        if self.profile.available_ram_gb < 16:
            bottleneck = "VRAM_CRITICAL" # APU specific: shared RAM exhaustion

        return MetabolicCapacity(
            cycles_per_second=round(cycles_sec, 2),
            max_context_window=8192 if compute_mode == "iGPU" else 4096,
            memory_fit_score=fit_score,
            bottleneck=bottleneck,
            compute_mode=compute_mode
        )

    def announce_limits(self) -> str:
        cap = self.estimate_capacity()
        return (
            f"METABOLIC LIMITS: Running on {platform.processor()} | "
            f"Cap: {cap.cycles_per_second} cycles/s | "
            f"Fit: {cap.memory_fit_score:.1%} | "
            f"Bottleneck: {cap.bottleneck}"
        )
