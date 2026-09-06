# Audit du ledger, des journaux et des chaînes existants

Date : 2026-09-03
Statut : audit read-only, non normatif
Révision observée : `0c4d2f8b`

## 1. Verdict

CYNIC possède déjà plusieurs mécanismes de journalisation, de preuve et de hash-chain. Aucun ne constitue seul le ledger constitutionnel répliqué du futur `CynicPeer`.

La situation actuelle est :

`plusieurs journaux spécialisés + plusieurs chaînes locales + diffusion P2P d’événements`

et non :

`transactions → état canonique → réplication → consensus → finalité`.

Le premier travail est donc une cartographie/qualification et non la création immédiate d’un nouveau crate blockchain.

## 2. Inventaire observé

### 2.1 Cold Evidence Journal — meilleur candidat de base de preuve

- contrat : `crates/cynic-protocol/src/evidence_journal.rs:1-149` ;
- adapter : `crates/cynic-kernel/src/infra/evidence_journal_file.rs:1-162`.

Propriétés :

- JSONL append-only ;
- séquence monotone par nœud ;
- `prev_hash` ;
- `entry_hash` calculé sur payload, prédécesseur, séquence et timestamp ;
- validation structurelle ;
- validation offline de la chaîne ;
- rotation par segments avec continuité ;
- corruption signalée.

Verdict : preuve locale de faits froids, très proche du besoin de journal de provenance. Ce n’est pas encore le ledger d’état : `event_type` reste une chaîne libre, aucun état métier n’est reconstruit, aucune transaction économique n’est validée, aucune réplication/consensus n’est définie.

Écart : le commentaire annonce une signature kernel, mais `EvidenceJournalEntry` ne porte pas de signature. La signature existe dans un autre chemin Surreal PoH, pas dans ce format de journal file.

### 2.2 Coordination Journal — WAL de claims

- contrat : `crates/cynic-protocol/src/coordination_journal.rs:1-80` ;
- adapter : `crates/cynic-kernel/src/infra/coordination_journal.rs:1-165`.

Propriétés :

- phases `Prepared`, `Committed`, `Aborted` ;
- séquence, term, fencing token ;
- preuve requise pour `Committed` ;
- verrou intra-process et lock de processus ;
- validation d’ordre ;
- replay/drain via spool.

Verdict : bon journal de récupération pour coordination. Il n’est pas hash-chainé et n’est pas le ledger global. La source d’autorité reste le `CoordPort`; le journal est explicitement une membrane de recovery.

### 2.3 Window Journal — réservation de ressources

- contrat : `crates/cynic-protocol/src/window_journal.rs:1-158` ;
- adapter : `crates/cynic-host/src/adapters/fs_window_journal.rs:1-158`.

Propriétés :

- `open` = réservation ;
- `close` = consommation ;
- replay pur ;
- fenêtres ouvertes conservées comme réservations ;
- close orphelin et doublons signalés ;
- cohérence `no violations + no open`.

Verdict : excellent ledger spécialisé de fenêtres de capacité. Il ne possède pas encore de hash chain, de signature, de relation à un mandat complet ou de transition d’état globale. Il doit être consommé comme preuve/resource journal, pas promu directement en blockchain.

Écart connu : `WindowStatus::Completed` ne porte pas le verdict budget `Within/Exceeded/Unknown`; le journal conserve la consommation mais pas encore la décision complète de conformité.

### 2.4 Local Observation Journal — WAL de health signals

- port : `crates/cynic-node/src/ports/observation_journal_port.rs:1-16` ;
- adapter : `crates/cynic-node/src/adapters/local_observation_journal.rs:1-83`.

Propriétés :

- JSONL append-only ;
- écriture et lecture de `HealthSignal` ;
- rotation ;
- persistante locale.

Défaut : les lignes JSON invalides sont ignorées pendant la lecture (`local_observation_journal.rs:53-63`) sans être retournées comme violations. Ce WAL ne peut donc pas être une preuve canonique de chaîne.

