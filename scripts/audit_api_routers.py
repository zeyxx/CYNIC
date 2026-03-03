"""
Audit API router mounting. Verify all routers in cynic/interfaces/api/routers/
are imported and mounted in server.py.
"""

from pathlib import Path
import re


def audit_routers():
    """Check that all routers are mounted."""
    routers_dir = Path("cynic/interfaces/api/routers")
    server_path = Path("cynic/interfaces/api/server.py")

    # Exclude utility modules that aren't actual routers
    excluded = {"__init__", "auto_register", "utils"}

    # Find all router modules
    router_modules = sorted(
        [
            f.stem
            for f in routers_dir.glob("*.py")
            if f.stem not in excluded and not f.stem.startswith("_")
        ]
    )

    server_content = server_path.read_text(encoding="utf-8")

    issues = []
    imported_routers = []
    mounted_routers = []

    # Check each router is imported
    for router in router_modules:
        import_pattern = f"from.*{router}.*import"
        if re.search(import_pattern, server_content, re.IGNORECASE):
            imported_routers.append(router)
        else:
            issues.append(f"Router not imported: {router}")

    # Check each router is mounted (include_router or app.mount)
    for router in router_modules:
        # Look for include_router or direct mounting
        mount_pattern = f"(include_router|mount).*{router}"
        if re.search(mount_pattern, server_content, re.IGNORECASE):
            mounted_routers.append(router)
        else:
            # Only report as unmounted if it's also not imported
            if router not in imported_routers:
                issues.append(f"Router not mounted: {router}")
            else:
                # It's imported but not mounted - that's still an issue
                issues.append(f"Router imported but not mounted: {router}")

    if issues:
        print(f"[FAIL] API router audit found {len(issues)} issues:")
        for issue in issues:
            print(f"  - {issue}")
        print(
            f"\nAvailable routers ({len(router_modules)} total): {', '.join(router_modules)}"
        )
        print(
            f"Imported routers ({len(imported_routers)}): {', '.join(imported_routers)}"
        )
        print(f"Mounted routers ({len(mounted_routers)}): {', '.join(mounted_routers)}")
        return False
    else:
        print(f"[PASS] All {len(router_modules)} API routers are mounted")
        print(
            f"   Routers: {', '.join(router_modules[:5])}{'...' if len(router_modules) > 5 else ''}"
        )
        if len(router_modules) > 5:
            print(
                f"            {', '.join(router_modules[5:10])}{'...' if len(router_modules) > 10 else ''}"
            )
            if len(router_modules) > 10:
                print(
                    f"            {', '.join(router_modules[10:15])}{'...' if len(router_modules) > 15 else ''}"
                )
        return True


if __name__ == "__main__":
    import sys

    if not audit_routers():
        sys.exit(1)
