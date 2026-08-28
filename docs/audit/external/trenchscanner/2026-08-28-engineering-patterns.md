# Audit : Qualité d'ingénierie TrenchScanner → cartographie CYNIC

**Date** : 2026-08-28
**Scope** : Étude de la qualité d'ingénierie de TrenchScanner (sollama58) et transposabilité vers CYCIC
**Méthode** : Lecture du code source réel (pas du README) + build/test effectifs des deux repos + cartographie `search_files` des surfaces CYNIC existantes
**Repos audités** :
- `github.com/sollama58/TrenchScanner` (backend : `packages/core` TS, `apps/api`, `apps/worker` ; dashboard dans `github.com/sollama58/CultScreener`, dossier `trenches/`)
- `github.com/sollama58/CultScreener/trenches` (React 19 + Vite, ~7.5k LOC)

**Preuves d'exécution (reproducibles)**
- TrenchScanner `packages/core` : `tsc --noEmit` vert (0 erreur) ; `vitest run` → **236 passés / 8 skipped**
- TrenchScanner `apps/api` + `apps/worker` : `tsc --noEmit` vert après build de `packages/core` (0 erreur)
- CultScreener `trenches` : `tsc --noEmit` vert (0 erreur) ; `vite build` vert (bundle 112.59 kB / gzip 33.54 kB)
- Bug de repro rencontré : `package-lock.json` fourni fait crasher `npm install` (SIGSEGV, exit 139) sur les deux repos ; contourné en supprimant le lockfile. Candidat PR pour @gcrtrd.

---

## Synthèse

TrenchScanner est un backend de production **déjà mature**, pas un starter. La qualité d'ingénierie est remarquable pour ce type de projet : chaque module commente le *pourquoi* (fail-closed, idempotence, sémantique de cache, races de concurrence documentées et résolues). Sur les 9 patterns d'ingénierie identifiés, **7 sont déjà couverts par CYNIC avec une rigueur supérieure** (preuve SHA256, dimensions de trust, sémaphores, détection de contamination train/eval). La valeur de transposition n'est donc pas d'importer du code, mais d'isoler les **2 écarts réels** où CYNIC n'a pas d'équivalent générique.

---

## 1. Patterns d'ingénierie identifiés chez sollama58

| # | Pattern | Fichier (TrenchScanner) | Pourquoi c'est bon |
|---|---|---|---|
| 1 | **Fail-closed + unknown-as-data** | `packages/core/src/scoring/rugScreen.ts`, `scoring/matchFilters.ts` | `unknown` jamais traité comme « passe ». Split documenté : MIN (fail-closed) vs MAX (skip) selon la direction de la contrainte |
| 2 | **Idempotence par contrainte UNIQUE DB + insert-first** | `packages/core/src/subscription/access.ts` (burn credit) | `signature` UNIQUE ; insert puis catch P2002 plutôt que check-then-act → aucune race entre claim et reconciler |
| 3 | **Cache à sémantique réfléchie** | `packages/core/prisma/schema.prisma` | 4 caches (permanent / TTL / immutable / décroissant) avec règle par cache, jamais confondus |
| 4 | **RPC resilience sans dépendance** | `packages/core/src/subscription/solanaRpc.ts` | batch reject → latch → séquentiel ; `finalized` commitment ; match par `id` pas par position |
| 5 | **Scheduler anti-overlap + heartbeat littéral** | `apps/worker/src/scheduler.ts`, `packages/core/src/heartbeat.ts` | `running` guard ; union de job names typée ; `lastRunAt` toujours avancé même en échec |
| 6 | **Env zod cross-validée au démarrage** | `packages/core/src/config/env.ts` | `.refine` : `HOLDER_GROWTH_WINDOW > RUGCHECK_CACHE_TTL` sinon growth lu à 0% silencieusement |
| 7 | **SSE nudge + fallback poll obligatoire** | `packages/core/src/notify.ts`, `apps/api/src/matchStream.ts` | NOTIFY only-id, refetch client, health du `LISTEN` exposé (`/health/stream`) |
| 8 | **ML walk-forward + promotion** | `packages/core/src/curation/trainer.ts` | split temporel (pas random), shadow bench, recency decay, seuil calibré sur le pace |
| 9 | **Fan-out concurrence bornée générique** | `packages/core/src/concurrency.ts` | `forEachWithConcurrency` partagé (29 lignes), importé par 10+ callers |

---

## 2. Ce que CYNIC a DÉJÀ (donc PAS à transposer — vérified)

Chaque pattern ci-dessus a un équivalent CYNIC, souvent plus rigoureux. Preuves par `search_files` :

