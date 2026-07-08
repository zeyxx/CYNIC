from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

from organs.domain.src.adapters.cli_commands import (
    cmd_pages_deploy,
    cmd_pages_verify,
    cmd_proxy_audit,
    cmd_registrar_check,
    cmd_registrar_expiry,
    cmd_registrar_portfolio,
    cmd_redirects_verify,
    cmd_status,
    cmd_zone_backup,
    cmd_zone_migrate,
    cmd_zone_set_ns,
    cmd_zone_verify_delegation,
)


def _resolve_repo_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return Path(__file__).resolve().parent


ROOT = _resolve_repo_root()
_DEFAULT_DESIRED = str(ROOT / "organs/domain/desired/domains.json")
_DEFAULT_REDIRECTS = str(ROOT / "organs/domain/desired/redirects.json")
_DEFAULT_BACKUP_DIR = str(ROOT / "organs/domain/state/backups")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-domain control surface")
    sub = parser.add_subparsers(dest="group")

    status = sub.add_parser("status", help="Emit non-mutating domain activation status")
    status.add_argument("--desired", default=_DEFAULT_DESIRED)

    reg = sub.add_parser("registrar", help="Porkbun registrar operations")
    reg_sub = reg.add_subparsers(dest="registrar_cmd", required=True)
    reg_sub.add_parser("portfolio", help="List domain portfolio")
    reg_sub.add_parser("expiry-audit", help="List expiry dates and auto-renew status")
    check = reg_sub.add_parser("check", help="Check domain availability")
    check.add_argument("domain")

    zone = sub.add_parser("zone", help="Cloudflare zone operations")
    zone_sub = zone.add_subparsers(dest="zone_cmd", required=True)

    z_backup = zone_sub.add_parser("backup", help="Snapshot current DNS zone")
    z_backup.add_argument("--desired", default=_DEFAULT_DESIRED)
    z_backup.add_argument("--backup-dir", default=_DEFAULT_BACKUP_DIR, dest="backup_dir")

    z_migrate = zone_sub.add_parser("migrate", help="Migrate DNS records to Cloudflare")
    z_migrate.add_argument("--desired", default=_DEFAULT_DESIRED)
    z_migrate.add_argument("--backup-dir", default=_DEFAULT_BACKUP_DIR, dest="backup_dir")
    z_migrate.add_argument("--dry-run", action="store_true")

    z_ns = zone_sub.add_parser("set-ns", help="Update NS at registrar to Cloudflare")
    z_ns.add_argument("--apex", required=True)
    z_ns.add_argument("--zone-id", required=True, dest="zone_id")
    z_ns.add_argument("--dry-run", action="store_true")

    z_verify = zone_sub.add_parser("verify-delegation", help="Verify TLD delegation")
    z_verify.add_argument("--apex", required=True)
    z_verify.add_argument("--zone-id", required=True, dest="zone_id")

    pages = sub.add_parser("pages", help="Cloudflare Pages operations")
    pages_sub = pages.add_subparsers(dest="pages_cmd", required=True)

    p_deploy = pages_sub.add_parser("deploy", help="Deploy a Pages app")
    p_deploy.add_argument("--desired", default=_DEFAULT_DESIRED)
    p_deploy.add_argument("--app", required=True, help="App subdomain (e.g. odysseum)")
    p_deploy.add_argument("--dry-run", action="store_true")

    p_verify = pages_sub.add_parser("verify", help="HTTP probe all Pages apps")
    p_verify.add_argument("--desired", default=_DEFAULT_DESIRED)

    redirects = sub.add_parser("redirects", help="Cloudflare bulk redirects operations")
    redirects_sub = redirects.add_subparsers(dest="redirects_cmd", required=True)
    redirects_verify = redirects_sub.add_parser("verify", help="Audit desired redirects state")
    redirects_verify.add_argument("--desired", default=_DEFAULT_REDIRECTS)

    proxy = sub.add_parser("proxy", help="Proxy policy operations")
    proxy_sub = proxy.add_subparsers(dest="proxy_cmd", required=True)
    proxy_audit = proxy_sub.add_parser("audit", help="Audit desired proxy policy state")
    proxy_audit.add_argument("--desired", default=_DEFAULT_DESIRED)

    return parser


def dispatch(args: argparse.Namespace) -> int:
    if args.group == "status":
        return cmd_status(args)
    if args.group == "registrar":
        if args.registrar_cmd == "portfolio":
            return cmd_registrar_portfolio(args)
        if args.registrar_cmd == "expiry-audit":
            return cmd_registrar_expiry(args)
        if args.registrar_cmd == "check":
            return cmd_registrar_check(args)
    if args.group == "zone":
        if args.zone_cmd == "backup":
            return cmd_zone_backup(args)
        if args.zone_cmd == "migrate":
            return cmd_zone_migrate(args)
        if args.zone_cmd == "set-ns":
            return cmd_zone_set_ns(args)
        if args.zone_cmd == "verify-delegation":
            return cmd_zone_verify_delegation(args)
    if args.group == "pages":
        if args.pages_cmd == "deploy":
            return cmd_pages_deploy(args)
        if args.pages_cmd == "verify":
            return cmd_pages_verify(args)
    if args.group == "redirects" and args.redirects_cmd == "verify":
        return cmd_redirects_verify(args)
    if args.group == "proxy" and args.proxy_cmd == "audit":
        return cmd_proxy_audit(args)
    print(json.dumps({"error": "unknown command", "group": args.group}))
    return 2


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    if args.group is None:
        parser.print_help()
        return 0
    return dispatch(args)
