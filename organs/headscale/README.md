# CYNIC organ-headscale

`organ-headscale` owns the sovereign access mesh. It provides
Headscale/Tailscale-compatible reachability for operators, cortex sessions, and
agents.

It does not own the deterministic Synapse service mesh. Native WireGuard
routing, service IPs, and `*.synapse.cynic` naming belong to `organ-wireguard`.

