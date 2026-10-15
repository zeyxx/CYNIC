# IaC Automation Data (what a watchdog/reconciler would need)

Status: data dossier for review. No code. Collected 2026-10-15,
all read-only unless noted.

## 1. DHCP: no declared source at all

Live truth, PVE host:

- System instance (`/etc/dnsmasq.d/10-dhcp.conf`, pre-existing, not
  CYNIC-managed): `dhcp-range=10.10.10.50,10.10.10.200,12h`,
  router `10.10.10.1`, dns-server `10.10.10.1,8.8.8.8`.
  NOTE: secondary `8.8.8.8` lets clients bypass fleet DNS on any
  PVE-DNS hiccup. Observed, not judged; change needs owner.
- CYNIC instance (`/etc/cynic/dns/dnsmasq-pve.conf`, hand-written
  2026-10-14, port 1053): authoritative DNS only, no DHCP.
- Static CT addresses live in pct configs (`/etc/pve/lxc/*.conf`):
  .40/.101/.102/.103/.106 static, CT 105 on dhcp (holds the single
  lease, .119). Registry `reservations` (8 entries) agrees on all
  statics but ALSO reserves dead `.100` and lists `.119` as if static
  while live is DHCP-held.
- Leases file has exactly 1 entry. Everything else is static.

What a declared DHCP section would need: range, router, dns-server
list (and the 8.8.8.8 question), per-MAC/IP statics (source: pct
configs reconciled against reservations), lease time. Raw material
exists in two places that agree except `.100` (dead) and `.119`
(static-declared, DHCP-live).

Watcher shape: a reconciler comparing pct configs + leases against
declared reservations, reporting drift (unknown MAC, IP outside
range, declared-but-absent). No apply proposed here.

## 2. Firewall: derived, audited, never applied

- `derive_firewall_policy_from_registry` exists (default-drop,
  per-plane scopes). `network firewall --audit` works read-only:
  cortex derived-vs-live reports `drift_detected` today (live empty
  without root to observe).
- PVE live ruleset (root read 2026-10-15): filter chains all
  `policy accept` (a sieve), plus the CYNIC DNS redirect table.
  No filtering exists anywhere on the fleet.
- NET-08 wants minimal policy (default-drop + scopes); NET-10 blocks
  any rule before ghost-listener identification (`ss -tlnp` with
  root, still unprobed).

Watcher shape: continuous audit (derived vs live per host) feeding
drift alerts; apply comes only after NET-10 closes. The audit arm
already emits the right schema; what is missing is the live loop
and the apply executor (which must not exist before NET-10).

## 3. Manual-mutation backlog (2026-10-14/15 session)

Every hand mutation, host, and declared-equivalent status:

| # | Mutation | Host | Declared equivalent? |
|---|---|---|---|
| 1 | `/etc/cynic/dns/dnsmasq-pve.conf` + hosts pair | PVE | HALF: hosts rendered, conf hand-written |
| 2 | `cynic-dns.service` unit + nft DNS redirects | PVE | NO: unit + ruleset manual |
| 3 | `/etc/resolv.conf` + dhclient supersede | PVE | NO |
| 4 | resolved drop-in (10.10.10.1) | builder | YES via `dns activate`, but applied by hand SSH, not by a loop |
| 5 | `/etc/hosts` stopgaps (since removed) | builder, PVE | SUPERSEDED by fleet DNS |
| 6 | `/etc/cynic/registry.json` refresh | cortex | YES via `network registry sync` as of 2026-10-15 |
| 7 | resolved drop-ins + dnsmasq :1053 | cortex | Predates session, path unclear |

Priority for automation (data-driven): DHCP declaration first (no
source at all), firewall apply last (blocked by NET-10), the rest
are one-shot converges a reconciler would own once the sources exist.

## What this dossier decides

Nothing. It bounds the automation surface: reconcilers need
sources (DHCP has none), watchers need live loops (none exist —
watchdog is one-shot), and applies need gates (NET-10 for
firewall). Next step is specs per item, each with its gate.
