# Audit des mathématiques et de la géométrie de CYNIC

Date : 2026-09-02
Statut : audit read-only, non normatif
Révision observée : `64311212`

Ce document est une preuve de discussion dans `reports/`. Il ne constitue ni une autorité runtime, ni une admission de mutation. L’autorité reste : observation runtime → contrats → registres → types Rust → preuves d’exécution.

## 1. Portée

Surfaces examinées :

- priorité organique et énergie : `cynic-protocol`, `cynic-host` ;
- admission, fenêtres et allocation multidimensionnelle ;
- géométrie d’inférence : tenseurs, Roofline, Pareto, embeddings, métriques de stream ;
- graphes, topologies, convergence et epochs ;
- E-Score, consensus épistémique, CPMM et `cynic-market` ;
- branchement réel des modèles dans les décisions host/node/inference.

Questions d’audit :

- Quelles grandeurs sont réellement mesurées ?
- Quels domaines, unités et invariants sont imposés ?
- Quels objets sont de vrais graphes, espaces, ordres ou métriques ?
- Quelles formules gouvernent une décision active ?
- Quelles inconnues sont conservées ou transformées en succès apparent ?

## 2. Verdict

Aucun défaut P0 ne compromet aujourd’hui une autorité économique de production : le framework déclare encore la futarchie et le métabolisme énergétique comme absents ou bloqués.

Les principaux risques actifs sont ailleurs :

1. le compass organique inverse ou approxime certaines dimensions de classement ;
2. l’admission n’impose pas toutes les dimensions annoncées et ne réserve pas atomiquement ;
3. le solveur d’inférence peut sélectionner une configuration depuis des mesures dépendantes du transport ou numériquement invalides ;
4. plusieurs « graphes », « topologies », « vecteurs » et « epochs causaux » sont des enregistrements nominaux sans les lois mathématiques annoncées ;
5. des configurations déclarées peuvent être élevées au rang d’observations ou de preuves.

La faiblesse systémique n’est donc pas l’absence de mathématiques. C’est l’écart entre le nom géométrique d’une structure et les invariants réellement imposés.

## 3. Constats prioritaires actifs

### P0 sémantique — fraîcheur triée à l’envers

- `crates/cynic-protocol/src/organic_opportunity.rs:331`
- consommateurs : `crates/cynic-host/src/organic_opportunity/mod.rs:68,138`

`freshness_seconds` est un âge : une valeur plus petite est plus fraîche. Le comparateur classe pourtant les âges décroissants. À égalité sur les dimensions précédentes, une observation vieille de 1000 secondes précède une observation vieille de 10 secondes.

### P0 sémantique — fan-out remplacé par la longueur du scope

- `crates/cynic-protocol/src/organic_opportunity.rs:325-329`
- frame par défaut : `crates/cynic-host/src/organic_opportunity/mod.rs:86-96`

Un scope court est traité comme un grand potentiel de déblocage. `host` gagne ainsi contre `crates/cynic-cli/src/audit/sub.rs` sans qu’aucune dépendance soit observée. La longueur d’une chaîne est un proxy manipulable, sans relation démontrée avec le fan-out.

### P1 — énergie invalide transformée en énergie optimale

- `crates/cynic-protocol/src/organic_opportunity.rs:352-369`

`NaN`, les valeurs négatives, zéro et toute valeur strictement inférieure à 1 joule produisent la clé entière zéro. Une mesure invalide n’est donc pas préservée comme `Unknown` ; elle peut être classée comme mesure idéale. Une range est par ailleurs classée uniquement par sa borne basse.

### P1 — plafond mural non appliqué par l’admission

- `crates/cynic-protocol/src/task_pipeline.rs:277-338`
- `crates/cynic-protocol/src/task_admission.rs:79-118`
- chemin actif : `crates/cynic-inference/src/domain/domain_task_queue.rs:139-152`

Une demande de 2000 ms sous une enveloppe limitée à 1000 ms est admise si les autres dimensions respectent leurs bornes. Le temps mural est déclaré mais exclu de l’ordre produit utilisé par la queue.

### P1 — admission et réservation séparées

- décision : `crates/cynic-protocol/src/task_admission.rs:86-118`
- insertion : `crates/cynic-inference/src/domain/domain_task_queue.rs:139-152`
- réservation tardive : `crates/cynic-host/src/adapters/window_runner.rs:75-88`