Verdict : projection locale de signaux, à conserver comme adapter ou à faire évoluer vers le contrat cold-evidence. Ne pas le confondre avec un ledger.

### 2.5 Kernel `StateBlock` — ancienne chaîne d’état

- `crates/cynic-kernel/src/domain/state_log.rs:1-166`.

Propriétés :

- snapshot des Dogs, système, ressources et organes ;
- `seq`, `prev_hash`, `hash` ;
- détection de modification ;
- comparaison de tendances ;
- endpoint d’historique.

Limites :

- le commentaire dit explicitement « Not a blockchain (no distributed consensus) » ;
- hash SHA-256 alors que la loi interne canonique exige SHA3-512 préfixé ;
- timestamp mural ;
- snapshots globaux mélangés avec plusieurs domaines ;
- aucune transition de compte ou de ressource ;
- aucune réplication/finalité.

Verdict : transparency log historique du kernel, pas état canonique du gouvernement.

### 2.6 Proof of History SurrealDB — chaîne globale de preuve

- port : `crates/cynic-kernel/src/domain/storage/mod.rs:448-472` ;
- adapter : `crates/cynic-kernel/src/storage/surreal/proof_of_history.rs:52-109`.

Propriétés :

- récupère le dernier hash ;
- chaîne payload + prédécesseur ;
- signe le hash avec l’identité kernel ;
- persiste dans `proof_of_history` ;
- peut relire les entrées.

Limites :

- lecture du dernier hash puis insertion non atomique : race entre producteurs ;
- ordering par timestamp, pas par séquence/fencing ;
- champ de hash stocké sans préfixe algorithmique dans ce chemin ;
- le payload brut est persisté ;
- la chaîne globale mélange les producteurs et domaines ;
- l’implémentation mémoire ne persiste rien et retourne une liste vide (`storage/memory/proof_of_history.rs:1-15`).

Verdict : adapter de preuve historique legacy/partiel, pas le ledger P2P. Il doit être comparé au Cold Evidence Journal avant toute convergence ; les deux ne doivent pas devenir deux autorités.

### 2.7 Agent Execution Receipt — jointure de provenance

- `crates/cynic-protocol/src/agent_execution_provenance.rs:1-130`.

Propriétés :

- lie agent, session, nœud, délégation, workload, lease et preuve ;
- contient `previous_receipt_digest` ;
- impose des clés de jointure selon le type de receipt ;
- impose des digests formatés.

Verdict : excellent join record de provenance, mais pas une chaîne causale validée : le prédécesseur est vérifié comme format, pas comme hash calculé ni comme relation d’ordre. Il ne porte pas la transition de compte ou de ressource.

### 2.8 KernelEvent — diffusion éphémère

- `crates/cynic-kernel/src/domain/events.rs:1-101`.

Propriétés :

- événements de session, tâches, services, anomalies et crashes ;
- consommation SSE/MCP/WebSocket/dashboard ;
- diffusion en mémoire.

Verdict : bus de projection, pas journal durable. Un `KernelEvent` perdu pendant un redémarrage ne peut pas être considéré comme événement de ledger.

### 2.9 Fédération Observer

- `crates/cynic-observer/src/federation.rs:21-35,124-169`.

Propriétés :

- libp2p, mDNS, Kademlia, Gossipsub ;
- diffusion d’événements et de `RuntimeProof` ;
- identité PeerId.

Verdict : transport/fédération de données. Aucune preuve de réception finale, d’inclusion dans un bloc, de validation d’état ou de consensus.

## 3. Incohérence structurante

Les mêmes mots désignent plusieurs objets :

- `journal` : evidence, coordination, windows, health signals ;
- `chain` : StateBlock, PoH Surreal, EvidenceJournal ;
- `proof` : payload digest, entry hash, signature, execution receipt ;
- `event` : KernelEvent éphémère, observation, PoH, health signal ;
- `state` : snapshot kernel, coordination state, workload state, future ledger state.

