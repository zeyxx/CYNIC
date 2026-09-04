# Spécification de Design — Convergence des Worktrees & Transparence CLI

- **Date** : 2026-09-04
- **Auteur** : CYNIC Core Engine
- **Branche** : `agent/gemini/worktree-convergence-engine`
- **Autorités** : `contracts/foundation.json` (§481, §489), `infra/SYSTEM_AGENTIC_MAPPING.md`, `infra/CRATE_MODELS.md`

---

## 1. Contexte & Objectifs

### 1.1 Problème Observé
1. **Prolifération des worktrees agents** : Chaque session d'agent autonome (Hermes, Codex, Gemini) matérialise une worktree temporaire sous `~/.workspace/` ou `/tmp/cynic-workspaces/`. Lorsqu'une tranche est terminée ou abandonnée, la worktree reste zombi sur le disque dur.
2. **Opacité et "mensonge" du CLI actuel** :
   - `cynic source worktree --apply` renvoie une erreur bloquée codée en dur (`emit_blocked_source_mutation`).
   - L'affichage sans `--apply` est une boîte noire résumant de simples décomptes sans afficher les worktrees observées, les chemins exacts, les branches associées, ni la justification causale des actions planifiées.
3. **Défaut de Séparation des Responsabilités (SoC)** :
   - La réconciliation pure (zéro I/O) a été historiquement placée dans `cynic-host` (crate monolithique de 85k lignes), alors que `cynic-protocol` doit posséder toute logique de décision pure, `cynic-node` doit posséder le déclenchement/scheduler, et `cynic-host` ne doit être qu'un simple actuateur de commandes argv `git`.

### 1.2 Objectifs
- **Transparence radicale du CLI** : Lister exhaustivement chaque worktree observée, son statut vis-à-vis de `main`, l'action planifiée et le motif. Lors de `--apply`, afficher le journal d'exécution pas-à-pas avec les preuves de lecture arrière (`read-back evidence`).
- **SoC pur** :
  - `cynic-protocol` : Modèle de données et algorithme de décision pur.
  - `cynic-host` : Exécution bornée via `ArgvRunner` injectable et tests avec mocks.
  - `cynic-node` : Usecase de cycle de vie et déclenchement périodique dans le scheduler.
  - `cynic-cli` : Projection transparente human-readable et JSON pour pipelines.
- **Attention intégrée** : Remonter les worktrees zombis et les conflits non résolus dans la surface d'attention de CYNIC (`/attention`).

---

## 2. Architecture & Répartition des Responsabilités

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

## 3. Modèle de Données & Décision Pure (`cynic-protocol`)

### 3.1 Types d'Actions Bailleurs
Dans `crates/cynic-protocol/src/source_worktree.rs` :

```rust
pub const SOURCE_WORKTREE_CONVERGENCE_PLAN_SCHEMA: &str =
    "cynic.source_worktree.convergence_plan.v1";
pub const SOURCE_WORKTREE_APPLY_PROOF_SCHEMA: &str =
    "cynic.source_worktree.apply_proof.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeConvergenceAction {
    /// 1. Branche déjà intégrée à main_head ou ancêtre strict de main -> élagage worktree + suppression branche locale
    PruneMergedWorkspace {
        workspace_path: PathBuf,
        branch: String,
        commit_sha: String,
        reason: String,
    },
    /// 2. Branche strictement en avance sur main_head -> Fast-forward merge sur main
    FastForwardMerge {
        branch: String,
        target_commit_sha: String,
        workspace_path: PathBuf,
    },
    /// 3. Branche divergée avec conflits potentiels -> Rebase dans scratch validator
    RebaseInScratchValidator {
        branch: String,
        original_sha: String,
        onto_main_sha: String,
        workspace_path: PathBuf,
    },
    /// 4. Worktree invalide, répertoire git manquant ou verrou expiré -> Nettoyage forcé
    PruneZombieWorkspace {
        workspace_path: PathBuf,
        branch: Option<String>,
        reason: String,
    },
    /// 5. Conflit structurel insoluble ou corruption de métadonnées -> Escalade à l'humain
    EscalateToHuman {
        branch: Option<String>,
        workspace_path: PathBuf,
        reason: String,
    },
}
```

