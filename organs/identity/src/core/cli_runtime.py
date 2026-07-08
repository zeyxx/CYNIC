from __future__ import annotations

import argparse
import getpass
import json
import sys

from organs.identity.src.adapters.env_adapter import ApiSmokecheckAdapter, EnvFileCredentialStore
from organs.identity.src.core.use_cases import (
    AddCredentialUseCase,
    AttestCredentialsUseCase,
    KNOWN_GROUPS,
    ListCredentialsUseCase,
    RemoveCredentialUseCase,
    SmokeCheckUseCase,
)


def _agent(data: dict) -> None:
    print(json.dumps(data, indent=2))


def _is_agent_mode() -> bool:
    return not sys.stdout.isatty()


def _status(present: bool) -> str:
    return "✓" if present else "✗"


def _store() -> EnvFileCredentialStore:
    return EnvFileCredentialStore()


def _resolve_key(user_input: str) -> str:
    """Map human-friendly names to canonical organism keys."""
    raw = user_input.upper().replace("-", "_")
    aliases = {
        "PORKBUN_SECRET": "PORKBUN_SECRET_API_KEY",
        "PORKBUN_SECRET_KEY": "PORKBUN_SECRET_API_KEY",
        "PORKBUN_API": "PORKBUN_API_KEY",
        "PORKBUN_KEY": "PORKBUN_API_KEY",
        "CF_TOKEN": "CF_API_TOKEN",
        "CLOUDFLARE_TOKEN": "CF_API_TOKEN",
        "CLOUDFLARE_API_TOKEN": "CF_API_TOKEN",
        "CF_ACCOUNT": "CF_ACCOUNT_ID",
        "CLOUDFLARE_ACCOUNT": "CF_ACCOUNT_ID",
        "CLOUDFLARE_ACCOUNT_ID": "CF_ACCOUNT_ID",
    }
    return aliases.get(raw, raw)


def cmd_list(args) -> int:
    group = getattr(args, "group", None)
    uc = ListCredentialsUseCase(_store())
    attestations = uc.execute(group=group)

    if _is_agent_mode():
        _agent({
            "op": "list",
            "source": _store().env_file_path(),
            "credentials": [
                {"key": a.key, "present": a.present, "shape": a.shape, "length": a.length}
                for a in attestations
            ],
        })
        return 0

    source = _store().env_file_path()
    title = f"Credentials — {group or 'all groups'}"
    print(f"\n  {title}")
    print(f"  {'─' * len(title)}")
    print(f"  Source: {source}\n")

    keyed = {a.key: a for a in attestations}
    for gname, keys in KNOWN_GROUPS.items():
        if group and gname != group:
            continue
        members = [keyed[k] for k in keys if k in keyed]
        if not members:
            continue
        print(f"  [{gname}]")
        for a in members:
            detail = f"  {a.shape}" if a.present and a.shape else "  (absent)"
            print(f"    {_status(a.present)}  {a.key}{detail}")
        print()

    missing = sum(1 for a in attestations if not a.present)
    if missing:
        print(f"  ⚠  {missing} credential(s) missing\n")
    else:
        print("  All credentials present.\n")
    return 0


def cmd_add(args) -> int:
    key = _resolve_key(args.key)

    if not _is_agent_mode():
        print(f"\n  Adding: {key}")
        print(f"  Source: {_store().env_file_path()}")
        print("  Value will not appear in shell history.\n")

    try:
        value = getpass.getpass(f"  Value for {key}: ").strip()
    except (KeyboardInterrupt, EOFError):
        if not _is_agent_mode():
            print("\n  Aborted.")
        return 1

    if not value:
        if not _is_agent_mode():
            print("  Empty value — aborted.")
        return 1

    uc = AddCredentialUseCase(_store())
    result = uc.execute(key, value)

    if _is_agent_mode():
        _agent({"op": "add", "ok": result.ok, "message": result.message, "details": result.details})
        return 0 if result.ok else 1

    print(f"\n  {'✓' if result.ok else '✗'}  {'Saved: ' + key if result.ok else result.message}\n")
    return 0 if result.ok else 1


def cmd_remove(args) -> int:
    key = _resolve_key(args.key)

    if not _is_agent_mode():
        store = _store()
        if not store.read_key(key):
            print(f"\n  ✗  {key} not found in {store.env_file_path()}\n")
            return 1
        print(f"\n  Removing: {key}  ({store.env_file_path()})\n")
        try:
            confirm = input("  Confirm? [y/N] ").strip().lower()
        except (KeyboardInterrupt, EOFError):
            print("\n  Aborted.")
            return 1
        if confirm != "y":
            print("  Aborted.\n")
            return 1

    uc = RemoveCredentialUseCase(_store())
    result = uc.execute(key)

    if _is_agent_mode():
        _agent({"op": "remove", "ok": result.ok, "message": result.message})
        return 0 if result.ok else 1

    print(f"\n  {'✓' if result.ok else '✗'}  {result.message}\n")
    return 0 if result.ok else 1


