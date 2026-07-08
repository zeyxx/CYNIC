"""CLI command handlers for organ-domain provider operations."""

from __future__ import annotations

import json
import os

from organs.domain.src.adapters.cloudflare_adapter import (
    CloudflarePagesAdapter,
    CloudflareZoneAdapter,
    HttpProbeAdapter,
)
from organs.domain.src.adapters.porkbun_adapter import PorkbunAdapter
from organs.domain.src.adapters.state_adapter import JsonBackupAdapter, JsonDesiredStateAdapter
from organs.domain.src.core.use_cases import (
    CheckAvailabilityUseCase,
    DomainStatusUseCase,
    PagesDeployUseCase,
    PortfolioAuditUseCase,
    SetNameserversUseCase,
    VerifyDelegationUseCase,
    ZoneBackupUseCase,
    ZoneMigrateUseCase,
)


def emit(data: dict) -> None:
    print(json.dumps(data, indent=2))


def cmd_registrar_portfolio(_args) -> int:
    adapter = PorkbunAdapter()
    domains = adapter.list_portfolio()
    emit({"op": "portfolio", "domains": domains, "count": len(domains)})
    return 0


def cmd_registrar_expiry(_args) -> int:
    adapter = PorkbunAdapter()
    expiry = adapter.list_expiry()
    emit({"op": "expiry_audit", "domains": expiry})
    return 0


def cmd_registrar_check(args) -> int:
    uc = CheckAvailabilityUseCase(PorkbunAdapter())
    result = uc.execute(args.domain)
    emit(
        {
            "op": "check_availability",
            "domain": result.domain,
            "available": result.available,
            "price_usd": result.price_usd,
            "premium": result.premium,
        }
    )
    return 0


def cmd_status(args) -> int:
    uc = DomainStatusUseCase(JsonDesiredStateAdapter())
    result = uc.execute(args.desired)
    emit({"op": "status", "ok": result.ok, "message": result.message, "details": result.details})
    return 0


def cmd_zone_backup(args) -> int:
    uc = ZoneBackupUseCase(PorkbunAdapter(), JsonDesiredStateAdapter(), JsonBackupAdapter())
    result = uc.execute(args.desired, args.backup_dir)
    emit({"op": "zone_backup", "ok": result.ok, "message": result.message, "details": result.details})
    return 0 if result.ok else 1


def cmd_zone_migrate(args) -> int:
    uc = ZoneMigrateUseCase(
        PorkbunAdapter(),
        CloudflareZoneAdapter(),
        JsonDesiredStateAdapter(),
        JsonBackupAdapter(),
    )
    result = uc.execute(args.desired, args.backup_dir, dry_run=args.dry_run)
    emit(
        {
            "op": "zone_migrate",
            "dry_run": args.dry_run,
            "ok": result.ok,
            "message": result.message,
            "details": result.details,
        }
    )
    return 0 if result.ok else 1


def cmd_zone_set_ns(args) -> int:
    uc = SetNameserversUseCase(PorkbunAdapter(), CloudflareZoneAdapter())
    result = uc.execute(args.apex, args.zone_id, dry_run=args.dry_run)
    emit(
        {
            "op": "set_nameservers",
            "dry_run": args.dry_run,
            "ok": result.ok,
            "message": result.message,
            "details": result.details,
        }
    )
    return 0 if result.ok else 1


def cmd_zone_verify_delegation(args) -> int:
    uc = VerifyDelegationUseCase(CloudflareZoneAdapter(), HttpProbeAdapter())
    result = uc.execute(args.apex, args.zone_id)
    emit({"op": "verify_delegation", "ok": result.ok, "message": result.message, "details": result.details})
    return 0 if result.ok else 1


def cmd_pages_deploy(args) -> int:
    account_id = os.environ.get("CF_ACCOUNT_ID", "")
    uc = PagesDeployUseCase(
        CloudflarePagesAdapter(account_id),
        CloudflareZoneAdapter(),
        HttpProbeAdapter(),
        JsonDesiredStateAdapter(),
        account_id,
    )
    result = uc.execute(args.desired, args.app, dry_run=args.dry_run)
    emit(
        {
            "op": "pages_deploy",
            "app": args.app,
            "dry_run": args.dry_run,
            "ok": result.ok,
            "message": result.message,
            "details": result.details,
        }
    )
    return 0 if result.ok else 1


def cmd_pages_verify(args) -> int:
    probe = HttpProbeAdapter()
    desired = JsonDesiredStateAdapter().load_domains(args.desired)
    results = []
    for app in desired.apps:
        result = probe.probe_http(f"https://{app.fqdn}", expected_status=200)
        results.append(
            {
                "fqdn": app.fqdn,
                "status_code": result.status_code,
                "success": result.success,
                "error": result.error,
            }
        )
    all_ok = all(item["success"] for item in results)
    emit({"op": "pages_verify", "ok": all_ok, "probes": results})
    return 0 if all_ok else 1


def cmd_redirects_verify(args) -> int:
    desired = JsonDesiredStateAdapter().load_redirects(args.desired)
    redirects = [
        {
            "source": item.source,
            "target": item.target,
            "include_subdomains": item.include_subdomains,
            "preserve_path": item.preserve_path,
            "preserve_query": item.preserve_query,
        }
        for item in desired
    ]
    emit({
        "op": "redirects_verify",
        "ok": True,
        "count": len(redirects),
        "redirects": redirects,
    })
    return 0


def cmd_proxy_audit(args) -> int:
    desired = JsonDesiredStateAdapter().load_domains(args.desired)
    emit(
        {
            "op": "proxy_audit",
            "ok": True,
            "apex": desired.apex,
            "proxy_policy": {
                "web_records": desired.proxy_policy.web_records.value,
                "mail_records": desired.proxy_policy.mail_records.value,
                "verification_txt": desired.proxy_policy.verification_txt.value,
            },
            "apps": [
                {
                    "subdomain": app.subdomain,
                    "fqdn": app.fqdn,
                    "status": app.status.value,
                }
                for app in desired.apps
            ],
        }
    )
    return 0
