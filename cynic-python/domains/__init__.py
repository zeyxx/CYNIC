"""
CYNIC Domain Intelligence Framework

6 domains of wisdom: D1-D6
Coverage-driven exploration: lowest coverage gaps = highest value signals
"""

DOMAINS = {
    "D1": {
        "name": "Solana/Tokens",
        "coverage": 0.11,
        "consumer": "Dogs (rug detection, token authenticity)",
        "coverage_metric": "583 raw high-signal tweets processed",
        "signals": ["rug_mechanics", "launch_patterns", "swap_signatures"]
    },
    "D2": {
        "name": "Inference/LLM",
        "coverage": 1.18,  # Oversampled
        "consumer": "Inference lab (model selection, VRAM math)",
        "coverage_metric": "236 curated tweets (118% baseline)",
        "signals": ["hardware_benchmarks", "quantization_tradeoffs", "agent_frameworks"]
    },
    "D3": {
        "name": "Sovereignty",
        "coverage": 0.0,  # ZERO — highest priority gap
        "consumer": "CYNIC identity (axiom grounding, philosophy)",
        "coverage_metric": "0 signals captured",
        "signals": ["infrastructure_independence", "epistemic_authority", "cultural_preservation"]
    },
    "D4": {
        "name": "Security/Scams",
        "coverage": 0.70,
        "consumer": "Dogs (exploit patterns, social engineering)",
        "coverage_metric": "139 curated scam patterns",
        "signals": ["rug_detection", "fake_launchpad", "honeypot", "social_exploit"]
    },
    "D5": {
        "name": "Macro/Politics",
        "coverage": 0.30,  # Noisy, needs cleanup
        "consumer": "Market context (regulatory, geopolitical)",
        "coverage_metric": "301 raw (mixed signal/noise ratio)",
        "signals": ["regulation", "geopolitical", "election_impact", "macro_cycles"]
    },
    "D6": {
        "name": "Epistemology/Philosophy",
        "coverage": 0.12,
        "consumer": "Axiom calibration (truth definition, confidence bounds)",
        "coverage_metric": "6 signals (undersampled)",
        "signals": ["calibration_methods", "confidence_bounds", "truth_definition", "falsification"]
    }
}

def priority_gap():
    """Return domain with lowest coverage"""
    return sorted(DOMAINS.items(), key=lambda x: x[1]["coverage"])[0]

def next_to_explore():
    """Return next highest-value domain to deepen"""
    # Priority: D3 (zero) > D6 (12%) > D5 (cleanup) > D1 (expand) > D2/D4 (sufficient)
    return priority_gap()