| Pattern TrenchScanner | Équivalent CYNIC | Preuve |
|---|---|---|
| 1. Fail-closed / unknown-as-data | `convergence/coherence.rs` : `assess_coherence_vector` downgrade honnête `Complete sans read_back → Unknown` ; `CoherenceDecision::{Unknown, Blocked}` | `crates/cynic-protocol/src/convergence/coherence.rs:132` + tests `converged_without_read_back_is_honestly_unknown` |
| 2. Idempotence | epoch binding idempotent + dimension `replay_safety` | `crates/cynic-protocol/src/convergence/index.rs:65` (« is idempotent; rebinding a coordinate is a conflict ») |
| 4. RPC resilience / circuit | `ResilientArgvRunner` (Open/HalfOpen/Closed, fail-closed, proof injecté) | `crates/cynic-host/src/resilient_runner.rs` (385 lignes, 3 tests) |
| 6. Validation cross-cutting | `validate_*(...) -> Vec<String>` partout | `storage_controller.rs:307`, `node_enrollment.rs:36`, `evidence_journal.rs:88`, `fleet_heartbeat.rs:79` |
| 7. Push temps-réel | SSE avec `sse_semaphore` borné + regression F23 (503 à quota) | `crates/cynic-kernel/tests/rest_routes.rs:874` (`events_rejects_when_sse_semaphore_exhausted`) |
| 8. ML hygiene / walk-forward | `scan_for_contamination` (train/eval overlap) | `crates/cynic-protocol/src/eval_hygiene.rs:64` |
| 5. Scheduler/heartbeat/tick | `fleet_heartbeat`, `emergence_runtime::tick`, `WindowRunner` | `crates/cynic-host/src/fleet_heartbeat.rs`, `emergence_runtime.rs:53`, `adapters/window_runner.rs` |

**Conclusion** : sur 9 patterns, 7 couverts par CYNIC avec rigueur supérieure. Ce n'est pas un défaut de TrenchScanner — CYNIC a été construit *pour* ces propriétés (preuve SHA256, trust dimensions, sémaphores, contamination train/eval).

---

## 3. Les 2 ÉCARTS RÉELS (transposition à faible risque)

### Écart A — Pas d'utilitaire de fan-out concurrence bornée générique

CYNIC a des `Semaphore` partout (SSE, slots) et du `spawn_blocking`, mais **aucun `forEachWithConcurrency` partagé** : chaque `reconcile_*` qui itère sur N sujets réimplémente sa boucle. C'est l'invariant #6 (« une source de décision par préoccupation ») violé pour *l'itération concurrente*.

- **TrenchScanner** : `packages/core/src/concurrency.ts` — 29 lignes, importé par 10+ callers.
- **CYNIC** : chaque `reconcile_*` réimplémente le pattern (cf. `cynic-kernel/src/api/rest/agent_workloads.rs`, boucles `for i in 0..N` + spawn ad hoc).
- **Transposition proposée** : `crates/cynic-protocol/src/concurrency.rs` avec `for_each_concurrent(items, limit, f)` (sync + async), testé, réutilisé par les `reconcile_*`.
- **Risque** : faible (utilitaire pur, pas de mutation de contrat).

### Écart B — Pas de cross-validation de config au démarrage

CYNIC valide des *records* (`validate_*`) mais pas des *contraintes entre champs d'un même env* (ex : « window > TTL sinon mesure silencieuse à 0% »). `config_loader.rs` lit env/chemin sans `.refine` inter-champs. Le bug de sollama58 (growth=0% invisible) est exactement le type d'erreur que CYNIC laisse passer au runtime.

- **TrenchScanner** : `packages/core/src/config/env.ts:260` — `validatedEnvSchema.refine(...)` refuse le démarrage si `HOLDER_GROWTH_WINDOW_MINUTES <= RUGCHECK_CACHE_TTL_MINUTES`.
- **CYNIC** : `crates/cynic-inference/src/config_loader.rs` — lit env/chemin, pas de validation croisée inter-champs.
- **Transposition proposée** : étape `validate_config_invariants(&cfg) -> Vec<String>` appelée dans `load_inference_config` (et les autres loaders) ; refuse le démarrage sur config incohérente.
- **Risque** : faible (ajout, pas rupture).

---

## 4. Ce que je NE recommande PAS de transposer

- **Burn gate / RPC Solana on-chain** : CYNIC a déjà `crates/cynic-kernel/src/backends/helius/` et sa propre gestion de preuve on-chain. Importer la logique de sollama58 = fork redondant + risque argent sur code déjà souverain.
- **Schéma Prisma / scoring memecoin** : hors périmètre CYNIC.
- **Toute PR pressée** : coordonner avec @gcrtrd (Twitter) pour TrenchScanner ; pour CYNIC, ajouts locaux testés seulement, pas de commit/push sans validation.

---

## 5. Recommandation

1. Documenter ces 9 patterns + cartographie (ce fichier) — utile pour relecture et discussion avec @gcrtrd.
2. Implémenter **Écart A** (utilitaire concurrence) en local, testé, sur branche `agent/hermes/*`, *sans* commit/push, pour relecture.
3. Écart B en option après A.

**État** : audit d'étude seulement. Aucun fichier CYNIC modifié, aucun fork/remote touché. Tout le code TrenchScanner/CultScreener reste en local `/tmp`.
