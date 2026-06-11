#!/usr/bin/env python3
"""
Entrypoint for organ-reflection.
Aggregates Proof-of-History datasets from all organs into a single unified state.
"""
import sys
from pathlib import Path

# Add CYNIC root to path for local imports if needed
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from organs.reflection.src.adapters.aggregator_adapter import FileSystemPoHAggregator, JsonStateStorage

def main():
    organs_dir = PROJECT_ROOT / "organs"
    human_kernel_log = Path.home() / ".cynic" / "memory" / "logs" / "human-kernel.jsonl"
    
    organism_state_file = PROJECT_ROOT / "organs" / "reflection" / "state" / "organism_state.json"
    human_state_file = PROJECT_ROOT / "organs" / "reflection" / "state" / "human_state.json"
    
    # Initialize Adapters
    aggregator = FileSystemPoHAggregator(organs_dir, human_kernel_log)
    organism_storage = JsonStateStorage(organism_state_file)
    human_storage = JsonStateStorage(human_state_file)
    
    print("Aggregating Proof-of-History...")
    organism_poh = aggregator.aggregate_organism()
    human_poh = aggregator.aggregate_human()
    
    print(f"Saving aggregated Organism PoH to {organism_state_file}...")
    organism_storage.save_poh_snapshot(organism_poh)
    print(organism_poh.summary)
    
    print(f"Saving aggregated Human PoH to {human_state_file}...")
    human_storage.save_poh_snapshot(human_poh)
    print(human_poh.summary)
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
