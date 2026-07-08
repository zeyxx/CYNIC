import requests
from typing import Dict, Any, Optional

class ProxmoxClient:
    """
    Handles exclusively the HTTP session, authentication (Tickets/CSRF),
    and raw REST requests to the Proxmox API.
    """
    def __init__(self, host: str, user: str, token_name: str, token_value: str, verify_ssl: bool = False):
        self.host = host
        self.user = user
        self.token_name = token_name
        self.token_value = token_value
        self.verify_ssl = verify_ssl
        self.base_url = f"https://{self.host}:8006/api2/json"
        
        self.session = requests.Session()
        self.session.verify = self.verify_ssl
        # Use API tokens (preferred over passwords for automation)
        self.session.headers.update({
            "Authorization": f"PVEAPIToken={self.user}!{self.token_name}={self.token_value}"
        })

    def get(self, endpoint: str) -> Dict[str, Any]:
        response = self.session.get(f"{self.base_url}{endpoint}")
        response.raise_for_status()
        return response.json().get("data", {})

    def post(self, endpoint: str, data: Dict[str, Any]) -> Dict[str, Any]:
        response = self.session.post(f"{self.base_url}{endpoint}", data=data)
        response.raise_for_status()
        return response.json().get("data", {})
    
    def put(self, endpoint: str, data: Dict[str, Any]) -> Dict[str, Any]:
        response = self.session.put(f"{self.base_url}{endpoint}", data=data)
        response.raise_for_status()
        return response.json().get("data", {})
