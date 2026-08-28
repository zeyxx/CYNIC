# Plan de benchmark LLM local — Ryzen 7 5700G + 32 Go

**Qualité × vitesse × mémoire pour raisonnement, coding et agents**

**Cible matérielle :** Ryzen 7 5700G (8C/16T) + Radeon Vega 8 Vulkan + DDR4-3200 partagée • ~28 Go réellement utilisables

---

## 1. Objectif et contraintes

L'objectif est d'identifier la frontière de Pareto des modèles locaux récents sur ce matériel : maximiser la qualité utile par unité de temps et de mémoire, plutôt que simplement faire rentrer le plus gros modèle. La priorité va aux MoE à faible nombre de paramètres actifs, aux quantifications agressives Q1/Q2/Q3/Q4, au KV cache quantifié et à l'exploitation CPU/Vulkan.

- **Usages prioritaires :** raisonnement, développement logiciel, compréhension/modification de codebases et workflows agentiques avec outils.
- **Budget mémoire réel :** environ 28 Go pour modèle, KV/state cache, runtime, contexte, slots et processus auxiliaires. Toute activité de swap invalide un benchmark de performance.
- **Backend matériel principal :** llama.cpp CPU/Vulkan, avec comparaison CPU-only, Vulkan et offload hybride. CPU et Vega 8 partagent la même bande passante DDR4.
- **Serving multi-agent :** Les moteurs vLLM/SGLang sont à conserver comme références de serving multi-agent (PagedAttention, prefix caching/RadixAttention), surtout pour un futur hardware GPU mieux supporté.

---

## 2. Modèles prioritaires

| Prio | Modèle | Architecture | Quants | Rôle |
| --- | --- | --- | --- | --- |
| P0 | Qwen3.6-35B-A3B | MoE ~35B / 3B actif | IQ2_M, Q2_K_XL, IQ3_S, Q4_K_S | Candidat principal qualité/coding/agentique |
| P0 | Gemma 4 26B-A4B | MoE ~26B / 4B actif | IQ2_M, Q2_K_XL, IQ3_S, Q4_K_S | Concurrent principal, efficacité MoE |
| P0 | Nemotron 3.5 Lightning 30B-A3B | MoE/hybride ~30B / 3B actif | Q2, Q3, Q4 disponibles | Vitesse et workflows agentiques |
| P0 | GPT-OSS-20B | MoE | MXFP4 + GGUF pertinent | Baseline MoE compacte |
| P1 | Qwen3.5-35B-A3B | MoE ~35B / 3B actif | Q2, IQ3, Q4 | Baseline génération précédente |
| P1 | Qwen3-30B-A3B | MoE ~30B / 3B actif | IQ2, Q2, IQ3, Q4 | Référence MoE mature |
| P1 | Gemma 4 31B | Dense | IQ1, IQ2, IQ3 | Mesurer le coût d'un gros dense |
| P1 | Meilleur Qwen ~8–9B récent | Dense/hybride | Q4, Q6, Q8 | Baseline petit modèle rapide |
| P2 | Meilleurs 1–4B récents / LFM | Dense/hybride | Q4, Q6, Q8 | Sous-agents et routage très rapide |

IQ1/Q1 est conservé comme expérience exploratoire : il n'est intéressant que s'il permet d'accéder à une classe de modèle supérieure sans franchir une forte 'quantization cliff'. La zone de production attendue est plutôt Q2/Q3/Q4.

---

## 3. Dimensions du benchmark

