from typing import List, Dict
from collections import defaultdict
from ..core.entities import LogEntry, Anomaly

class HeuristicAnalyzer:
    """Heuristics level: Fast rule-based aggregation to filter noise."""
    
    def __init__(self):
        self.error_counts = defaultdict(list)
        self.tool_counts = defaultdict(int)

    def process_stream(self, entries: List[LogEntry]) -> List[Anomaly]:
        anomalies = []
        
        for entry in entries:
            # 1. Track tool frequencies
            for tc in entry.tool_calls:
                name = tc.get("name") or tc.get("function", {}).get("name")
                if name:
                    self.tool_counts[name] += 1
            
            # 2. Track errors
            if entry.is_error:
                # Naive clustering by taking first 50 chars of error
                err_sig = entry.content.split("Encountered error")[1][:50].strip() if "Encountered error" in entry.content else "UNKNOWN ERROR"
                self.error_counts[err_sig].append(entry)

        # Generate anomalies for frequent errors (e.g., threshold > 3)
        for sig, err_entries in self.error_counts.items():
            if len(err_entries) >= 3:
                anomalies.append(Anomaly(
                    level="heuristic",
                    description=f"Frequent Error Loop Detected: {sig}",
                    occurrences=len(err_entries),
                    raw_entries=err_entries,
                    confidence_score=0.8
                ))
                
        return anomalies
