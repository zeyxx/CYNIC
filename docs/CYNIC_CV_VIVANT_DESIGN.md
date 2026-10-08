# CYNIC CV Vivant — Design v0.1

**Status:** Draft incrémental — enrichi session par session.
**Date:** 2026-10-06
**Owner:** Hermes Agent (Titouan Corneloup)

---

## 1. Concept

Le **CV vivant** est une **dérivée** des fonctions internes de CYNIC, pas la cible stricte. C'est une projection publique qui expose ce que CYNIC produit déjà :

1. **CYNIC produit** — les fonctions internes (API REST, agents, pipeline) génèrent du contenu
2. **Le CV vivant expose** — une projection publique de ce contenu (profil, preuves, analyses)
3. **Le recruteur vérifie** — chaque affirmation est prouvée par des données vérifiables

Le CV vivant n'est pas un système séparé. C'est une **surface de lecture** des fonctions internes de CYNIC.

### Ce que CYNIC produit déjà (réalité actuelle)

| Fonction interne | Implémentation | Contenu produit |
|---|---|---|
| **API REST** | `cynic-agent-bus-server` (axum), `cynic-node/builder_http.rs` (axum) | Health, tasks, workloads, proofs |
| **Agents autonomes** | `cynic-protocol` (agents, tasks, claims), `cynic-host` (agent_access) | Coordination, verdicts, observations |
| **Pipeline publication** | `cynic-lifecycle` (ArtifactPublisher, artifact_publication) | Artefacts publiés, preuves |
| **Webhook Forgejo** | `cynic-node/builder_http.rs` (webhook_forgejo) | Réception des push, workloads |
| **Surface registry** | 36 surfaces enregistrées | Inventaire des surfaces |
| **Builder HTTP** | `cynic-node/builder_http.rs` (submit_workload, get_proof) | Soumission de workloads, preuves |

### Ce que le CV vivant expose (projection publique)

