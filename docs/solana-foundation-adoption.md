# Solana Foundation repos — adoption notes for CYNIC/GASdf

## Quality of code / performance
- solana-invoke: drop-in for solana_program::program::invoke* (better compute/heap). Adopt in CYNIC programs.
- solana-verifiable-build: CLI for deterministic builds + on-chain verification. Adopt in CI/CD.
- litesvm-docs: docs for fast tests. Reference.

## Enrollment de compute-01 (formalisé 2026-10-28)
- Host: cynic-compute-01 (192.168.0.25), 16 cores, 15 Gi RAM.
- User cynic: uid=997, home=/var/lib/cynic, shell=/bin/bash.
- Repo CYNIC cloné dans /var/lib/cynic/CYNIC.
- Rust/cargo installés (rustc 1.99.1, cargo 1.99.1).
- Node installé (/usr/local/bin/node) mais pas dans PATH de cynic.
- SSH pour cynic: fonctionnel (clé configurée).
- Health endpoints: 8080, 8082, 8088, 9090 actifs.
- Service gasdf-relay: inactive (pas déployé ici).
- Enrollment partiel : node pas dans PATH, pas de service CYNIC déployé.
- Action restante : ajouter node au PATH de cynic, déployer les services CYNIC.

## Pay-skills
- pay-skills: community registry of stablecoin-gated APIs for `pay` CLI. Not a skill to install — registry for pay catalog.

## Payment-channels (adopté pour étude)
- Programme Pinocchio live on mainnet (CHNLxYvVA28MJP9PrFuDXccuoGXAx7jBacfLEkahyGsX).
- Lifecycle : open → settle (Ed25519 voucher, 50 bytes) → settle_and_seal → distribute → reclaim.
- Backs MPP `session` et x402 `upto`. Vouchers cumulatifs, replay-safe.
- V2 ADRs (proposed, not implemented) :
  - ADR-004 (batch voucher settlement) : un Ed25519 signature autorise les watermarks de N channels (shared authorized_signer). Magic [0x56, 0x02], count 1..=60, channels_hash SHA-256. ~948 CU/channel. Nouveau voucher type, capability negotiation requise. All-or-nothing validation.
  - ADR-005 (channel re-arm) : réutilise les mêmes accounts pour K sessions. rearm = settle + distribute + refund, channel reste OPEN. ~2.1× moins cher par boundary (60,709 → ~28,900 CU). Pas de changement de layout. payee signer requis.
  - ADR-006 (settlement rollup) : Groth16 proof pour N settlements cross-payer. Candidate only — warranted uniquement pour payers mutuellement méfiants sans shared signer. ~300k CU proof.

- Clients : TypeScript + Rust.
- Directly relevant to CYNIC computation market. Adopt Rust client as dependency for metered payments.
- Cloned to ~/.hermes/cache/scratch/payment-channels/.

## MPP / MPP-specs (étude fit)
- MPP = Machine Payments Protocol, co-authored by Tempo + Stripe. IETF draft (draft-ryan-httpauth-payment).
- HTTP 402 standard avec WWW-Authenticate: Payment. Intents: charge, authorize, subscription.
- Methods: Tempo, Stripe, ACH, etc. Extensions: discovery, identity.
- SDKs: TS, Python, Rust, Go, Ruby.
- Fit avec pay-kit confirmé : pay-kit supporte x402 et MPP. MPP specs pour comprendre le protocole.
- mpp-specs cloné à ~/.hermes/cache/scratch/mpp-specs/.

## References / examples
- program-examples, crypto-primitives-examples: cloned to ~/.hermes/cache/scratch/solana-refs/.
- payment-channels, mpp, mpp-specs: cloned to ~/.hermes/cache/scratch/.

## 2. Adoption Active : Surfpool (LiteSVM Mainnet Fork Local)

- **Date d'adoption :** 2026-10-28
- **Version adoptée :** `surfpool 1.7.0` (Docker officiel `surfpool/surfpool:latest`).
- **Hôte de déploiement :** `cynic-compute-01` (`192.168.0.25`).
- **Service systemd :** `surfpool.service` (user service `titouan`).
- **Endpoints :**
  - RPC JSON : `http://192.168.0.25:8899`
  - WebSocket : `ws://192.168.0.25:8900`
  - Studio : `http://192.168.0.25:18488`
- **Cas d'usage :**
  - Remplacement total de Devnet et de `solana-test-validator` pour GASdf et Kora.
  - Clone à la volée de l'état réel Mainnet (ex: USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`, pools Jupiter).
  - Tests d'intégration et simulations à 0 coût sans consommation de SOL réels.
- **Vérification machine :** Validé via `GASdf/scripts/test-local-surfpool.mjs` (slot 462536387, mint USDC parsé en direct).