Il n’existe pas encore de type commun séparant explicitement :

- fait observé ;
- événement admissible ;
- transaction ;
- transition d’état ;
- preuve ;
- checkpoint ;
- bloc ;
- projection.

## 4. Ce qui est déjà réutilisable

Ne pas créer un nouveau `cynic-common` ou un ledger parallèle avant cette décision.

Réutilisations candidates :

- `EvidenceJournalEntry` pour la provenance hash-chainée ;
- `WindowJournalRecord` pour les réservations et consommations ;
- `CoordinationJournalEntry` pour les phases de claim/fencing ;
- `AgentExecutionReceipt` pour les jointures d’attribution ;
- `EvidenceEnvelope` pour provenance et lineage ;
- `StateBlock` uniquement comme projection historique legacy, après alignement hash/temps ;
- `KernelEvent` uniquement comme projection temps réel ;
- `ProofOfHistoryStorage` comme port d’ancrage/adaptateur, après suppression de sa concurrence implicite.

## 5. Gap minimal vers un CynicPeer

Le premier peer a besoin d’une couche de transition qui compose les journaux sans les fusionner aveuglément :

`FactObservation → AdmissibleEvent → StateTransition → LocalCommit → ReplicationMessage → ConsensusDecision → FinalityCheckpoint`

Cette couche doit répondre :

- quel état est canonique ;
- qui peut soumettre un événement ;
- qui peut le valider ;
- quelle transition il provoque ;
- quelle preuve est obligatoire ;
- quelle idempotency key est consommée ;
- comment le replay reconstruit l’état ;
- comment deux peers détectent une divergence ;
- quand l’état devient final.

## 6. Ordre de convergence recommandé

1. Choisir `EvidenceJournal` comme provenance de faits et non comme ledger économique complet.
2. Définir un événement admissible typé, sans déplacer encore les journaux spécialisés.
3. Définir une machine de transition locale pour citoyen, node, ressource, agent, workload et CYD-calibration.
4. Relier `WindowJournal` et `AgentExecutionReceipt` à cette transition par références, pas par duplication de payloads.
5. Corriger les lectures silencieuses et les chaînes non atomiques.
6. Produire un checkpoint local replayable.
7. Seulement après, définir la réplication P2P, le consensus d’état et la finalité.
8. Tester Solana Devnet comme settlement externe, pas comme remplacement du ledger CYNIC.

## 7. Critère de non-confusion

Une preuve de journal, un événement diffusé, un snapshot de santé ou un hash-chain local ne doit jamais être annoncé comme :

- transaction acceptée ;
- état final ;
- consensus ;
- solde disponible ;
- contribution CYD définitive ;
- settlement public.

## 9. Diagnostic systémique de fragmentation

Les problèmes de synchronisation se répartissent en six axes :

1. Autorité : `EvidenceJournal`, `ProofOfHistory` Surreal, `StateBlock` et les stores de coordination ont chacun une notion locale de chaîne ou d’état.
2. Temps : timestamps muraux, séquences par nœud, epochs logiques et fencing tokens ne forment pas encore un ordre commun.
3. Identité : les receipts, claims, preuves, nodes et agents ne sont pas tous joints par un événement admissible canonique.
4. Persistance : certains événements sont durables, certains sont des WAL, certains sont en mémoire et certains sont silencieusement ignorés s’ils sont corrompus.
5. Transport : Gossipsub diffuse des événements mais ne synchronise pas un état, ne fournit pas de finalité et ne prouve pas l’inclusion.
6. Sémantique : les journaux ne produisent pas tous une transition d’état ; aucun replay ne reconstruit encore citoyen, node, ressource, agent, workload et CYD dans un même état.

Les symptômes attendus sont :

