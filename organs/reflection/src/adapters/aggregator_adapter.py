import json
import dataclasses
from pathlib import Path
from ..core.entities import ProofOfHistory, OrganEvent
from ..core.ports import IPoHAggregator, IStateStorage

class FileSystemPoHAggregator(IPoHAggregator):
    def __init__(self, organs_dir: Path, human_kernel_log: Path):
        self.organs_dir = organs_dir
        self.human_kernel_log = human_kernel_log

    def aggregate_organism(self) -> ProofOfHistory:
        events = []
        for dataset_file in self.organs_dir.glob("*/state/dataset.jsonl"):
            organ_name = dataset_file.parent.parent.name
            try:
                lines = dataset_file.read_text().splitlines()[-20:]
                for line in lines:
                    if not line.strip(): continue
                    data = json.loads(line)
                    ts = str(data.get("timestamp", ""))
                    events.append(OrganEvent(organ_name=organ_name, timestamp=ts, data=data))
            except Exception:
                pass
                
        events.sort(key=lambda x: str(x.timestamp), reverse=True)
        recent_events = events[:50]
        summary = f"Aggregated {len(recent_events)} recent events across organism."
        return ProofOfHistory(domain="organism", total_events=len(events), recent_events=recent_events, summary=summary)

    def aggregate_human(self) -> ProofOfHistory:
        events = []
        try:
            if self.human_kernel_log.exists():
                lines = self.human_kernel_log.read_text().splitlines()[-50:]
                for line in lines:
                    if not line.strip(): continue
                    data = json.loads(line)
                    ts = str(data.get("timestamp", ""))
                    events.append(OrganEvent(organ_name="human", timestamp=ts, data=data))
        except Exception:
            pass
            
        events.sort(key=lambda x: str(x.timestamp), reverse=True)
        recent_events = events[:50]
        summary = f"Aggregated {len(recent_events)} recent events from human interactions."
        return ProofOfHistory(domain="human", total_events=len(events), recent_events=recent_events, summary=summary)

class JsonStateStorage(IStateStorage):
    def __init__(self, state_file: Path):
        self.state_file = state_file

    def save_poh_snapshot(self, poh: ProofOfHistory) -> None:
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        data = dataclasses.asdict(poh)
        with open(self.state_file, 'w') as f:
            json.dump(data, f, indent=2)
