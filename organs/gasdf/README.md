# Organ GASdf (Percolator Node)

**Role:** `gas_delegation_surface_manager`
**Instance:** `organ-gasdf-systemd`

## Description
The GASdf Organ owns the CYNIC gas-delegation runtime membrane. The current
runtime is non-signing: it reports whether fee-payer signing gates are satisfied
and refuses to act as a relayer until registry lifecycle approval, human approval
and a dedicated audited signer adapter exist.

## Environment Variables
`FEE_PAYER_PRIVATE_KEY` may be present in `~/.cynic-env`, but the current organ
does not sign with it. Presence is reported as gate evidence only.

## Deployment
Deployed via CYNIC Sovereign Cloud using systemd:
```bash
systemctl --user start organ-gasdf
```
