from __future__ import annotations

import json
import os
from typing import List, Optional

import urllib.request
import urllib.error

from organs.domain.src.core.entities import (
    DelegationStatus,
    DnsRecord,
    HttpProbeResult,
    PagesDeployResult,
    ProxyMode,
    RecordType,
    Redirect,
    ZoneSnapshot,
)
from organs.domain.src.core.ports import ZonePort, PagesPort, RedirectPort, ProbePort
import subprocess
import socket


_CF_BASE = "https://api.cloudflare.com/client/v4"


def _cf_credentials() -> tuple[str, str]:
    """Read Cloudflare credentials from environment only."""
    token = os.environ.get("CF_API_TOKEN")
    account_id = os.environ.get("CF_ACCOUNT_ID")
    if not token or not account_id:
        raise EnvironmentError(
            "CF_API_TOKEN and CF_ACCOUNT_ID must be set in the environment. "
            "Source ~/.cynic-env before running organ-domain commands."
        )
    return token, account_id


def _cf_request(method: str, path: str, body: Optional[dict] = None, timeout: int = 15) -> dict:
    token, _ = _cf_credentials()
    url = f"{_CF_BASE}/{path}"
    data = json.dumps(body).encode() if body else None
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"Cloudflare API error {exc.code} on {method} {path}: {exc.read().decode()}") from exc


# ---------------------------------------------------------------------------
# ZonePort implementation
# ---------------------------------------------------------------------------

class CloudflareZoneAdapter(ZonePort):
    """Cloudflare DNS zone adapter — implements ZonePort."""

    def get_zone_id(self, apex: str) -> Optional[str]:
        resp = _cf_request("GET", f"zones?name={apex}&status=active")
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare zone lookup failed for {apex}: {resp}")
        results = resp.get("result", [])
        return results[0]["id"] if results else None

    def create_zone(self, apex: str) -> str:
        _, account_id = _cf_credentials()
        resp = _cf_request("POST", "zones", {"name": apex, "account": {"id": account_id}, "jump_start": False})
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare zone creation failed for {apex}: {resp}")
        return resp["result"]["id"]

    def list_records(self, zone_id: str) -> List[DnsRecord]:
        resp = _cf_request("GET", f"zones/{zone_id}/dns_records?per_page=500")
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare list records failed: {resp}")
        records = []
        for r in resp.get("result", []):
            try:
                rtype = RecordType(r["type"].upper())
            except ValueError:
                continue
            records.append(
                DnsRecord(
                    name=r["name"],
                    type=rtype,
                    content=r["content"],
                    ttl=r.get("ttl", 300),
                    priority=r.get("priority"),
                    proxy_mode=ProxyMode.PROXIED if r.get("proxied") else ProxyMode.DNS_ONLY,
                    record_id=r["id"],
                )
            )
        return records

    def get_nameservers(self, zone_id: str) -> List[str]:
        resp = _cf_request("GET", f"zones/{zone_id}")
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare get zone failed: {resp}")
        return resp["result"].get("name_servers", [])

    def create_record(self, zone_id: str, record: DnsRecord) -> str:
        body: dict = {
            "name": record.name,
            "type": record.type.value,
            "content": record.content,
            "ttl": record.ttl,
            "proxied": record.proxy_mode == ProxyMode.PROXIED,
        }
        if record.priority is not None:
            body["priority"] = record.priority
        resp = _cf_request("POST", f"zones/{zone_id}/dns_records", body)
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare create record failed: {resp}")
        return resp["result"]["id"]

    def update_record(self, zone_id: str, record: DnsRecord) -> bool:
        if not record.record_id:
            raise ValueError("record_id required to update a record")
        body = {
            "name": record.name,
            "type": record.type.value,
            "content": record.content,
            "ttl": record.ttl,
            "proxied": record.proxy_mode == ProxyMode.PROXIED,
        }
        resp = _cf_request("PUT", f"zones/{zone_id}/dns_records/{record.record_id}", body)
        return bool(resp.get("success"))

    def delete_record(self, zone_id: str, record_id: str) -> bool:
        resp = _cf_request("DELETE", f"zones/{zone_id}/dns_records/{record_id}")
        return bool(resp.get("success"))

    def set_proxy_status(self, zone_id: str, record_id: str, proxied: bool) -> bool:
        resp = _cf_request("PATCH", f"zones/{zone_id}/dns_records/{record_id}", {"proxied": proxied})
        return bool(resp.get("success"))


# ---------------------------------------------------------------------------
# PagesPort implementation (delegates deploy to wrangler CLI)
# ---------------------------------------------------------------------------

