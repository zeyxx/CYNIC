# Organ GASdf (Percolator Node)

**Role:** `gas_delegation_surface_manager`
**Instance:** `organ-gasdf-systemd`

## Description
The GASdf Organ is a stateless Node.js infrastructure proxy. It implements the "Percolator Model", acting as the off-chain relayer that signs Solana transactions to pay network fees (Base + Priority) on behalf of users. It interfaces directly with the `GASdf` on-chain Pinocchio smart contract, relying on the `RelayWithVault` instruction to be atomically reimbursed in SOL.

## Environment Variables
Requires `FEE_PAYER_PRIVATE_KEY` to be set in `~/.cynic-env`. This hot wallet must be funded with SOL.

## Deployment
Deployed via CYNIC Sovereign Cloud using systemd:
```bash
systemctl --user start organ-gasdf
```
