# DNS Dual-Zone Bundle (service canonical + synapse compat)

Status: proposed. Operator-approved for execution 2026-10-14.

## Problem (observed on the fleet, 2026-10-14)

`cynic network dns bundle` emits a single hosts file, hard-named
`synapse.hosts` (`crates/cynic-network/src/dns.rs:8`), whose CONTENT
varies with the registry: the zone is read from
`/network_policy/planes/service/dns_zone`
(`crates/cynic-node/src/adapters/registry/mod.rs:415`), so the same
filename carries `service.cynic` records on a current registry and
`synapse.cynic` records on a stale one. Filename lies about content.

The PVE fleet server (unit `cynic-dns.service`, live since 2026-10-14)
currently serves two files assembled from two separate bundles. That
assembly is manual and not regenerable with one command.

Live legacy callers that forbid dropping `synapse.cynic`: the forge
remote (`forgejo.synapse.cynic`), builder host entries, and the
`synapse.cynic` search domain baked into LXC images. Cutting the
compat zone breaks the builder clone path.

## Change (cynic-node DNS surface)

1. `dns_bundle.rs:build_dns_bundle` emits both files from one run:
   `service.hosts` (records in `planes/service/dns_zone`) and
   `synapse.hosts` (same records, legacy suffix). The pure renderer
   `render_service_hosts` (`dns.rs:21`) is already zone-agnostic.
2. The bundled `dnsmasq.conf` carries both `addn-hosts` lines, mirroring
   the dual-serve config already running on cortex
   (`/etc/cynic/dns/dnsmasq.conf`).
3. Rename `SERVICE_DNS_HOSTS_FILENAME` so each filename matches its
   content. No more lying names.
4. The activate path installs both files; the manifest digests both
   (the digest loop already iterates `bundle_files`, so it follows).

## Acceptance

- `cynic network dns bundle --registry infra/registry.json` produces
  both hosts files in one run.
- `dig` for a probe name in each zone resolves from the bundled config.
- The PVE server consumes the bundle as-is; the manual two-bundle
  assembly is deleted.
- Existing bundle tests updated; one test pins that both files exist
  and carry their own zone suffix.

## Operator note: provider selection follows the caller's PATH

Proven 2026-10-15: the bundle emits the CoreDNS config when `dnsmasq`
is not visible on the caller's `PATH`, even on a host where dnsmasq is
installed (observed on cortex, whose user `PATH` lacks `/usr/sbin`).
This is deterministic, not a flip-flop: provider observation uses
`find_on_path`, so the same host renders `dnsmasq.conf` under one
`PATH` and `Corefile` under another. Rule: run DNS bundle/activate
commands with the system `PATH` (include `/usr/sbin`). Bundle tests
that assert provider-specific files must sanitize `PATH` first.
