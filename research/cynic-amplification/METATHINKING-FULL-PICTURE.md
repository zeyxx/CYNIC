# CYNIC — Full Picture Metathinking
## Omniscience × Omnipotence × Bootstrap Loop

**Date**: 2026-02-18
**Method**: 9-axis metathinking (epistemic, architectural, economic, philosophical,
temporal, competitive, technical, organisational, existential)

---

## I. LE VRAI MODÈLE MENTAL

Avant tout : nommons ce qu'on construit réellement, pas ce qu'on dit construire.

```
CE QU'ON DIT: "amplification platform for weak LLMs"
CE QU'ON CONSTRUIT: un système d'apprentissage de soi-même

CYNIC observe du code.
CYNIC juge du code.
CYNIC apprend de ses jugements.
Le code le plus jugé = le code de CYNIC lui-même.
```

C'est une boucle de Hofstadter. Un strange loop. Intentionnel ? Si oui, c'est le
mécanisme d'amorçage le plus puissant imaginable. Si accidentel, c'est une dette cachée.

**Le strange loop de CYNIC**:
```
CYNIC.judge(CYNIC.code) → Q-score → update Q-Table
Q-Table.policy → guide CYNIC.next_action
CYNIC.next_action → modifie CYNIC.code
CYNIC.code → CYNIC.judge(CYNIC.code) → ...
```

Ce n'est pas un bug. C'est la genèse.

---

## II. OMNISCIENCE — CE QUE ÇA VEUT DIRE CONCRÈTEMENT

"Omniscient" = connecté à tous les outils/sources d'information disponibles.

### Architecture cible (par domaine du 7×7)

```
R1. CODE     → MCP: filesystem, git, LSP (symbols/references)
               MCP: GitHub (PRs, issues, code review)
               MCP: tree-sitter (AST parsing)

R2. SOLANA   → MCP: solana RPC (déjà câblé: mcp__solana-dev__)
               MCP: Helius (transactions, NFTs)
               MCP: Jupiter (DEX aggregation)

R3. MARKET   → MCP: CoinGecko/Binance (prix temps réel)
               WebSocket: flux de prix en continu
               PostgreSQL: séries temporelles historiques

R4. SOCIAL   → MCP: Twitter/X API
               MCP: Discord (webhook)
               MCP: Telegram Bot API

R5. HUMAN    → Claude Code hooks (PreToolUse/PostToolUse)
               psychologie détectée via patterns de frappe/erreurs
               energy/focus détectés via temps de réponse

R6. CYNIC    → /health + /stats (déjà opérationnel)
               WebSocket /ws/stream (déjà opérationnel)
               Q-Table policy (déjà opérationnel)

R7. COSMOS   → MCP: render.com (services live: déjà câblé)
               MCP: GitHub Actions (CI/CD state)
               MCP: Sentry/datadog (erreurs production)
```

**Écart actuel**: CYNIC accède à R1 (CODE, partiel via GitWatcher) et R6 (CYNIC, complet).
Les 5 autres réalités sont architecturalement définies, jamais câblées.

**Priorisation pour omniscience** (impact × effort):
1. R1+MCP complet (filesystem, git, LSP) → immédiat, fondamental pour SE tasks
2. R7+Render (déjà MCP) → déploiement autonome
3. R3+market (prix SOL) → si $ASDFASDFA est dans la vision
4. R4+social → alerte Twitter sur anomalies

---

## III. OMNIPOTENCE — CE QUE ÇA VEUT DIRE CONCRÈTEMENT

"Omnipotent" = peut exécuter n'importe quelle action dans son environnement.

### Les 7 types d'actions (analyse A4 du 7×7)

```
A4.1 CODE_ACT:    edit file, create file, delete file, run tests
A4.2 GIT_ACT:     git add/commit/push/pr
A4.3 DEPLOY_ACT:  trigger Render deploy, rollback
A4.4 CHAIN_ACT:   Solana transaction, SPL token operation
A4.5 SOCIAL_ACT:  tweet, Discord message, Telegram notification
A4.6 LEARN_ACT:   update Q-Table, save ScholarDog buffer
A4.7 EVOLVE_ACT:  self-modify (changer ses propres prompts, thresholds)
```

