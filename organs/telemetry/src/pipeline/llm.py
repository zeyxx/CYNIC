from typing import List
from ..core.entities import Anomaly

class LLMAnalyzer:
    """LLM level: High-level reasoning on filtered, aggregated anomalies."""
    
    def generate_rca(self, anomalies: List[Anomaly]) -> str:
        # Here we would call CYNIC MCP / Gemini with the aggregated data
        # to ask: "Why are these errors happening across 50 sessions?"
        
        report = "Rapport LLM (Simulation):\n"
        for a in anomalies:
            report += f"- L'erreur '{a.description}' (vue {a.occurrences} fois) indique un probleme probable avec les permissions MCP.\n"
        return report
