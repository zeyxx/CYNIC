import requests
from typing import Tuple
from ..core.ports import JupyterPort

class JupyterApiAdapter(JupyterPort):
    def __init__(self, api_url: str, token: str):
        self.api_url = api_url
        self.token = token
        self.headers = {"Authorization": f"token {self.token}"}

    def fetch_status(self) -> Tuple[str, int]:
        try:
            resp = requests.get(self.api_url, headers=self.headers, timeout=5)
            if resp.status_code == 200:
                kernels = resp.json()
                return "online", len(kernels)
            else:
                return "degraded", 0
        except Exception:
            return "offline", 0