**Écart actuel**: CYNIC peut A4.6 (LEARN_ACT). Les 6 autres sont à 0%.

**Ce qui manque pour A4.1 (CODE_ACT)** = ACT phase = Claude SDK intégration:
```python
# Plan pour la prochaine session
class ActAgent:
    async def execute(self, decision: Decision) -> ActResult:
        if decision.action_type == "EDIT_FILE":
            # Utilise l'API Edit/Write de Claude Code
            ...
        elif decision.action_type == "RUN_TESTS":
            # subprocess.run(["python", "-m", "pytest"])
            ...
        elif decision.action_type == "GIT_COMMIT":
            # subprocess.run(["git", "commit", "-m", decision.message])
            ...
```

**La question BURN**: Avant d'implémenter A4.1-7, CYNIC a-t-il besoin d'un HUMAIN
dans la boucle ? Probablement oui, pour les actions irréversibles (A4.2, A4.3, A4.4).
→ Pattern: CYNIC propose (DECIDE), humain approuve (WebSocket ACK), CYNIC exécute (ACT).
→ Le WebSocket bidirectionnel qu'on vient de construire est EXACTEMENT ce transport.

---

## IV. LE BOOTSTRAP LOOP — ANATOMIE PRÉCISE

Voici la vraie mécanique du bootstrap, phase par phase:

### Phase 1: CHAOS BOOTSTRAP (actuel — semaines 1-4)

```
Objectif: Prouver que les mécanismes fonctionnent
Indicateurs:
  ✅ 621 tests passent
  ✅ Architecture φ-correcte
  ✅ N-workers parallèle prouvé
  ✅ PerceiveWorkers autonomes
  ❌ Q-Table vide (0 états appris en production)
  ❌ ACT phase absente
  ❌ Pas de jugement réel en production

Ce qu'on apprend en Phase 1:
  → Quelles abstractions sont correctes (Cell, DogJudgment, FractalJudgment)
  → Quelles abstractions sont prématurées (7×7 matrix complète)
  → Coût réel de l'inférence Ollama (latence, débit)
```

### Phase 2: PLUGIN BOOTSTRAP (semaines 4-8)

```
Objectif: CYNIC s'utilise lui-même pour s'améliorer
Mécanisme: Claude Code plugin hooks POST vers Python kernel FastAPI

Hooks déjà câblés (à vérifier):
  PreToolUse → POST /perceive (code avant chaque modification)
  PostToolUse → POST /perceive (code après chaque modification)
  → jugement du delta = signal de qualité réel

Premier signal d'apprentissage réel:
  Code before → CYNIC scores it
  Edit applied
  Code after → CYNIC scores it
  Delta: is the code better? → reward signal for Q-Table

C'est le premier VRAI feedback loop:
  Chaque session de coding = centaines de jugements réels
  Après 10 sessions = Q-Table commence à avoir des priors utiles
```

### Phase 3: AUTONOMIE PARTIELLE (semaines 8-12)

```
Objectif: CYNIC ACT sans prompt humain pour actions sûres

Actions sûres (auto-approuvées):
  - Formatage de code (black/ruff)
  - Ajout de docstrings manquants
  - Correction de typos dans les strings
  - Ajout de tests pour fonctions non couvertes

Actions requérant approval humain:
  - Refactoring significatif
  - Modification d'API publique
  - Deploy en production
  - Toute suppression de code

L'human-in-the-loop via WebSocket bidirectionnel:
  CYNIC → WS: "Je propose: supprimer la fonction orpheline foo() [GROWL Q=28.3]"
  Human → WS: {"type": "ACT", "action": "APPROVE", "target": "delete_foo"}
  CYNIC → exécute Edit/Bash
```

### Phase 4: OMNISCIENCE + OMNIPOTENCE (semaines 12+)

