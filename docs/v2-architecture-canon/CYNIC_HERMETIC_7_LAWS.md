# CYNIC — Les 7 Lois Hermétiques : Mapping et Audit

> **Version :** 0.1-draft
> **Date :** 2026-09-05
> **Statut :** Projection normative avec audit mesuré du codebase
> **Autorité :** Subordonné à `contracts/foundation.json` et `infra/runtime-authority.json`

---

## Préambule

Ce document formalise le mapping entre les 7 principes hermétiques du Kybalion et l'architecture CYNIC. Il sert à la fois de :
- **Grille de lecture** : comprendre les contraintes de design comme des lois, pas comme des métaphores
- **Diagnostic** : identifier ce qui est vivant, documenté, ou manquant dans le codebase
- **Feuille de route** : prioriser les implémentations par ordre kairotique (juste à temps, pas par ordre de préférence)

---

## Les 7 Lois Hermétiques (Kybalion, 1908)

| # | Principe | Énoncé | Essence |
|---|----------|--------|---------|
| I | **Mentalisme** | "Le Tout est Esprit ; l'Univers est Mental" | La réalité est mentale, la conscience précède la matière |
| II | **Correspondance** | "Comme en haut, comme en bas" | Les plans de réalité sont connectés par analogie |
| III | **Vibration** | "Rien ne repose, tout bouge, tout vibre" | Fréquences, mouvement constant, énergie |
| IV | **Polarité** | "Tout est double, tout a ses paires d'opposés" | Dualité — les contraires sont identiques en nature, différents en degré |
| V | **Rythme** | "Tout s'écoule, au dedans et au dehors" | Cycles, flux/reflux, pendule, marées |
| VI | **Cause et Effet** | "Toute cause a son effet, tout effet a sa cause" | Causalité, déterminisme, pas de hasard |
| VII | **Genre** | "Le genre est en tout, tout a ses principes masculin et féminin" | Création, génération, les deux principes nécessaires |

---

## I. MENTALISME — L'intention humaine est la source

### Principe
> "Le Tout est Esprit ; l'Univers est Mental"

La conscience précède la matière. L'intention humaine est la source de toute valeur dans l'organisme.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| L'Esprit est la Réalité Substantielle | L'intention humaine est la source de toute valeur |
| L'Univers est une projection mentale | Les registres, marchés, preuves sont des constructions de l'esprit |
| La conscience crée la réalité | L'attention de l'opérateur détermine ce qui est réellement important |

### Preuves dans le codebase (audit 2026-09-05)

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `CitizenAccount` (humain souverain) | ✓ 3 refs | `cynic-protocol/src/peer/accounts.rs` |
| `Askesis` (modèle du comportement humain) | ✓ 36 refs | `cynic-askesis/src/` |
| `AgentAccount` (outil, pas source) | ✓ 2 refs | `cynic-protocol/src/peer/accounts.rs` |
| `Admission gate` (humain décide) | ✓ 2 refs | `contracts/foundation.json` |
| Prédiction d'intention | ✗ Manquant | Askesis prédit mais n'est pas connecté |

### Diagnostic
**Documenté mais faible.** Askesis existe comme concept (36 refs) mais ne prédit pas encore les décisions humaines en production. L'intention humaine n'est pas explicitement typée comme source de vérité.

### Ordre kairotique
**Après la boucle build→market.** La prédiction d'intention nécessite des données (les décisions passées), qui nécessitent un marché fonctionnel.

---

## II. CORRESPONDANCE — La fractalité connecte les échelles

### Principe
> "Comme en haut, comme en bas"

Les plans de réalité sont connectés par analogie. Ce qui est vrai à une échelle est vrai à toutes les échelles.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Les plans sont hiérarchisés mais connectés | L0 (Protocol) → L5 (Gouvernement) est un continuum |
| L'analogie est une loi | Chaque nœud CYNIC est un CYNIC complet |
| La fractalité est naturelle | Chaque agent est un CYNIC miniature |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `AgentScope` hierarchy (Fleet→File) | ✓ 53 refs | `cynic-protocol/src` |
| `Budgets monotones` (parents ≥ enfants) | ✓ 10 refs | `cynic-protocol/src/capability_budget.rs` |
| `Récursion agent` (délégation) | ✓ 3 refs | `cynic-protocol/src/agent_delegation_envelope.rs` |
| Fractalité documentée | ✗ 0 refs (dans code) | `AGENTS.md` |
| Échelles L0→L5 typées | ✗ 0 refs | `infra/runtime-authority.json` |

### Diagnostic
**Documenté mais faible.** La fractalité est dans AGENTS.md mais pas dans les types Rust. Les échelles L0→L5 ne sont pas explicitement connectées.

### Ordre kairotique
**Avec le self-modèle.** La correspondance émerge naturellement quand chaque verticale expose son statut.

---

## III. VIBRATION — L'attention et l'énergie sont mesurables

