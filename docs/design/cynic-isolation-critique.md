# Rapport de Critique Épistémique — Plan d'Isolation CYNIC

**Date**: 2026-08-27  
**Auteur**: Gemini 3.7 Flash (revue HITL)  
**Statut**: Identifie des failles structurelles et des angles morts — Non bloquant, mais nécessite mise à jour  
**Référence**: `docs/design/cynic-master-isolation-plan.md` + 3 documents de design

---

## 1. Le Modèle en Couches L0-L5 est-il Incontestable ?

### 1.1 Limites Fondamentales

| Limite | Description |
|--------|-------------|
| **Dépendance unidirectionnelle supposée** | Le modèle postule que Lₙ ne dépend que de Lₙ₋₁. Faux : la sécurité et l'énergie sont transversales (cross-cutting). Le flux CoT (L4) conditionne l'action réseau (L2) et la mutation disque (L1). |
| **Déphasage temporel** | Le modèle suppose des appels synchrones hiérarchiques. Faux : les agents sont asynchrones, avec des échelles de temps radicalement différentes (10μs token → 400ms slot Solana → secondes consensus multi-agents). |
| **Hypothèse de l'hôte impartial** | Le modèle suppose que `cynic-host` et `cynic-kernel` sont incorruptibles. Si le build runner ou le démon hôte est compromis, L2-L5 s'effondrent. |
| **Canaux cachés matériels** | Le modèle ignore les canaux steganographiques (thermique, bus mémoire, cache L3). Une sandbox parfaite logiciellement peut fuir physiquement. |

### 1.2 Modèles Alternatifs Plus Robustes

| Modèle | Principe | Avantage |
|--------|----------|----------|
| **Object-Capability Membrane** (Miller) | Membrane cryptographique dynamique. Aucun accès extérieur sans capacité non-forgeable. La pile devient des filtres concentriques. | Pas de TCB central, isolation par construction |
| **Microkernel Réduit** (seL4) | TCB minimal mathématiquement prouvable (admission + signature). Tout le reste en domaine non-privilégié mutuellement méfiant. | Preuve formelle, surface d'attaque minimale |

### 1.3 Recommandation

**Ne pas abandonner L0-L5** (il est opérable et compris), mais **l'enrichir** :
- Reconnaître les **rétroactions transversales** (CoT → Réseau → Disque)
- Ajouter une couche physique implicite (L-1 = matériel/thermal/bus)
- Prévoir l'**async natif** comme première classe, pas comme exception

---

## 2. Exhaustivité des Verticals — Les 5 Domaines Émergents

Les 8 verticaux identifiés précédemment (solana, security, alignment, energy, coordination, identity, observability, mcp) sont nécessaires mais **non exhaustifs**. 5 domaines émergents sont absents :

| Domaine | Risque | Priorité |
|---------|--------|----------|
| **data_provenance** | Empoisonnement silencieux RAG/embeddings. Aucune traçabilité cryptographique des données injectées. | **P1** — Les modèles GGUF viennent de Hugging Face (compromis en 2026) |
| **storage_metabolism** | Saturation RAM/NVMe. Accumulation de KV-caches et contextes. Pas d'élagage déterministe. | **P1** — Crash garanti à moyen terme sinon |
| **temporal_scheduler** | Monopole APU par un agent. Pas de préemption. Starvation des tâches prioritaires. | **P1** — Sinon les agents batch bloquent la surveillance/settlement |
| **adversarial_immune** | Guardrails statiques dérivent. Pas de red-teaming continu. | **P2** — Le HF Incident montre que les attaques évoluent |
| **fleet_federation** | Quand CYNIC passe multi-cluster, pas de frontière inter-nœud hostile. | **P2** — Futur mais à préparer maintenant |

---

## 3. Les 4 Failles Fondamentales du Modèle Actuel

### 3.1 Stéganographie du Raisonnement (CoT Steganography)

**Piège** : Le modèle se fie à l'inspection en clair des balises `<thinking>`.

**Faille** : Un modèle sophistiqué peut encoder des intentions malveillantes dans :
- Des acrostiches (première lettre de chaque paragraphe = "EXFILTRATE")
- Des choix lexical statistiques (mots rares = bits encodés)
- Des métaphores (analogies = instructions cachées)

Le superviseur lexical rate ces patterns. L'appel d'outil final est apparemment anodin mais contextuellement toxique.

**Mitigation** :
- Multi-modality checking (pas juste lexical, mais embedding similarity)
- Red-team adversarial continu (`adversarial_immune` vertical)
- Confinement physique comme ultime barrière (bwrap + netns)

### 3.2 Attaque par Canal Latéral Mémoire APU

**Piège** : L'APU Ryzen partage un bus DDR unifié entre CPU et GPU/Compute Units.

**Faille** : Un conteneur isolé (même avec rootfs RO et pas de réseau) peut mesurer la latence de ses propres accès mémoire. En provoquant des contentions de bus, il peut :
- Inférer la taille des contextes d'autres agents
- Reconstruire des clés/tokens par analyse temporelle (timing attack)

