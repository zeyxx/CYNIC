#!/usr/bin/env python3
"""
Fleet-based template substitution for backends.toml.

Reads ~/.config/cynic/fleet.toml, maps machines to placeholders,
substitutes all <TAILSCALE_*> placeholders in backends.toml template.

Usage:
    fleet-gen.py < backends.toml > ~/.config/cynic/backends.toml
    fleet-gen.py backends.toml > ~/.config/cynic/backends.toml
    fleet-gen.py --template backends.toml.tpl --output ~/.config/cynic/backends.toml

Environment variables:
    CYNIC_FLEET_PATH - Path to fleet.toml (default: ~/.config/cynic/fleet.toml)

Examples:
    # From stdin
    cat backends.toml | fleet-gen.py > ~/.config/cynic/backends.toml

    # From file
    fleet-gen.py < backends.toml > output.toml

    # With arguments
    fleet-gen.py backends.toml.tpl > ~/.config/cynic/backends.toml
"""

import sys
import os
import re
import argparse
from pathlib import Path


def load_fleet(fleet_path: str) -> dict[str, str]:
    """
    Load fleet.toml and build placeholder → IP mapping.

    Returns dict: {"<TAILSCALE_KEY>": "IP_ADDRESS"}
    """
    try:
        import tomllib  # Python 3.11+
    except ImportError:
        try:
            import tomli as tomllib  # fallback
        except ImportError:
            print("Error: tomllib (Python 3.11+) or tomli not available", file=sys.stderr)
            sys.exit(1)

    with open(fleet_path, 'rb') as f:
        fleet = tomllib.load(f)

    mapping = {}
    machines = fleet.get('machine', {})
    for machine_key, machine_data in machines.items():
        if 'tailscale_ip' in machine_data:
            # Convention: TAILSCALE_{suffix} where suffix is after the last hyphen (if any)
            # cynic-core → CORE, kairos → KAIROS
            suffix = machine_key.split('-')[-1].upper()
            placeholder = f"<TAILSCALE_{suffix}>"
            ip = machine_data['tailscale_ip']
            mapping[placeholder] = ip

    return mapping


def substitute(template: str, mapping: dict[str, str]) -> str:
    """Substitute all placeholders in template."""
    result = template
    for placeholder, value in mapping.items():
        result = result.replace(placeholder, value)
    return result


def check_unresolved(content: str) -> list[str]:
    """Find any remaining <TAILSCALE_*> placeholders."""
    pattern = r'<TAILSCALE_[A-Z_]+>'
    return re.findall(pattern, content)


def main():
    parser = argparse.ArgumentParser(
        description="Fleet-based template substitution for backends.toml",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  fleet-gen.py < backends.toml.tpl > ~/.config/cynic/backends.toml
  fleet-gen.py backends.toml.tpl > output.toml
  cat template.toml | fleet-gen.py > output.toml
        """,
    )
    parser.add_argument(
        "template",
        nargs="?",
        help="Template file (default: stdin)",
    )
    parser.add_argument(
        "--fleet",
        type=str,
        default=None,
        help="Fleet TOML path (default: ~/.config/cynic/fleet.toml)",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check for unresolved placeholders and exit with error if found",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print substitution mappings to stderr",
    )

    args = parser.parse_args()

    # Resolve fleet path
    fleet_path = args.fleet or os.environ.get("CYNIC_FLEET_PATH")
    if not fleet_path:
        fleet_path = str(Path.home() / ".config" / "cynic" / "fleet.toml")

    fleet_path = Path(fleet_path).expanduser()
    if not fleet_path.exists():
        print(f"Error: Fleet file not found: {fleet_path}", file=sys.stderr)
        sys.exit(1)

    # Load fleet and build mapping
    try:
        mapping = load_fleet(str(fleet_path))
    except Exception as e:
        print(f"Error loading fleet: {e}", file=sys.stderr)
        sys.exit(1)

    if args.verbose:
        print("Substitution mappings:", file=sys.stderr)
        for placeholder, ip in sorted(mapping.items()):
            print(f"  {placeholder} → {ip}", file=sys.stderr)

    # Read template
    if args.template:
        template_path = Path(args.template).expanduser()
        try:
            with open(template_path, 'r') as f:
                template = f.read()
        except Exception as e:
            print(f"Error reading template: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        template = sys.stdin.read()

    # Perform substitution
    result = substitute(template, mapping)

    # Check for unresolved placeholders
    unresolved = check_unresolved(result)
    if unresolved:
        print(f"Error: Unresolved placeholders found: {unresolved}", file=sys.stderr)
        if args.check:
            sys.exit(1)
        else:
            print("Warning: continuing anyway", file=sys.stderr)

    # Output
    print(result, end='')
    sys.exit(0)


if __name__ == '__main__':
    main()
