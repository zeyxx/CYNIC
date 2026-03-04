"""
CYNIC Heresy Aggregator - Groups anomalies by file for batch healing.
Respects Data Engineer & Solutions Architect Lenses.

Groups 1000+ individual syntax errors into file-level 'Tickets' for the 
Campaign Manager to process in a single Sandbox session.
"""
import json
from pathlib import Path
from collections import defaultdict

def aggregate_heresies(mine_path: str = "audit/heresy_mine.jsonl"):
    path = Path(mine_path)
    if not path.exists():
        return {}

    # Group by filename
    file_tickets = defaultdict(list)
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            record = json.loads(line)
            file_tickets[record["file"]].append(record)
            
    # Sort files by number of errors (start with easy ones)
    sorted_tickets = sorted(
        file_tickets.items(), 
        key=lambda x: len(x[1])
    )
    
    return sorted_tickets

if __name__ == "__main__":
    tickets = aggregate_heresies()
    print(f"Aggregated {sum(len(t[1]) for t in tickets)} errors into {len(tickets)} file-level tickets.")
    for filename, errors in tickets[:5]:
        print(f"  - {filename}: {len(errors)} errors")
