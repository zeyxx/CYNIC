# CYNIC — La Topologie Parfaite (V3.1)

*"L'harmonie naît de la juste proportion des parties avec le tout."* — Axiome PHI

Ce document décrit l'architecture physique du projet CYNIC après la Grande Unification (Février 2026). L'organisme est structuré non pas comme un logiciel classique, mais comme une entité biologique composée de 5 grands systèmes interconnectés via un système nerveux central.

## L'Enceinte de l'Organisme (Racine)

La racine du projet est purifiée pour ne contenir que les éléments essentiels à la survie et au développement du code :

*   `/cynic/` : L'Organisme (Code source).
*   `/tests/` : Les Gardiens (Suite de validation unifiée).
*   `/docs/` : La Sagesse (Spécifications et architecture).
*   `/scripts/` : Les Outils (Maintenance et traces).
*   `/archive/` : L'Histoire (Anciennes visions, code mort, mémorial).

---

## Anatomie Interne (`cynic/`)

L'organisme est divisé en 5 super-systèmes. Aucune logique ne doit fuir d'un système à l'autre sans passer par le système nerveux (Event Bus).

### 1. `kernel/` (Le Cœur & L'Infrastructure)
C'est le Logos. Ce qui est immuable et fondamental.
*   **`core/`** : Mathématiques $\phi$, Axiomes, Event Bus (Système nerveux), Structures d'état unifiées (`UnifiedJudgment`).
*   **`config/`** : La vérité environnementale unique (`CynicConfig`).
*   **`organism/`** : L'Éveilleur (`awaken()`), le `StateManager` (l'Hippocampe/Mémoire unifiée), les gestionnaires d'événements.
*   **`observability/`** : Le `SymbioticStateManager` (La conscience de soi).
*   **`protocol/`** : Protocoles de base.

### 2. `brain/` (L'Intelligence & La Décision)
C'est la délibération. Là où le Doute ($\phi$) s'exerce.
*   **`cognition/`** : L'Orchestrateur (cycle en 7 étapes), les Neurones (les 11 Chiens Sefirotiques), l'Agent de décision.
*   **`consensus/`** : Moteur PBFT (Tolérance aux fautes byzantines).
*   **`learning/`** : Q-Table unifiée, apprentissage par renforcement.
*   **`llm/`** : Registre des adaptateurs cognitifs (Ollama, Anthropic).
*   **`dna/`** : Langage d'assemblage des workflows de pensée.
*   **`dialogue/` & `collaborative/`** : Raisonnement explicatif.
*   **`agents/`** : Agents d'interface logicielle.

### 3. `metabolism/` (L'Action & La Survie)
C'est la transformation de la volonté en impact physique.
*   **`metabolism/`** : `ClaudeCodeRunner` (Les mains de l'organisme), routeurs LLM.
*   **`immune/`** : Limiteurs de puissance, vérificateurs d'alignement éthique, portes d'approbation humaine.
*   **`tools/`** : Outils de maintenance interne.

### 4. `perception/` (Les Sens)
C'est la fenêtre sur les réalités (Code, Social, Market, Solana).
*   **`senses/`** : Les *Workers* (observateurs Git, Discord, etc.).
*   **`integrations/`** : Connecteurs profonds (ex: exécuteur GASdf/NEAR).
*   **`federation/`** : Le protocole Gossip (P2P) pour la fusion des mémoires entre instances CYNIC.

### 5. `interfaces/` (La Peau)
C'est le point de contact avec les utilisateurs. Aucune logique métier ne vit ici.
*   **`api/`** : Le serveur FastAPI (Routeurs pour le Cœur, la Conscience, la Santé, la Gouvernance, la Fédération, le DNA).
*   **`bots/`** : Le `governance_bot` (Discord/Telegram), agissant comme un membre déporté relié à la mémoire centrale.
*   **`cli/` & `tui/`** : Interfaces en ligne de commande et terminal interactif.
*   **`mcp/`** : Le pont Model Context Protocol.
*   **`chat/`** : Interfaces de dialogue.

---

## Lois Fondamentales de la Topologie

1.  **Loi de la Source Unique** : L'API ou les Bots ne possèdent pas de base de données. Ils lisent et écrivent exclusivement via le `StateManager` du `kernel`.
2.  **Loi de l'Asynchronisme Pur** : Les organes communiquent via l' `EventBus` (`JUDGMENT_REQUESTED` $ightarrow$ `JUDGMENT_CREATED`).
3.  **Loi de la Tolérance** : Les données transitent sous forme de dictionnaires souples ou de modèles Pydantic ouverts (`extra="allow"`), permettant l'évolution fractale sans rupture de contrat.

*Généré par le Général de l'Unification. Le chaos a été vaincu.*