### 3.2 Algorithme de Réconciliation Pure
La fonction pure compare chaque entrée de l'inventaire :
1. **Worktree invalide / malformée** : `PruneZombieWorkspace` si git-prunable, ou `EscalateToHuman` si corrompue.
2. **Main branch elle-même** : Ignorée (ne jamais tenter d'élaguer le worktree principal).
3. **HEAD branche == HEAD main** :
   - Si aucun fichier modifié non commité $\rightarrow$ `PruneMergedWorkspace`.
   - Si fichiers modifiés $\rightarrow$ `EscalateToHuman` (travail en cours non commité).
4. **Main est ancêtre strict de la branche (Branche en avance)** :
   - Si les tests/preuves sont valides et aucun conflit $\rightarrow$ `FastForwardMerge`.
5. **Branche divergée** :
   - $\rightarrow$ `RebaseInScratchValidator`.

---

## 4. Actuateur Git Borné (`cynic-host`)

Dans `crates/cynic-host/src/source_worktree/apply.rs` :
- Injecte `ArgvRunner`.
- Pour `PruneMergedWorkspace` :
  1. `git -C <root> worktree remove --force <workspace_path>`
  2. `git -C <root> branch -d <branch>` (ou `-D` si validé)
- Pour `FastForwardMerge` :
  1. `git -C <root> merge --ff-only <target_commit_sha>`
- Produit systématiquement un `WorktreeConvergenceApplyProof` contenant pour chaque action :
  - `action_name`
  - `target_branch`
  - `success` (bool)
  - `message` (sortie stderr/stdout résumée)
  - `read_back_evidence` (ex: nouveau SHA de HEAD, vérification d'absence du répertoire).

---

## 5. Spécification de la Surface CLI (`cynic-cli`)

### 5.1 Mode Plan (par défaut : `cynic source worktree`)
Ne doit plus être un résumé aveugle de 5 lignes. Il affiche le tableau détaillé :

```text
=== CYNIC Worktree Inventory & Convergence Plan ===
Repository: /home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
Main HEAD:  c98534ed

Observed Worktrees (5):
  [1] agent/gemini/worktree-convergence-engine
      Path:   /home/user/Bureau/.../CYNIC (Active Root)
      HEAD:   eb998645 (+1 ahead of main)
      Status: Clean
      Action: Retain (Current Active Worktree)

  [2] agent/hermes/mandate-delivery-admission
      Path:   /home/user/.workspace/CYNIC-mandate-delivery-admission
      HEAD:   c98534ed (At main HEAD)
      Status: Clean (Fully merged)
      Action: PruneMergedWorkspace -> remove worktree & delete branch

  [3] agent/hermes/cynic-cloud
      Path:   /home/user/.workspace/CYNIC-cynic-cloud
      HEAD:   ef41497e (+40 ahead of main)
      Status: Clean, active kernel runner (PID 235585)
      Action: FastForwardMerge -> ready for linear integration

  [4] agent/hermes/git-recovery-operations
      Path:   /home/user/.workspace/CYNIC-git-recovery-operations
      HEAD:   1bfc1230 (-89 behind main)
      Status: Dirty (2 uncommitted files)
      Action: EscalateToHuman -> uncommitted changes require review

Actions to execute: 1 prune, 1 fast-forward, 1 human escalation.
To apply eligible actions, rerun with: cynic source worktree --apply
```

### 5.2 Mode Exécution (`cynic source worktree --apply`)
Exécute les actions applicables (`PruneMergedWorkspace`, `FastForwardMerge`, `PruneZombieWorkspace`) et affiche le journal des preuves :

```text
Applying convergence actions...
  ✓ Pruned merged worktree: /home/user/.workspace/CYNIC-mandate-delivery-admission
  ✓ Deleted merged local branch: agent/hermes/mandate-delivery-admission
  ✓ Fast-forwarded main to: ef41497e (agent/hermes/cynic-cloud)
  ⚠ Skipped /home/user/.workspace/CYNIC-git-recovery-operations (Dirty files, escalated)

Summary: 2 succeeded, 0 failed, 1 skipped.
Proof generated: cynic.source_worktree.apply_proof.v1 (digest sha3-512:...)
```

### 5.3 Mode Machine (`--json`)
Émet le JSON complet du plan ou de la preuve sans texte superflu.

---

## 6. Cadence & Supervision (`cynic-node`)

1. **Usecase dédié** : `cynic_node::usecases::worktree_convergence_uc`
   - Orchestre le flux complet sans dépendance circulaire.
2. **Tick de fond** :
   - Exécutable périodiquement (`cynic-node source-worktree-sweep --apply`).
   - Seuil de sécurité : ne touche JAMAIS aux worktrees qui ont des baux actifs non expirés ou des processus en cours d'exécution.

---

## 7. Plan de Test & Vérification

1. **Tests unitaires purs (`cynic-protocol`)** :
   - Test : une branche déjà à `main_head` sans modifications produit `PruneMergedWorkspace`.
   - Test : une branche en avance stricte produit `FastForwardMerge`.
   - Test : une branche avec fichiers modifiés non committés produit `EscalateToHuman`.
   - Test : une branche orpheline sans dossier git produit `PruneZombieWorkspace`.
2. **Tests d'actuation avec mocks (`cynic-host`)** :
   - `MapArgvRunner` simulant les commandes `git worktree remove` et `git branch -d`.
   - Validation que `WorktreeConvergenceApplyProof` capture les erreurs et les succès.
3. **Test d'intégration CLI (`cynic-cli`)** :
   - Vérifier le formatage lisible et l'émission JSON stricte.