### Principe
> "Rien ne repose, tout bouge, tout vibre"

La vibration est l'essence du mouvement. L'attention et l'énergie sont les vibrations mesurables de CYNIC.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Tout vibre à différentes fréquences | Attention (haute) > Énergie (moyenne) > Matière (basse) |
| La vibration est mesurable | PoH capture les événements, RAPL mesure les joules |
| Le mouvement est permanent | Heartbeat, observation, flux continu |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `Proof of History (PoH)` | ✓ 31 refs | `cynic-kernel/src/storage/` |
| `Heartbeat` | ✓ 112 refs | `cynic-protocol/src` |
| `Energy measurement` (joules/watts) | ✓ 99 refs | `cynic-protocol/src` |
| `E-Score` (7 vertus) | ✓ 2 refs | `cynic-intel/src/escore/types.rs` |
| `Attention humaine` | ✓ 1 ref | `contracts/foundation.json` |
| `Token budget` | ✗ 0 refs | Pas implémenté |

### Diagnostic
**VIVANT.** PoH (31 refs), Heartbeat (112), Energy (99) — la vibration est mesurée. Mais le token budget (attention agentique) manque.

### Ordre kairotique
**Avec la machine à états.** Le token budget est nécessaire pour borner l'attention agentique (session states).

---

## IV. POLARITÉ — Les dualités sont des continus transmutables

### Principe
> "Tout est double, tout a ses paires d'opposés"

Les contraires sont identiques en nature, différents en degré. La transmutation convertit un pôle en l'autre.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Les opposés coexistent | Humain↔Agent, PvP↔PvE, Ouvert↔Fermé |
| La transmutation est possible | Le marché futarchique convertit PvP en signal |
| Les extrêmes se touchent | Baseline↔Cynic (modes de coordination) |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `Admission/Rejection` | ✓ 365 refs | `cynic-protocol/src` |
| `Mode` (Baseline/Cynic) | ✓ 289 refs | `cynic-protocol/src` |
| `Authority/Decay` | ✓ 10 refs | `cynic-market/src/types.rs` |
| `Market` (PvP→signal) | ✓ 34 refs | `cynic-market/src` |
| `Humain/Agent` (Account) | ✓ 21 refs | `cynic-protocol/src/peer/accounts.rs` |

### Diagnostic
**VIVANT.** Admission/Rejection (365 refs), Mode (289 refs) — la polarité est implémentée et mesurée.

### Ordre kairotique
**Déjà vivant.** Pas de travail supplémentaire nécessaire.

---

## V. RYTHME — Les cycles sont inévitables

### Principe
> "Tout s'écoule, au dedans et au dehors"

Les cycles, le flux/reflux, le pendule — tout oscille entre des pôles.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Le balancement du pendule | Authority↔Decay, Build↔Dette, Focus↔Fatigue |
| Les cycles sont mesurables | Heartbeat TTL, observation window, decay rate |
| L'oscillation est constante | Claim→Heartbeat→Release |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `Claim lifecycle` | ✓ 684 refs | `cynic-protocol/src` |
| `Heartbeat TTL` | ✓ 125 refs | `cynic-protocol/src` |
| `Decay d'autorité` | ✓ 5 refs | `cynic-market/src/types.rs` |
| `Observation window` | ✓ 3 refs | `cynic-market/src/types.rs` |
| `Session states` (UNINITED→RELEASED) | ✗ 0 refs | Pas connecté au CLI |

### Diagnostic
**VIVANT.** Claim lifecycle (684 refs), Heartbeat TTL (125) — le rythme est implémenté. Mais la machine à états agent n'est pas connectée.

### Ordre kairotique
**Avant le token budget.** Les sessions agents sont prérequises.

---

## VI. CAUSE ET EFFET — La chaîne causale est inviolable

### Principe
> "Toute cause a son effet, tout effet a sa cause"

Le déterminisme absolu. Rien n'arrive par hasard. La chaîne causale est la loi la plus fondamentale.

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Tout effet a une cause | Chaque transition a une precondition |
| La chaîne est inviolable | Constituent→Identity→Capability→Resource→Workload→Action→Observation→Proof→State |
| Devenir la Cause | L'admission humaine est le point de liberté |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `Read-back proof` | ✓ 429 refs | `cynic-protocol/src` |
| `Admission gate` | ✓ 439 refs | `cynic-protocol/src` |
| `Precondition checks` | ✓ 490 refs | `cynic-protocol/src` |
| `PoH (hash chain)` | ✓ 8 refs | `cynic-kernel/src/storage` |
| `Merkle trees` | ✓ 17 refs | `cynic-protocol/src` |

### Diagnostic
**VIVANT.** Read-back (429), Admission (439), Precondition (490) — la causalité est implémentée et vérifiée.

### Ordre kairotique
**Déjà vivant.** Pas de travail supplémentaire nécessaire.