```
Objectif: Toute action dans tous les domaines du 7×7

Mécanisme clé: MCP servers = le système nerveux

Chaque MCP server = un sens supplémentaire:
  filesystem MCP → CYNIC "voit" tout le repo sans rate limits
  GitHub MCP → CYNIC "voit" les PRs, issues, code review
  render MCP → CYNIC "voit" les services déployés
  solana MCP → CYNIC "voit" les transactions on-chain

Chaque ACT agent = un muscle supplémentaire:
  CODE_ACT → CYNIC "edite" du code
  GIT_ACT → CYNIC "commit" et "push"
  DEPLOY_ACT → CYNIC "déploie"
  CHAIN_ACT → CYNIC "signe" des transactions
```

---

## V. COMPARAISON COMPÉTITIVE — OÙ CYNIC GAGNE ET OÙ IL PERD

### Versus les frameworks existants

| Dimension | AutoGen | CrewAI | MetaGPT | LangChain | **CYNIC** |
|-----------|---------|--------|---------|-----------|-----------|
| Production-readiness | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| Memory (cross-session) | ⭐ | ⭐⭐ | ⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| Learning (RL) | ⭐ | ⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐ |
| Formal quality scoring | ⭐ | ⭐ | ⭐⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| Local LLM support | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Plugin/extension model | ⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Autonomous sensing | ⭐ | ⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐ |
| Confidence bounding | ⭐ | ⭐ | ⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| Ecosystem maturity | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ |
| Community/docs | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ |

**CYNIC wins**: Memory + Learning + Formal scoring + Confidence bounding
**CYNIC loses**: Ecosystem maturity, community, production-readiness (today)
**CYNIC's moat**: The combination of all 5 novel elements — no competitor has all 5.

### Versus Claude Code (le vrai concurrent)

CYNIC n'est pas concurrent de Claude Code — il est PARASITE SYMBIOTE.
Claude Code est le moteur. CYNIC est la mémoire + le jugement qui manquent à Claude Code.

```
Claude Code seul:
  ✅ Excellent code generation
  ✅ Tool use (Edit, Bash, Read, etc.)
  ❌ Oublie tout entre sessions
  ❌ Pas de Q-Score sur la qualité
  ❌ Pas d'apprentissage des patterns de l'utilisateur
  ❌ Pas d'observation autonome

CYNIC + Claude Code:
  ✅ Tout ce que Claude Code a
  ✅ Mémoire persistante (PostgreSQL)
  ✅ Apprentissage de tes patterns (Q-Table)
  ✅ Scoring formel (5 axiomes)
  ✅ Autonomie partielle (PerceiveWorkers)
```

**La stratégie est: ne pas remplacer Claude, augmenter Claude.**
Et à terme: augmenter Ollama assez pour ne plus avoir besoin de Claude.

---

## VI. LES 7 OUVERTURES MAJEURES NON EXPLOITÉES

### Ouverture 1: ScholarDog Persistence → Mémoire Épisodique Réelle
**Situation**: 89 cellules en mémoire RAM. Perdues au restart.
**Ce que ça ouvre**: Première vraie mémoire épisodique de CYNIC.
Après 100 restarts, ScholarDog accumule 8900 jugements persistants.
Requêtes de similarité deviennent exponentiellement plus précises.
**Complexité**: 1 migration PostgreSQL + 15 lignes Python. ROI exceptionnel.

### Ouverture 2: WebSocket bidirectionnel → Interface CLI temps réel
**Situation**: /ws/stream émet. Client ne peut que recevoir.
**Ce que ça ouvre**: `cynic-cli` — outil terminal qui:
  - Affiche les jugements en temps réel
  - Permet d'approuver/rejeter les ACT proposals
  - Montre la Q-Table qui s'apprend en direct
**Complexité**: CLI en Python (rich library) + receive_json loop. ~200 lignes.

