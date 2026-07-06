import logging
import urllib.request
import urllib.error
from base64 import b64encode
from datetime import datetime
from typing import List

from organs.contracts.src.surface_contracts import json_object_or_default
from ..core.entities import Anomaly

logger = logging.getLogger(__name__)

class SurrealTelemetryAdapter:
    def __init__(self, url: str, ns: str, db: str, user: str, credential: str | None = None, **options):
        self.url = url.rstrip('/')
        self.ns = ns
        self.db = db
        credential_value = str(options.get("password") or credential or "")
        self.auth = b64encode(f"{user}:{credential_value}".encode()).decode()

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
                results = json_object_or_default(body, [])
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

    def store_anomalies(self, anomalies: List[Anomaly]) -> bool:
        if not anomalies:
            return True
            
        queries = []
        for a in anomalies:
            # Escape strings for SurrealQL
            desc_escaped = a.description.replace("'", "\\'")
            level = a.level
            occurrences = a.occurrences
            conf = a.confidence_score
            queries.append(
                f"CREATE telemetry_anomalies SET level = '{level}', description = '{desc_escaped}', "
                f"occurrences = {occurrences}, confidence_score = {conf}, timestamp = time::now();"
            )
            
        full_query = "\n".join(queries)
        res = self._run_sql(full_query)
        return len(res) > 0
