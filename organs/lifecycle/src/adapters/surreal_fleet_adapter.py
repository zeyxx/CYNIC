from __future__ import annotations

import time
from typing import Any, Dict

import httpx

from ..core.ports import FleetStatusPort


class SurrealFleetAdapter(FleetStatusPort):
    def __init__(self, surreal_url: str, auth_token: str):
        self.surreal_url = surreal_url.rstrip("/")
        self.headers = {
            "Accept": "application/json",
            "Authorization": f"Bearer {auth_token}",
        }

    async def is_node_online(self, node_name: str, threshold_seconds: int = 30) -> bool:
        last_seen = await self.get_node_last_seen(node_name)
        if last_seen == 0.0:
            return False
        return (time.time() - last_seen) <= threshold_seconds

    async def get_node_last_seen(self, node_name: str) -> float:
        query = "SELECT last_seen FROM mesh_nodes WHERE name = $name"
        async with httpx.AsyncClient() as client:
            try:
                response = await client.post(
                    f"{self.surreal_url}/sql",
                    headers=self.headers,
                    data=query,
                    params={"name": f'"{node_name}"'},
                )
                if response.status_code != 200:
                    return 0.0
                data = response.json()
                if isinstance(data, list) and data:
                    result = data[0].get("result", [])
                    if result and isinstance(result, list):
                        return float(result[0].get("last_seen", 0.0))
            except Exception:
                return 0.0
        return 0.0

    async def get_node_status(self, node_name: str) -> Dict[str, Any]:
        last_seen = await self.get_node_last_seen(node_name)
        return {
            "node_name": node_name,
            "online": await self.is_node_online(node_name),
            "last_seen": last_seen,
            "freshness_seconds": None if last_seen == 0.0 else max(0.0, time.time() - last_seen),
        }
