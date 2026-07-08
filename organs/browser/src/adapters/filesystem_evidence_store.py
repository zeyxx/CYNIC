import json
import datetime
from pathlib import Path
from ..core.ports import EvidenceStorePort
from ..core.entities import PageEvidence


def _resolve_state_root() -> Path:
    script = Path(__file__).resolve()
    for candidate in (script, *script.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate / "organs" / "browser" / "state"
    return script.parent.parent.parent / "state"


class FilesystemEvidenceStore(EvidenceStorePort):
    def __init__(self, base_dir: str = None):
        if base_dir:
            self.base_dir = Path(base_dir)
        else:
            self.base_dir = _resolve_state_root()
        
        self.evidence_dir = self.base_dir / "evidence"
        self.ledger_path = self.base_dir / "action_ledger.jsonl"
        self.evidence_dir.mkdir(parents=True, exist_ok=True)

    def save_evidence(self, task_id: str, evidence: PageEvidence) -> None:
        evidence_file = self.evidence_dir / f"{task_id}.json"
        
        evidence_dict = {
            "task_id": evidence.task_id,
            "status": evidence.status,
            "observed_at": evidence.observed_at.isoformat(),
            "url": evidence.url,
            "summary": evidence.summary,
            "mutations_performed": evidence.mutations_performed,
            "blockers": evidence.blockers,
            "evidence": []
        }
        
        for item in evidence.evidence:
            if item.kind == "screenshot_hex" and item.value:
                try:
                    screenshot_bytes = bytes.fromhex(item.value)
                    screenshot_path = self.evidence_dir / f"{task_id}_screenshot.png"
                    with open(screenshot_path, "wb") as sf:
                        sf.write(screenshot_bytes)
                    evidence_dict["evidence"].append({
                        "kind": "screenshot_path",
                        "value": str(screenshot_path),
                        "redaction": item.redaction
                    })
                except Exception:
                    evidence_dict["evidence"].append({
                        "kind": item.kind,
                        "value": "[failed_to_decode_screenshot]",
                        "redaction": item.redaction
                    })
            else:
                evidence_dict["evidence"].append({
                    "kind": item.kind,
                    "value": item.value,
                    "redaction": item.redaction
                })
        
        with open(evidence_file, "w") as f:
            json.dump(evidence_dict, f, indent=2)

    def log_action(self, action: dict) -> None:
        if "timestamp" not in action:
            action["timestamp"] = datetime.datetime.now().isoformat()
        with open(self.ledger_path, "a") as f:
            f.write(json.dumps(action) + "\n")
