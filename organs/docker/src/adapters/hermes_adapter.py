import sys
from pathlib import Path
from typing import Dict, Any
from src.core.ports import DispatcherPort

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent.parent / "scripts"))
from hermes_dispatch import HermesDispatcher

class DefaultHermesAdapter(DispatcherPort):
    def __init__(self, domain: str):
        self.dispatcher = HermesDispatcher(f"organ-{domain}")

    def dispatch_task(self, payload: Dict[str, Any]) -> bool:
        return self.dispatcher.dispatch_task(payload)
