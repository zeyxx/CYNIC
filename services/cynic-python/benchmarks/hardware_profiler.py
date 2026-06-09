#!/usr/bin/env python3
"""
Hardware profiler: observe real CPU, memory, GPU, thermal during benchmark.
Self-evolving organ framework.

Version: 0.1.0
"""

import subprocess
import json
import time
import logging
import os
from dataclasses import dataclass, asdict
from datetime import datetime
from typing import Dict, Optional

logger = logging.getLogger(__name__)


@dataclass
class HardwareSnapshot:
    """Single hardware observation."""
    timestamp: str
    cpu_percent: float  # 0-100
    memory_used_gb: float
    memory_percent: float  # 0-100
    swap_used_gb: float
    swap_percent: float  # 0-100
    thermal_celsius: Optional[float] = None  # CPU temp
    gpu_utilization_percent: Optional[float] = None
    gpu_memory_percent: Optional[float] = None

    def to_dict(self) -> Dict:
        return asdict(self)


class HardwareProfiler:
    """Monitor hardware during inference."""

    def __init__(self, dog_name: str, process_name: str = "llama-server"):
        self.dog_name = dog_name
        self.process_name = process_name
        self.snapshots = []

    def take_snapshot(self) -> HardwareSnapshot:
        """Capture hardware state right now."""
        cpu_percent = self._get_cpu_percent()
        memory = self._get_memory()
        swap = self._get_swap()
        thermal = self._get_thermal()
        gpu = self._get_gpu_stats()

        snap = HardwareSnapshot(
            timestamp=datetime.now().isoformat(),
            cpu_percent=cpu_percent,
            memory_used_gb=memory["used"],
            memory_percent=memory["percent"],
            swap_used_gb=swap["used"],
            swap_percent=swap["percent"],
            thermal_celsius=thermal,
            gpu_utilization_percent=gpu.get("util") if gpu else None,
            gpu_memory_percent=gpu.get("mem") if gpu else None,
        )
        self.snapshots.append(snap)
        return snap

    def _get_cpu_percent(self) -> float:
        """CPU % for llama-server process."""
        try:
            result = subprocess.run(
                ['ps', 'aux'],
                capture_output=True, text=True, timeout=2
            )
            for line in result.stdout.split('\n'):
                if self.process_name in line:
                    parts = line.split()
                    if len(parts) > 2:
                        return float(parts[2])  # %CPU
        except Exception as e:
            logger.debug(f"CPU sample failed: {e}")
        return 0.0

    def _get_memory(self) -> Dict[str, float]:
        """Memory usage GB + percent."""
        try:
            result = subprocess.run(
                ['free', '-b'],
                capture_output=True, text=True, timeout=2
            )
            for line in result.stdout.split('\n'):
                if 'Mem:' in line:
                    parts = line.split()
                    total = float(parts[1])
                    used = float(parts[2])
                    percent = (used / total) * 100 if total > 0 else 0
                    return {
                        "used": used / (1024**3),  # GB
                        "total": total / (1024**3),
                        "percent": percent
                    }
        except Exception as e:
            logger.debug(f"Memory sample failed: {e}")
        return {"used": 0.0, "total": 0.0, "percent": 0.0}

    def _get_swap(self) -> Dict[str, float]:
        """Swap usage GB + percent."""
        try:
            result = subprocess.run(
                ['free', '-b'],
                capture_output=True, text=True, timeout=2
            )
            for line in result.stdout.split('\n'):
                if 'Swap:' in line:
                    parts = line.split()
                    total = float(parts[1])
                    used = float(parts[2])
                    percent = (used / total) * 100 if total > 0 else 0
                    return {
                        "used": used / (1024**3),  # GB
                        "total": total / (1024**3),
                        "percent": percent
                    }
        except Exception as e:
            logger.debug(f"Swap sample failed: {e}")
        return {"used": 0.0, "total": 0.0, "percent": 0.0}

    def _get_thermal(self) -> Optional[float]:
        """CPU temperature Celsius (if available)."""
        try:
            result = subprocess.run(
                ['cat', '/sys/class/thermal/thermal_zone0/temp'],
                capture_output=True, text=True, timeout=2
            )
            temp_millidegrees = float(result.stdout.strip())
            return temp_millidegrees / 1000.0  # Convert to Celsius
        except Exception:
            return None

    def _get_gpu_stats(self) -> Optional[Dict[str, float]]:
        """GPU utilization (nvidia-smi), returns None if unavailable."""
        try:
            result = subprocess.run(
                ['nvidia-smi', '--query-gpu=utilization.gpu,utilization.memory', '--format=csv,noheader'],
                capture_output=True, text=True, timeout=2
            )
            parts = result.stdout.strip().split(',')
            if len(parts) >= 2:
                return {
                    "util": float(parts[0].replace('%', '').strip()),
                    "mem": float(parts[1].replace('%', '').strip()),
                }
        except Exception:
            return None

    def summary(self) -> Dict:
        """Aggregate statistics across all snapshots."""
        if not self.snapshots:
            return {}

        cpu_values = [s.cpu_percent for s in self.snapshots]
        mem_values = [s.memory_percent for s in self.snapshots]
        swap_values = [s.swap_percent for s in self.snapshots]

        return {
            "dog_name": self.dog_name,
            "num_samples": len(self.snapshots),
            "cpu_stats": {
                "mean": sum(cpu_values) / len(cpu_values) if cpu_values else 0,
                "max": max(cpu_values) if cpu_values else 0,
                "min": min(cpu_values) if cpu_values else 0,
            },
            "memory_stats": {
                "mean_percent": sum(mem_values) / len(mem_values) if mem_values else 0,
                "max_percent": max(mem_values) if mem_values else 0,
            },
            "swap_stats": {
                "mean_percent": sum(swap_values) / len(swap_values) if swap_values else 0,
                "max_percent": max(swap_values) if swap_values else 0,
                "peak_used_gb": max([s.swap_used_gb for s in self.snapshots]) if self.snapshots else 0,
            },
        }


def save_profile(profiler: HardwareProfiler, output_dir: str = "observations") -> str:
    """Save hardware profile to JSON."""
    os.makedirs(output_dir, exist_ok=True)

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"{output_dir}/{profiler.dog_name}-hwprofile-{timestamp}.json"

    data = {
        "metadata": {
            "dog_name": profiler.dog_name,
            "timestamp": datetime.now().isoformat(),
            "version": "0.1.0",
        },
        "summary": profiler.summary(),
        "snapshots": [s.to_dict() for s in profiler.snapshots],
    }

    with open(filename, "w") as f:
        json.dump(data, f, indent=2)

    logger.info(f"✅ Saved hardware profile to {filename}")
    return filename


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    profiler = HardwareProfiler("gemma-4-e4b")

    # Sample every 2 seconds for 10 seconds
    for i in range(5):
        snap = profiler.take_snapshot()
        print(f"[{i}] CPU:{snap.cpu_percent:.1f}% MEM:{snap.memory_percent:.1f}% SWAP:{snap.swap_percent:.1f}%")
        time.sleep(2)

    save_profile(profiler)
    print(f"\nSummary: {json.dumps(profiler.summary(), indent=2)}")
