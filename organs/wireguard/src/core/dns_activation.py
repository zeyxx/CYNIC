"""Synapse DNS activation contracts and templates."""

from __future__ import annotations


def dns_provider_policy() -> dict:
    return {
        "canonical_provider": "coredns",
        "canonical_binary": "coredns",
        "selection_source": "organ-wireguard Synapse DNS activation contract",
        "service_contract": "*.synapse.cynic",
        "approved_sources": [
            "distribution_package",
            "approved_internal_package_mirror",
        ],
        "known_noncanonical_candidates": [
            "dnsmasq",
            "unbound",
            "bind9",
            "systemd-resolved",
        ],
        "promotion_rule": (
            "A candidate provider becomes usable only after a dedicated adapter contract, "
            "bundle renderer, service template, and activation-status checks are added."
        ),
    }


def dns_activation_plan() -> dict:
    return {
        "schema": "cynic.organ_wireguard.dns_activation_plan.v1",
        "mutates": False,
        "target": {
            "zone": "synapse.cynic",
            "hosts": "/etc/coredns/synapse.hosts",
            "corefile": "/etc/coredns/Corefile",
            "service": "/etc/systemd/system/coredns-synapse.service",
            "resolved_dropin": "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
        },
        "required_owner": "root:root",
        "required_modes": {
            "hosts": "0644",
            "corefile": "0644",
            "service": "0644",
            "resolved_dropin": "0644",
        },
        "provider_policy": dns_provider_policy(),
        "package_preconditions": [
            {
                "name": "coredns",
                "binary": "coredns",
                "allowed_sources": [
                    "distribution_package",
                    "approved_internal_package_mirror",
                ],
                "forbidden_sources": [
                    "curl_pipe_shell",
                    "wget_pipe_shell",
                    "unverified_upstream_binary",
                ],
            }
        ],
        "activation_checks": [
            "install CoreDNS from a distribution package or approved internal package mirror",
            "command -v coredns",
            "install -o root -g root -m 0644 synapse.hosts /etc/coredns/synapse.hosts",
            "install -o root -g root -m 0644 Corefile /etc/coredns/Corefile",
            "install -o root -g root -m 0644 coredns-synapse.service /etc/systemd/system/coredns-synapse.service",
            "install -o root -g root -m 0644 cynic-synapse-dns.conf /etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
            "systemctl daemon-reload",
            "systemctl enable --now coredns-synapse.service",
            "systemctl restart systemd-resolved.service",
            "getent hosts organ-surrealdb.synapse.cynic resolves 10.20.0.3",
        ],
        "forbidden": [
            "manual /etc/hosts edits as canonical DNS",
            "192.168.* service endpoint",
            "remote installer piped to shell",
            "wget installer piped to shell",
            "unverified upstream CoreDNS binary",
            "automatic fallback to a noncanonical DNS provider without an adapter contract",
        ],
    }


def coredns_corefile() -> str:
    return """.:1053 {
    errors
    hosts /etc/coredns/synapse.hosts {
        fallthrough
    }
}
"""


def coredns_systemd_unit() -> str:
    return """[Unit]
Description=CYNIC Synapse CoreDNS
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/bin/env coredns -conf /etc/coredns/Corefile
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"""


def resolved_dropin() -> str:
    return """[Resolve]
DNS=127.0.0.1:1053
Domains=~synapse.cynic
"""


def dns_activation_readme() -> str:
    return """# organ-wireguard DNS activation bundle

This bundle is non-mutating. Install it only through an approved activation organ
or operator path. Cortex sessions must not install these files directly as root.

CoreDNS binary policy:

- Allowed: distribution package or approved internal package mirror.
- Forbidden: remote installer piped to shell, unverified upstream binary drops.
- Noncanonical providers such as dnsmasq, unbound or bind9 require their own
  adapter contract before they can activate Synapse DNS.

After installation, verify:

1. `systemctl is-active coredns-synapse.service`
2. `getent hosts organ-surrealdb.synapse.cynic`
3. `python3 organs/wireguard/src/entrypoints/cli.py status --resolve-dns --include-records`
"""
