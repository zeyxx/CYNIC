import json
import logging
import urllib.request
import urllib.error
from base64 import b64encode
from typing import List

from ..core.ports import MeshStatePort
from ..core.entities import MeshNode, MeshRoute

logger = logging.getLogger(__name__)

class SurrealMeshAdapter(MeshStatePort):
    def __init__(self, url: str, ns: str, db: str, user: str, password: str):
        self.url = url.rstrip('/')
        self.ns = ns
        self.db = db
        self.auth = b64encode(f"{user}:{password}".encode()).decode()

    def _run_sql(self, query: str) -> list:
        headers = {
            "Accept": "application/json",
            "surreal-ns": self.ns,
            "surreal-db": self.db,
            "Authorization": f"Basic {self.auth}",
            "Content-Type": "text/plain"
        }
        
        req = urllib.request.Request(f"{self.url}/sql", data=query.encode('utf-8'), headers=headers, method='POST')
        try:
            with urllib.request.urlopen(req, timeout=10) as response:
                body = response.read().decode('utf-8')
                results = json.loads(body)
                if results and isinstance(results, list):
                    last_result = results[-1]
                    if last_result.get("status") == "ERR":
                        logger.error(f"SurrealDB error: {last_result.get('result')}")
                        return []
                    return last_result.get("result", [])
        except urllib.error.HTTPError as e:
            body = e.read().decode('utf-8') if e.fp else ""
            logger.error(f"SurrealDB HTTP Error {e.code}: {body}")
        except Exception as e:
            logger.error(f"SurrealDB request failed: {e}")
        return []

    async def fetch_mesh_nodes(self) -> List[MeshNode]:
        raw_nodes = self._run_sql("SELECT * FROM mesh_nodes;")
        nodes = []
        for n in raw_nodes:
            nodes.append(MeshNode(
                id=n.get("id"),
                name=n.get("name"),
                public_key=n.get("public_key"),
                mesh_ip=n.get("mesh_ip"),
                external_endpoint=n.get("external_endpoint"),
                last_seen=n.get("last_seen")
            ))
        return nodes

    async def fetch_mesh_routes(self) -> List[MeshRoute]:
        raw_routes = self._run_sql("SELECT * FROM mesh_routes;")
        routes = []
        for r in raw_routes:
            routes.append(MeshRoute(
                id=r.get("id"),
                source_node=r.get("source_node"),
                target_node=r.get("target_node"),
                allowed_ips=r.get("allowed_ips", [])
            ))
        return routes

    async def publish_node_endpoint(self, node_id: str, endpoint: str) -> bool:
        query = f"UPDATE {node_id} SET external_endpoint = '{endpoint}', last_seen = time::now();"
        res = self._run_sql(query)
        return len(res) > 0
