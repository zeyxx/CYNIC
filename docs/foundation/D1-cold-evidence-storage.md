# D1 — Domicile à long terme des preuves froides CYNIC

Étude d'architecture storage · 2026-08-24 · statut: recommandation, non implémenté

Portée : PatternModels scellés (`cynic.pattern_gc.model.v1`), rapports digests,
lignées de skills — preuves **immuables, content-addressed, à vie**.

---

## 0. Faits du dépôt qui contraignent la décision

- `PatternModel` est déjà immuable et content-addressed (`model_id = sha256(report_digest+parent_id)`,
  `crates/cynic-protocol/src/pattern_gc/model.rs:100-120`). La valeur vit dans les
  octets ; le stockage ne fait que la porter.
- PoH existe en table SurrealDB (`storage/surreal/proof_of_history.rs`, INSERT append-only,
  hash chaîné + signature kernel).
- `infra/storage-audit.json` (mesuré ce jour) :
  - F2 : « SurrealDB mélange deux natures incompatibles (ops mutable vs preuve immuable)
    avec une seule story backup/rétention — THE structural finding ».
  - `kernel_surrealdb` : « unreachable from THIS host today » alors qu'il porte une
    chaîne PoH « rebuildable: NO ». Le serveur DB est donc un point de défaillance
    observé pour la lecture des propres preuves.
  - `pattern_models` : « MISSING PERSISTENCE », actuellement sur stdout/trace_miner
    sous /home/user (chemin non autoritaire selon AGENTS.md).
- Un journal JSONL append-only avec validation d'ordre à l'append existe déjà
  (`crates/cynic-kernel/src/infra/coordination_journal.rs` + `LocalEvidenceSpool`) :
  le format Option B a un précédent de code interne, pas un second moteur exotique.

## 1. Comparaison

