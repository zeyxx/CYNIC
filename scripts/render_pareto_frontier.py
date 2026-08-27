#!/usr/bin/env python3
"""
render_pareto_frontier.py — Multi-Objective Pareto Frontier Visualizer & Scorecard Generator.

Parses benchmark results, computes multi-objective non-dominated configurations,
and renders visual ASCII Pareto plots with detailed rankings.
"""

import json
import os
import sys

def dominates(a, b):
    """
    Returns True if configuration `a` dominates `b`:
    - higher decode_tps is better
    - higher prefill_tps is better
    - lower ttft_ms is better
    - lower rss_peak_mb is better
    """
    not_worse = (
        a["decode_tps"] >= b["decode_tps"] and
        a["prefill_tps"] >= b["prefill_tps"] and
        a["ttft_ms"] <= b["ttft_ms"] and
        a["rss_peak_mb"] <= b["rss_peak_mb"]
    )
    strictly_better = (
        a["decode_tps"] > b["decode_tps"] or
        a["prefill_tps"] > b["prefill_tps"] or
        a["ttft_ms"] < b["ttft_ms"] or
        a["rss_peak_mb"] < b["rss_peak_mb"]
    )
    return not_worse and strictly_better

def extract_pareto_frontier(results):
    frontier = []
    for item in results:
        is_dominated = False
        for other in results:
            if item != other and dominates(other, item):
                is_dominated = True
                break
        if not is_dominated:
            frontier.append(item)
    return frontier

def render_ascii_plot(results):
    print("\n" + "=" * 85)
    print(" 📈 CARTE DE PARETO : DÉBIT DE GÉNÉRATION (TG tok/s) vs TTFT (ms) — RYZEN 7 5700G")
    print("=" * 85)
    
    # Sort by decode_tps descending
    sorted_res = sorted(results, key=lambda x: x["decode_tps"], reverse=True)
    max_tps = max(r["decode_tps"] for r in results) if results else 50.0
    
    for r in sorted_res:
        alias = r["model_alias"]
        ngl = r["ngl"]
        ctx = r["ctx_size"]
        tps = r["decode_tps"]
        ttft = r["ttft_ms"]
        bw = r["effective_bw_gbps"]
        
        bar_len = int((tps / max_tps) * 35)
        bar = "█" * bar_len
        
        mode = "GPU-Vulkan" if ngl == 99 else ("Hybrid" if ngl > 0 else "CPU-Zen3")
        tag = f"{alias:15s} | {mode:10s} | ctx={ctx:5d}"
        print(f"{tag} | {bar:<35s} {tps:5.2f} t/s (TTFT: {ttft:6.0f}ms, BW: {bw:4.1f} GB/s)")
    print("=" * 85)

def main():
    json_path = "benchmark-5700g-empirical-results.json"
    if not os.path.exists(json_path):
        print(f"Error: {json_path} not found.")
        sys.exit(1)
        
    with open(json_path) as f:
        results = json.load(f)
        
    frontier = extract_pareto_frontier(results)
    
    print("\n" + "#" * 85)
    print("  RAPPORT D'ANALYSE DE LA FRONTIÈRE DE PARETO (AMD RYZEN 7 5700G)")
    print("#" * 85)
    print(f"\nTotal de configurations testées : {len(results)}")
    print(f"Configurations sur la frontière de Pareto (non dominées) : {len(frontier)}")
    
    print("\n🏆 CONFIGURATIONS SUR LA FRONTIÈRE DE PARETO :")
    print("-" * 85)
    print(f"{'Modèle':<18s} {'Mode':<12s} {'Prompt':<8s} {'Prefill (tok/s)':<16s} {'Decode (tok/s)':<15s} {'TTFT':<10s} {'RSS (Mo)':<10s}")
    print("-" * 85)
    for p in sorted(frontier, key=lambda x: x["decode_tps"], reverse=True):
        mode = "Full Vulkan" if p["ngl"] == 99 else ("Hybrid" if p["ngl"] > 0 else "CPU-only")
        print(f"{p['model_alias']:<18s} {mode:<12s} {p['prompt_tokens']:<8d} {p['prefill_tps']:<16.2f} {p['decode_tps']:<15.2f} {p['ttft_ms']:<8.0f}ms {p['rss_peak_mb']:<10.1f}")
    print("-" * 85)
    
    render_ascii_plot(results)

if __name__ == "__main__":
    main()
