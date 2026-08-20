# Fleet observation — 2026-08-20

Measured from the substrate, not copied from `infra/registry.json`. Every line
below came from a command whose output is quoted; nothing is inferred from the
registry's intent.

## Where the kernel actually runs

| | |
|---|---|
| binary | `/usr/local/bin/cynic-kernel` (pid 89772 at time of observation) |
| principal | `cynic` |
| uptime | `1-06:52:55` (`ps -o etime=`) |
| host | **cortex — this machine** |
| fleet-facing socket | `0.0.0.0:3030` |
| auth | Bearer / Signature; a bare `GET /health` returns `{"error":"Missing Bearer token or Signature headers"}` |

`127.0.0.1:8080` is **llama-server**, not the kernel. An earlier read of this
session mistook it for one; `ss -tlnp` attributes it to
`llama-server pid=2163859`.

Note on observation method: `ss -tlnp` hides sockets owned by other principals
when run unprivileged, so the kernel's listener is invisible in its output. It
was found by listing every listening socket (`ss -tln`) and probing the only
fleet-facing one.

## What the registry declares

```
nodes[2]  id=local-kernel  role=kernel_primary
          network.synapse=10.10.10.100  status=candidate  deploy_target=true
nodes[1]  id=cynic-host-cortex  role=cortex_host
          network.management=192.168.100.2  status=candidate  services=[]
```

`10.10.10.100` answers nothing — no ICMP reply, no HTTP on 8787/8080/3000/9000.
The registry is honest about this (`status: candidate`); it declares an intent
that was never materialised.

## Cortex is on a different plane, not off-network

```
$ ip -4 -o addr
  enp42s0    192.168.100.2/24        <- matches registry network.management
  wg0        10.20.0.3/24
$ ip route get 10.10.10.100
  10.10.10.100 via 192.168.100.1 dev enp42s0 src 192.168.100.2
```

Cortex sits on the `management` plane and *routes* to `10.10.10.0/24` without
holding an address in it. The registry declares exactly this
(`network_links[0].routes = ["10.10.10.0/24"]`), so substrate and registry agree
here.

## Discoverability gap

Cortex is the only node in the registry with **no synapse IP and an empty DNS
list**:

```
cynic-host-proxmox     10.10.10.1     [cynic-host-proxmox.synapse.cynic, pve...]
cynic-host-cortex      -              []                 <-- only node with none
forgejo                10.10.10.101   [forgejo.synapse.cynic, ...]
cynic-lxc-surrealdb    10.10.10.103   [...]
cynic-builder          10.10.10.119   [cynic-builder.synapse.cynic]
local-kernel           10.10.10.100   [kernel.synapse.cynic, ...]
```

Observed from the builder — the reverse direction that discoverability actually
means:

```
$ getent hosts cynic-host-cortex.synapse.cynic   -> UNRESOLVED
$ ping -c1 192.168.100.2                         -> REACHABLE
$ curl http://192.168.100.2:3030/health          -> 401
```

So cortex is **reachable by IP, invisible by name**, and the kernel it hosts is
serving the fleet.

## One name, three answers

`kernel.synapse.cynic` resolves differently depending on who asks:

| source | resolves to |
|---|---|
| cortex `/etc/hosts:13` | `127.0.0.1` |
| builder `/etc/hosts:9` | `192.168.100.2` |
| `infra/registry.json` | `10.10.10.100` |

The builder's `/etc/hosts` already encodes the truth (cortex hosts the kernel)
that the registry still denies. These are hand-maintained per-node overrides:
each host was patched to make the kernel reachable, and the registry was never
reconciled. `/etc/cynic/dns/synapse.hosts` contains no `kernel` or `cortex`
entry, so the CYNIC-managed zone is not the source of either mapping.

## Residual unknowns

Deliberately not resolved here, because resolving them requires mutation or
privilege this observation did not have:

- whether `10.10.10.100` was ever assigned to anything, or is purely aspirational;
- which component writes `/etc/hosts` on each node, if any (no CYNIC surface was
  observed doing it);
- the kernel's own view of its identity and bound address — `/proc/89772/environ`
  is unreadable unprivileged, and `/health` requires a credential this
  observation did not use.

## What this does not authorise

Giving cortex a synapse address changes routing on the host that carries
operator connectivity. That is a bounded mutation with a preflight, an explicit
`--apply`, and a rollback intent — not an `ip addr add`. This document is the
observation half only.
