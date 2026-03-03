"""
CYNIC Compute HAL  Universal Hardware Abstraction.

Routes tensor operations to the best available compute engine.
Specialized for Windows APUs (Ryzen 5700G + Radeon Vega 8).
Supports: CPU (AVX2), DirectML (AMD iGPU), CUDA (NVIDIA).
"""

import logging
import torch
from typing import Optional

logger = logging.getLogger("cynic.kernel.infrastructure.compute_hal")

import logging
import psutil
from typing import Any

logger = logging.getLogger("cynic.kernel.infrastructure.compute_hal")

class ComputeHAL:
    """
    Industrial-grade Hardware Abstraction Layer for CYNIC.
    Optimized for Ryzen 5700G (APU) and shared memory architectures.
    """
    def __init__(self):
        self.device_name = "cpu"
        self.backend = "pytorch-cpu"
        self.is_accelerated = False
        self.vram_limit_gb = 2.0 # Default conservative for APU
        self._probe()

    def _probe(self) -> None:
        """Hardware discovery with specific error handling and safety."""
        try:
            mem = psutil.virtual_memory()
            total_ram: float = mem.total / (1024**3)
            logger.debug(f"HAL: Probing hardware. System RAM: {total_ram:.2f}GB")
        except (AttributeError, PermissionError) as e:
            logger.error(f"HAL: Resource probe failed: {e}")
            total_ram = 8.0 # Conservative fallback
        
        # 1. DirectML (AMD/Intel on Windows)
        try:
            import torch_directml
            if torch_directml.is_available():
                self.device_name = str(torch_directml.device())
                self.backend = "directml"
                self.is_accelerated = True
                self.vram_limit_gb = round(total_ram * 0.25, 2)
                return
        except ImportError:
            logger.debug("HAL: torch-directml not installed.")
        except RuntimeError as e:
            logger.error(f"HAL: DirectML runtime error: {e}")

        # 2. CUDA (NVIDIA)
        try:
            if torch.cuda.is_available():
                self.device_name = "cuda"
                self.backend = "cuda"
                self.is_accelerated = True
                self.vram_limit_gb = round(torch.cuda.get_device_properties(0).total_memory / (1024**3), 2)
                return
        except (RuntimeError, AssertionError) as e:
            logger.error(f"HAL: CUDA acceleration unavailable: {e}")

        # 3. CPU AVX2 (Optimized Fallback)
        self.device_name = "cpu"
        self.backend = "cpu-avx2"
        self.is_accelerated = False
        logger.info(f"HAL: Final fallback to CPU. Cores: {psutil.cpu_count()}")

    def sync_tensor(self, data: Any) -> torch.Tensor:
        """Ensures a tensor is on the correct compute device with full safety."""
        if not isinstance(data, torch.Tensor):
            try:
                data = torch.tensor(data, dtype=torch.float32)
            except (ValueError, TypeError) as e:
                logger.error(f"HAL: Conversion failed: {e}")
                return torch.zeros((1,))
        
        try:
            target: str = self.get_device()
            return data.to(target) if data.device != target else data
        except (RuntimeError, ValueError) as e:
            logger.error(f"HAL: Device move failed: {e}")
            return data.to("cpu")

    def get_device(self):
        return self.device_name

    def announce_compute(self) -> str:
        status = "ACCELERATED" if self.is_accelerated else "EMULATED"
        return (
            f"COMPUTE ENGINE: {self.backend.upper()} | "
            f"Status: {status} | "
            f"VRAM Limit: {self.vram_limit_gb}GB | "
            f"Device: {self.device_name}"
        )

# Singleton for the organism
_GLOBAL_HAL: Optional[ComputeHAL] = None

def get_compute_hal() -> ComputeHAL:
    global _GLOBAL_HAL
    if _GLOBAL_HAL is None:
        _GLOBAL_HAL = ComputeHAL()
    return _GLOBAL_HAL
