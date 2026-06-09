#!/usr/bin/env python3
"""
Hardware-aware model recommendations for /judge endpoint.
Combines X Organ engagement data with hardware constraints.

APU: 4-9B models (limited memory, no discrete GPU)
GPU (RTX 4060 Ti 16GB): 9-27B models (full context + precision)
"""

import json
from pathlib import Path
from typing import Dict, List

def analyze_for_hardware(rankings_file: str = "observations/x_model_rankings.json") -> None:
    """Analyze X rankings filtered by hardware feasibility."""

    with open(rankings_file) as f:
        data = json.load(f)

    models = data["models"]

    # Filter: valid size data + meaningful engagement
    valid = [m for m in models if m["size_b"] and m["engagement"] > 100]

    # Separate by target hardware
    apu_models = [m for m in valid if m["size_b"] and m["size_b"] <= 9]
    gpu_models = [m for m in valid if m["size_b"] and 9 < m["size_b"] <= 27]

    print("\n" + "="*80)
    print("MODEL RECOMMENDATIONS FOR /JUDGE INFERENCE")
    print("="*80)

    print("\n## APU (cynic-core: limited memory, Vulkan/CPU)")
    print("### Constraint: ≤9B params, CPU/APU-optimized")
    print()

    for rank, model in enumerate(apu_models[:5], 1):
        print(f"{rank}. **{model['model']}**")
        print(f"   - Engagement: {model['engagement']:,.0f} (views: {model['views']:,})")
        print(f"   - Mentions: {model['mentions']} tweets from real deployments")
        if model["avg_throughput_toks"]:
            print(f"   - Inference speed: {model['avg_throughput_toks']:.1f} tok/s (reported avg)")
        if model["hardware_targets"]:
            print(f"   - Tested on: {', '.join(model['hardware_targets'])}")
        if model["vram_gb"]:
            print(f"   - RAM requirement: {model['vram_gb']} GB")
        print()

    print("\n## GPU (cynic-gpu: RTX 4060 Ti 16GB VRAM)")
    print("### Constraint: ≤27B params, CUDA/vLLM/ExLlama-optimized")
    print()

    for rank, model in enumerate(gpu_models[:5], 1):
        print(f"{rank}. **{model['model']}**")
        print(f"   - Engagement: {model['engagement']:,.0f} (views: {model['views']:,})")
        print(f"   - Mentions: {model['mentions']} tweets from real deployments")
        if model["avg_throughput_toks"]:
            print(f"   - Inference speed: {model['avg_throughput_toks']:.1f} tok/s (reported avg)")
        if model["hardware_targets"]:
            print(f"   - Tested on: {', '.join(model['hardware_targets'])}")
        if model["vram_gb"]:
            print(f"   - VRAM requirement: {model['vram_gb']} GB")
        print()

    # Generate JSON for programmatic use
    recommendations = {
        "generated": data["generated"],
        "apu": {
            "constraint": "≤9B params, CPU/Vulkan",
            "recommended": [
                {
                    "rank": rank,
                    "model": m["model"],
                    "engagement": m["engagement"],
                    "mentions": m["mentions"],
                    "estimated_throughput": m["avg_throughput_toks"],
                    "hardware": m["hardware_targets"],
                    "vram_gb": m["vram_gb"]
                }
                for rank, m in enumerate(apu_models[:3], 1)
            ]
        },
        "gpu": {
            "constraint": "≤27B params, CUDA-optimized",
            "recommended": [
                {
                    "rank": rank,
                    "model": m["model"],
                    "engagement": m["engagement"],
                    "mentions": m["mentions"],
                    "estimated_throughput": m["avg_throughput_toks"],
                    "hardware": m["hardware_targets"],
                    "vram_gb": m["vram_gb"]
                }
                for rank, m in enumerate(gpu_models[:3], 1)
            ]
        }
    }

    output_file = Path("observations/hardware_recommendations.json")
    output_file.parent.mkdir(parents=True, exist_ok=True)

    with open(output_file, "w") as f:
        json.dump(recommendations, f, indent=2)

    print(f"\n✅ Recommendations saved to {output_file}")
    print("\nNext steps:")
    print("1. APU: Test Qwen3.5-9B (8GB VRAM, real deployment data)")
    print("2. GPU: Test Qwen3.6-27B (64 tok/s from Twitter reports)")
    print("3. Run benchmarks/convergence.py to measure before/after on each hardware")


if __name__ == "__main__":
    analyze_for_hardware()
