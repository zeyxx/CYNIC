# Audit architecture + DAG + sécurité PR 133 — Verdict : APPROVE

Prisme : architecture / DAG / sécurité. Head `c06b759e`, base `63ffd4db`. Worktree détaché au SHA exact, lecture seule.

## 1. Cohérence foundation.json ↔ digest.rs — CONVERGENT

| Aspect | foundation.json (ligne 11) | digest.rs |
|---|---|---|
| Canonique | SHA-256, préfixe `sha256:` | `SHA2_PREFIX = "sha256:"` |
| Legacy | SHA3-512 `sha3-512:` parseable, non produit | `SHA3_PREFIX = "sha3-512:"` |
| Non-canonique | BLAKE3 permis caches | `BLAKE3_PREFIX = "blake3:"` |
| Non préfixé | Invalide | `parse_digest()` rejette |

`digest_json()` / `digest_bytes()` produisent `sha256:` + hex. `validate_digest(digest, allow_legacy)` implémente la règle exacte. Le commit `c06b759e` aligne le contrat sur le code T13 existant.

## 2. Consommateurs — CONVERGENT

12 fichiers consommateurs identifiés par `git grep` au SHA head. Tous utilisent `digest_json` ou `digest_bytes`. Aucun consommateur direct de `parse_digest`/`validate_digest` hors tests. Consommateurs identiques entre `main` et head — la PR ne les modifie pas.

## 3. DAG strict — CONVERGENT

Fichiers gasdf dans `crates/cynic-protocol/src/gasdf/` (Layer 0) : types purs, décisions pures, zéro I/O. Cohérent FCIS + DAG strict. Pas de couplage cross-family.

## 4. Surfaces CLI — CONVERGENT

Aucune nouvelle commande CLI avec trait d'union dans specs ou code. Les specs `CITIZEN_SURFACE_SPEC.md` et similaires sont des documents de design, pas des commandes.

## 5. Secrets — CONVERGENT

Aucun pattern (`sk-*`, `ghp_*`, `AKIA*`, `-----BEGIN`, `xox*`, `eyJ*`) dans le diff `origin/main..c06b759e`. Mentions "secret" uniquement conceptuelles.

## 6. Hosts/IP en dur — CONVERGENT

Aucune IP hardcodée dans le diff. URLs GitHub dans `infra/external-pins.json` = références de repo, pas des hosts codés.

## 7. Supply-chain — CONVERGENT

- `cynic-anchor` non-membre du workspace principal (vérifié `Cargo.toml`) ; `[lints] workspace = true` supprimé à raison + `[profile.release] overflow-checks = true`, cohérent upgrade Anchor 0.30 → 1.2+.
- `infra/external-pins.json` + `infra/external-project-registry.json` : SHAs explicites, drift gate (`external_registry_coherence.rs`).

## Inconnues explicites

- Preuve builder non relue directement (pas d'accès `10.10.10.119` depuis l'environnement reviewer ; preuve fournie dans le body : check workspace CONVERGED).
- `Cargo.lock` (+2340 lignes, upgrade Anchor) non audité ligne à ligne pour violations supply-chain.
