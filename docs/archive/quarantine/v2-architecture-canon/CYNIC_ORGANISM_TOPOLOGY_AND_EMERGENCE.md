# CYNIC — Organisme : Topologie, Convergence, Émergence

> **Version :** 0.4-draft
> **Date :** 2026-09-05
> **Statut :** Projection normative avec diagnostic mesuré et plan d'action
> **Autorité :** Subordonné à `contracts/foundation.json` et `infra/runtime-authority.json`

---

## 1. Le Cadre — Agents, Humain, Admission

### 1.1 Le principe

CYNIC est un système conçu pour être **piloté et utilisé par des agents**, où l'humain est l'opérateur final qui **admet** les propositions qui montent des verticales.

```
Agents (proposent, exécutent, observent, simulent)
  ↓
CYNIC (membrane : connecte, prouve, gouverne, observe)
  ↓
Humain (Meta-Constituent : ADMET ou REJETTE)
```

### 1.2 Les rôles

| Rôle | Qui | Ce qu'il fait | Ce qu'il ne fait PAS |
|------|-----|---------------|---------------------|
| **Agent** | LLM, Algorithme, Oracle, Système | Propose des actions, exécute des tâches, observe les conséquences, prédit les résultats | Rien n'admettre dans l'organisme |
| **CYNIC** | Runtime Rust | Connecte les verticales, produit des preuves, gouverne les budgets, observe le substrate | Rien n'admettre sans l'humain |
| **Humain** | Opérateur souverain (Jean) | Admet ou rejette les propositions, définit les axiomes, arbitre les conflits | Pas dans la boucle de contrôle quotidien |

### 1.3 L'admission gate

L'**admission** est le point de souveraineté. C'est l'acte par lequel une proposition (action, modification, politique, brique) entre dans l'organisme.

```
Proposition (agent ou humain)
  ↓
Simulation (verticales existantes prédisent les conséquences)
  ↓
Preuve (observation des conséquences attendues)
  ↓
Décision (humain admit ou rejette)
  ↓
  → ADMIS : l'action est exécutée, la preuve est produite
  → REJETÉ : la proposition est archivée, la raison est tracée
```

### 1.4 Les 3 niveaux d'admission

| Niveau | Blast radius | Décision | Exemples |
|--------|--------------|----------|----------|
| **Niveau 1** | 1 module, axiomes alignés | Auto-admission (agent exécute, humain valide a posteriori) | Correction de typo, test unitaire, observation substrate |
| **Niveau 2** | N surfaces, incertitude | Admission humaine requise avant exécution | Extraction de code, modification de politique, nouvel adapter |
| **Niveau 3** | Infrastructure, axiomes remis en cause | Marché formel + admission humaine | Modification d'axiome, nouvelle verticale, changement de gouvernance |

---

## 2. Le Spectre des Poïèses — Ordre de Construction

### 2.1 Le principe d'émergence

**Règle absolue :** Aucun niveau supérieur ne peut exister sans que les niveaux inférieurs soient fonctionnels et en production.

- Pas de O2 (futarchy) sans O1 (chaîne causale prouvée)
- Pas de O3 (Odysseum) sans O2 opérationnel
- Pas de méta-autonomie sans self-modèle émergent

### 2.2 Le spectre appliqué à CYNIC

| Niveau | Définition | CYNIC cible | État actuel |
|--------|-----------|-------------|-------------|
| **Hétéropoïèse** | Produit un objet extérieur ou est piloté par un agent externe | TALARIA produit du code, des artefacts, des rapports | ✅ Partiel |
| **Autopoïèse O1** | Auto-maintien de sa propre structure | Chaîne causale inviolable, decay, probation, read-back | ⚠️ Lab |
| **Autopoïèse O2** | Génération de sens et couplage sensorimoteur | Boucle thèse → prédiction → résolution → scoring | ❌ Prototype |
| **Autopoïèse O3** | Systèmes sociaux reproduisant de la communication | Odysseum — gouvernance de tokens communautaires | ❌ Aucun |
| **Sympoïèse** | Co-construction collective sans contrôle centralisé | Coordination multi-agents, gaming, écosystèmes | ❌ Aucun |
| **Méta-autonomie** | Modification délibérée de sa propre architecture | Le système observe, propose, simule, modifie ses règles | ❌ Aucun |

---

## 3. Les Verticales — Cartographie et États

### 3.1 Les 7 verticales

