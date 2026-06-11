#!/usr/bin/env python3
"""
Entrypoint for organ-twitter.
Perceives the Twitter proxy surface and persists the state.
"""
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from organs.twitter.src.adapters.twitter_adapter import SystemTwitterPerceiver, JsonStateStorage

def main():
    state_file = PROJECT_ROOT / "organs" / "twitter" / "state" / "state.json"
    
    perceiver = SystemTwitterPerceiver()
    storage = JsonStateStorage(state_file)
    
    print("Perceiving Twitter surface state...")
    state = perceiver.perceive()
    
    print(f"Proxy status: {state.proxy_status}")
    
    print(f"Saving audited state to {state_file}...")
    storage.save_state(state)
    
    print("Done.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
