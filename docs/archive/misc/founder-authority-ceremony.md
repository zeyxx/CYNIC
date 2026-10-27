# Founder Authority Ceremony

Offline root for node enrollment: the pre-existing **CYNIC Fleet Root CA**
(USB `cynic-root`, created 2026-10-15 by the operator). No second founder
key was created tonight — the existing anchor is adopted. Automation owns
helpers and proofs; the root belongs to the human.

## Trust anchor (public)

- Fingerprint: `5E:A0:BA:5D:05:F3:0E:E3:2D:9B:5B:60:33:8D:C8:A3:A9:29:68:10:52:DF:02:50:C8:C9:6F:93:54:8C:78:72`
- Algorithm: Ed25519, self-signed, 2026-10-15 → 2036-10-12
- Custody: USB `cynic-root/cynic-fleet-root/` (`cynic-root-ca.key` mode 400,
  HumanAdmin offline; cert + sha256 alongside)
- Scope: signs Node Enrollment Certificates / intermediates for the fleet
- Next (per USB CEREMONY.txt): intermediate CSR on kernel primary, signed
  by this root

## Rule

The root key never exists on agent-reachable disk, in agent logs, or in
agent output. Only fingerprints and certificates cross the gap.