- **Quantification des poids :** IQ1/Q1 → IQ2/Q2 → IQ3/Q3 → Q4, selon les formats réellement disponibles et optimisés pour chaque architecture.
- **KV / state cache :** Q8/Q8, Q8/Q4 et Q4/Q4. Mesurer séparément mémoire, qualité long-context et débit.
- **Contexte :** 32k = baseline réaliste ; 64k = cible principale ; 128k = long-context/stress réaliste ; 256k uniquement pour les finalistes si viable.
- **Slots / concurrence :** 1×128k, 2×64k et 4×32k à budget contextuel logique comparable ; puis 4×64k comme stress test si la RAM le permet.
- **Backend :** CPU-only, Vulkan maximal et plusieurs points d'offload partiel. Chercher la meilleure répartition CPU/Vega plutôt que supposer que full Vulkan gagne.
- **Serving :** llama.cpp comme moteur principal actuel. Garder une matrice compatible avec vLLM/SGLang pour comparer plus tard le serving à forte concurrence.
- **Optimisations :** Flash Attention, batch/ubatch, nombre de threads, MTP/speculative decoding quand disponible, prompt caching et réutilisation de contexte.

---

## 4. Charges de contexte réalistes

Pour une codebase de plusieurs milliers de lignes, surtout avec architecture hexagonale, tests, interfaces, logs, outils et historique, 4k/16k ne représente pas le workload cible. Un bon agent ne doit toutefois pas injecter le dépôt entier : il doit combiner une carte globale de l'architecture avec une sélection locale de fichiers, dépendances, tests et traces pertinentes.

| Contexte | Profil | Interprétation |
| --- | --- | --- |
| 32k | Coding ciblé | Architecture résumée + plusieurs fichiers + tests + historique court |
| 64k | Référence principale | Travail multi-fichiers, dépendances, tests/logs et historique agentique |
| 128k | Long context | Gros changement transversal, longue session, exploration large du repo |
| 256k | Stress finaliste | Uniquement si RAM, vitesse et rétention de qualité restent acceptables |

---

## 5. Métriques système

| Métrique | Unité | Pourquoi |
| --- | --- | --- |
| PP / prefill | tok/s | Critique pour gros prompts, RAG et codebases |
| TG / decode | tok/s | Débit de génération |
| TTFT | s | Latence avant premier token |
| RAM modèle / Peak RSS | Go | Contrainte dure : ~28 Go, swap = échec |
| KV/state | Mo/1k tokens | Permet d'extrapoler 32/64/128k et plusieurs slots |
| RAM bandwidth | Go/s | Bottleneck majeur du 5700G |
| CPU / iGPU usage | % | Identifier sous-utilisation et contention |
| Aggregate throughput | tok/s | Débit total avec plusieurs slots |
| Per-slot throughput | tok/s | Latence vécue par chaque agent |
| Load/cold start | s | Important si orchestration multi-modèle |
| Puissance / énergie | W, Wh | Mesurer Wh par tâche réussie |
| Thermal/throttling | °C / clocks | Vérifier la stabilité des longs runs |

---

## 6. Métriques qualité et agentiques

- **Raisonnement :** accuracy/pass@1, cohérence multi-étapes, tokens nécessaires jusqu'à la bonne solution.
- **Coding :** compilation, tests passés, pass@1/pass@n, correction du diff, régressions, compréhension multi-fichiers.
- **Agentique :** task success rate, choix du bon outil, validité JSON/schema, arguments corrects, appels inutiles, boucles et retries.
- **Long context :** qualité et vitesse à contexte effectivement rempli (32k/64k/128k), pas seulement contexte maximal annoncé.
- **Quantization retention :** score du quant / score de la meilleure quantification testée ; localiser la quantization cliff.
- **KV degradation :** comparer Q8/Q8, Q8/Q4 et Q4/Q4 aux différentes longueurs de contexte.
- **Tokens/task et retries/task :** un modèle plus lent en tok/s peut gagner s'il raisonne plus court et réussit plus souvent.

---

## 7. Métriques finales de décision

- **TTCS — Time To Correct Solution :** Temps entre le début du prompt et un résultat final validé. Capture prefill, TTFT, génération, outils, erreurs et retries.
- **Successful Tasks / Hour :** Mesure le travail utile réellement produit par la machine, en gardant le taux de réussite visible à côté.
- **Tokens / Successful Task :** Détecte les modèles qui compensent une faible efficacité de raisonnement par beaucoup de tokens.
- **Wh / Successful Task :** Pertinent pour les agents autonomes ou les workloads longs.
- **KV Reuse Ratio / Effective Prefill :** Mesure le bénéfice réel du prompt/prefix caching lorsque plusieurs agents partagent architecture, outils ou contexte commun.

