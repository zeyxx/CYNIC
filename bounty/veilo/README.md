# Bounty Veilo — Superteam Earn

- **Bounty**: Find Critical Vulnerabilities in Veilo Mainnet Smart Contracts for Fund Loss
- **Sponsor**: VeiloLayer
- **Reward**: 1st place = 2000 USDC
- **Program (mainnet)**: `GYy4kM6GHhpgLCUscuABbzkD2ZbJ2fneYryaZ6Ch7fFU` (privacy_pool)
- **Repo officiel**: https://github.com/VeiloSolana/privacy-program
- **Deadline**: 2026-08-20T22:59:59 UTC
- **Scope**: onchain Solana program UNIQUEMENT (pas site/extension/backend)
- **Agent access**: HUMAN_ONLY → soumission humaine requise (pas d'agent automatisé)
- **Règle**: ne pas exploiter mainnet / ne pas bouger de fonds réels

## Requirements du rapport (Superteam)
1. Affected mainnet contract or function
2. Clear explanation of the issue
3. How it leads to fund loss
4. Reproducible proof or simulation
5. Suggested fix
6. Confirmation that no live funds were moved/at risk

## Template de soumission (à remplir avec les findings des agents)

### Finding [X]: [Titre]

**Affected function**: `[nom] (phoenix.rs:Lxxx / perps.rs:Lxxx / ...)`
**Severity**: Critical (fund loss)
**Category**: [overcredit / access-control / arithmetic / signer-bypass / double-spend / ZK-forgery]

**Description**:
[explication claire du mécanisme]

**Path to fund loss**:
[comment un attaquant extrait des fonds]

**Proof of Concept**:
[Rust pseudo-code ou steps de reproduction + simulation]

**Suggested fix**:
[code corrigé proposé]

---

## Status des agents d'audit (parallèle)
- [ ] Agent 0: phoenix.rs
- [ ] Agent 1: perps.rs + predictions.rs
- [ ] Agent 2: swap.rs
- [ ] Agent 3: positions.rs + merkle_tree.rs
- [ ] Agent 4: zk.rs + groth16.rs

## Findings consolidés
_(rempli après retour des agents)_
