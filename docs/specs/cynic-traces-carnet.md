# Carnet de tranches — `cynic-traces` (rewrite Rust de `apps/cynic-data`)

## Statut du document

Ce document n'est PAS un cahier des charges. Un CDC fige un contrat ;
ce carnet est un journal vivant : chaque tranche y est cochée avec sa
preuve, chaque émergence (correction, leçon, dérive) y est AJOUTÉE en
append-only. On ne réécrit jamais une entrée — on la surmonte par une
nouvelle. Historique complet = traçabilité du chantier.

Conventions : `[x]` prouvé (lien preuve), `[>]` en cours, `[ ]` à venir.
`HITL` = feu vert opérateur requis avant de cocher.

## Tranches

### T1 — Entités canonales (port strict) [x]
- Preuves : PR #153 mergée ; builder check workspace CONVERGED ; parité
  machine 6/6 structs, 57/57 champs identiques au Python.
- Gel adopté : `hermes_union.sqlite` (intégrité ok, comptes conformes).
- Re-gel 2026-10-19 : `hermes_union_20261019.sqlite` (3149 sessions /
  418516 messages, même méthode, dérive 365 sessions absorbée ;
  flags `active/compacted` mutables actés primary-wins, BLOBs préservés,
  FTS 8-chars répliqué + consigné) — nouveau défaut des preuves.

### T2 — Noyau, conservation, ports [x]
- Preuves : PR #154 ; builder CONVERGED (`88715635`) ; 8/8 tests verts.
- Zéro stub : le digest sha256 a été implémenté (pas de `panic!`).

### T3 — Adaptateur Hermes (le vertical) [x]
- Preuves : builder CONVERGED ; parité Rust==Python prouvée par
  machine : 7 sessions JSON-identiques (plain, codex, compaction,
  corrections) + stats 200/200 sessions.
- Contrat FIELD_FATE transcrit par machine depuis le Python (zéro recopie).
- **E5 (T3)** : float repr — les flottants se comparent en epsilon, les
  entiers en exact.
- **E6 (T4)** : 1 167 insertions d'un coup, c'est trop pour une tranche —
  les preuves portent la revue mais le volume la ralentit. Tranches
  futures < 500 lignes de diff, quitte à multiplier les PR.

### T4 — Magasin canonique + runner conservation [x]
- Preuves : builder CONVERGED ; `tests/conservation_proof.rs` vert :
  chargement complet du gel (2 795 sessions, 0 échec), comptes
  source==canonique, clôture α1–α4 vide sur le DDL vivant, round-trip
  store->entités sur échantillons, comparaison au vieux Silver.
- `apps/cynic-data/verify_conservation.py` retraité (remplacé par la
  preuve Rust).
- Revue pré-merge (7 prismes) : `source_uids` malformé lève désormais
  (fail-closed), preuve hermétique (snapshot DDL gelé + détecteur de
  dérive), générateur de contrat versionné et rejouable, chemins
  surchargeables par env, `write()` transactionnel, frontière
  documentée dans `lib.rs`.

