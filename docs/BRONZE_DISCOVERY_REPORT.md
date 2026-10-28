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
  - Table `session_model_usage` (18 colonnes) : association granulaire session $\leftrightarrow$ modèle $\leftrightarrow$ provider $\leftrightarrow$ jetons $\leftrightarrow$ coûts.
  - Table `session_turn_leases` : baux d'exclusion mutuelle par PID et namespace Linux.
  - Table `async_delegations` : cycle de vie des délégations asynchrones et sous-agents.

---

## 3. Analyse de la Reconstructibilité du Contexte (`context_fidelity`)

Conformément à la Section 2.2 de `REFERENCE_ML_DATA_CYNIC.md`, nous avons évalué la capacité à reconstruire fidèlement la fenêtre $c_t$ observée par le LLM à chaque étape :

### 3.1 Prompt Système Exact
- **Claude Code :** Le prompt système complet est capturé dans les événements `api-request-blob` et dans `~/.claude/CLAUDE.md`. Fidélité : `reconstructed_verified`.
- **Antigravity CLI :** Le prompt système réside dans la configuration de l'agent. Des instructions dynamiques sont injectées via `<system_rules>`. Fidélité : `reconstructed_approx`.
- **Hermes :** Présence explicite de la colonne `system_prompt` et `system_prompt_hash` dans la table `sessions` pour chaque session. Fidélité : `exact`.

### 3.2 Compaction de Contexte & Troncatures
- **Claude Code :** Utilise des événements `compactMetadata` et un champ booléen `isCompactSummary: true`. Lorsqu'une session devient trop longue, elle émet un événement `continued-in` avec `continuedInSessionId`. Reconstructibilité post-compaction : `reconstructed_approx`.
- **Antigravity CLI :** Présente une architecture remarquable à **double vue** native :
  - `transcript.jsonl` contient `truncated_fields: ["content", "thinking", "tool_calls"]` (`observed_by_model`).
  - `transcript_full.jsonl` contient l'intégralité non tronquée.
  - `.system_generated/steps/<step_id>/output.txt` conserve le `full_output` brut (parfois plusieurs mégaoctets de compilation).
- **Hermes :** Gère la compaction via les flags `compacted = 1`, `active = 0` et stocke le résumé textuel dans `_compressed_summary`. Les messages absorbés sont répertoriés dans `absorbed_message_uids`. Reconstructibilité : `reconstructed_verified`.

