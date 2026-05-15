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


def generate_node_kernels(mapping: dict[str, str]) -> str:
    """
    Generate node_kernels.toml from fleet mapping.

    For each <TAILSCALE_KEY> → IP mapping, create a [[kernel]] entry:
    [[kernel]]
    name = "key"
    ws_url = "ws://IP:3030/node/ws"
    """
    lines = ["# Node kernel federation endpoints (auto-generated from fleet.toml)\n"]

    # Sort by suffix for consistent output
    sorted_items = sorted(mapping.items(), key=lambda x: x[0])

    for placeholder, ip in sorted_items:
        # Extract suffix: <TAILSCALE_CORE> → "core"
        match = re.match(r'<TAILSCALE_([A-Z_]+)>', placeholder)
        if match:
            suffix = match.group(1).lower()
            lines.append(f"\n[[kernel]]")
            lines.append(f"name = \"{suffix}\"")
            lines.append(f"ws_url = \"ws://{ip}:3030/node/ws\"")

    return "\n".join(lines) + "\n"


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
        "--mode",
        choices=["backends", "node-kernels", "both"],
        default="backends",
        help="What to generate: 'backends' (default), 'node-kernels', or 'both'",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=None,
        help="Output file for node-kernels mode (default: ~/.config/cynic/node_kernels.toml)",
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

    # Handle modes
    if args.mode == "node-kernels":
        # Generate node_kernels.toml
        result = generate_node_kernels(mapping)

        # Determine output path
        output_path = args.output or str(Path.home() / ".config" / "cynic" / "node_kernels.toml")
        output_path = Path(output_path).expanduser()

        try:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            with open(output_path, 'w') as f:
                f.write(result)
            print(f"Generated: {output_path}", file=sys.stderr)
            sys.exit(0)
        except Exception as e:
            print(f"Error writing node_kernels.toml: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.mode == "both":
        # Generate both backends.toml (via template) and node_kernels.toml
        # Read template for backends
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

        # Perform substitution for backends
        result_backends = substitute(template, mapping)

        # Check for unresolved placeholders
        unresolved = check_unresolved(result_backends)
        if unresolved:
            print(f"Error: Unresolved placeholders found: {unresolved}", file=sys.stderr)
            if args.check:
                sys.exit(1)
            else:
                print("Warning: continuing anyway", file=sys.stderr)

        # Output backends
        print(result_backends, end='')

        # Also generate node_kernels.toml to ~/.config/cynic/
        result_nodes = generate_node_kernels(mapping)
        node_kernels_path = Path.home() / ".config" / "cynic" / "node_kernels.toml"
        try:
            node_kernels_path.parent.mkdir(parents=True, exist_ok=True)
            with open(node_kernels_path, 'w') as f:
                f.write(result_nodes)
            print(f"Also generated: {node_kernels_path}", file=sys.stderr)
        except Exception as e:
            print(f"Warning: Failed to write node_kernels.toml: {e}", file=sys.stderr)

        sys.exit(0)

    else:
        # Default: backends mode
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
