# Context Map des arêtes Cargo de CYNIC

Date : 2026-09-02
Statut : carte observée, read-only, non normative
Révision de départ : `44924e10324f47d17cc81698e9ed05e10500a1b9`

Cette carte qualifie les dépendances internes déclarées par les manifests Cargo. Elle ne change ni l’ownership ni les règles de dépendance. Une arête Cargo exprime une possibilité de compilation ; la qualification ci-dessous décrit les usages source observés.

## 1. Taxonomie

- `projection` : une surface humaine/agent expose une capacité possédée ailleurs.
- `shared_contract` : consommation de types, schémas ou règles pures publiés comme langage partagé.
- `customer_supplier_port` : le downstream consomme un trait/port possédé par l’upstream et injecte l’implémentation.
- `adapter_plugin` : une composition root adapte une capacité canonique à un runtime externe.
- `compatibility` : façade ou réexport transitoire conservant une API ancienne.
- `concrete_coupling` : import d’adaptateur, d’état applicatif ou de fonction concrète cross-context sans port stable.
- `declared_unused` : dépendance Cargo sans référence source non-test observée.
- `mixed` : plusieurs relations coexistent sur la même arête.

Statuts :

- `allowed` : relation cohérente avec l’ownership observé ;
- `review` : relation légitime en partie, mais trop large ou mélangée ;
- `remove_or_absorb` : dépendance fantôme ou responsabilité à déplacer ;
- `unknown` : preuve insuffisante.

## 2. Vue globale

`cargo metadata` observe 26 arêtes internes et aucun cycle Cargo : 25 dépendances de production et une dépendance réservée aux tests (`cynic-host → cynic-intel`, `kind=dev`). La table principale qualifie les 25 arêtes de production. La topologie est acyclique, mais la qualité d’une arête ne découle pas de son orientation seule.

Classification initiale des arêtes de production :

- 12 arêtes essentiellement conformes ;
- 11 arêtes mixtes à réduire vers ports/projections explicites ;
- 2 dépendances déclarées sans usage observé.

La dépendance dev-only host → intel soutient des tests et n’autorise aucun couplage runtime ; elle doit être auditée séparément si des fixtures intel commencent à modeler une décision host.

## 3. Table des 25 arêtes

