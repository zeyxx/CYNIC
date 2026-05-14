#!/usr/bin/env python3
"""
Fleet-based template substitution for backends.toml.
Reads ~/.config/cynic/fleet.toml, maps machines to placeholders,
substitutes in backends.toml template (stdin or file).

Usage:
    fleet-gen.py < backends.toml > ~/.config/cynic/backends.toml
    fleet-gen.py backends.toml > ~/.config/cynic/backends.toml
"""

import sys
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ImportError:
    import tomli as tomllib  # Fallback for older Python


def load_fleet(fleet_path: str) -> dict[str, str]:
    """Load fleet.toml and build placeholder map.

    Args:
        fleet_path: Path to fleet.toml

    Returns:
        Mapping of <TAILSCALE_KEY> -> IP address
    """
    with open(fleet_path, 'rb') as f:
        fleet = tomllib.load(f)

    # Map: <TAILSCALE_KEY> -> IP
    mapping = {}
    for machine_key, machine_data in fleet.get('machine', {}).items():
        if 'tailscale_ip' in machine_data:
            placeholder = f"<TAILSCALE_{machine_key.upper()}>"
            ip = machine_data['tailscale_ip']
            mapping[placeholder] = ip

    return mapping


def substitute(template: str, mapping: dict[str, str]) -> str:
    """Substitute all placeholders in template.

    Args:
        template: Template string with <TAILSCALE_*> placeholders
        mapping: Dict mapping placeholders to IP addresses

    Returns:
        Template with all placeholders substituted
    """
    result = template
    for placeholder, ip in mapping.items():
        result = result.replace(placeholder, ip)
    return result


def main():
    fleet_path = Path.home() / ".config" / "cynic" / "fleet.toml"

    if not fleet_path.exists():
        print(f"Error: {fleet_path} not found", file=sys.stderr)
        sys.exit(1)

    mapping = load_fleet(str(fleet_path))

    # Read template from stdin or file
    if len(sys.argv) > 1:
        try:
            with open(sys.argv[1], 'r') as f:
                template = f.read()
        except FileNotFoundError:
            print(f"Error: {sys.argv[1]} not found", file=sys.stderr)
            sys.exit(1)
    else:
        template = sys.stdin.read()

    substituted = substitute(template, mapping)
    print(substituted)


if __name__ == '__main__':
    main()
