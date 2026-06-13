import os
import json
import urllib.request
import urllib.error
from pathlib import Path
from typing import List
from src.core.ports import VercelPort
from src.core.entities import Deployment

ENV_FILE = Path("/home/user/.cynic-env")

class RestVercelAdapter(VercelPort):
    def _get_vercel_token(self):
        if not ENV_FILE.exists():
            return os.environ.get("VERCEL_TOKEN")
        
        for line in ENV_FILE.read_text().splitlines():
            if line.startswith("VERCEL_TOKEN="):
                return line.split("=", 1)[1].strip().strip('"').strip("'")
        return os.environ.get("VERCEL_TOKEN")

    def fetch_deployments(self) -> List[Deployment]:
        auth_tok = self._get_vercel_token()
        if not auth_tok:
            print("VERCEL_TOKEN not found. Skipping Vercel monitor.")
            return []

        url = "https://api.vercel.com/v6/deployments?limit=10"
        req = urllib.request.Request(url, headers={
            "Authorization": f"Bearer {auth_tok}",
            "Content-Type": "application/json"
        })
        
        try:
            with urllib.request.urlopen(req) as response:
                data = json.loads(response.read().decode())
                if not data or "deployments" not in data:
                    return []
                    
                deployments = []
                for d in data["deployments"]:
                    deployments.append(Deployment(
                        id=d.get("uid", ""),
                        name=d.get("name", ""),
                        url=d.get("url", ""),
                        state=d.get("state", ""),
                        created=d.get("created", 0)
                    ))
                return deployments
        except urllib.error.URLError as e:
            print(f"Error fetching Vercel deployments: {e}")
            return []