| # | Arête | Relation observée | Statut | Preuve principale | Décision géométrique |
|---:|---|---|---|---|---|
| 1 | cli → askesis | projection/composition | allowed | `cynic-cli/src/memory_surface.rs:8-9` | Askesis possède log/rétention ; la CLI compose et projette. Vérifier séparément que la CLI ne possède pas le workflow de persistance. |
| 2 | cli → host | mixed: projection + composition de nombreux adaptateurs | review | 84 fichiers ; `deps_surface_probe.rs:4`, surfaces runtime/bootstrap | Direction légitime, surface trop large. Chaque sous-surface doit rester parse/inject/call/render ; les décisions host doivent rester dans host. |
| 3 | cli → inference | mixed: projection + composition + quelques types concrets | review | 31 fichiers ; `memory_surface.rs:10-11`, `operator_status.rs:10` | Direction légitime. Réduire les dépendances sur stores/implémentations concrètes lorsque des ports existent. |
| 4 | cli → node | mixed: projection + instanciation d’adaptateurs node | review | `agent_host_surface.rs:19`, `workload_observation.rs:17-18`, `fleet_surface.rs:188` | La CLI peut être composition root, mais ne doit pas appeler une logique fleet ou policy non encapsulée. |
| 5 | cli → observer | projection | allowed | `solana_surface.rs:4,149` | Projection Solana étroite ; ownership observer conservé. |
| 6 | cli → protocol | shared_contract/projection | allowed | 57 fichiers ; `agent_host_surface.rs:20,119-135` | Le protocole sert de published language. Interdire toute réconciliation parallèle dans CLI. |
| 7 | host → inference | customer_supplier_port | allowed | `autoresearch/reflection.rs:78-105`, `adapters/local_task_admission.rs:15` | Bon pattern : host fournit capacité/adaptation et consomme `ChatPort`/`TaskAdmissionPort`. |
| 8 | host → protocol | shared_contract + implémentation de ports/adaptateurs | allowed | 213 fichiers ; branch graph, build, journal, substrate | Relation fondamentale : protocol définit, host observe/applique. Surveiller la croissance du shared kernel. |
| 9 | inference → protocol | shared_contract | allowed | `math/tensor_geometry.rs:7`, `application/inference_gateway.rs:19,339-347` | Les types sérialisables partagés sont légitimes. Réévaluer les types utilisés uniquement par inference afin d’éviter un protocol fourre-tout. |
| 10 | intel → protocol | shared_contract avec réexport temporel | review | `intel/src/lib.rs:17`, `sop_sanitizer.rs:7`, `solana_safety_dog.rs:6` | Solana/agentic contracts sont légitimes. Le réexport générique `Epoch` doit être qualifié par bounded context. |
| 11 | kernel → host | mixed: concrete_coupling + runtime composition | review | `bootstrap/agent_session.rs:80-90`, `bootstrap/server.rs:196-213`, imports runtime authority/registry/layout | L’arête restera probablement nécessaire, mais les lectures concrètes doivent passer par un port host ou une composition root bornée. |
| 12 | kernel → inference | mixed: concrete_coupling + quelques ports | review | `bootstrap/rest_state.rs:8-18`, `api/rest/inference_router.rs:7-16` | Principal hotspot : kernel importe adapters/application/slot state. Remplacer par ports de routing, proxy, slot et embedding ; conserver uniquement la composition nécessaire. |
| 13 | kernel → intel | mixed: canonical domain reuse + façades de compatibilité | review | `domain/kscore.rs:5-35`, `domain/token_postprocessor.rs:7`, `backends/dexscreener.rs:11` | Intel est le bon owner, mais kernel conserve façades et implémente des vues. Réduire vers services/ports intel et retirer les doublons de domaine. |
| 14 | kernel → node | mixed: ports + fonctions concrètes fleet/mesh/verify | review | `storage/surreal.rs:622-653`, `api/rest/event.rs`, `api/rest/mesh.rs` | Les ports workspace sont légitimes ; fleet, mesh et verify concrets doivent être projetés derrière ports node. |
| 15 | kernel → protocol | shared_contract | allowed | 50 fichiers ; `build_scheduler.rs:14-17` | Kernel consomme les contrats purs pour admission/coordination. L’arête est conforme si kernel ne redéfinit pas la policy. |
| 16 | market → intel | declared_unused | remove_or_absorb | aucune référence source non-test observée | Retirer la dépendance tant que market n’intègre pas réellement une capacité intel ; ne pas garder une intégration narrative. |
| 17 | market → protocol | declared_unused | remove_or_absorb | aucune référence source non-test observée | Même verdict : dépendance fantôme. Toute activation future doit nommer les contrats réellement consommés. |
| 18 | node → host | mixed: orchestration node + nombreux concrets host | review | 109 fichiers ; runtime layout/harness/update lifecycle/registry/process observation | Direction nécessaire pour le substrat, mais l’arête est trop large. Node doit consommer des ports host ; seules les composition roots peuvent instancier les adaptateurs. |
| 19 | node → inference | compatibility/concrete application façade | review | `verify.rs:6-109`, `inference_node_ops.rs:7-11` | Les façades délèguent bien mais dupliquent des types et réexportent l’application. Introduire/consommer un port node-runtime puis déprécier les wrappers. |
| 20 | node → protocol | shared_contract + orchestration | allowed | 150 fichiers ; `node_control_runtime.rs:21-65` | Relation canonique : node consomme desired state, preuves et politiques pures. |
| 21 | observer → host | customer_supplier_port | allowed | `source.rs:15`, `main.rs:50`, `wal.rs:43` | Observer consomme `MetabolismProvider` et son observation. Relation étroite et attribuée. |
| 22 | observer → intel | mixed: shared types + policy métier dans observer | review | `signal_mapper.rs:10,23-33,81-98`, métriques E-Score | Les types intel sont légitimes, mais thresholds et mapping E-Score vivent dans observer. Déplacer la décision sémantique vers intel ; observer doit produire les faits bruts. |
| 23 | observer → protocol | shared_contract | allowed | `domain.rs:11`, `federation.rs:15`, `consensus.rs:2` | Published language de preuve et fédération ; relation conforme. |
| 24 | zeroclaw-solana → intel | adapter_plugin/composition | allowed | `plugin.rs:3-4,31-41` | Plugin externe composant le port et l’adaptateur canonique intel. L’adaptateur concret est acceptable dans cette composition root. |
| 25 | zeroclaw-solana → protocol | shared_contract | allowed | `plugin.rs:5-9`, `bin/demo.rs:31` | Budgets et requêtes wire partagés ; relation conforme. Le binaire demo ne constitue pas une preuve runtime. |

## 4. Relations DDD dominantes

### Published Language / Shared Kernel

`cynic-protocol` est le shared kernel de neuf consommateurs. Cela fonctionne tant qu’il contient uniquement des formes sérialisables et des règles pures réellement partagées. Les types propres à un seul domaine doivent rester chez leur owner.

Arêtes concernées : 6, 8, 9, 10, 15, 20, 23, 25 ; 17 est actuellement fantôme.

### Customer–Supplier par port

Le meilleur exemple observé est host → inference : host implémente l’adaptation de capacité et consomme les traits inference. Observer → host est également étroit et lisible.

Arêtes concernées : 7 et 21.

### Projection / Conformist

CLI adopte les API publiques de plusieurs contextes. C’est acceptable pour une composition root, mais cette relation devient dangereuse lorsque la CLI recalcule une décision ou accède directement à un store.

Arêtes concernées : 1 à 6.

