# CYNIC organ-wireguard

Legacy Synapse/WireGuard compatibility shim retained for fallback DNS bundles,
report forwarding, and migration evidence.

Canonical active authority no longer lives here:
- runtime network observation belongs to `cynic-node`
- active mesh and DNS surfaces belong to `organ-fleet`
- root-capable network application converges toward `network-actuator`
- no active mesh control loop remains in `organ-wireguard`

`organ-wireguard` remains in the repo as a legacy compatibility surface until
the remaining static references and fallback paths are either removed or fully
absorbed by the canonical network stack.
