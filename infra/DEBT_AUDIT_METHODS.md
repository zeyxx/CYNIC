# Systèmes Souverains d'Audit — Algorithmes & Méthodes

Figé v54 — 2026-08-31 — 121 brutes / 119 effectives — debt-audit câblé `cynic dev debt-audit --json`

Tous les audits sont souverains : pas de LLM, pas de réseau, pas de SaaS, pas de guess. Local FS only, déterministe, rejouable, schémas versionnés `cynic.*.v1`.

## Principe commun (tout audit)

1. **traversal statique** : `rust_source::rust_files_under(workspace_root)` + `line_count` + `read_to_string` — pas de grep externe, pas d'LLM.
2. **table de loi** : constante `&[Spec]` qui est la loi (KERNEL_AREAS, OWNERSHIP_SURFACES, CORE_CRATES, TIER1_DEBTS) — la table est la SSOT, pas la prose.
3. **prédicats purs** : fonctions pures `contains_*(content, token)` / `matches_scope(path, scan_paths)` / `threshold` — testables sans I/O.
4. **couverture** : `HashSet<relative_path>` covered vs uncovered, `hotspot >=400` lignes, `entrypoint <=220/300`.
5. **schéma versionné** : `schema: "cynic.cli.*.v1"` / `"cynic.dev.*.v1"` — JSON stable, `status ok/blocked`, `blocked == 0 ? ok : blocked`.
6. **unknown is data** : si non observable → status observe + residual_unknowns, jamais 0/100% fabriqué.
7. **preuve file:line** : chaque finding porte `file_path + line_count` ou `proof: "grep X 0 hits"` — pas de prose sans preuve.

## Par système

### crate-audit (`cynic dev crate-audit --json`)

Algorithme :
- `CORE_CRATES: &[(&str, main_threshold, lib_threshold)]` (300/220)
- Pour chaque crate : `rust_files_under(crate_root)` → compte fichiers, `line_count(path)` par fichier.
- `entrypoints = [src/main.rs, src/lib.rs]` → `line_count <= threshold ? ok : blocked`
- `hotspots = filter line_count >=400` → status observe (jamais bloquant, mais visible)
- Checks : `architecture_boundary_test_present` (tests/architecture_boundary.rs exists), `rust_files_present`, `entrypoints_bounded`, `hotspots_observed`
- `blocked_count = crates où checks != ok` → `status = blocked_count==0 ? ok : blocked`

Méthode : seuils, comptage, existence fichier — O(n_files).

### kernel-audit (`cynic dev kernel-audit --json`)

Algorithme :
- `KERNEL_AREAS: &[KernelAreaSpec { name, paths, status: extract|stabilize, anti_patterns }]` — 13 aires, table est la loi.
- Indexe tous les `src/**/*.rs` → `IndexedRustFile { relative_path, line_count }`
- Pour chaque aire : `matched = files.filter(|f| paths.any(|p| matches_scope(f, p)))` → `covered.insert(path)` → `hotspots = matched.filter(line_count>=400)`
- `uncovered_hotspots = files.filter(!covered && line_count>=400)`
- `repeated_surface_findings` : table de doublons connus (fleet, inference, etc.)
- `blocked_areas = areas.any(status in extract|stabilize) || uncovered_hotspots non vide` → blocked

Méthode : table-driven coverage, HashSet, anti-patterns prose → file:line.

### ownership-audit (`cynic dev ownership-audit --json`)

Algorithme :
- `OWNERSHIP_SURFACES: &[Spec { surface, crate_owner, scan_paths, must_not_depend_on_project_root, migration_target, mode }]`
- Pour chaque surface : `scan_files_for_project_root_dependency(crate_root, scan_paths, "CYNIC_PROJECT_ROOT")` → `read_to_string` + `contains_project_root_dependency` (patterns `env::var("CYNIC_PROJECT_ROOT")` etc.)
- Prédicats : `owns_user_home = expected_owner_filesystem.any(starts_with "/home/")` → blocked si runtime_authority non dev_only ; `depends_on_project_root && must_not_depend` → blocked ; `migration_target.is_some()` → status migrate
- Summary : `surface_count, blocked_count, migration_count, project_root_dependency_count`

Méthode : scan token précis, pas de regex lourde, BTreeSet dedup.

### debt-audit (`cynic dev debt-audit --json`) — nouveau v54

Algorithme :
- Sources : `infra/DEBT_TAXONOMY.md` v54 figée (121 brutes, 119 effectives) + `infra/DEBT_FINDINGS_v01_v39_empirical.md` (preuves grep vivant).
- Table `TIER1_DEBTS: &[DebtFinding { id, family, status: avéré, blast: 1/N/∞, faux_observé, proof: file:line, score_hint }]`
- ROI : `score = (unlock + risque * blast_weight) / (effort + coût_résiduel)` où `blast 1=1, N=10, ∞=1000` (tri), unlock/risque/effort sont ordinaux 1-3 issus de la table.
- Tri : Tier1 trié par score décroissant → purge par ROI (Sec1/Res1/Conv1 ∞ d'abord, puis N effort faible, puis 1).
- Status : `status = hotspot_check ? blocked : ok` (dette = bounded = 0 → blocked)
- Schéma : `cynic.dev.debt_audit.v1` avec `iteration, iteration_status: figée, total_brutes/effectives, roi_formula, tier1[], partial_count, ok_count, methods[]`

Méthode : table de 18 Tier1 avérés + 30 partial + 3 ok, tri ROI, preuves file:line citables. Méta-dette Tax1/Disc1 incluse (taxonomie = dette si non figée).

### Autres audits souverains (même méthodes)

- **surface-audit / policy-formality / schema-registry / agent-exposure** : même pattern table + scan + couverture + schéma versionné.
- **data-centric-audit / organism-audit / fleet-divergence** : lecture ledger/registry JSON + prédicats sur présence/coverage/OOM → `refuted` vs `unobservable` (unknown non fault).
- **inference-matrix / inference-optimal** : sweep paramétrique exhaustif (context tiers, KV-quant, threads, topologies, harnesses) → ledger empirique → solveur `optimal` (balanced/latency/throughput) → export TOML. Méthode : énumération produit cartésien + mesure, pas d'heuristique.

## Garanties souveraines

- **no vendor lock** : FS + Rust + jq, pas de SaaS, pas de token réseau.
- **déterministe** : même FS → même JSON (pas de timestamp aléatoire).
- **content-addressed** : preuves = chemins relatifs workspace, pas absolus machine.
- **fail-closed** : `make check` = per-crate `cargo check` avec `RUST_MIN_STACK` borné, `--offline --locked` cache integrity d'abord.
- **residual** : chaque audit expose `unknowns` / `uncovered_hotspots` / `blocked_count` au lieu de 100% fabriqué.

Fichiers : `crates/cynic-cli/src/{crate_audit,kernel_boundary_audit,ownership_audit,debt_audit}.rs` + `crates/cynic-cli/src/dev_surface.rs` + `infra/DEBT_TAXONOMY.md` v54 figée + `infra/DEBT_FINDINGS_v01_v39_empirical.md`.

Usage :
```
cynic dev debt-audit --json
cynic dev debt-audit --json --persist --output /var/lib/cynic/reports/debt-audit.json
cynic dev crate-audit --json && cynic dev kernel-audit --json && cynic dev ownership-audit --json
```

Prochaine purge ROI : Sec1 (capability ∞) → Res1/Conv1 (∞) → C18 (∞) → H2/G1/B7/N (N effort faible) → Layer1 (∞ effort élevé) — voir `debt_audit.rs:TIER1_DEBTS`.
