"""
MasterDog Expertise Plugins " Specialized ML and heuristic logic.

Unifies heuristic expertise (AST, ML, Network, Anomaly) into
reusable functions called by the MasterDog engine.
"""

from __future__ import annotations

import ast
import logging
import re
from typing import Any

from cynic.kernel.core.judgment import Cell

logger = logging.getLogger("cynic.neurons.expertise")


# --- UTILS ---
def _extract_code(cell: Cell) -> str | None:
    """Extract Python code string from cell content."""
    content = cell.content
    if isinstance(content, str) and content.strip():
        return content
    if isinstance(content, dict):
        for key in ("code", "source", "content", "diff", "text"):
            val = content.get(key)
            if isinstance(val, str) and val.strip():
                return val
    return None


# --- SCOUT EXPERTISE ---
_URL_PATTERN = re.compile(r'https?://[^\s<>"\'{}|\\^`\[\]]{4,}', re.IGNORECASE)


async def scout_expertise(cell: Cell) -> dict[str, Any]:
    """Expertise for URL reachability and discovery."""
    content = str(cell.content)
    urls = _URL_PATTERN.findall(content)
    if not urls:
        return {"q_score": 50.0, "confidence": 0.2, "reasoning": "No URLs found to scout."}

    found = len(urls)
    return {
        "q_score": 75.0 if found > 0 else 50.0,
        "confidence": 0.4,
        "reasoning": f"Found {found} URLs. Ground truth discovery active.",
        "evidence": {"urls_found": found},
    }


# --- ARCHITECT EXPERTISE (AST) ---
MAX_IMPORTS = 13
MAX_NESTING = 7


async def ast_analysis_expertise(cell: Cell) -> dict[str, Any]:
    """AST-based structural analysis."""
    code = _extract_code(cell)
    if not code:
        return {
            "q_score": 50.0,
            "confidence": 0.1,
            "reasoning": "No code found for structural analysis.",
        }

    try:
        tree = ast.parse(code)
        import_count = sum(
            1 for node in ast.walk(tree) if isinstance(node, ast.Import | ast.ImportFrom)
        )

        def get_depth(node, depth=0):
            children = list(ast.iter_child_nodes(node))
            if not children:
                return depth
            return max(get_depth(c, depth + 1) for c in children)

        max_depth = get_depth(tree)
        penalty = max(0, (import_count - MAX_IMPORTS) * 3) + max(0, (max_depth - MAX_NESTING) * 5)
        q_score = max(0.0, 100.0 - penalty)

        return {
            "q_score": q_score,
            "confidence": 0.5,
            "reasoning": f"AST Analysis: imports={import_count}, depth={max_depth}.",
            "evidence": {"imports": import_count, "depth": max_depth},
        }
    except Exception as e:
        return {"q_score": 20.0, "confidence": 0.8, "reasoning": f"Syntax Error: {e}"}


# --- JANITOR EXPERTISE (Smells) ---
async def static_analysis_expertise(cell: Cell) -> dict[str, Any]:
    """Heuristic code smell detection."""
    code = _extract_code(cell)
    if not code:
        return {
            "q_score": 50.0,
            "confidence": 0.1,
            "reasoning": "No code found for smell detection.",
        }

    markers = ("TODO", "FIXME", "HACK")
    smells = [m for m in markers if m in code.upper()]
    q_score = max(0.0, 100.0 - (len(smells) * 10.0))

    return {
        "q_score": q_score,
        "confidence": 0.6,
        "reasoning": f"Janitor: {len(smells)} smells found.",
        "evidence": {"smells": smells},
    }


# --- GUARDIAN EXPERTISE (Anomaly) ---
async def anomaly_detection_expertise(cell: Cell) -> dict[str, Any]:
    """IsolationForest-style anomaly detection (Heuristic version)."""
    # Simplified: Higher risk/novelty = Higher anomaly
    score = (cell.risk + cell.novelty) / 2.0
    q_score = max(0.0, 100.0 * (1.0 - score))

    threats = {"rm -rf": 1.0, "eval(": 0.8, "exec(": 0.8}
    content = str(cell.content).lower()
    threat_found = any(t in content for t in threats)

    if threat_found:
        q_score = min(q_score, 10.0)

    return {
        "q_score": q_score,
        "confidence": 0.382,
        "reasoning": f"Guardian: Anomaly risk {score:.2f}. {'CRITICAL THREAT' if threat_found else 'Safe'}.",
        "evidence": {"risk": cell.risk, "novelty": cell.novelty, "threat_detected": threat_found},
    }


