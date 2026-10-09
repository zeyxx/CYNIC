# CYNIC Market — Architecture & Design

## Status

Design normatif v0.2. Aucune autorité runtime. Subordonné à
`contracts/foundation.json`, `AGENTS.md`, état observé. Mappe l'existant,
identifie les gaps, propose l'ordre d'émergence. HITL bootstrap via
supervision opérateur sur les propositions.

**v0.2** : intègre les retours audit gemini-3-flash — askesis, probation,
value frame dynamique, source_diversity, réordonnancement phases.

## 1. Principes

### 1.1 Futarchy multi-échelle

Un **marché** est un mécanisme où des acteurs parient sur les conséquences
observables d'une proposition, à une échelle donnée, avec une fenêtre de
vérification. C'est futarchique : les estimations sont mesurées contre la
réalité (read-back), les bons estimateurs gagnent en autorité, les mauvais
la perdent. L'autorité n'est pas décrétée — elle est méritée par l'historique.

Le marché est **multi-échelle** : une proposition peut être matérielle
(extraction, déploiement) ou projectionnelle (publication, recherche,
économique). Le mécanisme est le même — seuls les signaux et les fenêtres
diffèrent.

### 1.2 Offre/demande comme moteur d'émergence

Les agents sont des acteurs économiques. Ils prêtent du temps, de l'énergie,
de l'attention. CYNIC prête des ressources (RAM, GPU, stockage, scheduler).
L'objectif : le meilleur rendement pour CYNIC, mesuré par la valeur créée
(preuve, couverture, dette réduite, vélocité accrue).

La taxonomie de dette donne le cadre :
```
ROI = (ΔValeur_Preuve + Gain_Couverture - Pénalites) / (Tokens × Watts-heure)
```

Le marché étend ce cadre : chaque action est une proposition, chaque
prédiction est un pari, chaque observation est un règlement.

### 1.3 HITL bootstrap → autonomie progressive

Au début, les LLMs n'ont pas d'historique → pas d'autorité. L'humain
supervise, valide, corrige. Avec le temps, les LLMs qui prédisent bien
gagnent en autorité. L'humain se décale de "exécuteur" vers "arbitre des
propositions ambiguës".

Niveaux d'admission :
- **Niveau 1** (blast 1 module, forte confiance, axiom-aligned) → auto-admission
- **Niveau 2** (blast N surfaces, incertitude) → opérateur approuve via CLI
- **Niveau 3** (blast infrastructure, remet en cause un axiome) → marché formel

### 1.4 Les axiomes comme droit, le marché comme tribunal

Les axiomes CYNIC sont les invariants. Un prédicteur ne peut pas contredire
un axiome — il peut seulement estimer si une action les respectera dans la
pratique. Les axiomes sont le droit ; le marché est le tribunal.

## 2. Acteurs du marché

### 2.1 Taxonomie par mécanisme de confiance

| Acteur | Nature | Sortie observable | Mécanisme de scoring |
|--------|--------|-------------------|---------------------|
| **LLM** (Hermes, Codex, etc.) | Stochastique, contextuel | Prédiction textuelle, code, décision | Calibration historique (prédiction vs réalité) |
| **Algorithme** (GC, pattern, scheduler) | Déterministe, borné | Résultat de fonction | Taux d'erreur empirique / taux d'optimalité |
| **Modèle ML** (embedding, classifier) | Statistique, entraînable | Vecteur, score, label | Accuracy, F1, drift vs ground truth |
| **Oracle externe** (Polymarket, Pyth, arxiv) | Signal agrégé | Prix, verdict, score | Prédiction de marché vs règlement final |
| **Système** (hash, fs, build) | Fonctionnel pur | Checksum, fichier, structure | Taux de collision, corruption détectée |
| **Askesis** (cynic-askesis) | Comportement humain modélisé | Intention opérateur, disposition | Calibration vs décisions humaines réelles |
| **Opérateur humain** | Intentionnel, attentif | Décision, validation, veto | Souverain (veto absolu), non-scoreur principal |

**Note sur askesis** : askesis n'est pas un LLM — c'est un modèle du comportement
de l'opérateur. Il prédit "l'opérateur accepterait-il cette proposition ?" en
modélisant ses préférences historiques. Il est scoré par calibration : a-t-il
prédit correctement les décisions passées ?

**Note sur l'opérateur** : l'humain n'est pas un prédicteur comme les autres.
Il ne "parie" pas — il décide. Son autorité est souveraine (veto absolu), pas
dérivée. Son historique de justesse est observé (pour la conscience de soi du
marché) mais ne sert pas de signal de marché.