`AdmittedWithReservation` ne persiste aucune réservation. Deux admissions concurrentes peuvent lire la même capacité puis dépasser ensemble l’enveloppe. C’est un contrôle TOCTOU, pas une transaction d’allocation.

Le runner ne compare pas non plus la consommation finale au budget. Une panique contourne la fermeture annoncée, car le travail n’est pas protégé par `catch_unwind` (`window_runner.rs:90-134`).

### P1 — mesures d’inférence dépendantes du transport HTTP/SSE

- `crates/cynic-inference/src/application/matrix_live_runner.rs:292-319`
- `crates/cynic-inference/src/domain/matrix_sweep/stream_metrics.rs:26-52`
- Pareto : `crates/cynic-inference/src/domain/matrix_sweep/empirical_store.rs:208-215`
- lancement : `crates/cynic-inference/src/application/inference_gateway.rs:319-356`

Les timestamps sont pris par chunk HTTP, tandis que `completion_tokens` compte des fragments SSE textuels. Un chunk peut contenir plusieurs événements, et un événement peut être fragmenté. TTFT, débit et jitter varient donc avec la segmentation réseau, puis gouvernent une sélection Pareto active.

### P1 — points invalides conservés sur la frontière de Pareto

- validation : `crates/cynic-inference/src/math/pareto.rs:21-35`
- frontière : `crates/cynic-inference/src/math/pareto.rs:184-201`
- sélection : `crates/cynic-inference/src/domain/matrix_sweep/empirical_store.rs:225-257`

Une mesure négative ou non finie est reconnue invalide, mais ne peut jamais être dominée. Elle reste donc sur la frontière. Un `ttft_ms = -1` peut gagner `MinLatency`; un seul point invalide peut aussi neutraliser le choix `Balanced` et provoquer un fallback silencieux.

### P1 — configuration storage projetée comme observation

- `crates/cynic-host/src/domain/storage/local_adapter.rs:29-57`
- `crates/cynic-host/src/domain/storage/remote_adapter.rs:17-48`
- `crates/cynic-host/src/domain/storage/probe.rs:35-65`

Une URL est construite depuis la configuration ou la cible SSH sans probe effectif du service, des membres ou du quorum. Une configuration valide peut devenir `observed` puis `proven` alors qu’aucun moteur n’écoute. L’autorité est inversée : configuration → prétendue observation.

### P1 — convergence complète sans couverture du produit cartésien

- `crates/cynic-protocol/src/convergence.rs:72-88`

L’assesseur vérifie seulement `cells.len() >= subjects.len() × verticals.len()`. Il ne vérifie ni appartenance aux axes, ni unicité, ni couverture exacte. Une cellule complète portant des coordonnées étrangères suffit à déclarer complète une matrice 1×1.

## 4. Géométrie d’inférence

### Géométrie exacte non canonique

- modèle explicite : `crates/cynic-inference/src/math/tensor_geometry.rs`
- heuristique active : `crates/cynic-inference/src/domain/runtime_plan.rs:263-306`

`ExactTensorGeometry` existe mais n’est pas utilisé comme décision canonique. Le plan actif utilise notamment `taille modèle + 2 GiB + contexte × 0,5 MiB` et un plafond de décodage basé sur `2 × model_size`.

Deux modèles concurrents décrivent donc le même concern : une géométrie structurée orpheline et une heuristique active. `ExactTensorGeometry` n’est en outre pas encore exact : il n’impose pas `embedding_dim = n_heads × head_dim`, `n_kv_heads ≤ n_heads` ou la cohérence entre KV partagé/dédié et slots parallèles.

### Roofline non physique aux extrêmes

- `crates/cynic-inference/src/math/roofline.rs:53-95`

Le cast `f64 → f32` peut transformer une entrée finie en `+∞`. Le débit observé n’est pas validé avant calcul de l’efficacité, permettant des ratios négatifs, infinis ou `NaN`.

### Variance nommée, écart-type calculé

- `crates/cynic-inference/src/domain/matrix_sweep/stream_metrics.rs:39-46`
- `crates/cynic-inference/src/domain/matrix_sweep/types.rs:97-109`

