#!/usr/bin/env python3
import json
from pathlib import Path
from collections import defaultdict

def analyze():
    input_file = Path.home() / ".cynic" / "organs" / "hermes" / "github" / "stars.jsonl"
    output_file = Path.home() / ".cynic" / "organs" / "hermes" / "github" / "analysis.json"
    
    stars = []
    with open(input_file, 'r') as f:
        for line in f:
            stars.append(json.loads(line))
            
    categories = defaultdict(list)
    
    for star in stars:
        name = star['name'].lower()
        desc = star['description'].lower()
        
        if any(kw in name or kw in desc for kw in ["agent", "orchestration", "maestro", "dexter", "conductor"]):
            categories["AGENT_OS"].append(star)
        elif any(kw in name or kw in desc for kw in ["stealth", "bypass", "scraping", "fingerprint", "cloak", "scraping"]):
            categories["AGENT_STEALTH"].append(star)
        elif any(kw in name or kw in desc for kw in ["polymarket", "trading", "prediction", "ccxt", "financial"]):
            categories["AGENT_FINANCE"].append(star)
        elif any(kw in name or kw in desc for kw in ["skill", "tool", "integration", "ecosystem", "wiki", "atlas"]):
            categories["AGENT_SKILL"].append(star)
        elif any(kw in name or kw in desc for kw in ["llm", "benchmarks", "serving", "database", "surrealdb", "firedancer", "rtx"]):
            categories["INFRA"].append(star)
        else:
            categories["MISC"].append(star)
            
    # Axiomatic Evaluation (Aggregate)
    evaluation = {
        "AGENT_OS": {
            "fidelity": 0.8, # High: structured missions
            "phi": 0.7,      # Harmonious coordination
            "verify": 0.9,   # Evidence-based
            "sovereignty": 0.8, # Local-first state
            "burn": 0.6      # Orchestration overhead
        },
        "AGENT_STEALTH": {
            "fidelity": 0.9, # Real bot detection bypass
            "phi": 0.5,      # Brittle patches
            "verify": 0.7,   # Passmarks
            "sovereignty": 0.9, # Independent browsing
            "burn": 0.4      # Heavy compute/infra
        },
        "AGENT_FINANCE": {
            "fidelity": 0.95,# Real money at stake
            "phi": 0.6,      # Complex APIs
            "verify": 1.0,   # P&L is the truth
            "sovereignty": 0.7, # Exchange dependency
            "burn": 0.8      # Efficient arbitrage
        }
    }
    
    report = {
        "categories": {k: [s['name'] for s in v] for k, v in categories.items()},
        "evaluation": evaluation,
        "summary": "T.'s stars reveal a high-fidelity focus on autonomous sovereign agents. The transition from 'browsing' to 'executing' is supported by a robust stealth and finance stack."
    }
    
    with open(output_file, 'w') as f:
        json.dump(report, f, indent=2)
        
    print(f"Analysis complete. Saved to {output_file}")

if __name__ == "__main__":
    analyze()
