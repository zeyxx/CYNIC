import dataclasses
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from ..core.entities import TwitterSurfaceState
from ..core.ports import IStateStorage, ITwitterPerceiver
from .context_repository import JsonlContextRepository, count_jsonl


class SystemTwitterPerceiver(ITwitterPerceiver):
    def __init__(self, state_dir: Path, hermes_dataset: Path | None = None):
        self.state_dir = state_dir
        self.hermes_dataset = hermes_dataset or Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"

    def perceive(self) -> TwitterSurfaceState:
        proxy_status = "offline"
        try:
            res = subprocess.run(["ss", "-tuln"], capture_output=True, text=True, check=False)
            if ":8888" in res.stdout:
                proxy_status = "online"
        except OSError:
            pass

        context_repo = JsonlContextRepository(self.state_dir / "context_items.jsonl")
        context_bundle = self.state_dir / "context" / "latest.md"
        return TwitterSurfaceState(
            total_tweets_captured=count_jsonl(self.hermes_dataset),
            total_verdicts_judged=0,
            total_context_items=context_repo.count_items(),
            latest_context_bundle=str(context_bundle) if context_bundle.exists() else "",
            latest_trends=[],
            proxy_status=proxy_status,
            timestamp=datetime.now(timezone.utc).isoformat(),
        )


class JsonStateStorage(IStateStorage):
    def __init__(self, state_file: Path):
        self.state_file = state_file

    def save_state(self, state: TwitterSurfaceState) -> None:
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        data = dataclasses.asdict(state)
        with open(self.state_file, "w", encoding="utf-8") as handle:
            json.dump(data, handle, indent=2)

        dataset_file = self.state_file.parent / "dataset.jsonl"
        with open(dataset_file, "a", encoding="utf-8") as handle:
            handle.write(json.dumps(data) + "\n")