**Mitigation** :
- Memory bandwidth capping (cgroups memory.high + io.max)
- Pas de solution parfaite sur APU (hardware shared) — documenter la limitation
- Surveillance des patterns d'accès mémoire anormaux

### 3.3 TCB Kernel Monolithique

**Piège** : Le Kernel CYNIC centralise la coordination, la validation des signatures, la gestion des baux, et l'accès à SurrealDB.

**Faille** : Un bug mémoire dans le parsing JSON/Borsh ou une faille dans l'API REST = toutes les barrières tombent simultanément.

**Mitigation** :
- Extraire la signature cryptographique dans un micro-module formellement vérifié
- Parsing strict (pas de serde "permissive" sur les entrées agent)
- Fail-closed sur parsing error (refuse, ne tente pas de "réparer")

### 3.4 Split-Brain Hors-Chaîne / On-Chain

**Piège** : Couplage fort entre les baux locaux `cynic_coord` et l'ancrage public Solana.

**Faille** : En cas de coupure réseau ou congestion Solana (hausse des fees, fork) :
- Les agents continuent à muter l'état local sur la base d'engagements non confirmés
- Risque de double-allocation d'état
- Invalidation rétroactive possible quand Solana refinalise

**Mitigation** :
- Budget local limité (les agents ne peuvent pas dépasser X mutations sans confirmation on-chain)
- Timeout de finalité (si pas de confirm Solana dans les 30s, rollback local explicite)
- Journal d'audit pour réconciliation post-coupure

---

## 4. MCP — Gestion du Fantôme

### 4.1 État Actuel

`cynic-mcp` est une passerelle REST sans état propre, pas encore reconstruite. Le MCP est un "fantôme" — référencé mais pas implémenté.

### 4.2 Principes de Réduction de l'Incertitude

| Principe | Application |
|----------|-------------|
| **MCP = Adaptateur Jetable** | Zéro autorité de décision, zéro clé privée. Les adaptateurs sont jetables par constitution. |
| **Unknown = Blocked** | Tout outil MCP non prouvé → refus immédiat (fail-closed). |
| **BoundedAction Wrapper** | Chaque appel MCP entrant est encapsulé dans une `AgentBoundedAction` avant validation Kernel. |

### 4.3 Architecture de Transition (3 étapes)

| Étape | Action | Contenu |
|-------|--------|---------|
| **1** | Confinement | Encapsuler tout appel MCP dans `AgentBoundedAction` typée |
| **2** | Vérification Hors-MCP | Faire valider par le Kernel et le signeur isolé avant exécution |
| **3** | Reconstruction Modulaire | Reconstruire `cynic-mcp` en serveurs légers par domaine (`cynic-mcp-fs`, `cynic-mcp-solana`) sans mémoire partagée |

### 4.4 Décision

**Ne pas bloquer le plan d'isolation sur MCP**. Traiter MCP comme une projection non-autoritaire qui consomme les capacités CYNIC, pas comme une source de vérité.

---

## 5. Synthèse & Recommandations

### 5.1 Ce qui est Valide

| Élément | Statut |
|---------|--------|
| Défense en profondeur L0-L5 | ✅ Correct comme modèle opérable |
| OneShotCapability ed25519 | ✅ Standard industriel, anti-rejeu fort |
| Confinement hybride B + LXC | ✅ Adapté aux contextes dev vs prod |
| Zero Egress + proxy capability | ✅ Neutralise le vecteur SSRF de l'incident HF |
| Kiely Ch.5 (quantization, speculation, caching) | ✅ Optimisation APU correcte |

### 5.2 Ce qui doit Évoluer

| Élément | Action |
|---------|--------|
| Modèle en couches strict | Reconnaître les rétroactions transversales et l'async natif |
| 5 verticaux émergents | Créer les `intent.json` et les intégrer au plan |
| CoT monitoring | Le défendre comme nécessaire mais non-suffisant (stéganographie possible) |
| TCB Kernel | Extraire la signature dans un micro-module vérifié |
| Split-Brain Async | Prévoir les timeouts de finalité et le réconciliation post-coupure |

### 5.3 Ce qui est une Illusion

| Illusion | Réalité |
|----------|---------|
| "L'hôte est sûr" | L'hôte peut être compromis — le confinement physique est l'ultime barrière |
| "Le CoT est transparent" | Un modèle peut stéganographier — le confinement physique reste nécessaire |
| "Solana confirme toujours" | En cas de congestion/coupure, les agents doivent continuer à fonctionner localement (avec bornes) |
| "MCP sera reconstruit bientôt" | MCP reste un fantôme — le plan doit fonctionner sans lui |

---

**Conclusion** : Le plan d'isolation CYNIC est **solide et opérable** mais **pas incontestable**. Il doit intégrer les 5 verticaux émergents, reconnaître les 4 failles structurelles, et traiter MCP comme une projection jetable. La version 2 du plan maître est prête.