### Anti-Corruption Layer / Plugin

ZeroClaw transpose son format de tool vers les contrats CYNIC. L’adaptateur protège le modèle interne du runtime externe.

Arêtes concernées : 24 et 25.

### Compatibility façades

Les wrappers node → inference et plusieurs réexports kernel préservent des chemins historiques. Ils doivent porter une condition de suppression ; sinon la compatibilité devient une seconde surface permanente.

Arêtes concernées : 13 et 19, partiellement 11-14.

## 5. Hotspots géométriques

### Kernel comme agrégateur concret

Les arêtes 11-14 sont toutes nécessaires au niveau fonctionnel, mais trop d’entre elles traversent directement des modules `application`, `adapters`, `runtime_registry`, `fleet_inventory`, `mesh_registry` ou des états concrets.

La correction n’est pas de supprimer toutes les dépendances du kernel. Elle est de réduire leur surface à des ports et contrats dont la décision reste chez le supplier.

### Node ↔ host : frontière poreuse

L’arête 18 touche au moins 109 fichiers source et de nombreuses familles host. Cette largeur rend difficile de dire si node orchestre ou réimplémente le substrat. C’est la première arête à inventorier symbole par symbole après les quatre arêtes kernel.

### CLI comme composition root diffuse

Les arêtes 2-4 couvrent un grand nombre de fichiers. Une composition root dispersée dans près de deux cents modules CLI rend la règle « thin surface » difficile à prouver. La bonne géométrie est une composition bornée par famille, pas une interdiction générale d’importer les crates métier.

### Observer contient une décision intel

`signal_mapper.rs` transforme directement des actions PumpFun en dimensions E-Score avec seuils et pondérations. Observer devrait attribuer des faits ; intel devrait posséder leur interprétation. C’est un défaut DDD plus précis qu’un simple mauvais sens de dépendance Cargo.

### Dépendances narratives de market

Les deux arêtes de market sont déclarées mais inutilisées. Elles donnent l’apparence d’une intégration avec protocol/intel que le code ne réalise pas.

## 6. Invariants exécutables à dériver

1. `cynic-market` ne déclare aucune dépendance interne inutilisée.
2. Les modules kernel hors composition root n’importent pas `cynic_inference::adapters` ou `::application`.
3. Les modules kernel hors adapter/compatibilité n’importent pas les implémentations concrètes host/node.
4. Observer ne possède pas les tables de seuils ou pondérations E-Score.
5. CLI peut importer les façades publiques, mais aucune fonction de réconciliation ou table de policy ne doit y être définie.
6. Toute façade `compatibility` nomme son owner canonique, sa raison et sa condition de suppression.
7. Tout type protocol consommé par un seul bounded context déclenche une revue d’ownership, pas une extraction automatique.
8. Le Context Map est régénéré depuis Cargo et une analyse syntaxique des imports ; les chaînes/commentaires/tests ne comptent pas comme arêtes runtime.

## 7. Ordre de convergence

1. Retirer ou justifier les deux dépendances fantômes de market.
2. Classifier les imports kernel → inference/host/node/intel par `port | composition | compatibility | concrete`.
3. Extraire la policy E-Score de observer vers intel derrière un port ou une fonction pure canonique.
4. Borner node → host autour de ports de substrate/lifecycle/runtime layout.
5. Auditer les trois grandes arêtes CLI sans empêcher la composition légitime.
6. Ajouter les architecture-boundary tests manquants et projeter leurs résultats dans `cynic dev crate-audit`.

## 8. Vérification et résiduels

Vérifications locales :

- `cargo metadata --no-deps --format-version 1` : 11 packages, 26 arêtes internes, DAG acyclique ;
- distinction confirmée : 25 arêtes de production + 1 arête dev-only ;
- table : 25 lignes uniques, séquence 1–25 complète ;
- répartition : 12 `allowed`, 11 `review`, 2 `remove_or_absorb` ;
- recherche source `cynic-market` : zéro référence à `cynic_intel::` ou `cynic_protocol::` ;
- recherche ciblée : imports concrets kernel et policy E-Score dans observer confirmés ;
- `git diff --check` : succès.

Une revue indépendante en cinq lots a été tentée. Les cinq reviewers ont échoué sur une limite fournisseur HTTP 429 avant de rendre un verdict. Cet échec reste une inconnue de couverture et ne constitue ni approbation ni réfutation de la carte.

Résiduels :

- Le comptage de références est lexical ; les décisions finales exigent une lecture syntaxique et des consommateurs.
- Les références dans `#[cfg(test)]` internes aux fichiers non nommés `tests.rs` peuvent encore gonfler certains comptes.
- `allowed` signifie cohérent avec l’ownership observé, pas preuve de convergence runtime.
- Le SHA peut évoluer dans le worktree partagé ; cette carte reste attachée au SHA de départ.
- Aucun changement Cargo ou Rust n’est admis par ce rapport.