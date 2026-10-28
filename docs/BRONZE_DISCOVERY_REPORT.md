# Rapport de Découverte & Profilage de la Couche Bronze — CYNIC Data Framework (Phase A — Étape 1)

**Schema:** `cynic.bronze.discovery_report.v1`  
**Date :** 2026-10-29  
**Auteur :** Antigravity CLI (Architecte Données CYNIC)  
**Référence théorique :** `REFERENCE_ML_DATA_CYNIC.md` (v4.0)  
**Statut :** POINT D'ARRÊT N°1 (En attente de validation opérateur avant schéma Silver)

---

## 1. Topologie des Fichiers et Formats Physiques

L'inspection empirique a été réalisée sur le substrat physique réel de la machine. Trois familles majeures de harnais ont été cartographiées et profilées :

| Harness | Emplacement Physique Réel | Format & Encodage | Volume Observé | Conventions de Nommage |
| :--- | :--- | :--- | :--- | :--- |
| **Claude Code** | `~/.claude/projects/<slug>/` | JSON Lines (`.jsonl`), UTF-8 | 8 sessions actives (28,5 Mo), 3 sous-agents, historique 401 Ko | `<session-uuid>.jsonl`<br>Sous-agents : `<uuid>/subagents/agent-<sub_id>.jsonl` + `.meta.json` |
| **Antigravity CLI** | `~/.gemini/antigravity-cli/brain/<uuid>/` | Double JSON Lines (`transcript.jsonl` + `transcript_full.jsonl`) + TXT | 501 répertoires de session, 550+ fichiers d'étapes | `<uuid>/.system_generated/logs/transcript*.jsonl`<br>`steps/<step_id>/output.txt`<br>`tasks/<task_id>.log` |
| **Hermes-Agent / CYNIC** | `~/.hermes/` (Live)<br>`/var/lib/cynic/vault/corpus/captures/` (Gel) | Base SQLite transactionnelle (`state.db`), WAL activé + Dumps JSONL | Live : 2,05 Go (2 309 sessions, 487 754 messages)<br>Gel : 1,3 Go (3 149 sessions) | `state.db` (tables `sessions`, `messages`, `conversation_generations`)<br>Sessions dump : `sessions/<timestamp>_<hash>.jsonl` |

---

## 2. Inventaire Exhaustif des Clés et Types d'Événements

### 2.1 Claude Code
- **Clés de premier niveau observées (102 clés) :**
  `type`, `uuid`, `parentUuid`, `logicalParentUuid`, `sessionId`, `sessionKind`, `timestamp`, `cwd`, `gitBranch`, `message`, `toolUseResult`, `thinkingDisplay`, `thinkingDurationMs`, `isSidechain`, `isCompactSummary`, `compactMetadata`, `continuedInSessionId`, `file-history-snapshot`, `file-history-delta`, `api-request-blob`, `api-request-shape`, `queue-operation`, `atis-latch`, `cost-state`, `totalCostUSD`, `modelUsage`.
- **Typologie des événements (`type`) :**
  - `user` (1 627) : prompts de l'opérateur et retours d'outils formatés.
  - `assistant` (3 247) : réponses de Claude incluant blocs `text`, `thinking` (1 460 blocs) et `tool_use` (1 792 blocs).
  - `system` (298) : messages système injectés en cours de session.
  - `attachment` (2 004) : fichiers et contextes attachés dynamiquement.
  - `api-request-blob` (1 668) / `api-request` (1 414) : payloads bruts des requêtes API sous-jacentes.
  - `file-history-snapshot` (211) : empreintes de l'état des fichiers suivis (tracking worktree).
  - `isCompactSummary` (avec `compactMetadata`) : marqueur explicite de compaction de contexte.

### 2.2 Antigravity CLI
- **Clés de premier niveau observées :**
  `step_index`, `type`, `source`, `status`, `created_at`, `content`, `thinking`, `tool_calls`, `media`, `truncated_fields`, `error`, `error_code`.
- **Typologie des événements (`type`) :**
  - `USER_INPUT` : directives utilisateur directes (`source: USER_EXPLICIT`).
  - `PLANNER_RESPONSE` : réflexion du modèle (`thinking`), texte visible (`content`), et tableaux d'appels d'outils (`tool_calls`).
  - `RUN_COMMAND`, `VIEW_FILE`, `CODE_ACTION`, `GREP_SEARCH`, `LIST_DIRECTORY` : exécution d'actions par le harnais.
  - `SYSTEM_MESSAGE` / `EPHEMERAL_MESSAGE` : notifications système asynchrones et retours de tâches.
  - `INVOKE_SUBAGENT` : délégation à un sous-agent avec conversation isolée (`recipient`).
  - `CHECKPOINT` : points d'ancrage transactionnels.
