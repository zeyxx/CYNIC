import os
import httpx
import datetime
from pathlib import Path
from ..core.ports import KernelObservationPort
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, kernel_url
from organs.contracts.src.surface_contracts import iter_json_objects, write_jsonl_file_atomic

class CynicKernelObservationAdapter(KernelObservationPort):
    def __init__(self, fallback_dir: str = None):
        self.rest_addr = os.environ.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
        self.api_key = os.environ.get("CYNIC_API_KEY", "")
        
        if fallback_dir:
            self.fallback_dir = Path(fallback_dir)
        else:
            self.fallback_dir = Path(__file__).resolve().parent.parent.parent / "state"
        
        self.fallback_file = self.fallback_dir / "observations_fallback.jsonl"
        self.fallback_dir.mkdir(parents=True, exist_ok=True)

    def observe(self, agent_id: str, title: str, content: dict, domain: str = "browser") -> None:
        payload = {
            "agent_id": agent_id,
            "title": title,
            "domain": domain,
            "content": content,
            "timestamp": datetime.datetime.now().isoformat()
        }
        
        url = kernel_url(self.rest_addr, "/observe", allow_empty=True)
        headers = kernel_auth_headers(self.api_key, content_type=False)
            
        try:
            if url:
                response = httpx.post(url, json=payload, headers=headers, timeout=3.0)
                if response.status_code in [200, 201]:
                    return
        except Exception:
            pass
            
        existing = []
        if self.fallback_file.exists():
            existing = list(iter_json_objects(self.fallback_file.read_text(encoding="utf-8").splitlines()))
        rows = list(existing)
        rows.append(payload)
        write_jsonl_file_atomic(self.fallback_file, rows)