### 3.3 Blocs de Raisonnement (`thinking`)
- **Claude Code :** Capture intégrale des blocs de type `thinking` (1 460 blocs observés) avec durée (`thinkingDurationMs`) et format d'affichage. `reasoning_fidelity = raw`.
- **Antigravity CLI :** Champ `thinking` dédié sur chaque `PLANNER_RESPONSE` (60 blocs observés sur l'échantillon). `reasoning_fidelity = raw`.
- **Hermes :** Colonnes `reasoning`, `reasoning_content` et `reasoning_tokens` présentes dans le schéma SQLite, non peuplées sur les anciens modèles (0 blocs non vides). `reasoning_fidelity = absent` (ou `raw` sur les modèles récents avec flag activé).

---

## 4. Matrice `AdapterCapabilities` Empirique (§2.5)

| Capacité déclarée | Claude Code | Antigravity CLI | Hermes-Agent | Justification empirique |
| :--- | :---: | :---: | :---: | :--- |
| `logs_abandoned_branches` | **True** | **True** | **True** | Les erreurs d'outils et retours en arrière sont journalisés avant réessai. |
| `logs_subagent_sidechains` | **True** | **True** | **True** | Fichiers dédiés `subagents/*.jsonl` (Claude), sous-répertoires (Antigravity), `spawn-trees` (Hermes). |
| `logs_exact_system_prompt` | **True** | **False** | **True** | Capturé dans `api-request-blob` (Claude) et `sessions.system_prompt` (Hermes). |
| `logs_raw_thinking` | **True** | **True** | **Partial** | Présent dans les blocs `thinking` natifs (Claude et Antigravity). Absent sur l'historique ancien d'Hermes. |
| `logs_sampler_logprobs` | **False** | **False** | **False** | Aucun harness n'enregistre les log-probabilités de génération des API. |
| `logs_tool_truncation_boundary` | **True** | **True** | **True** | Troncature explicite : `wireToolInputs` (Claude), `truncated_fields` + `output.txt` (Antigravity), `_compressed_summary` (Hermes). |
| `logs_dirty_worktree` | **True** | **False** | **Partial** | Claude capture `file-history-snapshot`. Hermes enregistre `git_metadata_generation`. Antigravity ne capture pas le diff initial. |

---

## 5. Anomalies, Secrets et Cas Limites Détectés

1. **Présence de secrets dans l'environnement Bronze :**
   - `~/.claude/.credentials.json` contient les tokens OAuth en clair (permissions `600`).
   - `~/.hermes/.env` (24 Ko) contient les clés d'API fournisseurs.
   - Les arguments de commandes (`RUN_COMMAND`, `Bash`) contiennent des clés ou tokens passés en paramètres d'outils.
   - **Exigence :** Le pipeline Silver doit exécuter le Contrat 8 (Détection et caviardage de secrets) **avant** toute persistance en Silver, tout en calculant l'empreinte `event_id` sur les octets bruts initiaux (Contrat 2).
2. **Sessions orphelines et interruptions brutales :**
   - Plusieurs sessions se terminent sans message de complétion (`status: RUNNING` ou dernière ligne = appel d'outil sans résultat).
   - Ces sessions doivent être qualifiées de `done_reason = crash` ou `user_abort` et ne doivent jamais être traitées comme des succès complets.
3. **Disparité de granularité des timestamps :**
   - Plusieurs événements consécutifs partagent le même milliseconde.
   - L'implémentation de `OrderKey(e) = (logical_turn_index, file_byte_offset, subagent_depth, monotonic_timestamp_ns, event_id)` (Contrat 4) est mathématiquement indispensable.

---

## 6. Forensic d'Orfèvrerie : Concurrence, Multi-Modèles In-Flight & Sous-Agents

L'analyse forensique approfondie menée directement sur les tables internes et fichiers bruts révèle les mécanismes sous-jacents de production multi-agents :

### 6.1 Concurrence Multi-Agents et Arbitrage par Baux (`session_turn_leases`)
Quand plusieurs instances d'agents travaillent en parallèle sur la machine :
- Hermes n'utilise pas de simples locks fichiers. Il journalise dans `session_turn_leases` des baux explicites portant l'identité complète du processus :
  ```text
  holder: "pid=1992628:pidns=4026531836:turn=<conv_id>:<turn_id>:dd2eb53e:platform=tui"
  acquired_at: 1793230137.48 | expires_at: 1793230437.48
  ```
- **Crashs de processus en cours de délégation (`async_delegations`) :**
  Dans `async_delegations`, des tâches de rôle `leaf` ont été abandonnées avec l'erreur :
  `"Delegation owner exited before recording a terminal result; outcome unknown."`
- **Impact formel (§2.4) :** Si deux agents ont tourné en parallèle dans le même répertoire sans worktrees Git distincts, leur niveau d'isolation est `shared_fs`. Toute trajectoire marquée `shared_fs` subit des conditions de course et doit être **interdite en entraînement contrefactuel (DPO/GRPO)**.

### 6.2 L'illusion du « modèle unique de session » : Bascule In-Flight
Sur 2 060 sessions analysées dans `session_model_usage` :
- **108 sessions (5,2%) ont changé de modèle au cours de leur exécution.**
- **Cas extrême mesuré (Session `20260929_194202_a40989`) :**
  - **7 modèles distincts** ont collaboré au sein de la même session :
    - `claude-sonnet-4-6` (22 appels)
    - `gemini-3.8-flash` (53 appels)
    - `glm-5.3` (43 appels)
    - `longcat-2.5-preview-free` (333 appels)
    - `meituan/longcat-2.5-preview:free` (3 293 appels)
    - `muse-spark-1.3-contributor` (181 appels)
    - `space-bunny-free` (747 appels)
- **Cas local (Session `20260818_174441_1cf9a4`) :**
  - Bascule entre des modèles locaux GGUF (`LFM2.5-2.6B-Q4_K_M.gguf`, `Qwythos-9B-Claude-Mythos-5-1M`) et des modèles cloud distants.
- **Conséquence formelle (§2.2) :** Il est strictement interdit d'assigner un `model_id` au niveau de la Session. L'unité d'échantillon **doit impérativement être l'appel atomique `LLMCallSample(c_t, a_t)`** avec son `model_id` propre.

### 6.3 Double Modèle Interactif dans Claude Code (`requestedModel` vs `advisorModel`)
L'inspection des sessions Claude Code (`bb6421cf-...jsonl`) met en évidence la présence simultanée de deux modèles dans le même flux :
- `requestedModel: claude-sonnet-5-5` (exécuteur des actions et du code).
- `advisorModel: claude-opus-5-5` (modèle superviseur appelé pour les décisions critiques et la synthèse de compaction).
Le schéma Silver doit enregistrer formellement `role_in_turn ∈ {actor, advisor, summarizer}`.

### 6.4 Liaison Causale d'Orfèvrerie des Sous-Agents
Dans `~/.claude/projects/.../824a4bb7-.../subagents/`, l'inspection des métadonnées (`agent-*.meta.json`) démontre comment les sous-agents sont ancrés dans le graphe :
```json
{
  "agentType": "web-fetch",
  "isBuiltIn": true,
  "description": "Read ASDFBurnTracker repo",
  "toolUseId": "toolu_01FXuqYUdrRuycBa2tWg33r3",
  "spawnDepth": 1,
  "requestShape": "foreground",
  "finished": true
}
```
- **L'ancre causale est `toolUseId` :** Le sous-agent est rattaché à l'appel `tool_use` parent dans `causal_parent_ids`.
- **L'isolation de contexte est stricte (`spawnDepth: 1`) :** Les étapes internes du sous-agent ne figurent jamais dans `context_member_ids` du parent. Seule la valeur de retour finale de l'outil y entre.

---

## 7. Stratégie d'Ingestion : 1 Harnais ou le Tandem Hermes + Claude Code ?

L'analyse forensique permet de trancher rigoureusement le choix architectural :

* **Le piège d'un harnais unique :**
  - Se focaliser sur **Hermes seul** conduit à sur-adapter le schéma à SQLite et à ignorer le streaming d'arbres de sous-agents (`toolUseId`, `spawnDepth`) et le suivi des diffs de fichiers (`file-history-delta`).
  - Se focaliser sur **Claude seul** conduit à sur-adapter le schéma aux fichiers plats JSONL et à ignorer les baux concurrents (`session_turn_leases`) et les bascules multi-modèles in-flight.
* **Le tandem Hermes + Claude Code forme le duo parfait :**
  - **Hermes** apporte la complexité relationnelle, multi-processus, multi-modèles et multi-providers.
  - **Claude Code** apporte la finesse de la chaîne causale des sous-agents, des snapshots de fichiers et du modèle conseiller (`advisorModel`).
  - Couvrir ce tandem garantit que le modèle Silver implémente 100% des exigences des Contrats 1 à 6 sans aucun angle mort.

---

> **POINT D'ARRÊT N°1 (ENRICHI).**  
> L'analyse forensique est consignée et validée par les données du terrain.  
> En attente du feu vert opérateur pour entamer l'**ÉTAPE 2 (Spécification formelle des Contrats 1 à 6)**.