| Contenu | Source | Projection |
|---|---|---|
| **Profil** | Système souverain CYNIC (base d'étude: `facts.json`) | Identité, compétences, projets |
| **Preuves** | API CYNIC, `verify_claims.sh` (base d'étude) | Commits, tests, audits, failles |
| **Analyses** | Agents CYNIC (code GitHub) | Qualité, sécurité, architecture |
| **E-Score éducatif** | Métriques CYNIC | CODE, AUDIT, INFRA, DOCS, COMMUNITY |

**Note:** `facts.json`, `render.py`, `verify_claims.sh` sont une **base d'étude** pour le système vraiment souverain dans CYNIC. Ils ne sont pas la source finale.

### Principe de dérivation

```
Fonctions internes CYNIC (API, agents, pipeline)
  → produisent du contenu (profil, preuves, analyses)
  → le CV vivant expose ce contenu (projection publique)
  → le recruteur vérifie les preuves (données vérifiables)
```

Le CV vivant ne produit rien. Il **expose** ce que CYNIC produit déjà.

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    TALARIA SAS (éditeur logiciel)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Code public │  │ Code fermé  │  │ Marques (CYNIC, etc.)   │  │
│  │ (on-chain)  │  │ (UI, ML)    │  │                          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
│         │                │                    │                  │
│         ▼                ▼                    ▼                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    CYNIC (protocole)                         ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
│  │  │ Agents      │  │ Pipeline    │  │ API REST/Websocket  │  ││
│  │  │ autonomes   │→ │ publication │→ │ exposition          │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────┘│
│         │                │                    │                  │
│         ▼                ▼                    ▼                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Forgejo     │  │ GitHub      │  │ Consommateurs           │  │
│  │ (SSOT)      │  │ (exposition)│  │ (CV vivant, etc.)       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CV vivant (projection publique)                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Profil      │  │ Analyses    │  │ Preuves                 │  │
│  │ (identité,  │  │ LLM +       │  │ (commits, tests,        │  │
│  │ compétences│  │ données     │  │ audits, failles)        │  │
│  │ projets)    │  │ vérifiables)│  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Principes

- **CYNIC = moteur autonome** — produit du contenu sans attendre de requêtes
- **Forgejo = source de vérité souveraine** — le dépôt SSOT
- **GitHub = exposition publique** — miroir pour les recruteurs
- **API REST/Websocket = exposition temps réel** — pour le site web et autres consommateurs
- **Site web = hybride** — parties statiques (pré-rendues par CYNIC) + parties dynamiques (consommées via API)

### Autonomie hybride

- **Opérateur humain = constituant** — définit les objectifs, les limites, les valeurs
- **Agents = acteurs** — exécutent, produisent, mettent à jour
- **CYNIC = gouvernement + usine** — coordonne (gouvernement) et produit (usine)

---

## 3. Croisement avec TALARIA

| Concept TALARIA | Rôle dans le CV vivant |
|---|---|
| **TALARIA SAS** (éditeur logiciel) | Le CV vivant est une projection de CYNIC (protocole), pas de TALARIA (société). Mais TALARIA est l'employeur potentiel pour l'alternance. |
| **CYNIC = Protocol Fuel** | Le CV vivant démontre la maîtrise du protocole CYNIC — preuve de compétence. |
| **E-Score = méritocratie on-chain** | Le CV vivant est un E-Score éducatif : CODE, AUDIT, INFRA, DOCS, COMMUNITY. |
| **Open Core Blindé** | Le CV vivant montre le code public mais pas le code fermé (UI, ML, observer). |
| **255 mois / cliff 76 mois** | Le CV vivant est un engagement à long terme — l'alternance est le début d'une trajectoire. |
| **Marques CYNIC, Odysseum, E-Score** | Le CV vivant utilise ces marques comme preuves d'appartenance à l'écosystème. |

### Points de convergence

1. **Le CV vivant est un E-Score éducatif** — même philosophie que l'E-Score crypto, adapté à un alternant.
2. **Le CV vivant est une projection de CYNIC** — pas de TALARIA. TALARIA est l'employeur, CYNIC est le protocole maîtrisé.
3. **Le CV vivant utilise les marques CYNIC** — comme preuve d'appartenance à l'écosystème.
4. **Le CV vivant est un engagement à long terme** — l'alternance est le début d'une trajectoire.

---

## 4. Tensions résolues

### 4.1 Open Core Blindé — RÉSOLU

**Problème:** Le CV vivant montre le code public mais pas le code fermé (UI, ML, observer). Comment montrer la maîtrise du code fermé sans le révéler ?

**Solution:** Le CV vivant expose les **preuves de maîtrise** sans révéler le code fermé.

| Ce qui est public | Ce qui reste fermé |
|---|---|
| Résultats d'analyses (qualité, architecture) | Le code source des analyses |
| Preuves d'audits (failles trouvées, rapports) | Les outils d'audit internes |
| Métriques d'infrastructure (uptime, LOC) | Les configurations internes |
| Spécifications et contrats | Les implémentations propriétaires |

**Principe:** Le CV vivant montre **ce que le code fait**, pas **comment il le fait**. Les agents CYNIC produisent des analyses qui prouvent la maîtrise sans révéler le code fermé.

**Statut:** Résolu.

### 4.2 Séparation TALARIA / CYNIC — RÉSOLU

**Problème:** Le CV vivant est une projection de CYNIC, mais l'alternance est chez TALARIA. Comment faire le lien sans confondre les deux entités ?

**Solution:** Le CV vivant est explicitement une **projection de CYNIC**, avec un lien clair vers TALARIA comme employeur potentiel.

```
CV vivant (projection de CYNIC)
  ├── Profil: "Étudiant en cybersécurité, je maîtrise CYNIC"
  ├── Projets: CYNIC, GASdf, Percolator (preuves de maîtrise)
  └── Lien: "CYNIC est développé par TALARIA SAS — je postule chez TALARIA"
```

**Principe:** Le CV vivant prouve la maîtrise de CYNIC (le protocole), et TALARIA (la société) est mentionnée comme l'employeur. Pas de confusion : CYNIC = protocole maîtrisé, TALARIA = employeur.

**Statut:** Résolu.

### 4.3 Marques — RÉSOLU

**Problème:** Le CV vivant utilise les marques CYNIC, mais elles sont déposées par TALARIA. Comment les utiliser sans violer la stratégie IP ?

**Solution:** Le CV vivant utilise les marques CYNIC avec une **clause de non-affiliation**.

```
Ce CV vivant est une projection publique de CYNIC, un protocole open source.
CYNIC est une marque de TALARIA SAS. Ce document n'est pas affilié à TALARIA SAS.
```

**Principe:** L'utilisation des marques est autorisée pour décrire la maîtrise du protocole, mais avec une clause de non-affiliation pour éviter toute confusion avec TALARIA SAS.

**Statut:** Résolu.

### 4.4 Licence BSL — SUJET D'ÉTUDE

**Problème:** Le code n'est pas sous licence BSL pour l'instant. C'est un sujet d'étude entier lié au lancement de TALARIA qui approche.

**Solution:** Le BSL est un **sujet d'étude séparé** qui n'impacte pas le CV vivant à court terme.

**Calendrier proposé:**
- **T0 (maintenant):** Code open source (MIT/Apache) — le CV vivant montre le code public
- **T1 (lancement TALARIA):** Étude BSL — décider quels fichiers passer en BSL
- **T2 (après BSL):** Le CV vivant continue de montrer le code public, avec une note sur la licence BSL

**Impact sur le CV vivant:** Aucun à court terme. Le code public reste public. Si le BSL est appliqué, le CV vivant ajoute une note: "Certainains fichiers sont sous licence BSL — voir TALARIA SAS."

**Statut:** Sujet d'étude ouvert — à creuser avec le lancement TALARIA.

---

## 5. Pipeline de publication

### 5.1 Principe de dérivation

Le CV vivant ne produit rien. Il **expose** ce que CYNIC produit déjà.

```
Fonctions internes CYNIC (API, agents, pipeline)
  → produisent du contenu (profil, preuves, analyses)
  → le CV vivant expose ce contenu (projection publique)
  → le recruteur vérifie les preuves (données vérifiables)
```

### 5.2 Flux de données

```
CYNIC (fonctions internes)
  → API REST/Websocket expose le contenu
  → Agents CYNIC analysent le code sur GitHub (qualité, sécurité, architecture)
  → Pipeline publication pousse vers Forgejo (SSOT)
  → GitHub Actions synchronise vers GitHub (exposition publique)
  → Site web consomme l'API (statique + dynamique)
```

### 5.3 Étapes

1. **CYNIC produit** — les fonctions internes (API, agents, pipeline) génèrent du contenu
2. **Agents analysent** — les agents CYNIC analysent le code sur GitHub (qualité, sécurité, architecture)
3. **Forgejo stocke** — le dépôt souverain reçoit le contenu
4. **GitHub Actions synchronise** — le miroir public est mis à jour
5. **API expose** — le contenu est servi en temps réel
6. **Site web affiche** — le frontend consomme l'API et affiche le CV vivant

### 5.4 Gating

- **Avant publication:** `verify_claims.sh` doit être vert (OK, pas de FAIL/DRIFT)
- **Avant push:** le contenu doit être validé par un agent autonome
- **Avant exposition:** le site web doit être testé (smoke test)

### 5.5 API CYNIC

L'API REST/Websocket est fournie par CYNIC. Elle expose:

- **Profil** — identité, compétences, projets
- **Analyses** — LLM + données vérifiables (code, sécurité, projets, compétences)
- **Preuves** — commits, tests, audits, failles
- **E-Score éducatif** — CODE, AUDIT, INFRA, DOCS, COMMUNITY

Les agents CYNIC analysent le code sur GitHub et produisent des analyses qui sont servies par l'API.

**Complexité du backend :** L'API REST ne se limite pas à exposer des endpoints. Elle doit gérer :
- Les domaines CYNIC (16 familles, 48 crates)
- Le réseau (Forgejo, GitHub, Vercel)
- La gestion des données (souveraineté, redondance, cohérence)
- La sécurité (authentification, autorisation, audit)

---

## 6. Stack technique

### 6.1 Principe de dogfooding

CYNIC se consomme lui-même. L'API REST expose les fonctions internes, le site web les consomme, les agents CYNIC analysent le code et produisent des analyses. CYNIC est à la fois le producteur et le consommateur.

### 6.2 Stack proposée

| Couche | Technologie | Justification |
|---|---|---|
| **Frontend** | SvelteKit | Léger, statique + dynamique, SSR/SSG, TypeScript natif |
| **Backend** | API REST/Websocket CYNIC (Rust, axum) | Déjà existe dans `cynic-agent-bus-server` et `cynic-node/builder_http.rs` |
| **Hébergement** | Vercel (frontend) + API CYNIC (déployé sur builder) | Vercel pour le statique, builder pour l'API |
| **Build** | Généré par CYNIC, poussé vers Forgejo/GitHub | Pipeline existant |
| **Données** | Système souverain CYNIC (base d'étude: `facts.json`, `verify_claims.sh`) | Sources existantes comme base d'étude |

### 6.3 Backend — Complexité

Le backend ne se limite pas à exposer des endpoints. Il doit gérer :

1. **Domaines CYNIC** — 16 familles, 48 crates, DAG strict
2. **Réseau** — Forgejo (SSOT), GitHub (exposition), Vercel (hébergement)
3. **Données** — Souveraineté, redondance, cohérence
4. **Sécurité** — Authentification, autorisation, audit

### 6.4 Données — Base d'étude

`facts.json`, `render.py`, `verify_claims.sh` sont une **base d'étude** pour le système vraiment souverain dans CYNIC. Ils ne sont pas la source finale.

**Principe :** Le système souverain dans CYNIC doit être capable de produire les mêmes données que `facts.json`, mais de manière autonome et vérifiable.

---

## 7. Site web — Structure

### 7.1 Pages

1. **Profil** — identité, compétences, projets (statique, pré-rendu)
2. **Analyses** — LLM + données vérifiables (dynamique, via API)
3. **Preuves** — commits, tests, audits, failles (dynamique, via API)
4. **E-Score éducatif** — CODE, AUDIT, INFRA, DOCS, COMMUNITY (dynamique, via API)

### 7.2 Types d'analyses LLM

1. **Analyses de code** — qualité, architecture, patterns
2. **Analyses de sécurité** — audits, vulnérabilités, preuves
3. **Analyses de projets** — CYNIC, GASdf, Percolator
4. **Analyses de compétences** — évaluation du profil

---

## 8. Questions ouvertes — RÉSOLUES

### 8.1 Qui génère les parties statiques du site web ?

**Réponse:** CYNIC génère les parties statiques via ses agents autonomes.

Les agents CYNIC produisent le contenu (profil, preuves, analyses) et le poussent vers Forgejo. Le site web consomme ce contenu via l'API REST/Websocket. Les parties statiques sont pré-rendues par CYNIC et servies par l'API.

**Statut:** Résolu.

### 8.2 Quelle stack technique pour le site web ?

**Réponse:** Stack légère et moderne.

- **Frontend:** React ou Svelte (statique + dynamique)
- **Backend:** API REST/Websocket fournie par CYNIC (Rust)
- **Hébergement:** Vercel ou Netlify (statique) + API CYNIC (dynamique)
- **Build:** Généré par CYNIC, poussé vers Forgejo/GitHub

**Statut:** Résolu — à affiner lors de l'implémentation.

### 8.3 Comment gérer les analyses LLM ?

**Réponse:** Hybride — pré-générées + à la volée.

- **Pré-générées:** Les agents CYNIC produisent des analyses régulièrement (ex: quotidiennement) et les stockent sur Forgejo
- **À la volée:** Le site web peut demander une analyse en temps réel via l'API (avec cache)
- **Cache:** Les analyses sont mises en cache pour éviter les appels LLM répétés

**Statut:** Résolu.

### 8.4 Comment ancrer les preuves on-chain ?

**Réponse:** Attestations signées + ancrage optionnel on-chain.

- **Court terme:** Les preuves sont signées par CYNIC (ed25519) et stockées sur Forgejo
- **Long terme:** Les preuves peuvent être ancrées on-chain (Solana) via un smart contract CYNIC
- **Vérification:** Le site web vérifie les signatures et affiche les preuves vérifiées

**Statut:** Résolu — à implémenter.

### 8.5 Quel calendrier de publication ?

**Réponse:** Publication continue + milestones.

- **Continue:** Les agents CYNIC produisent et publient en continu (ex: chaque commit)
- **Milestones:** Des versions stables sont publiées à des dates clés (ex: chaque mois)
- **Gating:** `verify_claims.sh` doit être vert avant chaque publication

**Statut:** Résolu.

### 8.6 Comment mesurer l'activité comme un E-Score ?

**Réponse:** E-Score éducatif avec 5 dimensions.

| Dimension | Mesure | Poids |
|---|---|---|
| **CODE** | Commits, PRs, lignes de code, tests | 0.30 |
| **AUDIT** | Failles trouvées, rapports, preuves | 0.25 |
| **INFRA** | Systèmes déployés, uptime, métriques | 0.20 |
| **DOCS** | Spécifications, contrats, documentation | 0.15 |
| **COMMUNITY** | Contributions open source, collaborations | 0.10 |

**Formule:**
```
E-Score_educatif = Σ(wi × Dimension_i) × Quadratic_Modifier
```

**Statut:** Résolu — à calibrer.

---

## 9. Références

- `contracts/publication.json` — contrat de publication (45 path_rules)
- `docs/CYNIC_TECHNICAL_WHITEPAPER.md` — whitepaper technique
- `docs/CYNIC_SOURCE_OF_TRUTH_HIERARCHY.md` — hiérarchie des sources de vérité
- `docs/CYNIC_OWNERSHIP_CONSTITUTION.md` — constitution d'ownership
- `docs/quarantine/v2-architecture-canon/CYNIC_TALARIA_CORPORATE_DOCTRINE.md` — doctrine TALARIA
- `docs/quarantine/v2-architecture-canon/CYNIC_TOKENOMICS_ARCHITECTURE.md` — architecture tokenomics
- `docs/quarantine/v2-architecture-canon/CYNIC_FOUNDERS_PACT_AND_ALLIES.md` — pacte des fondateurs
- `docs/quarantine/v2-architecture-canon/CYNIC_LEGAL_GENESIS_AND_TREASURY.md` — genèse légale
- `cv/src/facts.json` — source de faits CV
- `cv/src/render.py` — pipeline de rendu CV
- `cv/src/verify_claims.sh` — vérificateur de claims
