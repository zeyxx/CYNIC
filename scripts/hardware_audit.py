"""
CYNIC Hardware Audit — Remove the fog of infrastructure.

Detects CPU/iGPU capabilities (optimized for Ryzen 5700G) and 
recommends LLM inference settings for the V3.1 Organism.
"""
import os
import platform
import multiprocessing
import psutil
import sys

def audit():
    print("\n" + "="*60)
    print("  CYNIC HARDWARE AUDIT - INFRASTRUCTURE TRANSPARENCY")
    print("="*60 + "\n")

    # 1. OS & Platform
    print(f"OS        : {platform.system()} {platform.release()}")
    print(f"Machine   : {platform.machine()}")
    print(f"Python    : {sys.version.split()[0]}")

    # 2. CPU (The Ryzen Core)
    cpu_count = multiprocessing.cpu_count()
    cpu_freq = psutil.cpu_freq().max if psutil.cpu_freq() else "N/A"
    print(f"\nCPU CORE  : {platform.processor()}")
    print(f"Threads   : {cpu_count} cores detected")
    print(f"Frequency : {cpu_freq} MHz")

    # 3. Memory (The shared buffer)
    mem = psutil.virtual_memory()
    print(f"\nMEMORY    : {mem.total / (1024**3):.2f} GB total")
    print(f"Available : {mem.available / (1024**3):.2f} GB")

    # 4. Recommendation (The Unfoggy extension)
    print("\n" + "-"*60)
    print("  OPTIMIZATION STRATEGY (V3.1)")
    print("-" * 60)
    
    cpu_name = platform.processor()
    if "AMD" in cpu_name or "Ryzen" in cpu_name:
        print("DETECTED: AMD Ryzen high-performance CPU.")
        print("RECOMMENDATION: Use LlamaCppAdapter with GGUF models.")
        print(f"  - Set n_threads={cpu_count - 2} (keep 2 for OS/Kernel)")
        print("  - Use 4-bit quantization (Q4_K_M) for best speed/accuracy ratio.")
        print("  - If Vulkan is enabled, set n_gpu_layers=-1 for iVega 8 offload.")
    else:
        print("GENERIC: System detected as standard infrastructure.")
        print("RECOMMENDATION: OllamaAdapter (Service-based) is safer.")

    print("\n" + "="*60)
    print("  Organism limits: 4096 context window recommended for CPU.")
    print("="*60 + "\n")

if __name__ == "__main__":
    audit()
