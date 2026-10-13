# Solana Convergence Map

> Status: deliberation draft. Non-authoritative narrative.
> Runtime authority remains `contracts/foundation.json`.
> Facts carry their primary source and observation date.
> Items marked UNVERIFIED come from working knowledge, not a re-read primary page.

## Direction

CYNIC is a computational and scientific OS. Once the founding chantiers land,
every application built on it settles on Solana, with no exception.

## Ten dimensions

| # | Dimension | Solana (primary source) | CYNIC state (evidence) | Link | Deciding gap |
|---|-----------|------------------------|------------------------|------|--------------|
| 1 | Time and consensus | Slots at 250 ms target, stable skip rate, p99 leader window ~3 s to ~2 s. Debate on 200 ms pre-Alpenglow. Source: solana.com/news/slot-time-reduction-effects, Sep 28 2026. | Nightshift hourly rounds, SHA-256 digests, `cynic.nightshift_report.v1`. `record_poh_checkpoint` coded, never deployed. Evidence: `programs/cynic-anchor/src/lib.rs`, `docs/specs/NIGHTSHIFT_ORCHESTRATION_AND_POH_SPEC.md`. | Informal | Define epoch, Merkle leaves, anchor cadence, real PDA derivation, authority wallet, read-back. |
| 2 | Execution | Sealevel parallel runtime, Compute Units, full on-chain order books for compute, Payment Channels at 1M payments/sec in testing. Sources: solana.com/news (Sep 3 and Oct 2 2026). | `cynic-protocol/src/compute/` (types, decision, port), `cynic-sandbox`, `cynic-proxmox`, `cynic-workload`. Real on main. | Orthogonal | Price workloads in CU-comparable units, publish supply and demand, settle. |
| 3 | Accounts and programs | PDA: 32 bytes from program ID plus seeds, guaranteed off the ed25519 curve, no private key, program signs via `invoke_signed`. Limits: 16 seeds, 32 bytes each, bump 0-255. Source: solana.com/docs/core/pda. | Three canonical PDAs specified (`cynic_poh`, `futarchy_market`, `ignis_vault`). Client derivation is a SHA-256 stub, not `find_program_address`. Program ID placeholder, never deployed. Evidence: `crates/cynic-protocol/src/anchor_client.rs`, `docs/specs/SOLANA_ANCHOR_ARCHITECTURE_SPEC.md`. | Informal | Replace the stub with real derivation, name the signing identity, set account close policy, deploy. |
| 4 | Assets | Two token programs (original immutable, Token-2022 with extensions). Mint carries supply, decimals, optional mint and freeze authorities. Source: solana.com/docs/tokens. | ASDF exists on mainnet: `9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump`, fixed supply 853,137,556, 6 decimals, pump origin. Verified on explorer.solana.com, Oct 12 2026. `execute_ignis_burn` targets the Token-2022 program, which cannot burn this mint. Evidence: `programs/cynic-anchor/src/lib.rs`. | Informal, half obsolete | Burn via the Token program or issue a Token-2022 mint. Name supply, authorities, vault custody. |
| 5 | Data and engagement | Commitments (processed, confirmed, finalized), `getSignaturesForAddress`, `getAccountInfo`, WebSocket, Yellowstone gRPC. Providers tiered (Helius, Triton, Quicknode). Test layers: LiteSVM, Surfpool. Source: colosseum.com/worldsfair/resources, read Oct 12 2026. | Pure instruction builders, no solana-sdk (deliberate). Simulation tests only, fake keys, no network. No submission adapter, no commitment choice, no account reads. Evidence: `crates/cynic-protocol/src/anchor_client.rs`, `crates/cynic-protocol/tests/anchor_rpc_simulation.rs`. | Informal | Choose commitment level, build the RPC adapter, implement reference-key tracking, wire LiteSVM or Surfpool. |
| 6 | Keys and wallets | Phantom (injected and embedded), Wallet Standard, Squads/Altitude operating accounts, Mobile Wallet Adapter. Samsung Wallet: native USDC on 82M US Galaxy devices, late Oct 2026, fiat ramps, chain hidden. Source: solana.com/news/samsung-wallet, Oct 7 2026. | ed25519-dalek v2, node and storage identity enrollment, secret presence without value exposure, `secret_exposed` flag, Kyon encrypted vault with explicit export. No wallet signs anything. Evidence: `crates/cynic-host/src/storage_identity/`, `docs/design/KYON_V0_SPEC.md`. | Orthogonal | Choose custody (file, enclave, Squads multisig, embedded) per key class. Bridge the vault to a signer. Resolve seedless onboarding against key sovereignty. |
| 7 | Payments | Solana Pay URL spec, QR and Blinks distribution, x402 at 76% share, Payment Channels 1M tps in test, on-chain subscriptions with caps and revocation, Open USD free mint and burn ($1B committed), atomic DvP for institutions. Sources: solana.com/news (Sep 30, Oct 6, Oct 9 2026). | Spec-conformant URL builder, reference keys, allowance validation with recipient and mint allowlists, ZeroClaw plugin with SOP firewall before URL materialization, ASCII QR, takeover memo with monotonic fencing. Nothing submitted or confirmed on-chain. Amounts in f64 against on-chain u64. Evidence: `crates/cynic-solana-pay/src/ops.rs`, `crates/cynic-zeroclaw-solana/src/plugin.rs`. | Semi-formal | Integer amounts, real confirmation reads, on-chain allowance revocation, x402, first collected payment. |
| 8 | Distribution | Actions (GET metadata, POST signable transaction), Blinks (any shared link becomes a client), Dialect implementation (active Aug 2026, legacy web3.js line). Source: colosseum.com/worldsfair/resources, read Oct 12 2026. | `futard_audience_projection` ambition with explicit anti-target (never a Futard page as runtime authority). No action endpoint, no Blink. Only static QR. Evidence: `crates/cynic-protocol/src/framework.rs`. | Orthogonal | Collect before distributing. Then shareable proof-check and sponsorship links. |
| 9 | Markets | On-chain order books, Meteora DBC (bonding curve graduating to liquidity), pump curves graduating to fixed supply, live conditional markets via MetaDAO. Sources: solana.com/news (Oct 2 2026) and ASDF history. | CPMM bet in u128 integer math, bps-threshold resolution, 222 lines of tests, Futard.io buckets waiting for real demand. Pools in internal units, single-oracle resolution, no graduation path. Evidence: `programs/cynic-anchor/src/lib.rs`, `programs/cynic-anchor/src/cpmm_tests.rs`. | Informal | Real liquidity, decentralized resolution, audited pricing formula, graduation to liquid markets. |
| 10 | Proof and scale | State compression, confidential transfers, attestation services, durable nonces. UNVERIFIED: working knowledge, no re-read primary page. | Local-first proof, versioned exportable consent, versioned contestable E-Score registries, BLAKE3 roots, memo attestations. Nothing anchored, no portable credential format. Evidence: `docs/design/CYNIC_ATTENTION_SOVEREIGNTY_STRATEGY.md`, `crates/cynic-solana-pay/src/ops.rs`. | Orthogonal | Anchor compressed roots, portable consent credentials, named privacy mechanism, offline signing via durable nonces, a real E-Score registry. |