---

## 8. Benchmark multi-slot / multi-agent

| Slots | Ctx/slot | Budget logique | Objectif |
| --- | --- | --- | --- |
| 1 | 128k | 128k | Agent unique à mémoire longue |
| 2 | 64k | 128k | Deux spécialistes avec contexte substantiel |
| 4 | 32k | 128k | Petit swarm spécialisé |
| 4 | 64k | 256k | Stress test, seulement si mémoire viable |

Pour chaque profil : cold prefill, warm/cached prefill, génération après contexte rempli et génération concurrente. Mesurer le débit agrégé, le débit par slot, TTCS p50/p95, peak RSS et le taux de réutilisation du cache.

---

## 9. Rôle de llama.cpp, vLLM et SGLang

llama.cpp est le moteur prioritaire sur le 5700G grâce à GGUF, aux quants IQ/Q1/Q2, au CPU et à Vulkan. vLLM et SGLang deviennent particulièrement intéressants lorsque la charge dominante est le serving concurrent à contexte long : gestion paginée du KV, continuous batching et réutilisation de préfixes. Leur intérêt pratique sur le Vega 8 est cependant limité par le support matériel ; le protocole doit donc rester portable afin de les rebenchmarker lors d'un passage à un GPU dédié.

---

## 10. Protocole en trois étapes

1. **Screening performance :** Tous les P0 en 32k/64k ; Q2/Q3/Q4 ; CPU vs Vulkan ; KV Q4/Q4 par défaut. Éliminer les configurations lentes, instables ou proches de l'OOM.
2. **Qualité :** Finalistes sur raisonnement, coding et tâches agentiques reproductibles. Mesurer pass@1, task success, tokens/task et TTCS.
3. **Long context + concurrence :** Finalistes en 64k/128k et 1×128k / 2×64k / 4×32k, avec cold/warm cache et mesures de débit agrégé.

---

## 11. Tableau de résultats cible

| Model | Quant | KV | Backend | Slots×Ctx | PP | TG | TTFT | RAM | Reason | Code | Agent | Pass@1 | tok/task | TTCS | tasks/h |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| … | … | … | … | … | … | … | … | … | … | … | … | … | … | … | … |
| … | … | … | … | … | … | … | … | … | … | … | … | … | … | … | … |
| … | … | … | … | … | … | … | … | … | … | … | … | … | … | … | … |
| … | … | … | … | … | … | … | … | … | … | … | … | … | … | … | … |

---

## 12. Hypothèse de départ

La zone la plus prometteuse est un MoE ~26–35B avec ~3–4B paramètres actifs, quantifié en Q2/Q3, associé à un KV Q4 ou asymétrique Q8/Q4. Q4 reste indispensable comme référence qualité. Le gagnant ne sera pas nécessairement celui qui possède le meilleur tok/s mono-slot : pour un orchestrateur, la métrique déterminante peut devenir le nombre de tâches correctes par heure sous 2–4 slots et contexte long.

---

## 13. Routage adaptatif, escalade et apprentissage

Le benchmark ne doit pas produire un unique « meilleur modèle », mais une hiérarchie de modèles et de configurations utilisables par un routeur adaptatif. L'objectif opérationnel est de commencer par la configuration ayant le meilleur coût attendu pour une tâche donnée, puis d'escalader uniquement lorsque les signaux d'échec, d'incertitude ou de manque de contexte le justifient.

### 13.1 Boucle de décision