La racine de la variance échantillonnale est stockée dans `jitter_variance_ms`. La valeur est un écart-type en millisecondes, pas une variance en millisecondes carrées.

### Similarité cosinus : incomparable devient orthogonal

- `crates/cynic-inference/src/domain/embedding.rs:3-14`
- `crates/cynic-inference/src/vector_memory.rs:73-89`

Une différence de dimension ou un vecteur nul retourne `0.0`, comme une orthogonalité réelle. Avec un seuil de zéro, une géométrie incompatible peut devenir un voisin valide. `dimensions` n’est pas recoupé avec `vector.len()` et l’adaptateur HTTP accepte les embeddings vides.

## 5. Graphes, topologies et causalité

### `BranchGraph` n’est pas un DAG vérifié

- types : `crates/cynic-protocol/src/branch_graph.rs:9-27`
- construction : `crates/cynic-host/src/domain/branch_graph/probe.rs:60-66,110-133`

Aucune validation n’impose unicité, présence de racine, absence de parent pendant, absence de cycle ou connectivité. Le parent n’est découvert que si le parent immédiat du commit de tête correspond exactement à la tête d’une autre branche. Ce n’est pas la relation d’ancestralité Git.

### `GitTopology` est une distance relative, pas une topologie

- `crates/cynic-protocol/src/git_topology.rs:59-168`
- observation : `crates/cynic-host/src/git_topology_observer.rs:82-118`

Aucun sommet commit ni arête n’est représenté. `ahead/behind` décrit seulement la distance à `origin/main`. L’état déclaré n’est pas vérifié contre ces distances, `has_remote` ou la fraîcheur avant réconciliation.

### Epochs causaux sans ordre causal

- `crates/cynic-protocol/src/convergence/causal.rs:56-85`
- index : `crates/cynic-protocol/src/convergence/index.rs:52-149`

Une epoch causale porte un identifiant textuel et un timestamp mural, sans parent ni ordre partiel. `latest` dépend du timestamp puis de l’ordre lexical. Un recul d’horloge peut masquer une epoch logiquement plus récente.

### Vecteur de cohérence nominal

- `crates/cynic-protocol/src/convergence/coherence.rs:36-43`

Le `Vector` de cohérence contient des chaînes et options, sans magnitude numérique, unité, norme ou opération vectorielle. C’est une métaphore architecturale utile, pas un espace vectoriel mathématique.

### Topologie storage sans graphe ni quorum

- `crates/cynic-protocol/src/storage_topology.rs:24-63`

Le contrat porte un nombre de répliques, des URL et un leader optionnel. Il ne représente ni identité des membres, ni arêtes, ni membership observé, ni quorum, ni failure domains.

## 6. Modèles économiques et épistémiques latents

### Anti-whale inversé

- `crates/cynic-intel/src/escore/aggregator.rs:165-180`

Le modificateur est approximativement `(stake/max_stake)²`, puis le score est encore multiplié par le stake. La contribution devient proportionnelle à `stake³/max_stake²`.

Contre-exemple vérifié :

- stake 1000 → contribution 1000 ;
- stake 10 → contribution 0,001 ;
- rapport initial 100 ;
- rapport après « anti-whale » 1 000 000.

### Marché sans fermeture économique

- CPMM : `crates/cynic-intel/src/futarchy/cpmm.rs`
- vault : `crates/cynic-intel/src/futarchy/vault.rs:41-57`

L’arithmétique CPMM emploie utilement `u128`, mais les positions individuelles, le règlement, la conservation dépôts/réserves/parts/paiements et le slippage minimal ne sont pas imposés. Un identifiant dupliqué peut écraser une proposition tout en incrémentant compteurs et réserves.

### Autorité assignable et non zero-sum

- `crates/cynic-market/src/types.rs:56-83`
- `crates/cynic-market/src/registry.rs:13-20`
- `crates/cynic-market/src/market_loop.rs:108-139`

`authority`, `state` et les compteurs sont publics et désérialisables. Les deltas ±0,05 sont indépendants puis clampés : l’autorité peut être créée aux bornes. Les erreurs d’un acteur neuf sont gratuites, ce qui rend la stratégie Sybil favorable.

### Résolution et consensus non admissibles