## GASdf, first all-Solana vertical

GASdf is a mycelium branch, not trunk. Gas-as-a-service plus privacy (Aura).
Survival rule: profitable alone or dead, never subsidized.

Mechanism: a SOL fee payer covers users with no SOL. Billed in USDC.
Costs: RPC, data APIs, indexer. Strict waterfall: reimburse the payer, pay
infrastructure, tax reserve, LP and operator margin, risk reserve,
E-Score-multiplied contribution rewards, TALARIA IP fee, surplus into
buyback and burn, liquidity, grants. Never sells yield. Pays whoever makes
gasless possible.

Market: 82M US Galaxy devices with native USDC and no SOL for fees, late
October 2026. GASdf is the missing onboarding link. Go to market through
Futard.io, demand measured in real buckets.

CYNIC loop: each sponsored fee is attributed evidence, E-Score multiplies,
settlement in USDC and SOL on-chain. First vertical crossing membrane,
proof, and settlement.

Consolidates eight scattered documents (seven in `docs/archive/`, one design
mention). On formalization day a single canonical document survives and the
rest become pointers.

## Pending decisions

1. Burn path: Token program against ASDF or new Token-2022 mint.
2. Mint economics: supply, authorities, renunciation, decimals policy.
3. Custody per key class: fee payer, node authority, vault, user.
4. RPC provider and commitment level for proofs versus payments.
5. Real PDA derivation replacing the SHA-256 stub.
6. Anchor cadence, Merkle leaves, epoch definition, account close policy.
7. Market resolution: single oracle or staked resolution.
8. Obsolete prose: Livre IV slots (400 ms), foundation scope (study object).

## Verification log

- solana.com/news hub and Samsung, slots, x AI, DvP, Open USD articles: read Oct 12 2026.
- solana.com/docs/core/pda and solana.com/docs/tokens: read Oct 12 2026.
- colosseum.com/worldsfair and /worldsfair/resources: read Oct 12 2026.
- ASDF mint page on explorer.solana.com: read Oct 12 2026.
- CYNIC evidence paths above: read from `origin/main` in this worktree.
- Dimension 10 Solana column and parts of dimensions 2, 6, 8, 9: UNVERIFIED.
