from __future__ import annotations

import os
from dataclasses import dataclass


@dataclass(frozen=True)
class HeadscaleRuntimeView:
    endpoint: str
    reachable: bool


class HeadscaleRuntimeAdapter:
    def runtime_view(self) -> HeadscaleRuntimeView:
        endpoint = os.environ.get("HEADSCALE_URL", "http://127.0.0.1:8080")
        reachable = bool(os.environ.get("HEADSCALE_URL"))
        return HeadscaleRuntimeView(endpoint=endpoint, reachable=reachable)