---

## VII. GENRE — La création nécessite les deux principes

### Principe
> "Le genre est en tout, tout a ses principes masculin et féminin"

Toute création nécessite l'union de deux principes : l'un qui extériorise (masculin), l'autre qui intériorise (féminin).

### Mapping CYNIC

| Hermétique | CYNIC |
|------------|-------|
| Masculin = action, extériorisation | Intention, build, hétéropoïèse |
| Féminin = réception, intériorisation | Attention, observation, autopoïèse |
| Les deux sont nécessaires | L'organisme a besoin des deux pour se reproduire |

### Preuves dans le codebase

| Élément | Force | Emplacement |
|---------|-------|-------------|
| `Hétéropoïèse` (build externe) | ✓ 1269 refs | `cynic-protocol/src` |
| `Autopoïèse` (maintien interne) | ✓ 962 refs | `cynic-protocol/src` |
| `Askesis` (miroir/reflection) | ✓ 28 refs | `cynic-askesis/src` |
| `Intention` (masculin) | ✗ 0 refs | Pas explicitement typé |
| `Attention` (féminin) | ✓ 1 ref | `contracts/foundation.json` |

### Diagnostic
**Documenté mais faible.** Hétéro/Auto sont implémentés (1269/962 refs) mais les principes masculin/féminin ne sont pas explicitement typés.

### Ordre kairotique
**Après le Mentalisme.** La distinction intention/attention émerge avec Askesis.

---

## Synthèse des forces

```
         VIVANT          DOCUMENTÉ         MANQUANT
         (code fort)      (docs seulement)  (à faire)
            │                 │                 │
    ┌───────┴───────┐ ┌──────┴──────┐ ┌────────┴────────┐
    │ Cause/Effet   │ │ Mentalisme  │ │ Token budget    │
    │ Rythme        │ │ Correspond. │ │ Session states  │
    │ Polarité      │ │ Genre       │ │ Échelles L0→L5  │
    │ Vibration     │ │             │ │                 │
    └───────────────┘ └─────────────┘ └─────────────────┘
```

---

## Confluences mesurées

### Confluences fortes (> 100 refs cumulées)

| Combinaison | Refs | Signification |
|-------------|------|---------------|
| II+VII : Correspondance × Genre | 13 258 | Les deux principes à chaque échelle |
| II+VI : Correspondance × Cause/Effet | 5 053 | Causalité identique à chaque échelle |
| II+III : Correspondance × Vibration | 8 546 | Les échelles vibrent à l'unisson |
| I+VI : Mentalisme × Cause/Effet | 3 405 | L'intention comme Cause première |
| I+V : Mentalisme × Rythme | 2 532 | L'intention pulse dans le temps |
| I+II : Mentalisme × Correspondance | 171 | L'intention se reflète partout |

### Confluences faibles (à renforcer)

| Combinaison | Problème |
|-------------|----------|
| I+VII : Mentalisme × Genre | Intention + Attention mal distinguées |
| II : Correspondance | Fractalité documentée mais peu implémentée |
| VII : Genre | Principes masculin/féminin pas typés |

---

## Risques existentiels (lois violées)

| Loi violée | Risque | Manifestation |
|------------|--------|---------------|
| **Mentalisme** | AGI autonome | L'agent prétend avoir ses propres intentions |
| **Correspondance** | Excès de zèle vertical | Une verticale ne reflète pas les autres |
| **Vibration** | Entropie | Bruit, spam, context bloat |
| **Polarité** | Autarcie ou capture | Fermeture totale ou ouverture totale |
| **Rythme** | Burnout organismique | Overwork, accumulation de dette |
| **Cause/Effet** | Hallucination | Proposition sans cause claire |
| **Genre** | Stérilité | 100% humain ou 100% agent |

---

## Ordre Kairotique — Boucler avant d'avancer

Par ordre de dépendance (chaque étape débloque la suivante) :

| # | Étape | Ce qu'elle débloque | Lois renforcées |
|---|-------|---------------------|-----------------|
| 1 | **Machine à états agent** (UNINITED→RELEASED) | Token budget, sessions bornées | Rythme, Vibration |
| 2 | **Self-modèle** (readout des verticales) | Diagnostic organismique | Correspondance |
| 3 | **Token budget** (attention agentique) | Coupe-circuit thermodynamique | Vibration, Rythme |
| 4 | **Market ↔ Build** (boucle complète) | Futarchy opérationnelle | Polarité, Cause/Effet |
| 5 | **Askesis** (prédiction d'intention) | Mentalisme fonctionnel | Mentalisme, Genre |
| 6 | **Échelles L0→L5 explicites** | Fractalité typée | Correspondance |

---

*Ce document est la base de la compréhension hermétique de CYNIC. Il doit être relu à chaque décision architecturale majeure pour vérifier qu'aucune loi n'est violée.*