def cmd_verify(args) -> int:
    providers = getattr(args, "providers", None) or None

    if not _is_agent_mode():
        label = ", ".join(providers) if providers else "all providers"
        print(f"\n  Verifying — {label}")
        print(f"  {'─' * 36}\n")

    uc = SmokeCheckUseCase(_store(), ApiSmokecheckAdapter())
    results = uc.execute(providers)
    all_ok = all(r.ok for r in results)

    if _is_agent_mode():
        _agent({
            "op": "verify",
            "all_ok": all_ok,
            "results": [{"provider": r.provider, "ok": r.ok, "message": r.message} for r in results],
        })
        return 0 if all_ok else 1

    for r in results:
        print(f"  {'✓' if r.ok else '✗'}  {r.provider:<14}  {r.message}")
    print()
    print(f"  {'All verified.' if all_ok else 'Some failed.'}\n")
    return 0 if all_ok else 1


def cmd_setup(args) -> int:
    group = args.group.lower()
    if group not in KNOWN_GROUPS:
        print(f"\n  ✗  Unknown provider: {group}\n")
        return 1
        
    keys = KNOWN_GROUPS[group]
    print(f"\n  Setup — {group}")
    print(f"  {'─' * (10 + len(group))}")
    print(f"  Source: {_store().env_file_path()}\n")
    
    uc = AddCredentialUseCase(_store())
    for key in keys:
        try:
            value = getpass.getpass(f"  Value for {key}: ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\n  Aborted.\n")
            return 1
            
        if not value:
            print(f"  Skipped {key}.")
            continue
            
        result = uc.execute(key, value)
        if result.ok:
            print(f"  ✓  Saved {key}")
        else:
            print(f"  ✗  Failed: {result.message}")
            return 1
    
    print("\n  Verifying...")
    uc_verify = SmokeCheckUseCase(_store(), ApiSmokecheckAdapter())
    results = uc_verify.execute([group])
    if results and results[0].ok:
        print(f"  ✓  {group} API connection verified.\n")
    else:
        print(f"  ✗  Verification failed: {results[0].message if results else 'unknown'}\n")
    
    return 0


def cmd_attest(_args) -> int:
    uc = AttestCredentialsUseCase(_store())
    _agent(uc.execute())
    return 0


def cmd_groups(_args) -> int:
    if _is_agent_mode():
        _agent({"op": "groups", "groups": KNOWN_GROUPS})
        return 0
    print("\n  Known credential groups\n")
    for gname, keys in KNOWN_GROUPS.items():
        print(f"  [{gname}]")
        for k in keys:
            print(f"    · {k}")
        print()
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="organ-identity — credential management",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
  setup porkbun                Interactive setup for a provider
  add PORKBUN_API_KEY          Add a credential (value hidden)
  list                         Show all credentials (redacted)
  list --group porkbun         Show one group
  verify                       Test credentials against live APIs
  verify --providers porkbun   Test one provider
  remove PORKBUN_API_KEY       Remove a credential
  attest                       Full attestation for agents (JSON + SHA3-512)
  groups                       List known credential groups
""",
    )
    sub = parser.add_subparsers(dest="cmd")

    setup = sub.add_parser("setup", help="Interactive setup for a provider group")
    setup.add_argument("group", choices=list(KNOWN_GROUPS.keys()), help="Provider name (e.g. porkbun)")

    lst = sub.add_parser("list", help="Show credentials (redacted)")
    lst.add_argument("--group", choices=list(KNOWN_GROUPS.keys()), metavar="GROUP")

    add = sub.add_parser("add", help="Add or update a credential (value hidden)")
    add.add_argument("key", help="e.g. PORKBUN_API_KEY")

    rm = sub.add_parser("remove", help="Remove a credential")
    rm.add_argument("key")

    verify = sub.add_parser("verify", help="Test credentials against live APIs")
    verify.add_argument("--providers", nargs="*", choices=["porkbun", "cloudflare"], metavar="PROVIDER")

    sub.add_parser("attest", help="Full attestation for agents (JSON, SHA3-512 commitments)")
    sub.add_parser("groups", help="List known credential groups")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    if args.cmd is None:
        parser.print_help()
        return 0
    fn = {
        "list": cmd_list,
        "add": cmd_add,
        "remove": cmd_remove,
        "verify": cmd_verify,
        "setup": cmd_setup,
        "attest": cmd_attest,
        "groups": cmd_groups,
    }.get(args.cmd)
    if fn is None:
        parser.error(f"unknown command: {args.cmd}")
        return 2
    return fn(args)