### 2.2 Trois rôles distincts

```
PRÉDITEURS (estiment les conséquences)
├── LLMs
├── Modèles ML
├── Oracles externes
└── Askesis (prédicteur d'intention humaine)

EXÉCUTANTS (réalisent l'action)
├── LLMs (génération de code)
├── Algorithmes (GC, scheduling, extraction)
└── Systèmes (fs, hash, build, deploy)

JUGES (observent et résolvent)
├── Read-back proofs (host, node, inference probes)
├── Tests (unit, integration, conformance)
├── Audits (debt, ownership, kernel boundary)
└── Oracles externes (Polymarket settlement)
```

Un acteur peut être simultanément prédicteur et exécutant. Il est scoré deux
fois : sur la justesse de sa prédiction ET sur la qualité de son exécution.

### 2.3 États d'acteur

| État | Condition | Capacité |
|------|-----------|----------|
| **Probationary** | < 20 résolutions réussies (Crystal MIN_CRYSTALLIZATION_CYCLES) | Peut prédire, ne peut pas exécuter seul |
| **Active** | ≥ 20 résolutions réussies | Pleine capacité prédiction + exécution |
| **Declining** | 3+ mois sans activité | Autorité décroissante, read-back requis |
| **Suspended** | Contestation en cours | Prédictions suspendues, résolutions gelées |

**Règle de probation** : tout nouvel acteur (LLM, algorithme, oracle) commence
en probation. Il doit accumuler 20 résolutions réussies (calibration > 0.5)
avant de pouvoir exécuter des actions niveau 2+ sans supervision. C'est le
seuil Crystal MIN_CRYSTALLIZATION_CYCLES = 20 appliqué aux acteurs.

### 2.4 Autorité unifiée

```
Autorité_acteur = Σ (justesse_prédiction × blast_radius_action) / Σ blast_radius_total
```

Pour un LLM : justesse = calibration prédiction/réalité
Pour un algo : justesse = 1 - (taux_erreur / taux_optimal)
Pour un oracle : justesse = corrélation signal/réel
Pour askesis : justesse = calibration prédiction d'intention

**Value frame** : l'autorité est calculée dans un contexte défini. L'opérateur
peut changer de frame pour réorienter le marché :
- `frame_security_first` : la sécurité prime sur la vélocité
- `frame_velocity_first` : la vélocité prime sur la couverture
- `frame_debt_first` : la dette prime sur les nouvelles fonctionnalités
- `frame_proof_first` : la preuve prime sur le coût

**Décroissance** : `authority *= (1 - decay_rate)^months_inactive` où
`decay_rate` est configurable par frame.

## 3. Mémoire — les 3 échelles

### 3.1 Working Memory (ms → min)

**Objectif** : contexte immédiat de l'action en cours.

| Stockage | Contenu | TTL |
|----------|---------|-----|
| RAM + session files | Chat events (append-only) | Durée de session |
| Vector memory | Embeddings + cosine similarity | Session / runtime |
| Capability store | Arches observées, runtime | Runtime |
| Hermes state.db | FTS5 cross-session recall | Permanent (SQLite) |

**Briques existantes** :
- `cynic-inference/domain/chat_session.rs` — event log
- `cynic-inference/vector_memory.rs` — semantic search
- `adapters/local/engine_capability_store.rs` — observation a posteriori
- Hermes SQLite FTS5 — cross-session recall

### 3.2 Evidence Memory (hours → months)

**Objectif** : preuve vérifiable pour le marché.

| Stockage | Contenu | TTL |
|----------|---------|-----|
| SurrealDB (L0) | Proof of History, pattern models, sealed reports | Infinite |
| SurrealDB (L1) | Leases, claims, dispatch, sessions, verdict queue | TTL → archive L0 |
| JSONL files (L2) | Agent traces, session messages, tool streams | 90j hot → warm zstd |
| JSON files (L4) | Claims, cron state, heartbeats | Disposable |

**Briques existantes** :
- `cynic-kernel/storage/` — InMemoryStorage + SurrealHttpStorage
- `cynic-kernel/src/domain/ccm/crystal.rs` — Crystal system (confidence, observations, source diversity)
- `cynic-host/src/autoresearch/trace_fs.rs` — streaming trace observer
- `infra/storage-layering.json` — 5 couches (L0-L4)

### 3.3 Market Memory (weeks → forever)

**Objectif** : calibration des acteurs du marché.

