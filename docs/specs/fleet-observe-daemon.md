# Fleet Observe Daemon (design for review, no code)

## Problem

Every fleet truth that matters is sampled by hand: WG handshakes
(NET-11), listeners (NET-10, re-dirties), DHCP leases, nft drift,
clock skew (NET-12). `network registry observe` is one-shot;
`observed.nodes` sits empty. Per the probe rule, timers and crons
are not probes: this must be a living service.

## Proposal

One `cynic-observe` service per host (systemd unit, no timer):

- Loop with per-signal periods: WG handshakes 60s, listeners 300s,
  leases 300s, nft audit 300s, clock 900s. Periods declared in the
  registry (`network_policy.observe`), not hardcoded.
- Each sample writes `observed.nodes.<self>` in the LOCAL runtime
  copy via the same projection `network registry observe` uses
  (one code path, two callers). No cross-host writes: each host
  owns its section; fleet view is the union read at query time.
- Liveness is itself observed: the daemon touches a heartbeat file
  with monotonic timestamp; a stale heartbeat IS the alert (dead
  watcher is data, not silence).
- Output schema per sample reuses existing reports (watchdog,
  firewall audit, observation projection). No new schemas.

## Gates (each before code)

1. NET-12 first: samples are dated; undated fleet + certs later =
   garbage. Prove clocks before trusting timestamps.
2. Write path: local runtime copy only (no SSH, no central push).
3. No apply powers: observe-only binary. Reconcile/apply stays in
   human-gated commands until a gate spec says otherwise.

## Acceptance

- Kill the daemon on one host: fleet query shows its heartbeat
  stale within 2 periods, nothing else changes.
- `observed.nodes` holds one fresh section per live host.
- No cron, no timer unit anywhere in the design.
