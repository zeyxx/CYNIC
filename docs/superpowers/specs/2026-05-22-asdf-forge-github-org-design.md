# asdf-forge — GitHub Organization Design

**Date:** 2026-05-22
**Author:** T. + Claude
**Status:** DRAFT — awaiting T. review

---

## 1. What

A shared GitHub organization (`asdf-forge`) hosting SDK packages, commons libraries, and shared infrastructure for the ASDF builder collective. Products stay with their authors.

## 2. Why

Cross-repo audit found **4 independent Helius clients** (~2700 LOC), **3 phi constant definitions**, **3 retry/backoff implementations**, and **15 scattered config loaders** across 3 developers' repos (zeyxx, sollama58, ragnar-no-sleep). 25+ repos, ~32 independent reimplementations of the same patterns.

**Cost of status quo:** Bug fixes don't propagate. API changes (Helius, Jupiter) require patching each repo independently. New contributors copy-paste from whichever repo they find first.

## 3. Scope

### In scope (SDK-first, organic growth)

| Layer | What | When |
|-------|------|------|
| **SDK packages** | Solana clients, constants, scoring, retry, config | First — this IS the MVP |
| **Infra dev** | CI templates, shared lint configs, deploy scripts | When 2+ repos use the same CI pattern |
| **Infra runtime** | Shared RPC nodes, caching services | When load justifies shared infra |
| **Infra economic** | Burn mechanics, fee routing, treasury logic | When tokenomics tooling is needed by 2+ products |

### Out of scope

- **Products** — CultScreener, HolDex, KAIROS, CYNIC, blitz-and-chill stay with their authors
- **Governance tokens / DAOs** — no on-chain governance for the org
- **CYNIC kernel internals** — CYNIC consumes the SDK like any other project, no coupling

### Relationship to $ASDFASDFA

The token is the lab — the community that builds and burns. The org serves the builders. The connection is cultural (README, manifesto), not structural (no token-gating, no on-chain deps in the SDK).

### Relationship to CYNIC

CYNIC is one consumer among many. `@asdf-forge/scoring` may contain phi-bound and K-Score algorithms extracted from CYNIC, but CYNIC has no privileged position in the org. When CYNIC becomes distributed/federated, asdf-forge could host the shared node infrastructure — but that's organic growth, not a design requirement today.

## 4. Governance

**Simple. No bureaucracy.**

- **3 org owners:** zeyxx, sollama58, ragnar-no-sleep
- **CODEOWNERS per repo** — original author is captain, PRs welcome, captain merges
- **Branch protection on main** — PR + 1 approval + CI green (CI = lint + type-check + tests pass)
- **CODEOWNERS in monorepo** — per-package paths (e.g., `/packages/constants/ @zeyxx`, `/packages/solana-clients/ @sollama58 @zeyxx`)
- **Decisions:** 3 devs talk, rough consensus. No formal votes except for:
  - Transferring a repo to "adopted" status
  - Changing the manifesto
  - Adding/removing org owners

### Repo categories

| Category | Who decides | Example |
|----------|------------|---------|
| **Shared** | Any owner, PR review | SDK packages, CI templates |
| **Hosted** | Original dev (captain) | Migrated utilities from personal repos |
| **Adopted** | Org consensus (3 devs) | Abandoned projects the community maintains |

Forks live outside the org. If a fork wins, it can replace the original.

## 5. SDK Architecture

### Monorepo vs multi-repo

**Monorepo** (recommended). Reasons:
- Atomic cross-package changes (bump constants + clients in one PR)
- Single CI pipeline
- Shared lint/test config
- 3 devs is small enough for trunk-based development

Structure:
```
asdf-forge/
├── sdk/                          # monorepo
│   ├── package.json              # workspaces root (npm/pnpm)
│   ├── pyproject.toml            # Python workspace root (uv)
│   ├── packages/
│   │   ├── constants/            # @asdf-forge/constants
│   │   │   ├── src/index.ts
│   │   │   ├── python/asdf_constants/
│   │   │   └── package.json
│   │   ├── solana-clients/       # @asdf-forge/solana-clients
│   │   │   ├── src/
│   │   │   │   ├── helius.ts
│   │   │   │   ├── jupiter.ts
│   │   │   │   └── rpc.ts
│   │   │   ├── python/asdf_clients/
│   │   │   └── package.json
│   │   ├── scoring/              # @asdf-forge/scoring (future — when 2+ repos consume)
│   │   │   ├── src/
│   │   │   │   ├── phi.ts
│   │   │   │   ├── kscore.ts
│   │   │   │   └── verdicts.ts
│   │   │   └── package.json
│   │   └── infra/                # @asdf-forge/infra
│   │       ├── src/
│   │       │   ├── retry.ts
│   │       │   ├── config.ts
│   │       │   └── cache.ts
│   │       └── package.json
│   └── README.md
├── .github/
│   ├── CODEOWNERS
│   └── workflows/
│       ├── ci.yml
│       └── publish.yml
├── MANIFESTO.md                  # Cultural roots, link to $ASDFASDFA
└── README.md
```

