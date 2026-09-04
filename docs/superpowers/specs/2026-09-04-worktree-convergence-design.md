# Spécification de Design — Convergence des Worktrees & Communication Inter-Agents

- **Date** : 2026-09-04 (Amendé suite à la revue d'architecture Hermes sur `10.10.10.102`)
- **Auteurs** : CYNIC Core Engine (Gemini) + Revue Indépendante (Hermes sur `cynic-agent-host`)
- **Branche** : `agent/gemini/worktree-convergence-engine`
- **Autorités** : `contracts/foundation.json` (§481, §489), `infra/SYSTEM_AGENTIC_MAPPING.md`, `infra/CRATE_MODELS.md`

---

## 1. Contexte & Problèmes Résolus

### 1.1 Problème 1 : Prolifération des Worktrees & Boîte Noire CLI
1. **Prolifération des worktrees agents** : Chaque session d'agent autonome (Hermes, Codex, Gemini) matérialise une worktree temporaire sous `~/.workspace/` ou `/tmp/cynic-workspaces/`. Lorsqu'une tranche est terminée ou abandonnée, la worktree reste zombi sur le disque dur.
2. **Opacité et "mensonge" du CLI actuel** :
   - `cynic source worktree --apply` renvoie une erreur bloquée codée en dur (`emit_blocked_source_mutation`).
   - L'affichage sans `--apply` est une boîte noire résumant de simples décomptes sans afficher les worktrees observées, les chemins exacts, les branches associées, ni la justification causale des actions planifiées.
3. **Défaut de Séparation des Responsabilités (SoC)** :
   - La réconciliation pure (zéro I/O) a été historiquement placée dans `cynic-host` (crate monolithique de 85k lignes).

### 1.2 Problème 2 : Manque de Ciment dans la Communication Inter-Agents
- Les agents tournent dans des environnements séparés (Gemini en local, Hermes sur la VM Proxmox `10.10.10.102`, Builder sur `10.10.10.119`).
- Il n'existe pas de contrat de communication unifié : les agents dépendent soit d'interventions manuelles, soit de baux `.agent-claims.json` purement passifs, sans protocole d'échange de messages, de review ou de handoff typé.

---

## 2. Garde-Fous de la Revue d'Architecture (Hermes VM 10.10.10.102)

Suite à la revue formelle de la première version par Hermes, 5 garde-fous stricts sont intégrés :

1. **Périmètre v1 restreint au nettoyage sûr (`Safe Pruning Only`)** :
   - L'actuation automatique `--apply` exécute **uniquement** le nettoyage des worktrees déjà intégrées (`PruneMergedWorkspace`) et des worktrees zombis (`PruneZombieWorkspace`).
   - **Aucun merge automatique de code (`FastForwardMerge`) en v1** : toute branche en avance reste étiquetée `ReadyForMerge` mais requiert soit une preuve formelle de build vert (`RuntimeProofBundle`), soit l'arbitrage humain.
2. **Suppression du `RebaseInScratchValidator` opaque** :
   - Créer un worktree temporaire pour rebaser automatiquement recrée le problème qu'on essaie de résoudre. Toute divergence est classée `EscalateToHuman`.
3. **Verrou de Concurrence Exclusif (`Concurrency Lock`)** :
   - Un verrou `flock` sur `.git/cynic-convergence.lock` empêche toute collision entre un run CLI manuel et un tick périodique du scheduler.
4. **Idempotence Stricte de l'Actuation** :
   - Si un répertoire de worktree ou une branche locale a déjà été supprimé lors d'un crash intermédiaire, l'action réussit avec acquittement sans faire échouer l'ensemble du plan.
5. **Détection Dynamique de la Branche Principale** :
   - Résolution automatique de la branche de référence via `git symbolic-ref refs/remotes/origin/HEAD` avec repli sur `main`.

---

## 3. Architecture & Découpage des Responsabilités (SoC)

```
┌────────────────────────────────────────────────────────────────────────┐
│                               CYNIC-CLI                                │
│           Projection transparente : inventaire, plan, journal          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼────────────────────────────────────┐
│                               CYNIC-NODE                               │
│       Scheduler / Watchdog : cadence périodique & cycle de vie         │
└───────────────────┬────────────────────────────────┬───────────────────┘
                    │                                │
┌───────────────────▼───────────────┐ ┌──────────────▼───────────────────┐
│            CYNIC-HOST             │ │          CYNIC-PROTOCOL          │
│   Observation & Actuation Git     │ │     Décision Pure & Modèles     │
│  - source_worktree_inventory      │ │  - WorktreeConvergencePlan       │
│  - apply_worktree_convergence     │ │  - reconcile_worktrees (pur)     │
└───────────────────────────────────┘ └──────────────────────────────────┘
```

---

## 4. Modèle de Données & Décision Pure (`cynic-protocol`)

Dans `crates/cynic-protocol/src/source_worktree.rs` :

```rust
pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str =
    "cynic.source_worktree.apply_proof.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeConvergenceAction {
    /// 1. Branche déjà intégrée à main_head -> élagage worktree + suppression branche locale
    PruneMergedWorkspace {
        workspace_path: PathBuf,
        branch: String,
        commit_sha: String,
        reason: String,
    },
    /// 2. Branche strictement en avance sur main_head -> Prêt pour intégration (Review requise en v1)
    EligibleForIntegration {
        branch: String,
        target_commit_sha: String,
        workspace_path: PathBuf,
        ahead_count: usize,
    },
    /// 3. Branche divergée avec conflits potentiels -> Escalade humaine
    EscalateDivergence {
        branch: String,
        branch_head: String,
        onto_main_sha: String,
        workspace_path: PathBuf,
        behind_count: usize,
    },
    /// 4. Worktree invalide, répertoire git manquant ou verrou expiré -> Nettoyage forcé
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
    /// 5. Fichiers modifiés non commités ou verrou actif -> Protégé
    RetainActiveWorkspace {
        branch: Option<String>,
        workspace_path: PathBuf,
        reason: String,
    },
}
```

### Matrice de Décision Pure
1. **Worktree courante (Active Root)** : Toujours `RetainActiveWorkspace`.
2. **Worktree prunable / orpheline** : `PruneZombieWorkspace`.
3. **Fichiers modifiés non commités (`dirty > 0`)** : Toujours `RetainActiveWorkspace` avec alerte d'attention.
4. **Bail actif non expiré dans `.agent-claims.json`** : Toujours `RetainActiveWorkspace` (respect du fencage).
5. **Branche sans bail et propre (`clean`)** :
   - `HEAD == main_HEAD` $\rightarrow$ `PruneMergedWorkspace` (actionnable avec `--apply`).
   - `main_HEAD` est ancêtre de `HEAD` (avance stricte) $\rightarrow$ `EligibleForIntegration`.
   - Divergence $\rightarrow$ `EscalateDivergence`.

---

## 5. Actuateur Git Borné (`cynic-host`)

Dans `crates/cynic-host/src/source_worktree_reconcile/apply.rs` :
- **Verrou de fichier** : `let _lock = acquire_flock(".git/cynic-convergence.lock")`.
- **Exécution d'action `PruneMergedWorkspace`** :
  1. `git -C <root> worktree remove --force <workspace_path>` (idempotent : ignore si le dossier a déjà disparu).
  2. `git -C <root> branch -d <branch>` (refuse si non mergé).
- **Production de la Preuve** :
  `WorktreeConvergenceApplyProof` horodaté listant chaque action, son code de retour et la preuve de lecture arrière.

---

## 6. Transparence Radicale de la Surface CLI (`cynic-cli`)

### 6.1 Mode Plan (`cynic source worktree`)
Affiche le tableau exhaustif sans rien cacher :

```text
=== CYNIC Worktree Inventory & Convergence Plan ===
Repository Root:  /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
Detected Default: main (c98534ed)
Lock Status:      unlocked

Observed Worktrees (5):
  [1] agent/gemini/worktree-convergence-engine
      Path:   /home/user/Bureau/.../CYNIC (Active Root)
      HEAD:   eb998645 (+1 ahead of main)
      Status: Clean
      Decision: RetainActiveWorkspace (Current Active Session)

  [2] agent/hermes/cynic-cloud
      Path:   /home/user/.workspace/CYNIC-cynic-cloud
      HEAD:   ef41497e (+40 ahead of main)
      Status: Clean (PID 235585 active)
      Decision: EligibleForIntegration (Requires build proof before merge)

  [3] agent/hermes/git-recovery-operations
      Path:   /home/user/.workspace/CYNIC-git-recovery-operations
      HEAD:   1bfc1230 (-89 behind main)
      Status: Dirty (2 uncommitted files)
      Decision: RetainActiveWorkspace (Uncommitted changes protected)

Eligible actions to apply: 0 prune, 0 merge.
To apply safe prunes: cynic source worktree --apply
```

### 6.2 Mode Exécution (`cynic source worktree --apply`)
Exécute uniquement les actions sûres et affiche le rapport de preuve avec digest SHA3.

---

## 7. Fondations de la Communication Inter-Agents sur CYNIC

Pour que les agents (Gemini sur Cortex, Hermes sur VM Proxmox `10.10.10.102`, Builder sur `10.10.10.119`) collaborent sans intervention humaine constante, trois canaux explicites sont formalisés :

```mermaid
graph TD
    subgraph CANAL 1 : CONSTITUTION & ÉTAT SMR (Asynchrone Formel)
        SMR[CynicPeer / SMR Engine]
        CLAIMS[.agent-claims.json / Leases]
        ENV[AgentWorkloadEnvelope]
        SMR --- CLAIMS
        CLAIMS --- ENV
    end

    subgraph CANAL 2 : MESSAGERIE SYNAPSE MESH (Point-à-Point Direct)
        M1[Agent Cortex / Gemini] <-->|Synapse WireGuard 10.10.10.0/24| M2[Agent Host VM / Hermes 10.10.10.102]
        M1 <-->|HTTP / MCP RPC| BUILDER[cynic-builder 10.10.10.119]
    end

    subgraph CANAL 3 : ARTEFACTS & PREUVES (Content-Addressed SSOT)
        FORGE[Forgejo Git SSOT 10.10.10.101]
        DOCS[docs/superpowers/specs/]
        PROOF[RuntimeProof JSON / Digest]
        FORGE --- DOCS
        DOCS --- PROOF
    end
```

### 7.1 Canal 1 : Coordination & Baux Formels (`.agent-claims.json` + `CynicPeer`)
* **Principe** : Aucun agent n'écrit de code sans déclarer son bail (`scope`, `TTL`, `budget`).
* Tout agent peut interroger `cynic agent status` pour connaître les claims actifs des autres agents.

### 7.2 Canal 2 : Requêtes Directes sur le Réseau Synapse (`Synapse Agent Protocol`)
* Les agents se contactent directement via leurs adresses réseau Synapse dédiées :
  - Cortex : `10.10.10.100`
  - Agent-Host (Hermes) : `10.10.10.102`
  - Forgejo : `10.10.10.101`
  - Builder : `10.10.10.119`
* **Deux types de requêtes unifiées** :
  1. `RequestReview` : Un agent soumet une spec ou un commit SHA à un autre agent pour relecture formelle.
  2. `RequestBuild` : Un agent soumet son commit au builder distant (`cynic_validate`).

### 7.3 Canal 3 : Partage de Preuves Content-Addressed
* Les agents ne s'envoient pas de code brut par message : ils échangent des **références Git** (commit SHA sur Forgejo) et des **chemins de specs versionnés** (`docs/superpowers/specs/...`).
* La vérité reste dans le substrat (Forgejo + disque), jamais dans la mémoire vive des LLMs.