- un composant croit avoir écrit alors qu’un autre n’a pas reçu ;
- deux composants attribuent des séquences différentes au même fait ;
- une preuve existe sans transition correspondante ;
- un résultat existe sans consommation de ressource ;
- un crédit pourrait être calculé deux fois ;
- un snapshot paraît cohérent localement mais ne correspond pas au journal d’un autre nœud ;
- une projection de santé est confondue avec un état final.

Ce n’est pas encore une incohérence de consensus au sens blockchain : le consensus d’état n’existe pas. C’est une fragmentation pré-consensus qu’il faut résoudre avant de distribuer davantage le système.

## 10. Colonne vertébrale de synchronisation proposée

Sans créer un super-crate, CYNIC doit progressivement relier les surfaces existantes par une même séquence logique :

`CanonicalFact → AdmissibleEvent → StateTransition → LocalJournalCommit → PeerReplication → Consensus → Finality`

Chaque étape doit avoir un owner unique :

- protocol : enveloppe, types et prédicats purs ;
- host/observer : production de faits et preuves de substrat ;
- node : admission, exécution et réplication du peer ;
- kernel : transition d’état, ledger local et validation constitutionnelle ;
- intel/askesis : interprétations et évaluations, jamais état primaire ;
- Solana : settlement ou ancrage externe.

La première convergence ne doit donc pas fusionner les journaux. Elle doit les rendre raccordables : chaque entrée doit référencer un sujet, un objet, un événement parent, une idempotency key, une preuve source et la transition attendue.

## 11. Gate avant synchronisation multi-peer

Avant de lancer le consensus ou d’accueillir d’autres citoyens, le peer mono-opérateur doit démontrer :

- replay déterministe depuis un état initial ;
- même état après relecture des journaux dans un nouveau processus ;
- détection d’un double événement ;
- détection d’une preuve sans transition ;
- détection d’une transition sans preuve requise ;
- attribution unique d’un workload et de sa consommation ;
- lien entre allocation, exécution, résultat et crédit CYD ;
- distinction claire entre événement diffusé, événement accepté et état final ;
- export d’un checkpoint vérifiable par un second peer.

Tant que ce gate n’est pas satisfait, ajouter des pairs augmente la fragmentation plus vite que la décentralisation.

## 12. Slice BuildRunner réalisé

Le premier slice bottom-up a fermé un défaut concret du chemin actif :

- `crates/cynic-node/src/usecases/build_scheduler/evidence.rs` produit maintenant `build_grant_read_back`, exigé par `BUILD_GRANT_READ_BACK_PROOF` ;
- `crates/cynic-node/src/usecases/build_scheduler/prepare.rs` rattache la référence du grant persisté à la provenance ;
- le test de preuve couvre la classe, le workload et la provenance.

Preuves locales :

- `cynic-node --lib` : 318 tests passés, 0 échec ;
- test ciblé `grant_read_back_is_a_supported_completion_proof` : 1 passé ;
- tests webhook/grant ciblés : 10 passés ;
- rustfmt ciblé : succès ;
- `git diff --check` : succès.

Le slice ne ferme pas encore toute la verticale :

- `BuildExecutor` est testé mais non consommé par le scheduler actif, qui utilise `TokioBuildExecution` ;
- `build_submission_orchestrator_uc` est testé mais aucun consommateur de production n’a été observé ;
- l’énergie joules/CYD n’est pas encore produite.

La comptabilité d’exécution est maintenant structurée ainsi : wall-time mesuré
par horloge monotone, CPU mesuré par `CpuUsageMeter`, et pic RSS du processus
enfant observé via `/proc/<pid>/status` sur Linux. Le champ RSS est optionnel et
son absence produit l’unknown `build_peak_memory_unobserved`; aucune valeur zéro
n’est fabriquée. Cette observation reste une mesure de consommation, pas une
preuve de qualité et pas un crédit CYD automatique.