class CloudflarePagesAdapter(PagesPort):
    """
    Cloudflare Pages adapter — implements PagesPort.
    Deploy uses wrangler CLI subprocess; project management uses CF API.
    """

    def __init__(self, account_id: str) -> None:
        self._account_id = account_id

    def get_project(self, account_id: str, project_name: str) -> Optional[dict]:
        try:
            resp = _cf_request("GET", f"accounts/{account_id}/pages/projects/{project_name}")
            if resp.get("success"):
                return resp["result"]
            return None
        except RuntimeError:
            return None

    def create_project(self, account_id: str, project_name: str) -> dict:
        resp = _cf_request(
            "POST",
            f"accounts/{account_id}/pages/projects",
            {"name": project_name, "production_branch": "main"},
        )
        if not resp.get("success"):
            raise RuntimeError(f"Cloudflare Pages create project failed: {resp}")
        return resp["result"]

    def deploy(self, account_id: str, project_name: str, source_dir: str) -> PagesDeployResult:
        # Ensure project exists
        project = self.get_project(account_id, project_name)
        if project is None:
            self.create_project(account_id, project_name)

        token, _ = _cf_credentials()
        cmd = [
            "wrangler",
            "pages",
            "deploy",
            source_dir,
            "--project-name",
            project_name,
        ]
        env = {**os.environ, "CLOUDFLARE_API_TOKEN": token, "CLOUDFLARE_ACCOUNT_ID": account_id}
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=120)
            success = result.returncode == 0
            pages_hostname = None
            if success:
                # wrangler outputs deployment URL — parse it
                for line in result.stdout.splitlines():
                    if ".pages.dev" in line:
                        pages_hostname = line.strip().split()[-1]
                        break
            return PagesDeployResult(
                project=project_name,
                fqdn="",
                pages_hostname=pages_hostname,
                success=success,
                error=result.stderr if not success else None,
            )
        except FileNotFoundError:
            return PagesDeployResult(
                project=project_name,
                fqdn="",
                success=False,
                error="wrangler not found in PATH — install with: npm install -g wrangler",
            )

    def list_custom_domains(self, account_id: str, project_name: str) -> List[str]:
        resp = _cf_request("GET", f"accounts/{account_id}/pages/projects/{project_name}/domains")
        if not resp.get("success"):
            return []
        return [d["name"] for d in resp.get("result", [])]

    def attach_custom_domain(self, account_id: str, project_name: str, hostname: str) -> bool:
        resp = _cf_request(
            "POST",
            f"accounts/{account_id}/pages/projects/{project_name}/domains",
            {"name": hostname},
        )
        return bool(resp.get("success"))

    def check_custom_domain_activation(self, account_id: str, project_name: str, hostname: str) -> bool:
        domains = self.list_custom_domains(account_id, project_name)
        return hostname in domains


# ---------------------------------------------------------------------------
# ProbePort implementation
# ---------------------------------------------------------------------------

class HttpProbeAdapter(ProbePort):
    """
    Non-mutating HTTP probe adapter.
    Supports --resolve style DNS pinning to test the authoritative edge
    without waiting for global propagation.
    """

    def probe_http(
        self,
        url: str,
        resolve_via: Optional[str] = None,
        expected_status: int = 200,
        follow_redirects: bool = True,
    ) -> HttpProbeResult:
        import urllib.parse

        parsed = urllib.parse.urlparse(url)
        hostname = parsed.hostname
        port = parsed.port or (443 if parsed.scheme == "https" else 80)

        # If resolve_via is set, pin DNS resolution to that nameserver
        # This mimics curl --resolve behaviour
        resolved_ip: Optional[str] = None
        if resolve_via:
            try:
                resolved_ip = socket.getaddrinfo(hostname, port, proto=socket.IPPROTO_TCP)[0][4][0]
            except Exception:
                pass

        # Use subprocess curl for reliable resolve/follow-redirect behaviour
        cmd = ["curl", "-sI", "--max-time", "10"]
        if follow_redirects:
            cmd.append("-L")
        if resolved_ip:
            cmd.extend(["--resolve", f"{hostname}:{port}:{resolved_ip}"])
        cmd.append(url)

        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
            headers = proc.stdout
            status_line = headers.splitlines()[0] if headers else ""
            status_code = None
            if "HTTP/" in status_line:
                try:
                    status_code = int(status_line.split()[1])
                except (IndexError, ValueError):
                    pass
            location = None
            for line in headers.splitlines():
                if line.lower().startswith("location:"):
                    location = line.split(":", 1)[1].strip()
            return HttpProbeResult(
                url=url,
                status_code=status_code,
                location=location,
                resolved_via=resolve_via,
                success=status_code == expected_status,
            )
        except Exception as exc:
            return HttpProbeResult(url=url, success=False, error=str(exc))

    def verify_delegation(self, apex: str, expected_ns: List[str]) -> DelegationStatus:
        """
        Uses `dig` to query the TLD authoritative nameservers for the apex NS records.
        Compares observed NS against expected_ns (case-insensitive).
        """
        cmd = ["dig", "+short", "NS", apex]
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
            observed = [ns.rstrip(".").lower() for ns in proc.stdout.strip().splitlines() if ns]
            expected_lower = [ns.lower() for ns in expected_ns]
            delegated = all(ns in observed for ns in expected_lower)
            return DelegationStatus(
                apex=apex,
                expected_nameservers=expected_ns,
                observed_nameservers=observed,
                delegated=delegated,
                checked_via="dig",
            )
        except Exception as exc:
            return DelegationStatus(
                apex=apex,
                expected_nameservers=expected_ns,
                observed_nameservers=[],
                delegated=False,
                checked_via=f"error: {exc}",
            )