| # | Verticale | Niveau cible | État actuel | Prérequis |
|---|-----------|--------------|-------------|-----------|
| V1 | **Substrate Observation** | O1 | ⚠️ Fonctionnel mais noyé dans host | Extraction de host |
| V2 | **Runtime Authority** | O1 | ⚠️ Défini mais chevauchant host/kernel | Clarification frontière kernel/host |
| V3 | **Inference** | O1 | ✅ Fonctionnel | — |
| V4 | **Futarchic Market** | O2 | ❌ Prototype orphelin | V1 + V2 fonctionnels |
| V5 | **Agent Coordination** | O2 | ❌ Concept dispersé | V2 fonctionnel |
| V6 | **Companion (Askesis)** | O2 | ❌ Concept | V3 fonctionnel |
| V7 | **Public Settlement** | O3 | ❌ Stub | V4 fonctionnel |

### 3.2 Les chevauchements toxiques (mesurés)

| Chevauchement | Crates | Gravité | LOC concernés |
|---------------|--------|---------|---------------|
| **Fleet Observation** | host + node | 🔴 Critique | ~2,400 LOC dans host |
| **Runtime Authority** | host + kernel + cli | 🔴 Critique | ~6,000 LOC dans host |
| **Source Control** | host + cli | 🟠 Élevé | ~1,400 LOC dans host |
| **Build** | host + node + cli | 🟠 Élevé | ~2,000 LOC dans host |

### 3.3 Les orphelins

| Crate | Taille | Problème |
|-------|--------|----------|
| `cynic-market` | 6 fichiers | Aucun import réel de cynic-intel (claim faux). Personne ne l'importe. |
| `cynic-zeroclaw-solana` | 4 fichiers | Stub SolanaPay. Aucun import. |
| `cynic-askesis` | 55 fichiers | Pas de connexion avec inference. |

---

## 4. La Convergence — Plan d'Action

### 4.1 Principe

Pas de big-bang. **Construire les verticales à côté de host, puis laisser host se dessécher.** (Strangler Fig Pattern)

Chaque étape doit laisser l'organisme dans un état fonctionnel. Chaque étape produit un livrable utilisable.

### 4.2 Les jalons

| Jalon | Objectif | Livrable | Critère de sortie |
|-------|----------|----------|-------------------|
| **J1** | Connexions minimales | `cynic market create/predict/resolve` fonctionnel | Un use case de futarchy complet (ex: gouvernance de build) |
| **J2** | V1 propre | FleetReport généré par du code isolé | host-fleet-observers devient obsolète |
| **J3** | V2 clair | Chaîne causale exécutée sans violation | kernel = déf, host = exec |
| **J4** | Self-modèle | `cynic dev self-model --json` | Le readout reflète l'état réel |
| **J5** | Méta-autonomie | Premier MetaProposal simulé et admis | Cohérence vérifiée post-modification |
| **J6** | Odysseum | Standard externe | CYNIC autopoïétique ET hétéropoïétique |

### 4.3 Le premier use case : Gouvernance de build

Le use case le plus immédiat : **un agent propose un build, les acteurs prédisent les conséquences, l'humain admit, l'action est exécutée, la preuve est produite.**

```
1. Agent : "Je propose de build cynic-cli"
2. Simulation : "Ça va prendre 15min, 95% de succès, dette -5%"
3. Décision : Humain admit via `cynic market admit <id>`
4. Exécution : BuildRunner est déclenché
5. Preuve : BuildReport produit (succès/échec, durée, dette réelle)
6. Scoring : authority_delta pour l'agent prédicteur
```

Ce use case valide V4 (Market) + V2 (Authority) + V1 (Observation des conséquences) en une seule boucle.

---

## 5. Le Self-Modèle — Émergence, pas Architecture

### 5.1 Principe

Le self-modèle est un **readout agrégé des preuves de chaque verticale**. Il n'est pas écrit à la main.

### 5.2 Construction

```
Chaque verticale produit des preuves (read-back)
  └── L'agrégation de ces preuves = self-modèle global
       └── Le self-modèle permet la méta-autonomie
```

### 5.3 Types Rust

```rust
pub struct SelfModel {
    pub schema: String, // "cynic.self_model.v1"
    pub generated_at_unix_s: u64,
    pub verticals: Vec<VerticalStatus>,
    pub coherence: CoherenceReport,
}

pub struct VerticalStatus {
    pub name: String,
    pub state: VerticalState, // functional | degraded | blocked | planned
    pub last_proof_unix_s: u64,
    pub proof_count: u64,
    pub health: f64, // 0..1
}

pub struct CoherenceReport {
    pub invariants_checked: u32,
    pub invariants_passed: u32,
    pub drift_detected: bool,
}
```

