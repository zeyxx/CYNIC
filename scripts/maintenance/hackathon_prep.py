"""
CYNIC - Hackathon Preparation Kit (Engineering Foundation)

This script provides utilities to prepare CYNIC for high-performance 
kernel integration without implementing the kernels themselves.
"""

import time
import torch
from typing import List
from cynic.kernel.core.phi import weighted_geometric_mean, Aggregator, set_aggregator

class ProfilingAggregator:
    """
    A decorator aggregator that measures performance of the underlying strategy.
    Use this during the hackathon to prove your speedups.
    """
    def __init__(self, strategy: Aggregator):
        self.strategy = strategy
        self.total_calls = 0
        self.total_time = 0.0
        self.min_time = float('inf')
        self.max_time = 0.0

    def compute(self, values: List[float], weights: List[float]) -> float:
        t0 = time.perf_counter()
        result = self.strategy.compute(values, weights)
        t1 = time.perf_counter()
        
        duration = t1 - t0
        self.total_calls += 1
        self.total_time += duration
        self.min_time = min(self.min_time, duration)
        self.max_time = max(self.max_time, duration)
        
        return result

    def report(self):
        if self.total_calls == 0:
            return "No calls recorded."
        
        avg = self.total_time / self.total_calls
        return (
            f"--- 📊 Aggregation Profile ---
"
            f"Calls: {self.total_calls}
"
            f"Avg Latency: {avg*1000:.6f} ms
"
            f"Min/Max: {self.min_time*1000:.6f} / {self.max_time*1000:.6f} ms
"
            f"Throughput: {1/avg if avg > 0 else 0:,.2f} ops/sec
"
        )

def run_readiness_check():
    """Verify that CYNIC is ready for the Helion integration."""
    print("🚀 CYNIC Hackathon Readiness Check")
    
    # 1. Check Hardware
    print(f"CUDA Available: {torch.cuda.is_available()}")
    if torch.cuda.is_available():
        print(f"GPU: {torch.cuda.get_device_name(0)}")
    
    # 2. Check Interface
    from cynic.kernel.core.phi import _ACTIVE_AGGREGATOR
    print(f"Active Aggregator: {_ACTIVE_AGGREGATOR.__class__.__name__}")
    
    # 3. Test Baseline Performance
    prof = ProfilingAggregator(_ACTIVE_AGGREGATOR)
    set_aggregator(prof)
    
    print("
Running baseline profiling (Python)...")
    for _ in range(1000):
        weighted_geometric_mean([10.0, 20.0, 30.0], [1.0, 1.0, 1.0])
    
    print(prof.report())

if __name__ == "__main__":
    run_readiness_check()
