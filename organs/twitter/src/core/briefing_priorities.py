"""Domain priority analysis for Gemini briefing consumption."""

from __future__ import annotations


DOMAIN_TARGETS = {
    "D1": 500,
    "D2": 200,
    "D3": 100,
    "D4": 200,
    "D5": 100,
    "D6": 50,
}

DOMAIN_PRIORITIES = {
    "D1": "MONITOR",
    "D2": "MEDIUM",
    "D3": "MEDIUM",
    "D4": "HIGH",
    "D5": "HIGH",
    "D6": "HIGH",
}


def analyze_domain_priorities(briefing: dict) -> dict:
    """Analyze domain coverage against target signal counts."""
    distribution = briefing.get("analyses", {}).get("distribution", {})

    analysis = {}
    for domain_id, target in DOMAIN_TARGETS.items():
        current = distribution.get(domain_id, {}).get("count", 0)
        deficit = max(0, target - current)
        status = "aligned" if deficit < target * 0.2 else "under-assigned"
        analysis[domain_id] = {
            "target": target,
            "current": current,
            "deficit": deficit,
            "status": status,
            "priority": DOMAIN_PRIORITIES.get(domain_id, "MEDIUM"),
        }

    return analysis


def task_content(domain_id: str, briefing: dict) -> str:
    """Build a Hermes exploration task body from briefing analysis."""
    domain_info = briefing.get("recommendation", {})
    if domain_info.get("domain") == domain_id:
        context = f"Recommended by lab analysis: {domain_info.get('reason')}"
    else:
        context = "Under-assigned domain needing exploration"

    distribution = briefing.get("analyses", {}).get("distribution", {}).get(domain_id, {})
    avg_signal = distribution.get("mean", 0)

    return (
        f"Explore {domain_id} signals on X (current signal avg: {avg_signal:.2f})\n"
        f"Context: {context}\n"
        f"Actions:\n"
        f"1. Browse X for {domain_id} domain signals\n"
        f"2. Look for patterns in high-engagement tweets\n"
        f"3. Post 3+ observations to /observe\n"
        f"Success: 3+ observations with clear domain signals"
    )
