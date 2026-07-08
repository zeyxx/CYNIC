from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional

from organs.domain.src.core.entities import (
    DomainDesiredState,
    DnsAuthorityConfig,
    DnsAuthority,
    MigrationStatus,
    PagesApp,
    AppStatus,
    ProxyPolicy,
    ProxyMode,
    Redirect,
    ZoneRecord,
    ZoneSnapshot,
    RecordType,
)
from organs.domain.src.core.ports import DesiredStatePort, BackupPort


class JsonDesiredStateAdapter(DesiredStatePort):
    """
    Reads desired state from JSON files.
    Non-mutating — only loads and parses, never writes.
    """

    def load_domains(self, path: str) -> DomainDesiredState:
        with open(path, encoding="utf-8") as f:
            raw = json.load(f)

        da_raw = raw["dns_authority"]
        dns_authority = DnsAuthorityConfig(
            current=DnsAuthority(da_raw["current"]),
            target=DnsAuthority(da_raw["target"]),
            migration_status=MigrationStatus(da_raw.get("migration_status", "pending")),
        )

        pp_raw = raw.get("proxy_policy", {})
        proxy_policy = ProxyPolicy(
            web_records=ProxyMode(pp_raw.get("web_records", "proxied")),
            mail_records=ProxyMode(pp_raw.get("mail_records", "dns_only")),
            verification_txt=ProxyMode(pp_raw.get("verification_txt", "dns_only")),
        )

        apps = []
        for a in raw.get("apps", []):
            apps.append(
                PagesApp(
                    subdomain=a["subdomain"],
                    fqdn=a["fqdn"],
                    pages_project=a["pages_project"],
                    source_dir=a["source_dir"],
                    analytics=a.get("analytics", False),
                    status=AppStatus(a.get("status", "pending")),
                )
            )

        return DomainDesiredState(
            schema=raw.get("schema", ""),
            apex=raw["apex"],
            registrar=DnsAuthority(raw["registrar"]),
            dns_authority=dns_authority,
            apps=apps,
            proxy_policy=proxy_policy,
        )

    def load_redirects(self, path: str) -> List[Redirect]:
        with open(path, encoding="utf-8") as f:
            raw = json.load(f)
        return [
            Redirect(
                source=r["source"],
                target=r["target"],
                include_subdomains=r.get("include_subdomains", False),
                preserve_path=r.get("preserve_path", False),
                preserve_query=r.get("preserve_query", False),
            )
            for r in raw.get("redirects", [])
        ]


class JsonBackupAdapter(BackupPort):
    """
    Persists ZoneSnapshot to state/backups/ as timestamped JSON files.
    Non-mutating reads; writes only to the state/ directory.
    """

    def save_snapshot(self, snapshot: ZoneSnapshot, output_dir: str) -> str:
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)

        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        filename = f"{snapshot.apex}-{snapshot.provider.value}-{ts}.json"
        dest = output_path / filename

        payload = {
            "schema": "cynic.domain-ops.zone-snapshot.v1",
            "apex": snapshot.apex,
            "provider": snapshot.provider.value,
            "captured_at": ts,
            "nameservers": snapshot.nameservers,
            "records": [
                {
                    "name": r.name,
                    "type": r.type.value,
                    "content": r.content,
                    "ttl": r.ttl,
                    "priority": r.priority,
                    "record_id": r.record_id,
                }
                for r in snapshot.records
            ],
        }
        dest.write_text(json.dumps(payload, indent=2), encoding="utf-8")
        return str(dest)

    def load_latest_snapshot(self, apex: str, output_dir: str) -> Optional[ZoneSnapshot]:
        output_path = Path(output_dir)
        if not output_path.exists():
            return None
        candidates = sorted(output_path.glob(f"{apex}-*.json"), reverse=True)
        if not candidates:
            return None

        with open(candidates[0], encoding="utf-8") as f:
            raw = json.load(f)

        records = []
        for r in raw.get("records", []):
            try:
                rtype = RecordType(r["type"].upper())
            except ValueError:
                continue
            records.append(
                ZoneRecord(
                    name=r["name"],
                    type=rtype,
                    content=r["content"],
                    ttl=r.get("ttl", 300),
                    priority=r.get("priority"),
                    record_id=r.get("record_id"),
                )
            )

        return ZoneSnapshot(
            apex=raw["apex"],
            provider=DnsAuthority(raw["provider"]),
            records=records,
            nameservers=raw.get("nameservers", []),
            captured_at=raw.get("captured_at"),
        )