1. **Fingerprint de la tâche :** embedding sémantique + métadonnées structurelles.
2. **Recherche d'expériences similaires dans la mémoire.**
3. **Estimation de la difficulté, du contexte nécessaire et de $P(\text{success} \mid \text{tâche}, \text{modèle/configuration})$.**
4. **Sélection du modèle, quantification, contexte, stratégie de retrieval et budget de calcul.**
5. **Exécution avec outils.**
6. **Vérification objective ou par juge.**
7. **Acceptation, retry ciblé, retrieval supplémentaire, changement de stratégie ou escalade.**
8. **Enregistrement du résultat pour améliorer les décisions futures.**

### 13.2 Fingerprint et Experience Store

Chaque exécution doit produire un enregistrement exploitable par le routeur. L'embedding capture la proximité sémantique ; les métadonnées décrivent la structure et le coût réel de la tâche.

| Famille | Champs |
| --- | --- |
| Sémantique | task embedding, type de tâche, langage, objectif |
| Codebase | repository, sous-système, architecture, fichiers, profondeur de dépendances |
| Contexte | tokens injectés, éléments retrieved, cache hit/reuse |
| Configuration | modèle, quant poids, KV quant, backend, slots, paramètres runtime |
| Résultat | succès/échec, verifier score, tests, compilation, régressions |
| Coût | TTCS, PP/TG, tokens I/O, retries, appels d'outils, RAM peak, énergie |
| Escalade | chemin suivi, raison de l'escalade, modèle qui a finalement récupéré la tâche |

### 13.3 Règles d'escalade

Une escalade ne signifie pas nécessairement « modèle plus gros ». Le système doit choisir l'action la moins coûteuse susceptible de résoudre la cause de l'échec.

| Signal | Action privilégiée |
| --- | --- |
| Tests/validation OK | Accepter et mémoriser |
| Compilation ou test échoue | Retry ciblé avec erreur et contexte pertinent |
| Contexte insuffisant | Retrieval supplémentaire avant changement de modèle |
| Même erreur / même outil en boucle | Changer de stratégie ou de modèle |
| Faible confiance / résultat non vérifiable | Second modèle ou verifier |
| Contradiction entre résultats | Arbitrage par modèle diversifié / plus fiable |
| Échec persistant | Escalade compute : quant supérieure, modèle supérieur ou décomposition |
| Timeout / raisonnement trop long | Modèle plus efficient, décomposition ou stratégie différente |

### 13.4 Diversité et récupération après échec

La diversité architecturale est une ressource : deux modèles ayant des scores globaux proches peuvent échouer sur des tâches différentes. Le benchmark doit donc mesurer non seulement le taux de réussite individuel, mais aussi la capacité d'un modèle B à récupérer les échecs d'un modèle A.

**Failure Recovery Rate (FRR) :**
$$\text{FRR}(B \mid A) = P(\text{succès de } B \mid \text{échec de } A)$$

Il faut également mesurer l'overlap/corrélation des échecs. Une paire de modèles moins corrélés peut former une chaîne d'escalade plus efficace qu'une paire de modèles individuellement meilleurs mais échouant sur les mêmes cas.

### 13.5 Routage par coût attendu

Pour une tâche donnée, le routeur doit estimer à partir des expériences similaires le taux de réussite et le coût attendu de chaque configuration. Une formulation simple peut optimiser $P(\text{success}) / E(\text{TTCS})$, puis intégrer RAM, énergie et tokens selon les contraintes :

$$\text{Utility}(m, t) \approx \frac{P(\text{success} \mid m, t)}{E(\text{TTCS} \mid m, t)}$$

À terme, la décision ne porte pas uniquement sur le modèle : elle peut sélectionner conjointement modèle, quantification, taille de contexte, retrieval, backend et stratégie de raisonnement.

---

## 14. Embeddings, retrieval et représentation de la codebase

Le contexte long ne doit pas être rempli naïvement avec l'intégralité du dépôt. Le système doit combiner retrieval sémantique, structure du code et résumé architectural afin de fournir au modèle le contexte minimal suffisant.

### 14.1 Index hiérarchique