- **Sources (`source`) :** `USER_EXPLICIT` (22), `SYSTEM` (64), `MODEL` (153).
- **Statuts (`status`) :** `DONE` (238), `RUNNING` (1), `ERROR`.

### 2.3 Hermes-Agent / CYNIC
- **Structure relationnelle SQLite (`state.db`) :**
  - Table `sessions` (2 309 lignes, 64 colonnes) : `id`, `model`, `system_prompt`, `system_prompt_hash`, `parent_session_id`, `started_at`, `ended_at`, `end_reason`, `message_count`, `tool_call_count`, `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_write_tokens`, `cwd`, `git_branch`, `git_repo_root`, `actual_cost_usd`, `compression_failure_streak`.
  - Table `messages` (487 754 lignes, 30 colonnes) : `id`, `session_id`, `role` (`assistant`: 226k, `tool`: 239k, `user`: 22k, `session_meta`: 1), `content`, `tool_call_id`, `tool_calls` (207k appels), `tool_name`, `effect_disposition`, `timestamp`, `token_count`, `active`, `compacted`, `_compressed_summary`.
  - Table `conversation_generations` : trace de chaque inférence individuelle avec association des coûts et modèles.

---

## 3. Analyse de la Reconstructibilité du Contexte (`context_fidelity`)

Conformément à la Section 2.2 de `REFERENCE_ML_DATA_CYNIC.md`, nous avons évalué la capacité à reconstruire fidèlement la fenêtre $c_t$ observée par le LLM à chaque étape :

### 3.1 Prompt Système Exact
- **Claude Code :** Le prompt système complet n'est pas logué sur chaque ligne de message, mais il est capturé dans les événements `api-request-blob` et dans `~/.claude/CLAUDE.md`. Fidélité : `reconstructed_verified`.
- **Antigravity CLI :** Le prompt système n'est pas réémis à chaque tour dans `transcript.jsonl`. Il réside dans la configuration de l'agent. Des instructions dynamiques sont injectées via `<system_rules>`. Fidélité : `reconstructed_approx` si le manifeste système n'est pas capturé conjointement.
- **Hermes :** Présence explicite de la colonne `system_prompt` et `system_prompt_hash` dans la table `sessions` pour chaque session. Fidélité : `exact`.