| Critère | A : table SurrealDB SCHEMAFULL | B : journal JSONL signé Ed25519 |
|---|---|---|
| Intégrité/vérifiabilité | Hash chaîné en colonnes ; vérif = requête SQL. Mais la vérification dépend du serveur vivant. | Chaînage SHA3-512 + signature par ligne ; vérifiable offline (`jq`, script Rust, n'importe quel outil), y compris post-mortem. |
| Survie crash serveur DB | Nulle : DB inaccessible ⇒ preuves illisibles (état mesuré aujourd'hui). | Totale : fichier plat ; rsync/tar suffisent ; aucune reconstruction. |
| Réplication multi-nœuds | Dump SurrealDB + import, version-matched, transactionnel. | rsync/scp de fichiers signés = exactement la contrainte projet (« réplication = copie de fichiers signés »). Fusion multi-nœuds triviale : ordre total par (node_id, seq). |
| Requêtabilité | SQL riche — mais les vraies lectures de preuve froide sont ponctuelles : audit, reprise après incident, drift entre deux snapshots (API pure existante). | Scan séquentiel. À ~100-500 seals/jour (~10-50 Mo/an), un scan complet < quelques s ; index side-car reconstruisible à volonté car dérivé. |
| Coût ops | Dump/restore plus lourd, migrations, version matching ; un seul moteur à opérer (vrai). | Rotation + index side-car à construire ; second format à maintenir — mais le format est déjà écrit dans le code. |
| Croissance | Triviale à cette échelle (~50 Mo/an). | Triviale (~50 Mo/an). |

Verdict intermédiaire : l'avantage SQL de A sert surtout les requêtes chaudes et
dérivées ; la propriété critique des preuves froides — rester lisibles et
vérifiables quand tout le reste est mort — appartient à B.

## 2. Précédents industriels

1. **Solana/Agave** : les validators gardent le ledger récent localement puis
   *prunent* vers un stockage froid séparé (archive nodes, Bigtable) ; l'historique
   complet est un artefact tiers, pas une table du runtime chaud.
   https://www.alchemy.com/overviews/solana-archival-data ;
   https://docs.solanalabs.com/ar/operations/best-practices/general
2. **Firedancer** : blockstore local pour blocs récents, snapshot/accounts-db séparés ;
   RocksDB retiré de l'arbre core au profit d'outils autonomes — le runtime chaud ne
   dépend pas d'un moteur lourd pour ses données froides.
   https://docs.firedancer.io/guide/configuring.html ;
   https://github.com/firedancer-io/firedancer/pull/10557
3. **Certificate Transparency / RFC 9162** : un log = arbre de Merkle append-only ;
   l'intégrité se prouve par inclusion/consistency checks sur les entrées séquencées,
   pas par un SGBD. https://datatracker.ietf.org/doc/html/rfc9162
4. **Trillian** (implémentation CT) : personnalité *Log* = séquence append-only signée,
   séquencée, exposée en entrées numérotées. https://github.com/google/trillian
5. **QLDB** : *journal* = séquence de blocs hash-chaînés immuables (la source de vérité),
   et *tables* = vues matérialisées indexées **projetées depuis** le journal — jamais
   l'inverse. Exactement la forme hybride proposée ici.
   https://aws.amazon.com/blogs/aws/now-available-amazon-quantum-ledger-database-qldb/
6. **FoundationDB** : les TLogs sont un WAL séquentiel durables avant tout ; la lisibilité
   vient de couches de stockage séparées qui les consomment.
   https://apple.github.io/foundationdb/architecture.html

Convergence : la couche de vérité froide est partout un journal séquentiel signé/haché ;
les structures requêtables sont des projections dérivées et reconstructibles.

## 3. Risques de l'option recommandée (B)

- **Taille** : négligeable (~18-90 Mo/an à 100-500 seals/j). Non-ingérable à horizon
  décennal ; zstd si besoin.
- **Rotation** : découpage par segment (p. ex. `evidence-YYYYMM.journal`) avec
  continuation explicite du chaînage (`prev_hash` du segment n+1 = hash final du
  segment n). Jamais de truncate en place.
- **Index séparé nécessaire** : oui, mais side-car pur (`evidence.index.sqlite`,
  model_id → offset/segment) — reconstructible intégralement depuis le journal ;
  sa perte n'est jamais une perte de preuve. Pas de requête ad-hoc sans lui :
  accepter ce coût, c'est le prix de la souveraineté offline.
- **Résiduel** : horodatage monotone par nœud à garantir côté writer ; fusion
  multi-nœuds = concat triée par (node_id, seq) — à tester tôt.

## 4. Recommandation : **hybride « journal-first » (B primaire, projection A dérivée)**

1. `/var/lib/cynic/evidence/evidence-*.journal` — domicile **autoritaire** des preuves
   froides : JSONL, 1 ligne = 1 événement signé Ed25519 (clé kernel existante), haché
   chaîné (SHA3-512, même loi que PoH), append-only physique.
2. Projection requêtable **reconstructible** : table SurrealDB `evidence_projection`
   SCHEMAFULL remplie par replay du journal (idempotent par event_id). Elle sert les
   requêtes chaudes ; elle n'est jamais autorité ; sa corruption se répare par replay.
3. Ancrage PoH inchangé : émettre `pattern_gc.model.sealed` dans `proof_of_history`
   (option b du skill cynic-pattern-gc) pour lier journal ↔ chaîne kernel.
4. Backup : rsync du répertoire evidence (fichiers signés) — conforme à la règle
   « réplication = copie de fichiers signés entre nœuds » ; le dump DB redevient
   affaire d'état opérationnel uniquement (referme F2).

**Critère déclencheur pour basculer vers A pur** : si les lectures deviennent
fréquemment interactives/ad-hoc (jointures, agrégats interactifs > ~q/s ou latence
< ~1 s exigée hors batch) ET que la projection-replay ne suffit plus à les servir.
À l'inverse, si le serveur DB doit rester requis pour lire des preuves, c'est le
signal que l'on a dérivé de B — revenir au journal comme autorité.

## 5. Preuves attendues à l'implémentation

- `cynic evidence verify --json` : rejoue le chaînage + signatures, read-back offline.
- Test propriété : même entrée ⇒ même model_id ; corruption d'un octet ⇒ échec verify.
- Drill : DB down ⇒ lecture + vérification du journal réussissent sans réseau.