### T5 — Guards ré-ancrés + non-vacuité en CI [>]
- Harnais généralisé (`apps/cynic-data/check_nonvacuity.py`) : une table,
  deux jambes. Python ré-ancré (fallback `infra/data_system/src/` pour
  les SHAs d'avant le déménagement), Rust : premier FIX, pas genèse (E8).
- `make audit.nonvacuity` ajouté ; le run refuse un arbre sale, restaure
  en finally, traite la suppression des fichiers-genèse.
- Preuves : 44a109c0 non-vacuous (parité échoue sans adapters),
  ef91f1af non-vacuous (preuve ne compile pas sans store),
  f3767cff known-vacuous (E7 : durcissement transactionnel sans
  fault-injection — revue seule).
- Python (4 gardes, re-ancrées fallback pre-move) : 9f76aa88, eb3c43b6
  (5 failed), 311058ac (4 failed), 7c64d2e2 (7 failed) — toutes
  non-vacuous. Full run : OK 6 + 1 known-vacuous.

### T6 — Retrait des normaliseurs Python, un par un [>]
- Un `HITL` par harness (windows, codex, cursor, vscode, kiro, gemini,
  amp, qwen, droid, opencode). Jamais en lot.
- `windows_gemini` (premier, HITL opérateur) : normaliseur 598 lignes +
  capture + runner + 3 tests supprimés ; registre UNCONTRACTED nettoyé ;
  capsule savoir `docs/specs/harness-windows-gemini.md` (RAG-ingérable,
  FIELD_FATE complet, 4 racines sources, réparations `_load_chat`).
  Données intactes (34 sessions silver + 69k fichiers vault). Perte
  assumée : ingestion restante impossible sans vertical Rust (T7).
- `codex` (deuxième, HITL opérateur) : normaliseur 332 lignes (sans
  FIELD_FATE ; 5 tests contractuels supprimés) + capture + runner
  supprimés ; registre UNCONTRACTED + doc `ports.py` nettoyés ; capsule
  `docs/specs/harness-codex.md` (5 types de lignes, nommage natif,
  écart de provenance 193 sessions / 94 fichiers gelés consigné).
  Données intactes (193 sessions silver + 94+95 fichiers).
- `cursor, vscode, kiro, amp, qwen, droid, opencode` : SANS OBJET —
  prouvé 2026-10-19 : zéro session silver, zéro normaliseur/capture/runner/test,
  seules mentions = bruit (modèle `Qwen3.6` dans hermes_contract, variable
  `cursor` sqlite, docs). Rien à retirer, rien à capsuler.
- Restent : `antigravity` (500, vertical partiel).
- `claude_code` (quatrième, HITL opérateur) : normaliseur 333 lignes
  (sans FIELD_FATE ; 5 tests contractuels + sweep supprimés avec
  `test_normalizers_contract.py`) + capture + runner supprimés ; registre
  UNCONTRACTED + doc `ports.py` nettoyés ; capsule
  `docs/specs/harness-claude-code.md`. Données intactes (106 sessions
  silver + 349 fichiers gelés). Perte assumée : live ACTIF
  (`~/.claude/projects`, 107 fichiers et ça grandit) — ingestion
  stoppée jusqu'au vertical Rust (T7), re-dérivable à tout moment.
- `omp` (troisième, HITL opérateur) : normaliseur 867 lignes (FIELD_FATE
  129 lignes, famille NATIVE-pointer : REPLY/TOOL_CALL natifs, edges
  COMPACTION/MODEL_CHANGE) + capture + runner + 3 tests supprimés ;
  registre UNCONTRACTED nettoyé ; capsule `docs/specs/harness-omp.md`.
  Données intactes (88 sessions silver + 1341 fichiers gelés, miroir
  du live). Perte assumée : ingestion restante sans vertical Rust (T7).

### T7 — Modèle graphe + projection géodésique [ ]
- Gated derrière T1–T6 (loi de Gall). Implémente R1–R6 du delta géodésique
  (`docs/specs/geodesic-retrieval-delta.md`).

## Journal des émergences (append-only)

- **E1 (T2)** : `panic!` intermédiaire dans `content_hash` refusé avant
  commit — remplacé par vraie implémentation sha2. Leçon : un stub nommé
  "temporaire" reste un stub.
- **E2 (T1)** : ligne `pub mod session;` parasite apparue dans `domain.rs`
  (artefact d'écriture). Leçon : le compilateur est le premier reviewer,
  erreur lue jusqu'au bout avant de toucher.
- **E3 (T1+T2)** : builder `--locked` bloqué deux fois (nouveau membre,
  nouvelles deps). Leçon : régénérer le lockfile AVANT le trigger builder.
- **E4 (ouvert)** : `verify_conservation.py` pointe un CANON absent —
  résolu par T4, pas avant.
- **E7 (T5)** : un durcissement sans différence observable sur le chemin
  de succès (write transactionnel) n'est pas gardable sans injection de
  faute — documenté known-vacuous, pas caché.
- **E8 (T5)** : un commit-genèse n'est pas un garde. La non-vacuité
  commence au premier FIX : prouvé en tentant T2 (0 tests lib restants,
  boundary passif) puis en retirant la ligne.
