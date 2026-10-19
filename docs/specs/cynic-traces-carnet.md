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

### T5 — Guards ré-ancrés + non-vacuité en CI [ ]
- Nouveaux ancres post-déménagement ; `check_nonvacuity` sur le pipeline Rust.

### T6 — Retrait des normaliseurs Python, un par un [ ]
- Un `HITL` par harness (windows, codex, cursor, vscode, kiro, gemini,
  amp, qwen, droid, opencode). Jamais en lot.

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
