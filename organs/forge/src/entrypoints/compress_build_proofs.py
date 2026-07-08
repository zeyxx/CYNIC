#!/usr/bin/env python3
import json
import os
from pathlib import Path
from urllib.parse import urlparse

from organs.contracts.src.adapters.kernel_http import kernel_json_request
from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.forge.src.adapters.kernel_adapter import normalize_kernel_base_url


def _allow_local_kernel_if_needed(base_url: str) -> None:
    host = urlparse(base_url).hostname or ""
    if host == "localhost" or host.startswith("127."):
        os.environ.setdefault("CYNIC_ALLOW_LOCAL_KERNEL", "1")

def main():
    kernel_url = normalize_kernel_base_url(os.environ.get("CYNIC_REST_ADDR", "http://localhost:5002"))
    _allow_local_kernel_if_needed(kernel_url)
    endpoint = "/proof-of-history?limit=100"

    try:
        status, text, blocks = kernel_json_request("GET", kernel_url, endpoint, "", timeout=5)
        if status != 200:
            raise RuntimeError(f"HTTP {status}: {text[:200]}")
        blocks = blocks if isinstance(blocks, list) else json_loads_or_default(text, [])
    except Exception as e:
        print(f"Failed to fetch PoH from {endpoint}: {e}")
        return

    # Sort blocks by timestamp ascending if possible
    # Assuming the API returns them ordered by some criteria, we just dump them
    
    proofs = []
    for b in blocks:
        proofs.append({
            "id": b.get("id"),
            "hash": b.get("cryptographic_hash"),
            "prev_hash": b.get("prev_hash"),
            "timestamp": b.get("timestamp"),
            "event_type": b.get("event_type"),
            "source_organ": b.get("source_organ"),
            "signature": b.get("cryptographic_signature")
        })

    # Write to packages/public-ui/src/assets/build-proofs.json
    project_root = Path(__file__).parent.parent.parent.parent.parent
    assets_dir = project_root / "packages" / "public-ui" / "src" / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)
    
    out_file = assets_dir / "build-proofs.json"
    with open(out_file, "w") as f:
        json.dump({"proofs": proofs}, f, indent=2)
    
    print(f"Compressed {len(proofs)} proofs into {out_file}")

if __name__ == "__main__":
    main()