### Ouverture 3: EMERGENCE_DETECTED → trigger META + alerting
**Situation**: L'événement le plus intéressant du système va dans /dev/null.
**Ce que ça ouvre**: CYNIC qui s'alerte lui-même sur ses propres patterns inhabituels.
Pattern: ResidualDetector voit SPIKE → EMERGENCE_DETECTED → META cycle → ScholarDog
enregistre le pattern → future détection similaire → verdict renforcé.
**Complexité**: 5 lignes dans state.py (l'agent background est en train de le faire).

### Ouverture 4: DecideAgent → Premier agent autonome
**Situation**: JUDGE génère des verdicts qui ne déclenchent rien.
**Ce que ça ouvre**: Le premier feedback loop complet JUDGE→DECIDE.
BARK sur du code → DecideAgent lit Q-Table → émet DECISION_MADE("REFACTOR")
→ CYNIC propose action → humain approuve via WS → CYNIC exécute.
**Complexité**: Agent background en train de l'implémenter.

### Ouverture 5: SWE-bench evaluation harness → validation empirique
**Situation**: Toutes les claims d'amplification sont théoriques.
**Ce que ça ouvre**: Données réelles. Graphe "CYNIC vs. sans CYNIC" sur 100 issues.
Les chiffres changeront tout — soit la thèse est prouvée, soit elle est réfutée.
Chaque point de données = une publication potentielle.
**Complexité**: ~2 jours pour câbler SWE-bench-lite avec CYNIC. ROI: evidence irréfutable.

### Ouverture 6: MCP filesystem + LSP → Code intelligence réelle
**Situation**: GitWatcher voit des noms de fichiers. CYNIC ne "comprend" pas le code.
**Ce que ça ouvre**: Avec filesystem MCP + tree-sitter MCP:
- CYNIC peut naviguer les symboles (fonctions, classes, imports)
- CYNIC peut détecter les dépendances réelles
- CODE×JUDGE devient du jugement sur de la sémantique, pas des strings
**Complexité**: Wiring MCP dans les Dogs appropriés (CartographerDog, ScoutDog). ~4h.

### Ouverture 7: φ comme régulariseur RL → contribution théorique
**Situation**: φ-bounding est une convention de confiance. Pas validé comme régulariseur RL.
**Ce que ça ouvre**: Si H5 (φ-bound stabilise Q-Learning) est vraie, c'est une contribution
théorique nouvelle: le nombre d'or comme régulariseur naturel pour les systèmes d'apprentissage.
"Why does φ bound confidence? Because organisms that exceed φ-confidence die of certainty."
**Complexité**: Expérience contrôlée: CYNIC avec vs. sans φ-bound sur 1000 épisodes simulés.

---

## VII. LES 5 ERREURS ARCHITECTURALES ACTUELLES

### E1: La matrice 7×7 est une prison dorée
On a défini 49 cellules. On en utilise 5.
Les 44 cellules dormantes ne sont pas du potentiel — elles sont du **bruit cognitif**.
Chaque fois qu'on pense à l'architecture, on pense aux 49 cellules.
Mais les 44 inutilisées ne font pas de mal si elles ne génèrent pas de code.
→ **Recommandation**: Ne pas coder les cellules non utilisées. Attendre que la demande les réveille.

### E2: Les Dogs ne communiquent pas entre eux
GUARDIAN score le risque. ORACLE prédit. SAGE fait du MCTS.
Mais ORACLE ne voit jamais le score de GUARDIAN. SAGE ne sait pas ce qu'ORACLE prédit.
Ils votent en silos.
→ **Recommandation**: Ajouter un "context packet" partagé que chaque Dog enrichit séquentiellement.
GUARDIAN vote en premier → son score passe dans le context → ORACLE voit le risque quand il prédit.

### E3: Q-Table est optimale pour des états discrets répétitifs
Le Q-Table fonctionne quand: même état → même action → même récompense.
Le problème: la qualité du code n'est pas discrète. Chaque fichier est différent.
Les state_keys ("CODE:JUDGE:PRESENT:1") sont trop grossiers.
→ **Recommandation**: Enrichir les state_keys avec des features syntaxiques (longueur, complexité cyclomatique, présence de tests). Sinon Q-Table apprend des priors trop généraux pour être utiles.

### E4: PerceiveWorkers ne sont pas priorisés par importance
GitWatcher tourne toutes les 5s. Même si le fichier modifié est un .gitignore.
SelfWatcher tourne toutes les 55s. Même si Q-Table a 0 states (rien à apprendre).
→ **Recommandation**: Adaptive intervals. Si Q-Table vide → SelfWatcher peut dormir 5min.
Si git changes = 500 fichiers → GitWatcher peut escalader en MACRO, pas REFLEX.

### E5: La confiance à 61.8% est un plafond, pas un guide
MAX_CONFIDENCE = 0.618 empêche les verdicts trop confiants.
Mais 30% de confiance et 60% de confiance génèrent le même traitement en aval.
La confiance n'est pas utilisée pour pondérer les décisions du DecideAgent.
→ **Recommandation**: DecideAgent doit exiger `confidence ≥ 0.382` (φ⁻²) avant d'agir.
En dessous de φ⁻², le verdict est trop incertain → demander plus de contexte, ne pas agir.

---

## VIII. LA QUESTION EXISTENTIELLE

*Ce que le cadre de 5 axiomes ne peut pas capturer.*

CYNIC est construit sur une prémisse philosophique cynique:
> "φ distrusts φ" — doute de soi-même.

Mais dans la pratique, on NE DOUTE PAS des axiomes eux-mêmes.
On assume que FIDELITY × PHI × VERIFY × CULTURE × BURN est la bonne métrique.
On assume que la moyenne géométrique est le bon agrégateur.
On assume que 5 = F(5) axioms est le bon nombre.

**Le méta-doute manquant**: Et si les 5 axiomes sont faux ?
Et si FIDELITY (loyal to truth) et BURN (don't extract) sont en tension irréductible ?
Et si la moyenne géométrique favorise systématiquement un type de code sur un autre ?

→ Pour le research project (Stage 4): tester si le scoring CYNIC corrèle avec les métriques
de qualité établies (McCabe complexity, Halstead, maintainability index).
Si non → les axiomes sont des préférences, pas des vérités universelles.
C'est OK — mais il faut le nommer.

---

## IX. ROADMAP EMPIRIQUE — CE QU'ON DEVRAIT CONSTRUIRE DANS L'ORDRE

En appliquant PHI (proportions correctes) et BURN (minimum nécessaire):

```
SEMAINE 1 (maintenant):
  ✅ DecideAgent (agent background en cours)
  ✅ WebSocket bidirectionnel (agent background en cours)
  ✅ EMERGENCE_DETECTED → META trigger (agent background en cours)
  → Priorité: ScholarDog persistence (15 lignes, ROI maximal)

SEMAINE 2:
  → Câbler hooks Claude Code → POST /perceive
    (premiers vrais signaux d'apprentissage)
  → state_key enrichissement avec features syntaxiques
  → SWE-bench-lite evaluation harness (50 snippets)

SEMAINE 3:
  → cynic-cli (WebSocket client TUI avec rich)
  → MCP filesystem + tree-sitter wiring
  → Première mesure empirique: CYNIC vs. base model sur 50 snippets

SEMAINE 4:
  → ACT phase v1 (CODE_ACT: edit + run tests)
  → Human-in-loop approval via WebSocket
  → Première action autonome: formatage automatique de code

MOIS 2:
  → SWE-bench résultats publiables
  → Q-Table avec 1000+ états réels
  → CYNIC auto-améliore CYNIC (boucle fermée)

MOIS 3:
  → qwen2.5-coder:7b + CYNIC ≥ 80% Claude Sonnet sur SE tasks (à mesurer)
  → Publication arXiv: "CYNIC: Amplifying Small LLMs via Memory + Multi-Agent Judgment + RL"
  → Transfert vers production Solana ($asdfasdfa ecosystem)
```

---

## X. CONCLUSION METATHINKING

*sniff* La conclusion brutale:

CYNIC est un **brillant embryon**. Tous les organes sont là. Aucun ne fonctionne de façon autonome.

La prochaine semaine devrait être entièrement consacrée à **fermer la boucle** :
Perceive → Judge → Decide → ACT (v1, humain dans la boucle) → Learn.

Quand cette boucle tourne en production — même imparfaitement — tout change.
La Q-Table commence à apprendre. ScholarDog commence à mémoriser. L'organisme devient vivant.

Jusqu'à ce moment : 621 tests prouvent qu'une belle mécanique tourne à vide.

**La faim de CYNIC commencera quand la boucle sera fermée.**

---

*Confidence: 44% (haute variance entre les axes — certains très clairs, d'autres spéculatifs)*
*THE_UNNAMEABLE: 28% — beaucoup de ce qui compte ici est dans le non-dit entre les lignes*