### Packages — priority order

Based on duplication audit (observed, not guessed):

| # | Package | Source material | Dedup impact | Lang |
|---|---------|----------------|-------------|------|
| 1 | `constants` | GASdf/constants.js + KAIROS/types.py + CYNIC/dog.rs | 3 repos, ~120 LOC | TS + Python |
| 2 | `solana-clients` | GASdf/helius.js + KAIROS/helius.py + CYNIC-python/wallet_behavior_helius.py | 4 impls, ~2700 LOC | TS + Python |
| 3 | `infra` | KAIROS/db_utils.py retry + GASdf/config.js + GASdf/redis.js | 3 repos, ~1400 LOC | TS + Python |
| — | `scoring` | CYNIC/dog.rs phi_bound + KAIROS/types.py verdicts | 2 repos, ~300 LOC | TS + Python |
| — | `crypto` | GASdf/fee-payer.js + GASdf/validator.js | 1 repo only | TS |

**`scoring` and `crypto` are deferred.** `scoring` needs a stable interface contract extracted from CYNIC first (doesn't exist yet). `crypto` deduplicates within GASdf only — single-repo dedup belongs in that repo, not in the commons. Both promote to SDK when a second consumer appears.

### Dual-language strategy

- **TypeScript first** — GASdf, CultScreener, blitz are JS/TS. Largest consumer base.
- **Python mirrors** — KAIROS, CYNIC-python are Python. Each TS package has a `python/` subdirectory with equivalent code. Published to PyPI as `asdf-forge-constants`, `asdf-forge-clients`, etc. Python packaging via standalone `pyproject.toml` per package (not uv workspaces — tooling isn't mature enough for monorepo Python).
- **Rust stays in CYNIC** — The kernel's Helius/RPC code is too coupled to port traits. CYNIC reads SDK artifacts (JSON constants) but doesn't depend on SDK as a crate.

## 6. Package Design — `@asdf-forge/constants` (first ship)

The simplest package. Validates the pipeline. If this ships, everything else follows.

```typescript
// @asdf-forge/constants

// Golden ratio
export const PHI = 1.618033988749895;
export const PHI_INV = 0.6180339887498949;    // φ⁻¹
export const PHI_INV_2 = 0.3819660112501051;  // φ⁻²
export const PHI_INV_3 = 0.2360679774997897;  // φ⁻³
export const PHI_INV_4 = 0.1458980337503155;  // φ⁻⁴

// Verdict thresholds
export const HOWL_MIN = 0.5278640450004947;   // φ⁻² + φ⁻⁴
export const WAG_MIN = PHI_INV_2;              // 0.382
export const GROWL_MIN = PHI_INV_3;            // 0.236
// BARK = anything ≤ GROWL_MIN

// Max epistemic confidence
export const MAX_CONFIDENCE = PHI_INV;         // 0.618

// $ASDFASDFA — real mint address (verified on-chain)
export const ASDF_MINT = "<REAL_MINT_ADDRESS>"; // TODO: replace with actual mint before v0.1.0
export const BURN_RATIO = 0.764;               // 1 - φ⁻²
export const TREASURY_RATIO = PHI_INV_2;       // 0.382
```

```python
# asdf_forge_constants/__init__.py
# Mirror of TypeScript constants — single source of truth
PHI: float = 1.618033988749895
PHI_INV: float = 0.6180339887498949
# ... (identical values)
```

## 7. Package Design — `@asdf-forge/solana-clients` (second ship, after constants validates the pipeline)

Best-of extraction from GASdf + KAIROS + CYNIC-python:

```typescript
// @asdf-forge/solana-clients

// Helius
export class HeliusClient {
  constructor(key: string, options?: { timeout?: number; retries?: number });
  getAsset(mint: string): Promise<DasAsset>;
  getAssetBatch(mints: string[]): Promise<DasAsset[]>;
  getTokenLargestAccounts(mint: string): Promise<TokenAccount[]>;
  getSignaturesForAddress(address: string, opts?: SignatureOpts): Promise<Signature[]>;
  getEnhancedTransactions(signatures: string[]): Promise<EnhancedTx[]>;
  batchIdentity(wallets: string[]): Promise<WalletIdentity[]>;
}

// Jupiter
export class JupiterClient {
  constructor(key?: string, options?: { timeout?: number });
  getQuote(params: QuoteParams): Promise<Quote>;
  getSwapTransaction(quote: Quote, userPubkey: string): Promise<SwapTx>;
  getPrice(mints: string[]): Promise<Record<string, PriceInfo>>;
}

// Shared RPC connection (singleton per endpoint)
export function getConnection(rpcUrl: string, commitment?: string): Connection;
```

Source mapping:
- `HeliusClient.getAsset` ← GASdf `helius.js` + KAIROS `helius.py` (merged)
- `HeliusClient.getTokenLargestAccounts` ← CYNIC `helius.rs` (most robust, with retry)
- `HeliusClient.batchIdentity` ← CYNIC `helius.rs`
- `JupiterClient.getQuote/getSwapTransaction` ← GASdf `jupiter.js`
- `JupiterClient.getPrice` ← KAIROS `jupiter.py`
- `getConnection` ← GASdf `helius.js` (singleton pattern, deduplicated)

## 8. Testing

- **TS:** vitest (fast, ESM-native, workspace-aware)
- **Python:** pytest + mypy --strict
- **CI green means:** lint (eslint/ruff) + type-check (tsc/mypy) + tests pass
- **Coverage target:** 80% per package (enforced in CI)
- **Integration tests:** real Helius/Jupiter API calls in `solana-clients` (gated behind `HELIUS_API_KEY` env var, skipped in CI if absent)

## 9. Publishing

- **npm:** `@asdf-forge/*` scoped packages (requires npm org `asdf-forge`)
- **PyPI:** `asdf-forge-*` (hyphens, PEP compliant)
- **Versioning:** SemVer. Start at `0.1.0`. Breaking changes = minor bump until 1.0.
- **Build tool:** tsup (TS → ESM + CJS bundles)
- **CI:** GitHub Actions. Test on push, publish on tag.
- **Secrets required:** `NPM_TOKEN` (org-scoped publish token) + `PYPI_API_TOKEN` — stored as GitHub org-level secrets. Setup on Day 1 before first publish.

## 10. Migration Plan

**Not a big bang.** Incremental adoption:

1. **Ship `constants`** — T. adopts in GASdf first (owner, no coordination needed). sollama58 + ragnar adopt at their pace.
2. **Ship `solana-clients`** — GASdf and KAIROS adopt first (JS/Python primary consumers)
3. **Ship `infra`** — retry, config, cache extracted
4. CYNIC-python adopts Python packages when ready (T. owns this, no external dependency).
5. Each repo migrates at its own pace. No deadline, no forced adoption.

## 11. What Falsifies This

| Claim | Falsification | Who checks, when |
|-------|--------------|------------------|
| SDK reduces maintenance | If after 3 months, <2 repos have deleted their local copies of deduplicated code | T. reviews at 3-month mark (2026-08-22) |
| Monorepo works for 3 devs | If merge conflicts exceed 2/week (check `gh pr list --state merged` for conflict mentions) | Any dev, ongoing |
| Dual-language is worth it | If <2 repos import the PyPI package in a requirements file after 2 months | T. reviews at 2-month mark (2026-07-22) |
| Governance works without bureaucracy | If a merge dispute takes >48h to resolve — add formal process | Any dev, on occurrence |
| Organic growth happens | If after 6 months the org has only `constants` — the forge is a shelf, reassess | T. reviews at 6-month mark (2026-11-22) |

## 12. Day 1 Checklist

### T. does (no external dependency)
- [ ] Create `asdf-forge` org on GitHub (T. = initial owner)
- [ ] Create npm org `asdf-forge` on npmjs.com
- [ ] Store `NPM_TOKEN` + `PYPI_API_TOKEN` as org-level GitHub secrets
- [ ] Create `sdk` monorepo with pnpm workspaces + tsup build
- [ ] Wire CI: `.github/workflows/ci.yml` (lint + type-check + test)
- [ ] Wire publish: `.github/workflows/publish.yml` (on tag → npm + PyPI)
- [ ] Ship `@asdf-forge/constants` v0.1.0 to npm + PyPI
- [ ] Write README.md (what the org is, what it isn't, how to contribute)
- [ ] Write MANIFESTO.md — consumer: new contributors reading it orient to the project's values and decide whether to participate. Not documentation — a filter.
- [ ] Setup branch protection on `main` + CODEOWNERS file
- [ ] GASdf adopts `@asdf-forge/constants` (T. owns GASdf, first consumer validates the pipeline)

### Coordination (async, no rush)
- [ ] Invite sollama58 + ragnar-no-sleep as org owners
- [ ] sollama58 adopts constants in CultScreener (at his pace)
- [ ] ragnar adopts constants in blitz / ASDF-Web (at his pace)

---

*"Complete at its current scale."*
