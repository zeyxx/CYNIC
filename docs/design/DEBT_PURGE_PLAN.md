# Plan de Correction par Vagues — Dette v54 Figée

Source: `infra/DEBT_TAXONOMY.md` v54 FIGÉE (121 brutes / 119 effectives) + `infra/DEBT_FINDINGS_v01_v39_empirical.md` + `cynic dev debt-audit --json`
ROI = (unlock+risque*blast)/(effort+coût_résiduel) — blast 1/N/∞ — purge Tier1 d'abord (18 avérés), puis partial 30.

Principe par vague: une vague = un sous-système qui converge (desired→observe→reconcile→bounded action→proof) + preuves file:line + gate `make check` / `cynic dev *-audit --json` + rollback intent. Pas de vague N+1 avant preuve vague N.

## Vague 0 — Figée & Audit vivant [FAIT 2026-08-31]

Objectif: socle mesurable.
- Figer v54, reconstituer empirique v1-v39, câbler `cynic dev debt-audit --json` (schéma cynic.dev.debt_audit.v1)
- Preuves: `infra/DEBT_TAXONOMY.md` FIGÉE, `debt_audit.rs` fmt ok, `DEBT_AUDIT_METHODS.md`
- Gate: `cargo fmt --check` PASS, `debt_audit` schema test PASS
- Unlock: tout ROI triable — débloque vagues 1-5

## Vague 1 — Blast ∞ effort faible — souveraineté & honnêteté (ROI max)