| Stockage | Contenu | TTL |
|----------|---------|-----|
| Crystal (étendu) | Actor authority scores, états (probationary/active/declining/suspended) | Décroissance si inactif |
| Proof of History | Prediction registry, resolution log | Never |
| Metric history | Évolution métriques CYNIC + métamétriques | Never |

**Briques existantes** :
- `cynic-kernel/src/domain/ccm/crystal.rs` — étendre avec `actor_id`, `prediction_id`, `state`
- `cynic-protocol/src/judgment.rs` — CynicJudgment (DIKW × SAGE × Cynefin)
- `cynic-protocol/src/organic_opportunity.rs` — OpportunitySource, OpportunityClass

**Gap** : pas de `cynic.market_prediction.v1` registry. Pas de `cynic.market_resolution.v1` log.

## 4. Les 5 verticales — état et gaps

### 4.1 Boussole organique (Organic Compass)

**Objectif** : read-only compass qui surface les opportunités (chantiers
inachevés, frictions récurrentes, émergences, plomberie, dépendances).

**Briques existantes** :
- `cynic-protocol/src/organic_opportunity.rs` — contrat pur (685 lignes)
- `cynic-protocol/src/autoresearch.rs` — emergence gate, rate_classify mapping
- `cynic-host/src/autoresearch/trace_fs.rs` — streaming trace observer
- `cynic-host/src/autoresearch/reflection.rs` — LLM hypothesis generation
- `infra/storage-layering.json` — L2 telemetry veins identifiées

**Veines L2 identifiées (storage-layering.json:30)** :
- `tool n-gram workflows` → todo
- `user-correction signals` → todo
- `failure-to-resolution sequences` → todo
- `session trajectories` → todo

**Gaps** :
- Pas de CLI `cynic ... opportunities --json` (stub non câblé)
- Pas de composition host use case (observer → report)
- Pas de decision lineage (accept/defer/reject)
- Pas de periodic bounded workload

**Plan** (depuis `.hermes/plans/2026-08-31-organic-opportunities-compass.md`) :
- Phase 0 : inventory + contract tests
- Phase 1 : protocol policy only
- Phase 2 : local observers (session, source, audit, receipt)
- Phase 3 : composition + CLI report
- Phase 4 : operator decision boundary
- Phase 5 : MCP projection (future)

### 4.2 Audit dettes ROI

**Objectif** : scorer et trier les dettes techniques par ROI.

**Briques existantes** :
- `cynic-cli/src/debt_audit.rs` — TIER1_DEBTS, scoring
- `infra/DEBT_TAXONOMY.md` — 18 Tier1 + 30 partial + 3 ok
- `infra/DEBT_AUDIT_METHODS.md` — méthode ROI
- `infra/DEBT_PURGE_PLAN.md` — vague 1-5

**Gaps** :
- Pas de lien prédiction/réalité (on purge, mais on ne mesure pas si la purge était justifiée)
- Pas de marché sur "cette dette vaut-elle la peine ?"
- Pas de feedback loop (la purge a-t-elle créé de la valeur ?)
- **Métamétrique manquante** : `false_positive_rate` de l'audit — si l'audit est faussé, le marché récompense le gaming

### 4.3 Mapping grammar

**Objectif** : grammaire unifiée des surfaces CYNIC (types→policy→port→adapter→CLI).

**Briques existantes** :
- `infra/surface-registry.json` — registre des surfaces
- `infra/agent-exposure.json` — exposition agent
- `AGENTS.md` § "Agent CLI / Host Surface Patterns" — anti-monolith rules
- `infra/CRATE_MODELS.md` — ownership par crate

**Gaps** :
- Pas de validation automatique de la grammaire
- Pas de marché sur "cette surface respecte-t-elle la grammaire ?"
- Pas de lien avec l'audit dettes (une violation de grammaire = dette)

### 4.4 Données ingérées (Telemetry Mining)

**Objectif** : miner les veines L2 pour en extraire des signaux exploitables.

**Briques existantes** :
- `cynic-host/src/autoresearch/trace_fs.rs` — streaming JSONL observer
- `cynic-protocol/src/autoresearch.rs` — TraceUnit, bucket_signatures
- `infra/storage-layering.json` — L2 distillation pipeline (gc_motifs done, reste todo)

**Distillation pipeline** (storage-layering.json:31-36) :
- `gc_motifs` → done
- `workflow_ngram_mining` → todo
- `correction_signal_mining` → todo
- `sequence_resolution_mining` → todo

