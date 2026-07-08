from __future__ import annotations
from abc import ABC, abstractmethod
from typing import List, Optional

from .entities import (
    DomainAvailability,
    DomainDesiredState,
    DnsRecord,
    DelegationStatus,
    HttpProbeResult,
    PagesDeployResult,
    Redirect,
    ZoneSnapshot,
)


# ---------------------------------------------------------------------------
# Registrar port — Porkbun or any future registrar
# ---------------------------------------------------------------------------

class RegistrarPort(ABC):
    """Read and write operations against the domain registrar."""

    @abstractmethod
    def check_availability(self, domain: str) -> DomainAvailability:
        """Non-mutating: check whether a domain is available and its price."""

    @abstractmethod
    def list_portfolio(self) -> List[str]:
        """Non-mutating: list all domains in the account portfolio."""

    @abstractmethod
    def list_expiry(self) -> List[dict]:
        """Non-mutating: list domains with expiry dates and auto-renew status."""

    @abstractmethod
    def get_nameservers(self, domain: str) -> List[str]:
        """Non-mutating: read the registrar-configured nameservers for a domain."""

    @abstractmethod
    def set_nameservers(self, domain: str, nameservers: List[str]) -> bool:
        """Mutating — gate: approval_required. Update registrar NS delegation."""

    @abstractmethod
    def register_domain(self, domain: str, years: int = 1) -> bool:
        """Mutating — gate: paid_approval_required. Register a new domain."""

    @abstractmethod
    def list_dns_records(self, domain: str) -> List[DnsRecord]:
        """Non-mutating: read all DNS records from registrar-hosted DNS."""


# ---------------------------------------------------------------------------
# Zone port — Cloudflare zone management
# ---------------------------------------------------------------------------

class ZonePort(ABC):
    """Read and write operations against the Cloudflare DNS zone."""

    @abstractmethod
    def get_zone_id(self, apex: str) -> Optional[str]:
        """Non-mutating: resolve the Cloudflare zone id for an apex domain."""

    @abstractmethod
    def create_zone(self, apex: str) -> str:
        """Mutating: create a new zone and return its id."""

    @abstractmethod
    def list_records(self, zone_id: str) -> List[DnsRecord]:
        """Non-mutating: read all records from a Cloudflare zone."""

    @abstractmethod
    def get_nameservers(self, zone_id: str) -> List[str]:
        """Non-mutating: get the assigned Cloudflare nameservers for a zone."""

    @abstractmethod
    def create_record(self, zone_id: str, record: DnsRecord) -> str:
        """Mutating — gate: approval_required. Create a DNS record. Returns record id."""

    @abstractmethod
    def update_record(self, zone_id: str, record: DnsRecord) -> bool:
        """Mutating — gate: approval_required. Update an existing DNS record by id."""

    @abstractmethod
    def delete_record(self, zone_id: str, record_id: str) -> bool:
        """Mutating — gate: approval_required. Delete a DNS record."""

    @abstractmethod
    def set_proxy_status(self, zone_id: str, record_id: str, proxied: bool) -> bool:
        """Mutating — gate: approval_required. Toggle Cloudflare proxy on a record."""


# ---------------------------------------------------------------------------
# Pages port — Cloudflare Pages deployment and custom domains
# ---------------------------------------------------------------------------

class PagesPort(ABC):
    """Deploy and manage Cloudflare Pages projects."""

    @abstractmethod
    def get_project(self, account_id: str, project_name: str) -> Optional[dict]:
        """Non-mutating: fetch Pages project metadata or None if not found."""

    @abstractmethod
    def create_project(self, account_id: str, project_name: str) -> dict:
        """Mutating: create a new Pages project."""

    @abstractmethod
    def deploy(self, account_id: str, project_name: str, source_dir: str) -> PagesDeployResult:
        """Mutating — gate: approval_required. Run wrangler pages deploy."""

    @abstractmethod
    def list_custom_domains(self, account_id: str, project_name: str) -> List[str]:
        """Non-mutating: list custom domains attached to a Pages project."""

    @abstractmethod
    def attach_custom_domain(self, account_id: str, project_name: str, hostname: str) -> bool:
        """Mutating — gate: approval_required. Attach a custom domain to Pages."""

    @abstractmethod
    def check_custom_domain_activation(self, account_id: str, project_name: str, hostname: str) -> bool:
        """Non-mutating: poll whether a custom domain has been activated."""


# ---------------------------------------------------------------------------
# Redirect port — Cloudflare bulk redirects
# ---------------------------------------------------------------------------

class RedirectPort(ABC):
    """Manage Cloudflare account-level bulk redirect list and ruleset."""

    @abstractmethod
    def list_redirects(self, account_id: str) -> List[Redirect]:
        """Non-mutating: read current redirect entries."""

    @abstractmethod
    def reconcile(self, account_id: str, desired: List[Redirect]) -> bool:
        """Mutating — gate: approval_required. Reconcile desired vs live redirects."""


# ---------------------------------------------------------------------------
# Probe port — HTTP verification (authoritative, edge-direct)
# ---------------------------------------------------------------------------

class ProbePort(ABC):
    """Non-mutating HTTP probes that bypass recursive DNS propagation."""

    @abstractmethod
    def probe_http(
        self,
        url: str,
        resolve_via: Optional[str] = None,
        expected_status: int = 200,
        follow_redirects: bool = True,
    ) -> HttpProbeResult:
        """
        Probe a URL. resolve_via pins the DNS resolution to a specific nameserver
        (e.g. Cloudflare authoritative NS) to test the intended edge path
        without waiting for global propagation.
        """

    @abstractmethod
    def verify_delegation(self, apex: str, expected_ns: List[str]) -> DelegationStatus:
        """
        Resolve the apex via TLD authoritative nameservers and compare against
        expected_ns. Returns a DelegationStatus with delegated=True only when
        all expected NS are observed.
        """


# ---------------------------------------------------------------------------
# Desired state port — read desired/domains.json
# ---------------------------------------------------------------------------

class DesiredStatePort(ABC):
    """Read the desired state files — no writes, no mutations."""

    @abstractmethod
    def load_domains(self, path: str) -> DomainDesiredState:
        """Parse desired/domains.json into a DomainDesiredState entity."""

    @abstractmethod
    def load_redirects(self, path: str) -> List[Redirect]:
        """Parse desired/redirects.json into a list of Redirect entities."""


# ---------------------------------------------------------------------------
# Backup port — zone snapshots before any write
# ---------------------------------------------------------------------------

class BackupPort(ABC):
    """Persist zone snapshots to state/backups/ before mutating writes."""

    @abstractmethod
    def save_snapshot(self, snapshot: ZoneSnapshot, output_dir: str) -> str:
        """Save a zone snapshot and return the path written."""

    @abstractmethod
    def load_latest_snapshot(self, apex: str, output_dir: str) -> Optional[ZoneSnapshot]:
        """Load the most recent snapshot for an apex domain, or None."""
