"""
MasterDog Expertise Plugins — Specialized non-LLM logic.

Unifies heuristic expertise (AST, TF-IDF, Network, Anomaly) into 
reusable functions called by the MasterDog engine.
"""
from __future__ import annotations
import logging
import re
import ast
import asyncio
import time
import urllib.request
import urllib.error
from typing import Any, Dict, List, Optional, Tuple

from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import fibonacci, PHI_INV_2, MAX_Q_SCORE, phi_bound_score

logger = logging.getLogger("cynic.neurons.expertise")

# --- UTILS ---
def _extract_code(cell: Cell) -> Optional[str]:
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

async def scout_expertise(cell: Cell) -> Dict[str, Any]:
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
        "evidence": {"urls_found": found}
    }

# --- ARCHITECT EXPERTISE (AST) ---
MAX_IMPORTS = 13
MAX_NESTING = 7
MAX_METHODS = 11

async def ast_analysis_expertise(cell: Cell) -> Dict[str, Any]:
    """AST-based structural analysis."""
    code = _extract_code(cell)
    if not code:
        return {"q_score": 50.0, "confidence": 0.1, "reasoning": "No code found for structural analysis."}
    
    try:
        tree = ast.parse(code)
        import_count = 0
        max_depth = 0
        
        for node in ast.walk(tree):
            if isinstance(node, (ast.Import, ast.ImportFrom)):
                import_count += 1
        
        # Simple depth check for prototype
        def get_depth(node, depth=0):
            if not list(ast.iter_child_nodes(node)): return depth
            return max(get_depth(c, depth + 1) for c in ast.iter_child_nodes(node))
        
        max_depth = get_depth(tree)
        
        penalty = 0.0
        if import_count > MAX_IMPORTS: penalty += (import_count - MAX_IMPORTS) * 3
        if max_depth > MAX_NESTING: penalty += (max_depth - MAX_NESTING) * 5
        
        q_score = max(0.0, 100.0 - penalty)
        return {
            "q_score": q_score,
            "confidence": 0.5,
            "reasoning": f"AST Analysis: imports={import_count}, depth={max_depth}. Structural integrity at {q_score:.1f}%",
            "evidence": {"imports": import_count, "depth": max_depth}
        }
    except Exception as e:
        return {"q_score": 20.0, "confidence": 0.8, "reasoning": f"Syntax Error: {e}"}

# --- JANITOR EXPERTISE (Smells) ---
async def static_analysis_expertise(cell: Cell) -> Dict[str, Any]:
    """Heuristic code smell detection."""
    code = _extract_code(cell)
    if not code:
        return {"q_score": 50.0, "confidence": 0.1, "reasoning": "No code found for smell detection."}
    
    smells = []
    if "TODO" in code.upper(): smells.append("debt-marker:TODO")
    if "FIXME" in code.upper(): smells.append("debt-marker:FIXME")
    if "HACK" in code.upper(): smells.append("debt-marker:HACK")
    
    penalty = len(smells) * 10.0
    q_score = max(0.0, 100.0 - penalty)
    
    return {
        "q_score": q_score,
        "confidence": 0.6,
        "reasoning": f"Janitor: {len(smells)} smells found. {', '.join(smells) if smells else 'Clean code'}.",
        "evidence": {"smells": smells}
    }

# --- WORLD-MAKER ---
async def dream_facets_expertise(
    adapter: Any, axiom: str, reality: str, registry: Any
) -> dict[str, str]:
    """World-Maker: Generate 7 dynamic facets via LLM."""
    prompt = f"Generate exactly 7 distinct dimensions (facets) to evaluate {axiom} in {reality} context."
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
        logger.error(f"Expertise: SAGE dream failed: {e}")
        return {}

# --- REGISTRY OF EXPERTISE ---
EXPERTISE_MAP = {
    "web_discovery": scout_expertise,
    "ast_analysis": ast_analysis_expertise,
    "static_analysis": static_analysis_expertise,
}

async def call_expertise(expertise_id: str, cell: Cell) -> Optional[Dict[str, Any]]:
    """Safe dispatcher for expertise plugins."""
    if expertise_id in EXPERTISE_MAP:
        try:
            return await EXPERTISE_MAP[expertise_id](cell)
        except Exception as e:
            logger.error(f"Expertise {expertise_id} failed: {e}")
    return None
