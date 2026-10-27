# GASdf — Single Source of Truth

Status: draft v0.1, 2026-10-26. Observed facts only. Unknowns are marked `UNKNOWN`.

## 1. Canonical source

- Upstream repo: `github.com/zeyxx/GASdf` (GitHub is the public projection).
- Canonical clone path (declared in `infra/external-project-registry.json`):
  `/var/lib/cynic/source/repositories/external/gasdf`
- Its `origin` currently points to a local working copy, not to GitHub.
  This inverts the expected chain and must be corrected (step 3 of the plan).
- Tip of the canonical clone on 2026-10-26: `6538b4d`, present on `main` through PR #2
  (merge `5b7e3b7`).

## 2. Copies that exist (not canonical)

| Location | Versioned | Role | Action |
|---|---|---|---|
| `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/GASdf` | yes (git) | working copy | keep as dev clone, reset to `origin/main` |
| `/home/user/Documents/organs/gasdf` | no (no `.git`) | unknown | `UNKNOWN` content; classify before any deletion |
| `CYNIC/services/relay-gateway` | yes (in CYNIC) | Node relay, old path | retire after migration to Kora (step 3) |
| `/var/lib/cynic/projects/kora/relay-gateway` | `UNKNOWN` | runs in production (`gasdf-facilitator.service`) | `UNKNOWN` SHA; measure (step 2) |

## 3. Upstream dependency

- GASdf backend is Kora (solana-foundation). Decision recorded in GASdf
  `docs/GASDF_REBUILD_PLAN.md`: Kora signs and relays; GASdf keeps only what Kora cannot do.
- Kora is pinned as a binary tag plus policy file `kora.toml`.
- Kora tag currently pinned: `UNKNOWN` (to read from GASdf `docs/` and from the deployed unit).

## 4. Running services (declared in `infra/os/systemd/`)

- `gasdf-relay.service` — Node relay, port 3000.
- `gasdf-facilitator.service` — x402 facilitator, port 4021, Node, `/var/lib/cynic/projects/kora/relay-gateway`.
- `gasdf-kora-mainnet.service` — Kora signer, port 14001, `/var/lib/cynic/projects/kora`.

## 5. CYNIC coupling (observed)

- `infra/external-pins.json`: GASdf pinned to `86d917f` (June 2026) on `main`.
  A pin update to `6538b4d` exists on branch `agent/hermes/gasdf-pin-refresh`, not merged.
- `crates/cynic-protocol/src/gasdf/`: pure types and decisions, exported by `lib.rs` and `exports.rs`.
- `crates/cynic-knowledge`, `crates/cynic-proxmox`: string references to GASdf.
- 47 tracked files in CYNIC mention GASdf.

## 6. Rules proposed

1. One canonical source per external project, declared in the registry.
2. Consumers pin a SHA that is reachable from the canonical upstream (GitHub).
3. CYNIC reads GASdf surfaces and proofs; it does not run or copy GASdf code.
4. Kora is an upstream pin of GASdf, not of CYNIC.
5. No secret value is stored in this document. Paths to secrets are allowed.

## 7. Open questions

- Which SHA runs in production (`UNKNOWN`, step 2).
- Content of `/home/user/Documents/organs/gasdf` (`UNKNOWN`).
- Whether `cynic-protocol/src/gasdf/` should become a port (step 4).
