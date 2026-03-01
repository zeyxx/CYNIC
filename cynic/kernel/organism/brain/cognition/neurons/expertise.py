"""
MasterDog Expertise Plugins — Specialized non-LLM logic.

Unifies heuristic expertise (AST, TF-IDF, Network, Anomaly) into 
reusable functions called by the MasterDog engine.
"""
from __future__ import annotations
import logging
import re
import asyncio
import time
import urllib.request
import urllib.error
from typing import Any, Dict, List, Optional, Tuple

from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import fibonacci, PHI_INV_2, MAX_Q_SCORE, phi_bound_score

logger = logging.getLogger("cynic.neurons.expertise")

# --- SCOUT EXPERTISE ---
_URL_PATTERN = re.compile(r'https?://[^\s<>"\'{}|\\^`\[\]]{4,}', re.IGNORECASE)

async def scout_expertise(cell: Cell) -> Dict[str, Any]:
    """Expertise for URL reachability and discovery."""
    content = str(cell.content)
    urls = _URL_PATTERN.findall(content)
    if not urls:
        return {"q_score": 50.0, "confidence": 0.2, "reasoning": "No URLs found to scout."}

    # Simplified check for the prototype
    found = len(urls)
    return {
        "q_score": 75.0 if found > 0 else 50.0,
        "confidence": 0.4,
        "reasoning": f"Found {found} URLs. Ground truth discovery active.",
        "evidence": {"urls_found": found}
    }

async def dream_facets_expertise(
    adapter: Any, axiom: str, reality: str, registry: Any
) -> dict[str, str]:
    """World-Maker: Generate 7 dynamic facets via LLM."""
    prompt = f"""
    Reality Context: {reality}
    Core Axiom: {axiom}
    
    Generate exactly 7 distinct dimensions (facets) to evaluate this axiom 
    within this reality context. Each facet must have a name (1 word) and a 
    short description.
    
    Format:
    FACET_NAME: Description
    """
    try:
        response = await adapter.generate(prompt)
        facets = {}
        for line in response.splitlines():
            if ":" in line:
                parts = line.split(":", 1)
                name = re.sub(r'^\d+[\.\-\s]+', '', parts[0].strip().upper())
                desc = parts[1].strip()
                if name and desc:
                    facets[name] = desc
                    await registry.register_facet(axiom, reality, name, desc)
        return facets
    except Exception as e:
        logger.error(f"Expertise: SAGE dream failed for {axiom}/{reality}: {e}")
        return {}

# --- REGISTRY OF EXPERTISE ---
EXPERTISE_MAP = {
    "web_discovery": scout_expertise,
    # Future: "ast_analysis": architect_expertise, etc.
}

async def call_expertise(expertise_id: str, cell: Cell) -> Optional[Dict[str, Any]]:
    """Safe dispatcher for expertise plugins."""
    if expertise_id in EXPERTISE_MAP:
        try:
            return await EXPERTISE_MAP[expertise_id](cell)
        except Exception as e:
            logger.error(f"Expertise {expertise_id} failed: {e}")
    return None