# --- ORACLE EXPERTISE (Q-Table) ---
async def qtable_prediction_expertise(cell: Cell) -> dict[str, Any]:
    """Thompson Sampling / Q-Table prediction lookup."""
    # In a real system, we'd inject the qtable instance.
    # For now, return a placeholder that looks for the state_key.
    return {
        "q_score": 50.0,
        "confidence": 0.2,
        "reasoning": "Oracle: Q-Table consult pending (placeholder).",
        "evidence": {"state_key": cell.state_key()},
    }


# --- WORLD-MAKER ---
async def dream_facets_expertise(
    adapter: Any, axiom: str, reality: str, registry: Any
) -> dict[str, str]:
    """World-Maker: Generate 7 dynamic facets via LLM."""
    prompt = (
        f"Generate exactly 7 distinct dimensions (facets) to evaluate {axiom} in {reality} context."
    )
    try:
        response = await adapter.generate(prompt)
        facets = {}
        for line in response.splitlines():
            if ":" in line:
                parts = line.split(":", 1)
                name = re.sub(r"^\d+[\.\-\s]+", "", parts[0].strip().upper())
                desc = parts[1].strip()
                if name and desc:
                    facets[name] = desc
                    await registry.register_facet(axiom, reality, name, desc)
        return facets
    except Exception as e:
        logger.error(f"Expertise: SAGE dream failed: {e}")
        return {}


# --- MARKET EXPERTISE ---
async def market_health_expertise(cell: Cell) -> dict[str, Any]:
    """Expertise for price volatility and market sentiment."""
    if cell.reality != "MARKET":
        return {"q_score": 50.0, "confidence": 0.0, "reasoning": "Incompatible reality."}
    
    # We assume content is MarketPayload (validated by realities.py)
    content = cell.content
    if not isinstance(content, dict):
        return {"q_score": 50.0, "confidence": 0.1, "reasoning": "Raw content, no structured market data."}
    
    volatility = content.get("volatility", 0.0)
    change = content.get("change_24h", 0.0)
    
    # Logic: High volatility or sharp drops reduce Q-Score
    penalty = (volatility * 50.0) + (abs(change) if change < 0 else 0.0)
    q_score = max(0.0, 100.0 - penalty)
    
    return {
        "q_score": q_score,
        "confidence": 0.618,
        "reasoning": f"Market analysis for {content.get('symbol')}: vol={volatility:.2f}, change={change:.2f}%",
        "evidence": {"volatility": volatility, "change_24h": change}
    }

# --- SOLANA EXPERTISE ---
async def solana_integrity_expertise(cell: Cell) -> dict[str, Any]:
    """Expertise for Solana on-chain health."""
    if cell.reality != "SOLANA":
        return {"q_score": 50.0, "confidence": 0.0, "reasoning": "Incompatible reality."}
    
    content = cell.content
    if not isinstance(content, dict):
        return {"q_score": 50.0, "confidence": 0.1, "reasoning": "No on-chain data."}
    
    health = content.get("health", "ok")
    tps = content.get("tps", 0.0)
    
    q_score = 90.0 if health == "ok" else 20.0
    if tps < 1000: # Arbitrary threshold for congestion
        q_score -= 20.0

    return {
        "q_score": max(0.0, q_score),
        "confidence": 0.75,
        "reasoning": f"Solana Health: {health}, TPS: {tps:.0f}",
        "evidence": {"health": health, "tps": tps}
    }

# --- REGISTRY OF EXPERTISE ---
EXPERTISE_MAP = {
    "web_discovery": scout_expertise,
    "ast_analysis": ast_analysis_expertise,
    "static_analysis": static_analysis_expertise,
    "anomaly_detection": anomaly_detection_expertise,
    "qtable_lookup": qtable_prediction_expertise,
    "market_health": market_health_expertise,
    "solana_integrity": solana_integrity_expertise,
}


async def call_expertise(expertise_id: str, cell: Cell) -> dict[str, Any] | None:
    """Safe dispatcher for expertise plugins."""
    if expertise_id in EXPERTISE_MAP:
        try:
            return await EXPERTISE_MAP[expertise_id](cell)
        except Exception as e:
            logger.error(f"Expertise {expertise_id} failed: {e}")
    return None
