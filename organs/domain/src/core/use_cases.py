from __future__ import annotations
from dataclasses import dataclass, field
from typing import List, Optional

from .entities import (
    DomainAvailability,
    DomainDesiredState,
    DelegationStatus,
    DnsRecord,
    HttpProbeResult,
    MigrationStatus,
    PagesDeployResult,
    ProxyMode,
    RecordType,
    Redirect,
    ZoneSnapshot,
)
from .ports import (
    BackupPort,
    DesiredStatePort,
    PagesPort,
    ProbePort,
    RegistrarPort,
    RedirectPort,
    ZonePort,
)


# ---------------------------------------------------------------------------
# Shared output shape
# ---------------------------------------------------------------------------

@dataclass
class OperationResult:
    ok: bool
    message: str
    details: dict = field(default_factory=dict)


class DomainStatusUseCase:
    """Non-mutating status report for domain organ activation readiness."""

    def __init__(self, desired_state: DesiredStatePort) -> None:
        self._desired = desired_state

    def execute(self, desired_path: str) -> OperationResult:
        state = self._desired.load_domains(desired_path)
        migration_status = state.dns_authority.migration_status.value
        app_statuses = [
            {
                "subdomain": app.subdomain,
                "fqdn": app.fqdn,
                "pages_project": app.pages_project,
                "status": app.status.value,
            }
            for app in state.apps
        ]
        ready = migration_status == MigrationStatus.COMPLETE.value and all(
            item["status"] == "live" for item in app_statuses
        )
        blockers = []
        if migration_status != MigrationStatus.COMPLETE.value:
            blockers.append(f"zone migration is {migration_status}")
        pending_apps = [item["fqdn"] for item in app_statuses if item["status"] != "live"]
        if pending_apps:
            blockers.append("pages apps not live: " + ", ".join(pending_apps))

        return OperationResult(
            ok=ready,
            message="Domain runtime activation verified" if ready else "Domain runtime activation pending",
            details={
                "schema": "cynic.domain.status.v1",
                "mutates": False,
                "apex": state.apex,
                "registrar": state.registrar.value,
                "dns_authority": {
                    "current": state.dns_authority.current.value,
                    "target": state.dns_authority.target.value,
                    "migration_status": migration_status,
                },
                "apps": app_statuses,
                "blockers": blockers,
            },
        )


# ---------------------------------------------------------------------------
# Use case: CheckAvailability
# ---------------------------------------------------------------------------

class CheckAvailabilityUseCase:
    """
    Non-mutating. Safe to run at any time.
    Reads registrar availability and price for a domain name.
    """

    def __init__(self, registrar: RegistrarPort) -> None:
        self._registrar = registrar

    def execute(self, domain: str) -> DomainAvailability:
        return self._registrar.check_availability(domain)


# ---------------------------------------------------------------------------
# Use case: PortfolioAudit
# ---------------------------------------------------------------------------

class PortfolioAuditUseCase:
    """
    Non-mutating. Lists portfolio and expiry dates.
    """

    def __init__(self, registrar: RegistrarPort) -> None:
        self._registrar = registrar

    def execute(self) -> dict:
        domains = self._registrar.list_portfolio()
        expiry = self._registrar.list_expiry()
        return {"portfolio": domains, "expiry": expiry}


# ---------------------------------------------------------------------------
# Use case: ZoneBackup
# ---------------------------------------------------------------------------

class ZoneBackupUseCase:
    """
    Non-mutating against the live zone. Reads records from the current
    DNS authority (registrar or Cloudflare) and persists a snapshot
    to state/backups/ before any migration write.

    Must be called before ZoneMigrateUseCase.
    """

    def __init__(
        self,
        registrar: RegistrarPort,
        desired_state: DesiredStatePort,
        backup: BackupPort,
    ) -> None:
        self._registrar = registrar
        self._desired = desired_state
        self._backup = backup

    def execute(self, desired_path: str, backup_dir: str) -> OperationResult:
        state = self._desired.load_domains(desired_path)
        records = self._registrar.list_dns_records(state.apex)
        ns = self._registrar.get_nameservers(state.apex)

        snapshot = ZoneSnapshot(
            apex=state.apex,
            provider=state.dns_authority.current,
            records=records,
            nameservers=ns,
        )
        path = self._backup.save_snapshot(snapshot, backup_dir)
        return OperationResult(
            ok=True,
            message=f"Zone snapshot saved: {path}",
            details={"record_count": len(records), "nameservers": ns, "path": path},
        )


