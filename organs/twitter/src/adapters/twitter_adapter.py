import json
import dataclasses
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from ..core.entities import TwitterSurfaceState, Trend
from ..core.ports import ITwitterPerceiver, IStateStorage

class SystemTwitterPerceiver(ITwitterPerceiver):
    def perceive(self) -> TwitterSurfaceState:
        # Check if mitmproxy is running (simple heuristic)
        proxy_status = "offline"
        try:
            # Check if anything listens on 8888
            res = subprocess.run(["ss", "-tuln"], capture_output=True, text=True)
            if ":8888" in res.stdout:
                proxy_status = "online"
        except Exception:
            pass

        # In a real environment, we would parse ~/.cynic/memory/twitter/raw_tweets.jsonl
        # For now, we return a baseline state
        return TwitterSurfaceState(
            total_tweets_captured=0,
            total_verdicts_judged=0,
            latest_trends=[],
            proxy_status=proxy_status,
            timestamp=datetime.now(timezone.utc).isoformat()
        )

class JsonStateStorage(IStateStorage):
    def __init__(self, state_file: Path):
        self.state_file = state_file

    def save_state(self, state: TwitterSurfaceState) -> None:
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        data = dataclasses.asdict(state)
        
        # Save current state
        with open(self.state_file, 'w') as f:
            json.dump(data, f, indent=2)
            
        # Append to time-series dataset
        dataset_file = self.state_file.parent / "dataset.jsonl"
        with open(dataset_file, 'a') as f:
            f.write(json.dumps(data) + '\n')
