from __future__ import annotations
from dataclasses import dataclass, field
from typing import List, Optional
from enum import Enum


class DnsAuthority(str, Enum):
    PORKBUN = "porkbun"
    CLOUDFLARE = "cloudflare"


class MigrationStatus(str, Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETE = "complete"


class AppStatus(str, Enum):
    PENDING = "pending"
    LIVE = "live"
    PAUSED = "paused"


class RecordType(str, Enum):
    A = "A"
    AAAA = "AAAA"
    CNAME = "CNAME"
    MX = "MX"
    TXT = "TXT"
    NS = "NS"


class ProxyMode(str, Enum):
    PROXIED = "proxied"
    DNS_ONLY = "dns_only"


@dataclass
class DnsRecord:
    name: str
    type: RecordType
    content: str
    ttl: int = 300
    priority: Optional[int] = None  # MX only
    proxy_mode: ProxyMode = ProxyMode.DNS_ONLY
    record_id: Optional[str] = None  # Cloudflare record id when known


@dataclass
class ProxyPolicy:
    web_records: ProxyMode = ProxyMode.PROXIED
    mail_records: ProxyMode = ProxyMode.DNS_ONLY
    verification_txt: ProxyMode = ProxyMode.DNS_ONLY


@dataclass
class DnsAuthorityConfig:
    current: DnsAuthority
    target: DnsAuthority
    migration_status: MigrationStatus = MigrationStatus.PENDING


@dataclass
class PagesApp:
    subdomain: str
    fqdn: str
    pages_project: str
    source_dir: str
    analytics: bool = False
    status: AppStatus = AppStatus.PENDING
    pages_hostname: Optional[str] = None  # e.g. odysseum.pages.dev


@dataclass
class DomainDesiredState:
    """
    Root entity parsed from desired/domains.json.
    Encapsulates the full desired state for the apex domain and all apps.
    """
    schema: str
    apex: str
    registrar: DnsAuthority
    dns_authority: DnsAuthorityConfig
    apps: List[PagesApp] = field(default_factory=list)
    proxy_policy: ProxyPolicy = field(default_factory=ProxyPolicy)


@dataclass
class ZoneRecord:
    """A DNS record as read from a live zone (Porkbun or Cloudflare)."""
    name: str
    type: RecordType
    content: str
    ttl: int = 300
    priority: Optional[int] = None
    record_id: Optional[str] = None
    proxied: bool = False


@dataclass
class ZoneSnapshot:
    """Full zone read from provider — used for backup and migration."""
    apex: str
    provider: DnsAuthority
    records: List[ZoneRecord] = field(default_factory=list)
    nameservers: List[str] = field(default_factory=list)
    captured_at: Optional[str] = None


@dataclass
class DomainAvailability:
    domain: str
    available: bool
    price_usd: Optional[float] = None
    premium: bool = False


@dataclass
class DelegationStatus:
    """Result of an authoritative NS delegation check."""
    apex: str
    expected_nameservers: List[str] = field(default_factory=list)
    observed_nameservers: List[str] = field(default_factory=list)
    delegated: bool = False
    checked_via: Optional[str] = None  # which resolver was used


@dataclass
class PagesDeployResult:
    project: str
    fqdn: str
    pages_hostname: Optional[str] = None
    custom_domain_active: bool = False
    http_status: Optional[int] = None
    success: bool = False
    error: Optional[str] = None


@dataclass
class HttpProbeResult:
    url: str
    status_code: Optional[int] = None
    location: Optional[str] = None
    resolved_via: Optional[str] = None
    success: bool = False
    error: Optional[str] = None


@dataclass
class Redirect:
    source: str
    target: str
    include_subdomains: bool = False
    preserve_path: bool = False
    preserve_query: bool = False
