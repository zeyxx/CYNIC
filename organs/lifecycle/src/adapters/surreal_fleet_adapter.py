import time
import httpx
from ..core.ports import NetworkFleetPort

class SurrealFleetAdapter(NetworkFleetPort):
    def __init__(self, surreal_url: str, auth_token: str):
        self.surreal_url = surreal_url
        self.headers = {
            "Accept": "application/json",
            "Authorization": f"Bearer {auth_token}"
        }

    async def is_node_online(self, node_name: str, threshold_seconds: int = 30) -> bool:
        last_seen = await self.get_node_last_seen(node_name)
        if last_seen == 0.0:
            return False
            
        current_time = time.time()
        return (current_time - last_seen) <= threshold_seconds

    async def get_node_last_seen(self, node_name: str) -> float:
        # Expected to interact with the SurrealDB REST endpoint to query `mesh_nodes`
        # SELECT last_seen FROM mesh_nodes WHERE name = $node_name
        query = "SELECT last_seen FROM mesh_nodes WHERE name = $name"
        
        async with httpx.AsyncClient() as client:
            try:
                response = await client.post(
                    f"{self.surreal_url}/sql",
                    headers=self.headers,
                    data=query,
                    params={"name": f'"{node_name}"'} # Simplified parameter passing for now
                )
                
                if response.status_code == 200:
                    data = response.json()
                    if isinstance(data, list) and len(data) > 0:
                        result = data[0].get("result", [])
                        if result and isinstance(result, list) and len(result) > 0:
                            # Assuming last_seen is stored as a unix timestamp (float)
                            return float(result[0].get("last_seen", 0.0))
            except Exception as e:
                # Log error in production
                pass
                
        return 0.0