# ---------------------------------------------------------------------------
# Use case: ZoneMigrate
# ---------------------------------------------------------------------------

class ZoneMigrateUseCase:
    """
    Mutating — requires approval gate before execution.

    Copies DNS records from Porkbun-hosted DNS into Cloudflare zone.
    Filters: imports A, AAAA, CNAME, MX, TXT — drops NS (registrar-specific)
    and other provider-alias records.

    Does NOT update nameservers at the registrar — that is SetNameserversUseCase.
    Separation is intentional: zone staging ≠ NS cutover.
    """

    # Record types that are safe to migrate across providers
    _MIGRATABLE_TYPES = {RecordType.A, RecordType.AAAA, RecordType.CNAME, RecordType.MX, RecordType.TXT}

    def __init__(
        self,
        registrar: RegistrarPort,
        zone: ZonePort,
        desired_state: DesiredStatePort,
        backup: BackupPort,
        proxy_policy_resolver: Optional[callable] = None,
    ) -> None:
        self._registrar = registrar
        self._zone = zone
        self._desired = desired_state
        self._backup = backup
        self._proxy_policy = proxy_policy_resolver or self._default_proxy_policy

    @staticmethod
    def _default_proxy_policy(record: DnsRecord, policy) -> bool:
        if record.type in (RecordType.MX, RecordType.TXT):
            return False
        if record.type in (RecordType.A, RecordType.AAAA, RecordType.CNAME):
            return policy.web_records == ProxyMode.PROXIED
        return False

    def execute(self, desired_path: str, backup_dir: str, dry_run: bool = False) -> OperationResult:
        state = self._desired.load_domains(desired_path)

        # Guard: backup must exist or we create one now
        snapshot = self._backup.load_latest_snapshot(state.apex, backup_dir)
        if snapshot is None:
            backup_uc = ZoneBackupUseCase(self._registrar, self._desired, self._backup)
            backup_uc.execute(desired_path, backup_dir)

        # Read source records
        source_records = self._registrar.list_dns_records(state.apex)
        migratable = [r for r in source_records if r.type in self._MIGRATABLE_TYPES]

        if dry_run:
            return OperationResult(
                ok=True,
                message=f"[DRY RUN] Would migrate {len(migratable)} records to Cloudflare zone for {state.apex}",
                details={"records": [{"name": r.name, "type": r.type, "content": r.content} for r in migratable]},
            )

        # Locate or create Cloudflare zone
        zone_id = self._zone.get_zone_id(state.apex)
        if zone_id is None:
            zone_id = self._zone.create_zone(state.apex)

        written = []
        skipped = []
        for record in migratable:
            proxied = self._proxy_policy(record, state.proxy_policy)
            record.proxy_mode = ProxyMode.PROXIED if proxied else ProxyMode.DNS_ONLY
            try:
                record_id = self._zone.create_record(zone_id, record)
                written.append({"name": record.name, "type": record.type, "id": record_id})
            except Exception as exc:
                skipped.append({"name": record.name, "type": record.type, "reason": str(exc)})

        return OperationResult(
            ok=len(skipped) == 0,
            message=f"Migrated {len(written)} records; {len(skipped)} skipped",
            details={"zone_id": zone_id, "written": written, "skipped": skipped},
        )


# ---------------------------------------------------------------------------
# Use case: SetNameservers
# ---------------------------------------------------------------------------