---

## 6. La Méta-Autonomie — Pattern, pas Couche

### 6.1 Principe

La méta-autonomie n'est pas un crate. C'est le **même pattern que la boucle de base**, appliqué au self-modèle.

### 6.2 Le pattern méta

```
Meta-desired state (observé dans le self-modèle)
  → Observe self-modèle
  → Reconcile (qu'est-ce qui peut changer sans briser la cohérence ?)
  → Meta-bounded action (modification de paramètre, pas de code)
  → Read-back coherence proof
```

### 6.3 Types Rust

```rust
pub struct MetaProposal {
    pub schema: String, // "cynic.meta_proposal.v1"
    pub proposal_id: String,
    pub target_rule: RuleRef,
    pub proposed_change: RuleChange,
    pub counterfactual_sim: SimulationResult,
    pub coherence_proof: CoherenceProof,
    pub admitted_by: Option<String>, // None = pending
    pub admitted_at_unix_s: Option<u64>,
}

pub enum RuleChange {
    ParameterUpdate { name: String, old_value: String, new_value: String },
    PolicyUpdate { name: String, old_policy: String, new_policy: String },
    PortUpdate { name: String, old_adapter: String, new_adapter: String },
}
```

### 6.4 Invariants non-négociables

| Invariant | Pourquoi |
|-----------|----------|
| Truth flows upward | Inverser le flux = perdre la réalité |
| Read-back obligatoire | Supprimer la preuve = supprimer la responsabilité |
| Monotonicité des budgets | Permettre l'expansion = permettre la capture |
| Explicit scope | Supprimer le scope = supprimer la bornitude |

---

## 7. Odysseum — La Condition d'Existence

### 7.1 Principe

Odysseum n'est pas un produit CYNIC. C'est un **standard externe** qui utilise CYNIC comme membrane de gouvernance.

### 7.2 Condition d'existence

> Odysseum ne peut exister que quand CYNIC est **à la fois autopoïétique ET hétéropoïétique**.

- **Autopoïétique** : CYNIC se maintient lui-même (chaîne causale, auto-maintien, cohérence)
- **Hétéropoïétique** : CYNIC produit des artefacts externes (preuves, résolutions, engagements)

---

## 8. Hypothèses Implicites — Ce qu'on n'a pas encore questionné

| # | Hypothèse | Question ouverte |
|---|-----------|------------------|
| 1 | **L'extraction de host est-elle vraiment nécessaire maintenant ?** | Strangler Fig évite l'extraction — host reste en place tant que les nouvelles verticales ne le remplacent pas. L'extraction n'est pas un prérequis, c'est une conséquence. |
| 2 | **Le market est-il vraiment le premier use case ?** | Le market est un mécanisme de gouvernance. Mais le premier use case concret est "un agent propose un build, l'humain admit, la preuve est produite". Le market est un outil, pas le but. |
| 3 | **V1 (Substrate) doit-elle être extraite pour que V4 fonctionne ?** | Non. V4 a besoin de preuves d'observation, pas d'une architecture propre. Les fleet observers dans host peuvent produire ces preuves tant qu'ils sont connectés au market via CLI. |
| 4 | **La timeline est-elle réaliste ?** | Les jalons sont des jalons, pas des dates. Le temps réel dépend de l'énergie disponible et des agents. |
| 5 | **Les agents peuvent-ils vraiment prédire les conséquences d'un build ?** | Oui : durée, succès/échec, impact sur la dette. Ces métriques sont mesurables. |
| 6 | **L'humain a-t-il le temps d'admettre chaque proposition ?** | Niveau 1 = auto-admission. Niveau 2 = admission requise. La plupart des actions sont Niveau 1. |
| 7 | **Les types Rust du self-modèle sont-ils corrects ?** | À valider par l'implémentation. |
| 8 | **Odysseum est-il vraiment le but final ?** | Non. Le but final est l'autosuffisance et l'antifragilité. Odysseum est une projection externe possible. |

---

## 9. Ce que ce document ne fait pas

- Il ne décrit pas l'état actuel comme acceptable (il montre les chevauchements toxiques)
- Il ne propose pas de timeline rigide (les jalons sont des jalons, pas des dates)
- Il ne crée pas de nouveaux crates (il réorganise l'existant)
- Il ne décrète pas la méta-autonomie (il la fait émerger)
- Il ne remplace pas `contracts/foundation.json` (il le complète)

---

*Ce document est une projection normative avec diagnostic mesuré et hypothèses identifiées. Il doit être validé par l'observation de l'état réel et ajusté en conséquence.*