- résolution : `crates/cynic-market/src/market_loop.rs:88-128`
- consensus : `market_loop.rs:60-85,145-173`
- collusion : `crates/cynic-market/src/collusion.rs:23-27`

La résolution compare une chaîne libre à une prédiction et inscrit `manual_resolution` sans preuve. Le consensus mélange les métriques et permet plusieurs prédictions d’un même acteur. La détection de collusion retourne toujours `None`.

### Constantes esthétiques non calibrées

- `crates/cynic-intel/src/epistemic/types.rs`

φ, φ⁻¹, φ⁻² et les seuils Fibonacci 21/233 ne sont reliés à aucune fonction de perte, calibration empirique ou modèle probabiliste. Ce sont actuellement des choix esthétiques structurés, pas des lois épistémiques démontrées.

## 7. Points solides

- Les capacités économiques sont déclarées absentes plutôt que présentées comme convergées.
- `EnergyEstimate::Unknown` et les résiduels explicites sont de bonnes bases contractuelles.
- L’arithmétique tensorielle et CPMM emploie souvent des entiers larges.
- Pareto conserve prudemment l’incomparabilité lorsque la qualité optionnelle manque ; le défaut concerne les points invalides, pas les points valides incomplets.
- Les surfaces économiques restent latentes : aucun appelant externe à `cynic-market` n’a été trouvé.
- La plupart des projections restent read-only : les défauts peuvent être corrigés avant amplification d’autorité.

## 8. Couverture et preuves

Tests ciblés passés :

- `cynic-market` : 13 ;
- `cynic-intel` : 78 ;
- `cynic-inference` : 21 ;
- total : 112 tests, 0 échec.

Tests `cynic-protocol task_pipeline::tests` non exécutés : rustc 1.96 a subi l’ICE connu `unexpected unresolved segments` pendant `hir_crate`. Les tentatives globales 1.95/1.96 ont également ICE dans le graphe transitive. Aucun échec mathématique n’est déduit de ce blocage de toolchain.

Aucun usage de `proptest`, `quickcheck` ou `bolero` n’a été trouvé dans les surfaces auditées. Les tests actuels sont majoritairement des exemples nominaux ; ils ne prouvent pas conservation, monotonicité, invariance d’échelle, robustesse aux non-finis, couverture exacte ou résistance Sybil.

## 9. Comparaison externe

Patterns retenus :

- unités typées plutôt que nombres nus : <https://docs.rs/uom/latest/uom/> ;
- scoring probabiliste propre — Brier ou logarithmique — plutôt qu’un delta indépendant de la confiance ;
- vecteurs validés et incompatibilité représentée par `Result`/`Option`, jamais par une fausse similarité zéro ;
- DAG/ordres partiels avec validation de couverture, d’unicité et d’acyclicité ;
- Roofline paramétré par les octets réellement déplacés et une bande passante attribuée au bon chemin ;
- tests de propriétés pour conservation, bornes, monotonicité et invariance.

## 10. Ordre de correction recommandé

### État initial

Les dimensions existent dans les types, mais certaines sont inversées, omises ou remplacées par des proxies. Les mesures invalides peuvent entrer dans les décisions. Les objets géométriques ne portent pas toujours leurs lois.

### État transitoire

1. Corriger le sens de `Freshness`.
2. Remplacer `scope.len()` par un fan-out observé ou `Unknown`.
3. Valider confiance et énergie avant classement.
4. Intégrer `wall_time_ms` dans l’admission.
5. Rendre admission et réservation atomiques.
6. Produire `Within | Exceeded | Unknown` à la fermeture.
7. Rejeter les points Pareto invalides avant construction de la frontière.
8. Mesurer les tokens au niveau tokenizer/événement décodé, jamais au niveau chunk HTTP.
9. Ajouter les validations structurelles aux graphes, matrices et géométries tensorielle/vectorielle.

### État final

Chaque surface géométrique possède :

- un espace et des unités explicitement définis ;
- des valeurs invalides ou inconnues distinctes des valeurs neutres ;
- des invariants testés par propriétés ;
- une décision canonique unique ;
- une observation indépendante ;
- une preuve qui ne peut pas être fabriquée depuis le désiré.

Le premier move reste la réparation du compass organique : il choisit l’ordre de toutes les corrections suivantes. La seconde tranche cohérente est l’admission atomique multidimensionnelle.