class SetNameserversUseCase:
    """
    Mutating — requires explicit approval gate.
    Changes NS records at the registrar to delegate DNS authority.
    This is the NS cutover. Separate from zone migration by design.
    """

    def __init__(self, registrar: RegistrarPort, zone: ZonePort) -> None:
        self._registrar = registrar
        self._zone = zone

    def execute(self, apex: str, zone_id: str, dry_run: bool = False) -> OperationResult:
        cf_ns = self._zone.get_nameservers(zone_id)
        if not cf_ns:
            return OperationResult(ok=False, message="No Cloudflare nameservers found for zone", details={"zone_id": zone_id})

        if dry_run:
            return OperationResult(
                ok=True,
                message=f"[DRY RUN] Would set nameservers at registrar: {cf_ns}",
                details={"apex": apex, "nameservers": cf_ns},
            )

        ok = self._registrar.set_nameservers(apex, cf_ns)
        return OperationResult(
            ok=ok,
            message="Nameservers updated at registrar" if ok else "Failed to update nameservers",
            details={"apex": apex, "nameservers": cf_ns},
        )


# ---------------------------------------------------------------------------
# Use case: VerifyDelegation
# ---------------------------------------------------------------------------

class VerifyDelegationUseCase:
    """
    Non-mutating. Verifies that TLD delegation points to the expected NS.
    Uses authoritative nameserver resolution to avoid false negatives
    during propagation.
    """

    def __init__(self, zone: ZonePort, probe: ProbePort) -> None:
        self._zone = zone
        self._probe = probe

    def execute(self, apex: str, zone_id: str) -> OperationResult:
        expected_ns = self._zone.get_nameservers(zone_id)
        status = self._probe.verify_delegation(apex, expected_ns)
        return OperationResult(
            ok=status.delegated,
            message="Delegation verified" if status.delegated else "Delegation not yet propagated",
            details={
                "apex": apex,
                "expected": status.expected_nameservers,
                "observed": status.observed_nameservers,
                "checked_via": status.checked_via,
            },
        )


# ---------------------------------------------------------------------------
# Use case: PagesDeployUseCase
# ---------------------------------------------------------------------------

class PagesDeployUseCase:
    """
    Mutating — requires approval gate.
    Deploys a Pages site, attaches custom domains, verifies HTTP probes.
    """

    def __init__(
        self,
        pages: PagesPort,
        zone: ZonePort,
        probe: ProbePort,
        desired_state: DesiredStatePort,
        account_id: str,
    ) -> None:
        self._pages = pages
        self._zone = zone
        self._probe = probe
        self._desired = desired_state
        self._account_id = account_id

    def execute(self, desired_path: str, app_subdomain: str, dry_run: bool = False) -> OperationResult:
        state = self._desired.load_domains(desired_path)
        app = next((a for a in state.apps if a.subdomain == app_subdomain), None)
        if app is None:
            return OperationResult(ok=False, message=f"No app '{app_subdomain}' in desired state")

        if dry_run:
            return OperationResult(
                ok=True,
                message=f"[DRY RUN] Would deploy {app.pages_project} → {app.fqdn}",
                details={"project": app.pages_project, "fqdn": app.fqdn, "source_dir": app.source_dir},
            )

        # Deploy
        result = self._pages.deploy(self._account_id, app.pages_project, app.source_dir)
        if not result.success:
            return OperationResult(ok=False, message=f"Deploy failed: {result.error}", details={"project": app.pages_project})

        # Attach custom domain (fqdn)
        attached = self._pages.attach_custom_domain(self._account_id, app.pages_project, app.fqdn)

        # Probe
        probe = self._probe.probe_http(f"https://{app.fqdn}", expected_status=200)

        return OperationResult(
            ok=attached and probe.success,
            message="Deploy complete" if (attached and probe.success) else "Deploy done but verification pending",
            details={
                "fqdn": app.fqdn,
                "pages_hostname": result.pages_hostname,
                "custom_domain_attached": attached,
                "http_status": probe.status_code,
                "http_success": probe.success,
            },
        )