- Repository / architecture globale
- Bounded context / domaine / module
- Ports et interfaces
- Adapters / infrastructure
- Classes, fonctions et symboles
- Tests associés
- Documentation, décisions et historique pertinent

L'embedding répond principalement à *« qu'est-ce qui est sémantiquement pertinent ? »*. Un graphe de dépendances/symboles répond à *« qu'est-ce qui est structurellement lié ? »*. Les deux signaux doivent être combinés, particulièrement pour une architecture hexagonale.

### 14.2 Retrieval avant escalade

Avant de consommer davantage de compute, le système doit vérifier si l'échec provient simplement d'un contexte incomplet. Un retrieval additionnel ciblé peut être beaucoup moins coûteux qu'un passage immédiat à un modèle plus lourd.

### 14.3 Apprentissage du contexte optimal

L'Experience Store permet d'estimer progressivement la quantité et le type de contexte nécessaires pour chaque famille de tâches. Une modification locale peut n'exiger que 15–30k tokens alors qu'un changement architectural transversal peut nécessiter 64–128k. Le contexte devient donc lui-même une variable de routage.

---

## 15. Escalade entre quantifications

Les différentes quantifications d'un même modèle peuvent servir de niveaux de compute. Une configuration Q2 très efficiente peut traiter les tâches courantes ; Q3/Q4 peuvent être réservées aux familles de tâches pour lesquelles leur gain de réussite compense leur coût.

| Quant | Rôle | Usage |
| --- | --- | --- |
| Q1/IQ1 | Exploratoire | Accéder à une classe de modèle autrement impossible ; surveiller la quantization cliff |
| Q2/IQ2 | Worker efficient | Candidat pour débit élevé et gros budget KV/contexte |
| Q3/IQ3 | Balanced | Candidat probable au meilleur compromis qualité/vitesse |
| Q4 | Quality tier | Référence qualité et niveau d'escalade lorsque le gain est mesurable |

---

## 16. Nouvelles métriques du benchmark adaptatif

| Métrique | Interprétation |
| --- | --- |
| FRR(B\|A) | Taux de récupération de B après échec de A |
| Failure overlap | Proportion d'échecs communs entre deux modèles |
| Escalation depth | Nombre moyen de niveaux nécessaires avant succès |
| Escalation cost | TTCS/tokens/énergie cumulés de toute la chaîne |
| Expected TTCS | Temps attendu jusqu'à une solution correcte, escalades incluses |
| Route accuracy | Fréquence à laquelle le routeur choisit directement une configuration efficiente |
| Retrieval recovery rate | Échecs récupérés par ajout de contexte sans escalade compute |
| KV/prefix reuse | Part du contexte commun effectivement réutilisée |
| Context efficiency | Succès obtenu par token de contexte injecté |
| Quality-adjusted tasks/hour | Travail correct produit par heure en tenant compte du taux de réussite |

---

## 17. Architecture cible

La cible finale est une boucle :
$$\text{Task} \to \text{Fingerprint} \to \text{Experience Retrieval} \to \text{Adaptive Router} \to \text{Model/Quant/Context/Strategy} \to \text{Execution} \to \text{Verification} \to \text{Success ou Escalation} \to \text{Experience Store}$$

Au départ, le routeur peut être déterministe (règles + statistiques + embeddings). Une fois suffisamment d'expériences accumulées, un petit modèle de routage local peut apprendre à prédire directement la configuration ayant le meilleur coût attendu.

---

## 18. Conséquence pour le protocole expérimental

Le benchmark doit donc produire deux frontières de Pareto :
1. **La frontière par modèle/configuration**
2. **La frontière par chaîne d'escalade**

Le meilleur système n'est pas nécessairement le meilleur modèle individuel : une chaîne composée d'un worker rapide, d'un retrieval efficace et d'un modèle de récupération diversifié peut fournir davantage de tâches correctes par heure avec moins de RAM, de tokens et d'énergie.
