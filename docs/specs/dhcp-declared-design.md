# Declared DHCP (design for review, no code)

## Problem

DHCP is the only fleet-critical service with zero declared source.
Live state (PVE, observed 2026-10-15):

- Range `.50-.200/12h`, router `.1`, dns `10.10.10.1,8.8.8.8` in
  `/etc/dnsmasq.d/10-dhcp.conf` (system instance, pre-existing).
- Statics live in pct configs; registry `reservations` mirrors them
  except dead `.100` and DHCP-held `.119` listed as static.

## Proposal

New `network_policy.dhcp` section in the registry:

```json
"dhcp": {
  "range": ["10.10.10.50", "10.10.10.200"],
  "lease_time": "12h",
  "router": "10.10.10.1",
  "dns_servers": ["10.10.10.1"],
  "statics": [{"mac": "BC:..", "ip": "10.10.10.101", "name": "forgejo"}]
}
```

- `statics` reconciled against pct configs (live) at sync time;
  mismatch = drift report, never silent overwrite.
- Renderer produces the dnsmasq `10-dhcp.conf` fragment from this
  section (same bundle pattern as DNS hosts). Apply via the CYNIC
  instance only after the operator moves DHCP serving there — NOT
  proposed now (the system instance serves today; migration is a
  separate cutover with rollback).
- Open: the `8.8.8.8` secondary (bypass hole) — decision required,
  default proposal is fleet-only DNS.

## Acceptance

- `network dhcp plan` renders the fragment from the registry;
  byte-identical to live except decided diffs.
- Drift between pct statics, leases, and declared statics is a
  report, not an edit.