Dettes: **Sec1, Res1, C18, H2, G1** + **Unknown1/Res1, Truth1**
- Sec1 OneShotCapability 0 hits → capability bornée (capability issuance + monotonic budget)
- Res1/C18/Unknown1 target_percent 0 → residual_unknowns budgétés, metrics avec target_percent + null
- H2 71 modified + 0 claims → git isolation + claim avant mutation (branch agent/*, git status read-back)
- G1 .agent-claims.json vide → worktree governance (1 claim/scope, heartbeat TTL, merge queue)
- Truth1 registry pris pour health → read-back proof upward

Effort: faible (typer, ajouter champs, brancher). Blast: ∞ (toute capability, toute metric, tout agent).
Subsystem: **claims + metrics + git** — converge coordination.
Preuves: `cynic dev debt-audit --json` tier1 Sec1/Res1/C18 passent à partial/ok ; `cynic dev crate-audit --json` hotspots visibles ; `git status --short` == staged exact.
Gate: `cynic dev debt-audit --json | jq .tier1` Sec1/Res1 ok ; `cynic dev ownership-audit --json` ok
Rollback: revert champs capability/metrics, requeue claims.

## Vague 2 — Blast N effort faible — monolithe & vecteur

Dettes: **B7, GC1, Infra1, C13, C15, B8/Surf1**
- B7 18 fichiers >400 → split monolithes (health_signal 1468, source_worktree 1450) en modules <400
- GC1 doublon organ non GC → pattern-GC (trace→source, doublon supprimé)
- Infra1 SurrealHttpStorage single endpoint → diagnostic SPOF (tunnel/pool, probe)
- C13 vecteur 5R+humain 0 budget → budget tokens/op par ressource dans FleetReport
- C15 coût observation non mesuré → coût poll mesuré dans observer
- B8/Surf1 host 77 traits vs kernel 27 + KERNEL_AREAS stale → ownership registry + surface-registry à jour

Effort: faible-moyen (split, ajouter champs). Blast: N (tout monolithe, toute observation).
Subsystem: **host + kernel split** — converge runtime core.
Preuves: `cynic dev crate-audit --json` hotspots -5 ; `cynic dev kernel-audit --json` uncovered -3 ; `grep -rn OneShotCapability` >0
Gate: `make check` per-crate PASS (per-crate toolchain, RUST_MIN_STACK 256MiB) ; `cynic dev kernel-audit --json` blocked-- mais hotspots réduits
Rollback: revert splits, garder anciens fichiers en .bak

## Vague 3 — Blast N effort moyen — provenance & mémoire & fédération

Dettes: **C16, D19, D22, F1, Obs1, D21/SecretClass1**
- C16 doublon kernel organ 1044 vs inference backend_identity → extract inference_stack vers cynic-inference (ports)
- D19 mempool drop non compté → compteur amnésie + preuve
- D22 registry_path local → federation localité (Promoted/Takeover, budget monotonic)
- F1 0 registry par env → customer env registry par env (F1/Rem1)
- Obs1 8232 LOC sans bound → ingestion bornée (bound + backpressure)
- D21 disclosure Private non versionné → evidence-preserving disclosure (Private + consent)

Effort: moyen (extract, ports). Blast: N (toute provenance, tout fleet).
Subsystem: **inference + fleet + memory** — converge pipeline & federation.
Preuves: `grep -rn backend_identity` 1 owner ; `cynic dev ownership-audit --json` migration cynic-inference ok
Gate: `cynic dev ownership-audit --json` surface inference ok ; `cynic dev data-centric-audit --json` coverage up
Rollback: garder feature flag split, fallback kernel.

## Vague 4 — Blast ∞ effort élevé — membrane & convergence

Dettes: **Layer1, Conv1, Scale1, Mem1/Fed1, Boot1, Policy1/Adapt1**
- Layer1 kernel lit /proc direct → 4 layers ownées (physical/volume/usage/pressure) avec types cynic_protocol::<family> + probe + FleetReport
- Conv1 pas de preuve par échelle → convergence par sous-système (initial→transitoire→final + proof bundle)
- Scale1 scale clone modèle → fractal 1 host = first fleet case (data+adapters, pas nouveau modèle)
- Mem1/Fed1 membrane effondrée → sovereign membrane 8 plans (ports, pas controller global)
- Boot1 fresh node non standalone → standalone artifact bootstrap
- Policy1/Adapt1 policy non portable → policy portable, substrate injecté (port)

Effort: élevé (découper, ports). Blast: ∞ (organisme entier).
Subsystem: **organisme** — converge membrane souveraine.
Preuves: `cynic dev debt-audit --json` Conv1/Layer1 passent ; `cynic dev check --json` layers ok ; harness proof bundle
Gate: `cynic dev check --json` + `cynic dev debt-audit --json` tier1 0 blocked sur ces ids ; `make build` artifact standalone
Rollback: canary + rollback intent, staged rollout (A11/Loop1)

## Vague 5 — Partial 30 — file:line effort faible (fermeture)

Dettes: tous les `partial` v1-v54 (file:line ci-dessous) — chaque fix = 1 commit isolé, 1 preuve.
Exemples (DEBT_FINDINGS):
- `build_proof_compressor.rs:54 /home/user` → injecter layout
- `solana_ops.rs:246 fencing_token ok` → étendre à budget
- `health_signal.rs:305 observed_at u64` → Option + stale check
- `forensics.rs:11 v1 Unknown` → versionner
- `health_signal.rs:98 FleetReport sans observed_at` → ajouter
- `convergence/causal.rs:145 shared_origin` → tracer
- `agent_skill/metadata.rs:34 Private` → versionner
- `C17 Cargo.lock --locked` → vérif
- `harness 624 hits` → typer 5 dims
- `forensics host collection` → brancher
- `U1 proof bundle non exposé` → exposer
- `Ext1 .workspace/` → consenter
- `MANIFESTO/Proto` → transpiler
- `Build1 RUST_MIN_STACK` → doc 256MiB/8GiB
- `Ask1 askesis isolation` → membrane
- `S1 repo-remotes 10.10.10.101:3000` → bootstrap
- `P1/N1 pattern/network` → ports
- `Plat1 /home/user tests` → isoler
- `T1/E1 harness/eval` → versionner
- `Chat1 probe_live_model_ids 5` → router
- `Poh1 CAS 1` → retry loop
- `Fed1 Promoted 1` → membrane
- `Hook1/Art1 webhook/metadata` → layer
- `Coord1 envelope 1` → scheduler
- `Repair1/Design1 orphan/plans` → check
- `SensH1 watchdog 77` → lease

Effort: faible par fix, mais 30 = vague dédiée. Blast: 1 par fix, N cumulé.
Gate: `cynic dev debt-audit --json` partial → 0 ; `make check` green
Rollback: 1 revert par commit

## Ordonnancement & Vérif

Ordre ROI: 1 (∞ faible) → 2 (N faible) → 3 (N moyen) → 4 (∞ élevé) → 5 (partial)
Dépendances: 1 avant 2 (metrics/claims avant split) ; 2 avant 3 (ownership avant extract) ; 3 avant 4 (ports avant membrane)

Vérif globale après chaque vague:
```
cynic dev debt-audit --json | jq '.tier1[] | [.id,.status]'
cynic dev crate-audit --json | jq .blocked_count
cynic dev kernel-audit --json | jq .uncovered_hotspots
cynic dev ownership-audit --json | jq .summary.blocked_count
make check  # per-crate toolchain, RUST_MIN_STACK 256MiB / TEST 8589934592
```

Critère done: debt-audit `status ok` + `tier1_count 0 blocked` sur v54 — puis nouvelle itération empirique si épuisement non atteint (v55+ avec grep vivant).

Fichiers: ce plan = `infra/DEBT_PURGE_PLAN.md` (projection) + `.hermes/plans/purge-v54-waves.md` (exécution).