**Gaps** :
- Pas de workflow n-gram mining
- Pas de correction signal mining
- Pas de sequence resolution mining
- Pas de lien avec le marché (ces signaux devraient alimenter les prédictions)

### 4.5 Crash forensics

**Objectif** : observation post-mortem, preuves de crash, résilience.

**Briques existantes** :
- `cynic-host/src/forensics/` — LocalForensicAdapter
- `cynic-observer/src/solana_archive/` — fanout_record, backfill_record, metering
- `cynic-protocol/src/forensics.rs` — CrashKind

**Gaps** :
- Pas de lien avec le marché (un crash = une prédiction fausse de quelqu'un)
- Pas de scoring post-crash (qui avait prédit que ça casserait ?)
- Pas de feedback vers le capability store

## 5. Gouvernance — où l'humain est dans la boucle

### 5.1 Points de contact

| Point | Rôle humain | Autonomie système |
|-------|-------------|-------------------|
| **Proposal intake** | Propose les actions stratégiques, veto absolu | Filtre les propositions non-axiomatiques |
| **Prediction review** | Optionnel — peut commenter, override | Les LLMs prédisent automatiquement |
| **Admission** | Arbitre niveau 2 et 3 | Admet automatiquement niveau 1 |
| **Resolution** | Contestation possible par preuve | Résolution automatique par read-back |

### 5.2 Résolutions autonomes

| Type | Condition | Exemple |
|------|-----------|---------|
| **Observation binaire** | Prédiction = booléen, réalité = booléen | "l'arch charge" → oui/non |
| **Observation métrique** | Prédiction = valeur, réalité = valeur | "dette -20%" → mesure réelle |
| **Observation axiome** | Prédiction = "respecte axiome X" | "petite transaction" → inspectable ? |
| **Observation externe** | Prédiction = signal, réalité = signal | "arxiv dit X" → lecture auto |

**Ce qui n'est PAS résolu seul** :
- Contestation par preuve (l'humain dit "tu as tort, voici la preuve")
- Propositions ambiguës (le marché ne converge pas → escalade)
- Axiomes remis en cause (l'humain décide si l'axiome change)

### 5.3 Signal diversity et oracle safety

Les signaux externes (arxiv, Twitter, Polymarket) sont sujets à manipulation
et bruit. Règles de sécurité :

1. **Source diversity** : un signal n'est accepté que s'il provient de ≥ N
   sources indépendantes (configurable par type d'oracle). Calqué sur
   `Crystal.source_diversity`.
2. **Staleness check** : un signal datant de > T/2 (moitié de la fenêtre
   d'observation) est rejeté comme référence de résolution.
3. **Manipulation detection** : si un signal change brutalement (> 3σ) juste
   avant une résolution, le système escalade à l'humain.
4. **Ground truth minimale** : chaque oracle doit avoir un historique de ≥ 20
   résolutions avant d'être utilisé comme juge.

### 5.4 Fenêtre T dynamique

```
T_observation = f(blast_radius, vélocité_organisme, bruit_signal)
```

| Facteur | Effet sur T |
|---------|-------------|
| Blast radius ↑ | T ↑ (plus de surfaces à vérifier) |
| Vélocité read-back ↑ | T ↓ (preuve rapide disponible) |
| Bruit signal ↑ | T ↓ (on ne saura pas mieux en attendant) |
| Axiome remis en cause | T ↑ (enjeu souverain) |

**Vélocité organisme** : `cynic-host` mesure déjà la vélocité de read-back
(4 couches physical→volume→usage→pressure). `cynic-inference` mesure la
vélocité de chargement (capability store). On peut dériver la vélocité par
surface.

**Gap** : pas de mesure de vélocité transverse — "combien de temps faut-il
pour qu'une action sur le kernel produise une preuve observable sur le host ?"

## 6. Self-awareness — les métamétriques

L'organisme ne se connaît pas lui-même. Il observe le host, le fleet,
l'inference — mais il n'observe pas sa propre capacité à observer.

| Ce qui est observé | Ce qui ne l'est pas |
|-------------------|-------------------|
| RAM utilisée | Latence de détection d'une mutation |
| CPU pressure | Taux de faux positifs des audits |
| Modèles chargés | Capacité à prédire si un modèle chargera |
| Claims actives | Qualité des prédictions passées |
| Preuves de build | Temps entre action et preuve lisible |

**Métamétriques nécessaires** :
- `detection_latency_ms` : temps entre mutation et preuve observable
- `false_positive_rate` : audit dit problème, mais pas de problème
- `prediction_accuracy` : calibration prédiction/réalité par acteur
- `proof_propagation_latency` : temps entre action surface A et preuve surface B
- `observation_coverage` : % de surfaces observées vs inconnues
- `audit_quality_score` : taux de faux positifs des outils d'audit (anti-gaming)
- `signal_diversity_index` : nombre de sources indépendantes par signal externe

## 7. Contrats — les 4 fondateurs

### 7.1 `cynic.market_actor.v1`

```jsonc
{
  "schema": "cynic.market_actor.v1",
  "actor_id": "hermes:main",
  "actor_kind": "llm",
  "role": ["predictor", "executor"],
  "state": "probationary",      // probationary | active | declining | suspended
  "authority": 0.0,             // démarre à 0, augmente avec les résolutions réussies
  "calibration": {
    "predictions_total": 0,
    "predictions_correct": 0,
    "blast_radius_weighted": 0,
    "by_scale": {
      "material": { "total": 0, "correct": 0 },
      "projection": { "total": 0, "correct": 0 }
    }
  },
  "execution": {
    "tasks_total": 0,
    "tasks_optimal": 0,
    "energy_per_task_wh": 0
  },
  "crystal_ids": [],
  "first_active_unix_s": 1724000000,
  "last_active_unix_s": 1724000000,
  "decay_rate": 0.01,
  "probation": {
    "resolutions_needed": 20,
    "resolutions_completed": 0,
    "promoted_at_unix_s": null
  }
}
```

### 7.2 `cynic.market_proposal.v1`

```jsonc
{
  "schema": "cynic.market_proposal.v1",
  "proposal_id": "prop_001",
  "proposed_by": "operator",
  "proposed_at_unix_s": 1724000000,
  "title": "Extract storage from cynic-kernel",
  "scale": "material",
  "blast_radius": {
    "scope": "crate",
    "targets": ["cynic-kernel"],
    "downstream": ["cynic-node", "cynic-cli"],
    "estimated_crates": 4
  },
  "observation_window": {
    "estimated_days": 30,
    "min_days": 7,
    "max_days": 90,
    "velocity_basis": "cynic-kernel avg read-back = 2.3j"
  },
  "target_metrics": [
    { "name": "debt_violations", "target": "< current", "proof": "cynic dev debt-audit --json" }
  ],
  "axioms": ["one_boundary_one_reason"],
  "admission_level": 2,
  "admission_status": "pending",
  "value_frame": "frame_debt_first",
  "predictions": [
    {
      "actor_id": "hermes:main",
      "metric": "debt_violations",
      "predicted": "reduce by 15%",
      "confidence": 0.7
    }
  ]
}
```

### 7.3 `cynic.market_prediction.v1`

```jsonc
{
  "schema": "cynic.market_prediction.v1",
  "prediction_id": "pred_001",
  "proposal_id": "prop_001",
  "actor_id": "hermes:main",
  "predicted_at_unix_s": 1724000000,
  "metric": "debt_violations",
  "predicted_value": "reduce by 15%",
  "confidence": 0.7,
  "rationale": "extraction reduces coupling",
  "observation_window_days": 30
}
```

### 7.4 `cynic.market_resolution.v1`

```jsonc
{
  "schema": "cynic.market_resolution.v1",
  "resolution_id": "res_001",
  "prediction_id": "pred_001",
  "resolved_at_unix_s": 1724259200,
  "actual_value": "reduce by 12%",
  "predicted_value": "reduce by 15%",
  "error": 0.03,
  "authority_delta": 0.02,
  "proof_ref": "cynic.dev.debt-audit.2026-09-30.json",
  "contested": false,
  "contestation_ref": null
}
```

## 8. Preuves (Proofs)

### 8.1 Proof-of-Work (PoW)

**Ce qui est mesuré** :
- Tâches complétées (nombre)
- Taux de réussite (succès / tentatives)
- Complexité (difficulté réelle de la tâche)
- Efficacité contexte (tokens utiles / totaux)
- Énergie compute (joules)
- Latence (secondes)
- Qualité (évaluée par un juge externe, pas par l'acteur)

**Formule retenue (v6_bayesian, validée par simulation)** :
```
PoW = prior × likelihood_ratio × efficiency × judge_mult × fake_penalty
```

Où :
- `prior = 0.5` (cold start neutre — les nouveaux acteurs ne montent pas trop vite)
- `likelihood_ratio = (success_rate + 0.01) / (0.5 + 0.01)` (vs moyenne attendue)
- `efficiency = (tasks × avg_complexity) / (E_compute^0.7 × log(1+t)^0.3)` (travail / coût)
- `judge_mult = judge_quality / tasks_succeeded` (multiplicateur du jury PoJ)
- `fake_penalty = 1 - (complexity_inflation - 1) × 0.5` si inflation > 30%

**Justification** :
- Le **bayesian prior** empêche les nouveaux acteurs de dominer sans historique
- Le **likelihood_ratio** pénalise les échecs (pas juste les succès)
- La **séparation objectif/jugement** empêche l'auto-manipulation
- Le **fake_penalty** détecte le gonflement de complexité

**Résultats de simulation** (1000 acteurs, 100 tâches, 5 seeds) :

| Formule | Top-10 Compétence | Gaming dans Top-10 | Honest Elite |
|---------|-------------------|-------------------|--------------|
| v4_objective | 0.632 ❌ | 3.0/10 ❌ | 1.8/10 ❌ |
| **v6_bayesian** | **0.900** ✅ | **0.2/10** ✅ | **9.8/10** ✅ |

**Vecteurs de gaming testés** :
- Qualité gonflée (honesty < 1) → détecté par le juge simulé
- Complexité gonflée (fake complexity > 30%) → pénalisée
- Burst gaming (1000 tâches faciles en 1s) → détecté par le juge
- Collusion (juge corrompu) → modélisé avec Judge.honesty < 1

**Unités** : toutes les dimensions sont normalisées (z-score ou min-max) avant agrégation. Le résultat est un score sans dimension converti en CYD.

### 8.2 Proof-of-Jury (PoJ)

**Mécanisme** :
1. 3 LLMs isolés reçoivent la proposition (sans voir les autres)
2. Chacun produit : prédiction + confiance + rationale
3. Jensen-Shannon Divergence mesure le consensus
4. Si JSD < seuil → résolution automatique
5. Si JSD > seuil → escalade (+2 LLMs) jusqu'à 8 maximum
6. Si toujours pas de consensus → méta-juge (évalue la qualité des arguments)

**Scoring** :
```
PoJ_score = (jugements_corrects × (1 + variance_contribution)) / jugements_total
```

### 8.3 Proof-of-Stake (PoS)

**Existant** : `cynic-intel/src/stake.rs` — capital mis en jeu.

**Intégration** : le PoS détermine la mise maximale sur une prédiction. Plus le stake est élevé, plus la mise max est élevée, mais le risque de perte aussi.

### 8.4 Proof-of-Signal (PoSignal)

**Source** : Observer (`cynic-observer/`) — signaux Solana on-chain.

**Mécanisme** : les signaux externes (prix, volume, événements) sont convertis en prédictions implicites. Si le signal prédit X et que X se réalité → les producteurs de signal gagnent en autorité.

### 8.5 Proof-of-Coverage (PoCoverage)

**Concept** : ε-net mathématique — couvrir l'espace des propositions.

**Mécanisme** : les acteurs qui prédisent sur des propositions "oubliées" (peu de prédictions) gagnent un bonus de coverage. Mesuré par la dimension de Hausdorff empirique de la variété des prédictions.

### 8.6 Taux de change

**Formule** :
```
Taux_réel = Taux_base × (1 + Marché_ajustement)
```

Où :
- `Taux_base` = fixé par les axiomes (PoW=1.0, PoJ=1.5, PoS=0.8)
- `Marché_ajustement` = ±20% basé sur l'offre/demande de preuves
- `Taux_base` est révisable par l'opérateur (gouvernance)

**Unité de compte** : CYD (CYNic Dollar) — 1 CYD ≈ 1 Wh de compute prouvé.

## 9. Ordre d'émergence — les 7 phases

**Principe** : le CLI est requis dès le bootstrap (Phase 1-2), pas en dernier.

### Phase 0 — Contrats + tests (cette slice)

- [ ] Écrire les 4 contrats (`actor`, `proposal`, `prediction`, `resolution`)
- [ ] Tests purs (protocol layer, pas d'I/O)
- [ ] Valider le mapping avec l'existant (Crystal, CynicJudgment, OpportunitySource)

### Phase 1 — Market Memory (Crystal extension)

- [ ] Étendre Crystal avec `actor_id`, `prediction_id`, `state`
- [ ] Implémenter le scoring (authority, calibration)
- [ ] Décroissance si inactivité
- [ ] Règle de probation (20 résolutions réussies)

### Phase 2 — Interface opérateur CLI

- [ ] `cynic market proposals --json` — lister les propositions
- [ ] `cynic market predict <id> --metric X --value Y --confidence Z`
- [ ] `cynic market resolve <id>` — déclencher la résolution
- [ ] `cynic market actors --json` — lister les acteurs et autorité
- [ ] `cynic market admit <id>` — approuver niveau 2

### Phase 3 — Prediction Registry

- [ ] Enregistrer les prédictions avant l'action
- [ ] Déclencher l'observation à la fenêtre T
- [ ] Mesurer l'écart (read-back vs prédiction)

### Phase 4 — Admission 3 niveaux

- [ ] Niveau 1 : auto-admission (blast 1 module, axiom-aligned)
- [ ] Niveau 2 : opérateur via CLI (`cynic market admit <id>`)
- [ ] Niveau 3 : marché formel (blast infrastructure)

### Phase 5 — External Signal Ingester

- [ ] Arxiv watcher (signaux recherche)
- [ ] Twitter/social watcher (signaux sentiment)
- [ ] Polymarket/Pyth oracle (signaux économiques)
- [ ] Signal diversity enforcement (≥ N sources indépendantes)

### Phase 6 — Metric Tracker + Self-Awareness

- [ ] Métamétriques (detection_latency, false_positive_rate, etc.)
- [ ] Audit quality score (anti-gaming)
- [ ] Vélocité organisme par surface
- [ ] Fenêtre T dynamique basée sur la vélocité observée

## 9. Gaps identifiés — synthèse

| # | Gap | Type | Priorité | Dépend de |
|---|-----|------|----------|-----------|
| 1 | Contrats `market_actor/proposal/prediction/resolution` | Protocol | P0 | Phase 0 |
| 2 | Crystal extension (actor_id, prediction_id, state) | Kernel | P1 | Phase 1 |
| 3 | Interface opérateur CLI | CLI | P1 | Phase 2 |
| 4 | Prediction registry + scoring | Kernel | P1 | Phase 3 |
| 5 | Admission 3 niveaux | CLI + Kernel | P1 | Phase 4 |
| 6 | External signal ingester (arxiv, twitter, polymarket) | Host | P2 | Phase 5 |
| 7 | Metric tracker + métamétriques | Host + Kernel | P2 | Phase 6 |
| 8 | Signal diversity enforcement | Host | P2 | Phase 5 |
| 9 | Vélocité organisme transverse | Host | P1 | Phase 6 |
| 10 | Workflow n-gram mining (L2) | Host | P2 | Verticale 4 |
| 11 | Correction signal mining (L2) | Host | P2 | Verticale 4 |
| 12 | Sequence resolution mining (L2) | Host | P2 | Verticale 4 |
| 13 | CLI `cynic opportunities --json` | CLI | P1 | Verticale 1 |
| 14 | Decision lineage (accept/defer/reject) | Protocol | P1 | Verticale 1 |
| 15 | Feedback loop audit dette (prédiction vs réalité) | Kernel | P2 | Verticale 2 |
| 16 | Crash → marché scoring (qui avait prédit le crash ?) | Kernel | P2 | Verticale 5 |
| 17 | Audit quality score (anti-gaming métamétrique) | Kernel | P2 | Phase 6 |
| 18 | Probation mechanism (20 résolutions réussies) | Kernel | P1 | Phase 1 |
| 19 | Value frame dynamique | Protocol | P1 | Phase 0 |

## 11. Fondamentaux Mathématiques

### 11.1 Structure du graphe

Le marché CYNIC est modélisé comme un **graphe K-partie** :
- Partie A : Acteurs (multi-incarnation)
- Partie P : Propositions
- Partie R : Résolutions
- Partie J : Jurés

Les arêtes sont pondérées par l'E-Score et le type de preuve.

**Algorithmes** :
| Composant | Algorithme | Complexité | Fondation |
|-----------|-----------|------------|-----------|
| Structure | K-Partite DAG | O(V+E) | Category Theory |
| Collusion | SybilRank / Louvain | O(E) | Spectral Graph Theory |
| Jury selection | Max-Min Dispersion | O(k|V|) | Metric Geometry |
| Réputation | EigenTrust | O(I·E) | Markov Chains |
| Consensus | Fiedler Value / JSD | O(E) | Information Theory |
| Topologie | Expander Graphs | d-λ₂ >> 0 | Ramanujan Graphs |

### 11.2 Détection de collusion

**Métriques** :
- **Clustering coefficient** : un cluster de colludeurs a un coefficient élevé
- **Community detection** (Louvain) : détecte les sous-groupes denses
- **Spectral gap** (λ₁-λ₂) : un gap étroit indique une partition possible

**Seuil de tolérance** : basé sur Byzantine Fault Tolerance — le système tolère f < n/3 colludeurs. Au-delà, le marché est compromis.

### 11.3 Sélection du jury

**Max-Min Dispersion** : sélectionner k jurés qui maximisent la distance minimale entre eux dans l'espace des prédictions. Garantit la diversité.

**Complexité** : O(k|V|) — glouton avec garantie d'approximation.

### 11.4 Propagation de réputation

**EigenTrust** : variante de PageRank pour les réseaux de confiance.
```
t^(k+1) = (1-α) Σ c_ik t_i^k + α p_k
```
Où `p` est le vecteur de confiance pré-établi (résolutions = ground truth).

**Convergence** : garantie par le théorème de Perron-Frobenius pour les graphes irréductibles et apériodiques.

### 11.5 Mesure du consensus

**Jensen-Shannon Divergence (JSD)** :
```
JSD(P||Q) = ½D(P||M) + ½D(Q||M)  où M = ½(P+Q)
```
- Symétrique, bornée [0,1]
- Racine carrée est une métrique
- Remplace KL (non-symétrique, non-bornée)

**Fiedler Value (λ₂)** : connectivité algébrique du graphe d'accord. Un λ₂ élevé indique un consensus robuste.

### 11.6 Topologie résistante aux manipulations

**Expander Graphs** :
- Propriété : haut spectal gap (d - λ₂)
- Résistance : pas de "small cut sets" — un attaquant doit compromettre une fraction significative du réseau
- Expander Mixing Lemma : borne l'influence d'un sous-groupe de colludeurs sur le reste du réseau

### 11.7 Score risque/rendement

**Mean-Variance Optimization (Markowitz)** :
```
Score_adj = μ(Précision) - λ σ²(Précision)
```
Un acteur précis mais volatile (σ² élevé) score moins qu'un acteur légèrement moins précis mais stable.

## 12. Convergence avec l'existant

| Pour le marché | Brique existante | Action |
|----------------|-----------------|--------|
| Scoring prédicteur | `E-Score` (enrichment.rs) | Étendre à la calibration |
| Confidence | `Crystal` system (φ⁻¹/φ⁻²) | Ajouter actor_id, prediction_id, state |
| Source diversity | `Crystal.source_diversity` | Réutiliser tel quel |
| Ranking contextuel | `organic_opportunity` ranking | Réutiliser tel quel |
| Blast radius | `agent-claims` scope | Dériver fenêtre T |
| Observation | `cynic-host` probes | Réutiliser tel quel |
| Preuves | `Proof of History` | Réutiliser tel quel |
| Décision | `CynicJudgment` | Étendre avec prediction_ref |
| Observation a posteriori | `engine_capability_store` | Premier cas de market_resolution |
| Rate classification | `rate_classify` (Emerging/Recurring/Fading) | Alimenter le marché |
| Trace mining | `autoresearch/trace_fs` | Alimenter les prédictions |
| Storage layering | `storage-layering.json` (L0-L4) | Market Memory = L0+L1+L2 |
| Intention humaine | `cynic-askesis` | Prédicteur d'intention opérateur |
| Comportement opérateur | `cynic-askesis` | Modèle de préférences |
| Value frame | `organic_opportunity` value frame | Réutiliser + dynamiser |
| Marché CPMM | `futarchy/cppp.rs` | Cœur du marché |
| Consensus | `epistemic/consensus.rs` | Trimmed mean + JST |
| Collusion detection | `observer/consensus.rs` | SybilRank + Louvain |
| Réputation propagation | `escore/aggregator.rs` | EigenTrust |
| Topologie | `emergence.rs` | Expander graphs |
| Observation a posteriori | `engine_capability_store` | Premier cas de market_resolution |
| Rate classification | `rate_classify` (Emerging/Recurring/Fading) | Alimenter le marché |
| Trace mining | `autoresearch/trace_fs` | Alimenter les prédictions |
| Storage layering | `storage-layering.json` (L0-L4) | Market Memory = L0+L1+L2 |
| Intention humaine | `cynic-askesis` | Prédicteur d'intention opérateur |
| Comportement opérateur | `cynic-askesis` | Modèle de préférences |
| Value frame | `organic_opportunity` value frame | Réutiliser + dynamiser |