### 3.2 Compaction de Contexte & Troncatures
- **Claude Code :** Utilise des événements `compactMetadata` et un champ booléen `isCompactSummary: true`. Lorsqu'une session devient trop longue, elle émet un événement `continued-in` avec `continuedInSessionId`. Reconstructibilité post-compaction : `reconstructed_approx`.
- **Antigravity CLI :** Présente une architecture remarquable à **double vue** native :
  - `transcript.jsonl` contient `truncated_fields: ["content", "thinking", "tool_calls"]` (ce que l'interface compacte).
  - `transcript_full.jsonl` contient l'intégralité non tronquée.
  - `.system_generated/steps/<step_id>/output.txt` conserve le `full_output` brut (parfois plusieurs mégaoctets de compilation), tandis que `transcript.jsonl` ne conserve que ce qui a été `observed_by_model`.
  - C'est **exactement** la séparation requise par le Contrat §2.2 !
- **Hermes :** Gère la compaction via les flags `compacted = 1`, `active = 0` et stocke le résumé textuel dans `_compressed_summary`. Les messages absorbés sont répertoriés dans `absorbed_message_uids`. Reconstructibilité : `reconstructed_verified`.

### 3.3 Blocs de Raisonnement (`thinking`)
- **Claude Code :** Capture intégrale des blocs de type `thinking` (1 460 blocs observés) avec durée (`thinkingDurationMs`) et format d'affichage. `reasoning_fidelity = raw`.
- **Antigravity CLI :** Champ `thinking` dédié sur chaque `PLANNER_RESPONSE` (60 blocs observés sur l'échantillon). `reasoning_fidelity = raw`.
- **Hermes :** Colonnes `reasoning`, `reasoning_content` et `reasoning_tokens` présentes dans le schéma SQLite, mais non peuplées sur les anciens modèles (0 blocs non vides détectés sur les sessions échantillonnées). `reasoning_fidelity = absent` (ou `raw` sur les modèles récents avec flag activé).

### 3.4 Concurrence, Sous-Agents et Lignée
- **Claude Code :**
  - Sous-agents matérialisés par `isSidechain: true`, avec identifiants hiérarchiques stricts (`parentUuid`, `logicalParentUuid`).
  - Chaque sous-agent possède son propre fichier de log dans le sous-dossier `subagents/`.
  - Isole parfaitement le contexte du parent (`context_member_ids` ne contient pas les étapes internes du sous-agent).
- **Antigravity CLI :**
  - Événements `INVOKE_SUBAGENT` générant un `conversationId` dédié, exécuté en tâche de fond (`tasks/<task_id>.log`) et communiquant par boîte aux lettres (`messages/`).
- **Hermes :**
  - Champ `parent_session_id` dans `sessions`.
  - Gestion des arbres de spawn dans `~/.hermes/spawn-trees/`.
  - Traçage des délégations asynchrones dans la table `async_delegations`.

---

## 4. Matrice `AdapterCapabilities` Empirique (§2.5)

Table de vérité déduite de l'observation physique des logs Bronze :

| Capacité déclarée | Claude Code | Antigravity CLI | Hermes-Agent | Justification empirique |
| :--- | :---: | :---: | :---: | :--- |
| `logs_abandoned_branches` | **True** | **True** | **True** | Les erreurs d'outils et retours en arrière sont journalisés avant réessai. |
| `logs_subagent_sidechains` | **True** | **True** | **True** | Fichiers dédiés `subagents/*.jsonl` (Claude), sous-répertoires (Antigravity), `spawn-trees` (Hermes). |
| `logs_exact_system_prompt` | **True** | **False** | **True** | Capturé dans `api-request-blob` (Claude) et `sessions.system_prompt` (Hermes). Antigravity s'appuie sur un système prompt externe non dupliqué dans chaque turn. |
| `logs_raw_thinking` | **True** | **True** | **Partial** | Présent dans les blocs `thinking` natifs (Claude et Antigravity). Absent sur l'historique ancien d'Hermes. |
| `logs_sampler_logprobs` | **False** | **False** | **False** | Aucun harness n'enregistre les log-probabilités de génération des API (OpenAI/Anthropic/Google ne les exposent pas par défaut). |
| `logs_tool_truncation_boundary` | **True** | **True** | **True** | Troncature explicite : `wireToolInputs` (Claude), `truncated_fields` + `output.txt` (Antigravity), `_compressed_summary` (Hermes). |
| `logs_dirty_worktree` | **True** | **False** | **Partial** | Claude capture `file-history-snapshot`. Hermes enregistre `git_metadata_generation` et commits, mais pas le diff complet uncommitted. Antigravity journalise les commandes mais pas le snapshot d'arbre initial. |

---

## 5. Anomalies, Secrets et Cas Limites Détectés

1. **Présence de secrets dans l'environnement Bronze :**
   - Le répertoire `~/.claude/.credentials.json` contient les tokens OAuth en clair (permissions `600`).
   - Le répertoire `~/.hermes/.env` (24 Ko) contient les clés d'API fournisseurs.
   - Les arguments de commandes (`RUN_COMMAND`, `Bash`) peuvent contenir des clés ou tokens passés en paramètres d'outils.
   - **Exigence impérative :** Le pipeline d'ingestion Silver doit exécuter le Contrat 8 (Détection et caviardage de secrets) **avant** toute persistance en Silver, tout en calculant l'empreinte `event_id` sur les octets bruts initiaux (Contrat 2).
2. **Sessions orphelines et interruptions brutales :**
   - Plusieurs sessions dans `~/.claude/projects/` et `~/.gemini/antigravity-cli/brain/` se terminent sans message de complétion (`status: RUNNING` ou dernière ligne = appel d'outil sans résultat).
   - Ces sessions doivent être qualifiées de `done_reason = crash` ou `user_abort` et ne doivent en aucun cas être traitées comme des succès complets.
3. **Disparité de granularité des timestamps :**
   - Claude Code et Hermes utilisent des horodatages ISO millisecondes ou secondes UNIX. Plusieurs événements consécutifs partagent le même milliseconde.
   - **Validation du Contrat 4 (Ordre Total) :** Il est strictement impossible de trier les événements uniquement par horodatage. L'implémentation de `OrderKey(e) = (logical_turn_index, file_byte_offset, subagent_depth, monotonic_timestamp_ns, event_id)` est mathématiquement indispensable.

---

## 6. Conclusion de l'Étape 1 & Recommandations pour la Phase Silver

1. **La Couche Bronze est riche et exploitable :** Nous disposons de plus de **500 000 événements agentiques réels** sur la machine, couvrant l'ensemble des scénarios (succès, échecs, sous-agents, compactions, réflexion interne).
2. **L'approche Bottom-Up est confirmée :**
   - Les schémas de `apps/cynic-data` ignoraient la double vue `observed_by_model` vs `full_output` pourtant présente physiquement dans Antigravity.
   - Le modèle Silver doit impérativement respecter la séparation `context_member_ids` vs `causal_parent_ids` pour gérer proprement les sous-agents de Claude Code et Antigravity.

---

> **POINT D'ARRÊT N°1 ATTEINT.**  
> Le rapport ci-dessus documente l'état physique exact de la Couche Bronze.  
> Aucune structure Silver n'a été codée à ce stade.  
> En attente du feu vert de l'opérateur pour passer à l'**ÉTAPE 2 (Spécification des Contrats 1 à 6 et Architecture Silver)**.
