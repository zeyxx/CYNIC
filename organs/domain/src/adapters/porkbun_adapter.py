from __future__ import annotations

import json
import os
from typing import List, Optional

import urllib.request
import urllib.error

from organs.domain.src.core.entities import (
    DnsRecord,
    DomainAvailability,
    ProxyMode,
    RecordType,
    ZoneRecord,
    ZoneSnapshot,
)
from organs.domain.src.core.ports import RegistrarPort


_PORKBUN_BASE = "https://api.porkbun.com/api/json/v3"


def _porkbun_credentials() -> dict:
    """
    Read Porkbun credentials from environment only — never from files.
    Raises EnvironmentError when variables are absent.
    """
    api_key = os.environ.get("PORKBUN_API_KEY")
    secret = os.environ.get("PORKBUN_SECRET_API_KEY")
    if not api_key or not secret:
        raise EnvironmentError(
            "PORKBUN_API_KEY and PORKBUN_SECRET_API_KEY must be set in the environment. "
            "Source ~/.cynic-env before running organ-domain commands."
        )
    return {"apikey": api_key, "secretapikey": secret}


def _post(path: str, body: dict, timeout: int = 15) -> dict:
    url = f"{_PORKBUN_BASE}/{path}"
    data = json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"Porkbun API error {exc.code} on {path}: {exc.read().decode()}") from exc


class PorkbunAdapter(RegistrarPort):
    """
    Adapter implementing RegistrarPort against the Porkbun v3 API.
    Credentials are loaded from environment variables at call time —
    never stored on the instance.
    """

    # -----------------------------------------------------------------------
    # Non-mutating
    # -----------------------------------------------------------------------

    def check_availability(self, domain: str) -> DomainAvailability:
        creds = _porkbun_credentials()
        resp = _post("domain/checkAndReturnPricing", {**creds, "domain": domain})
        if resp.get("status") != "SUCCESS":
            raise RuntimeError(f"Porkbun availability check failed: {resp}")
        pricing = resp.get("pricing", {})
        # pricing key is the TLD, e.g. {"build": {"registration": "12.00", ...}}
        tld = domain.rsplit(".", 1)[-1]
        tld_pricing = pricing.get(tld, {})
        available = bool(tld_pricing)
        price = float(tld_pricing.get("registration", 0)) if available else None
        return DomainAvailability(domain=domain, available=available, price_usd=price)

    def list_portfolio(self) -> List[str]:
        creds = _porkbun_credentials()
        resp = _post("domain/listAll", {**creds, "includeLabels": "yes"})
        if resp.get("status") != "SUCCESS":
            raise RuntimeError(f"Porkbun portfolio list failed: {resp}")
        return [d["domain"] for d in resp.get("domains", [])]

    def list_expiry(self) -> List[dict]:
        creds = _porkbun_credentials()
        resp = _post("domain/listAll", {**creds, "includeLabels": "yes"})
        if resp.get("status") != "SUCCESS":
            raise RuntimeError(f"Porkbun expiry audit failed: {resp}")
        return [
            {
                "domain": d["domain"],
                "expires": d.get("expireDate"),
                "auto_renew": d.get("autorenew") == "1",
            }
            for d in resp.get("domains", [])
        ]

    def get_nameservers(self, domain: str) -> List[str]:
        creds = _porkbun_credentials()
        resp = _post(f"domain/getNs/{domain}", creds)
        if resp.get("status") != "SUCCESS":
            raise RuntimeError(f"Porkbun get nameservers failed: {resp}")
        return resp.get("ns", [])

    def list_dns_records(self, domain: str) -> List[DnsRecord]:
        creds = _porkbun_credentials()
        resp = _post(f"dns/retrieve/{domain}", creds)
        if resp.get("status") != "SUCCESS":
            raise RuntimeError(f"Porkbun DNS retrieve failed: {resp}")
        records = []
        for r in resp.get("records", []):
            try:
                rtype = RecordType(r["type"].upper())
            except ValueError:
                continue  # skip unsupported types
            records.append(
                DnsRecord(
                    name=r["name"],
                    type=rtype,
                    content=r["content"],
                    ttl=int(r.get("ttl", 300)),
                    priority=int(r["prio"]) if r.get("prio") else None,
                    record_id=str(r.get("id", "")),
                )
            )
        return records

    # -----------------------------------------------------------------------
    # Mutating — callers must pass through approval gate before invoking
    # -----------------------------------------------------------------------

    def set_nameservers(self, domain: str, nameservers: List[str]) -> bool:
        creds = _porkbun_credentials()
        resp = _post(f"domain/updateNs/{domain}", {**creds, "ns": nameservers})
        return resp.get("status") == "SUCCESS"

    def register_domain(self, domain: str, years: int = 1) -> bool:
        creds = _porkbun_credentials()
        resp = _post("domain/create", {**creds, "domain": domain, "years": years})
        return resp.get("status") == "SUCCESS"
