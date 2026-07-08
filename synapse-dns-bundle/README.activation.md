# organ-wireguard DNS activation bundle

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
