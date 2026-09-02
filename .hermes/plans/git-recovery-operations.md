# Git recovery operations — plan admis localement

## Admission locale bornée

- Workspace: `agent/hermes/git-recovery-operations`
- Base observée: Forgejo `origin/main` @ `3c48e76687a5a2a7a9f341c84ede09efca8e28b5`
- Mode: `human_scoped_fallback`, MCP observé incompatible (`cynic runtime mcp-observation` absent)
- Autorité source: Forgejo; GitHub est un relai divergent et ne peut déclencher ni push ni merge.
- Stop: dérive de HEAD/worktree, scope concurrent, ou exigence d'une mutation distante.

## Incident reproduit

L'artefact installé `cynic source status --json` rapporte `all layers converged` alors que:

- la couche Forgejo tente le remote inexistant `forgejo`;
- la couche GitHub lit le remote Forgejo `origin`;
- Forgejo est `reachable=false`;
- la décision contient néanmoins `no_op` et `needs_review=false`.

La source confirme les causes:

- `crates/cynic-host/src/source_sync/observe.rs` hardcode `forgejo` et `origin`;
- `crates/cynic-host/src/source_sync/reconcile.rs` fabrique des push/archive à partir de compteurs;
- `crates/cynic-host/src/source_sync/apply.rs` exécute des commandes Git puis fait `reset --hard` en rollback;
- `crates/cynic-host/src/source_worktree_reconcile/apply.rs` exécute rebase, merge et `worktree remove --force`;
- `crates/cynic-cli/src/source_surface.rs` expose tous ces apply directement.

## État initial → transitoire → final

### Initial

`cynic source` peut observer des rôles inversés, conclure une convergence fausse, et possède des apply directs incompatibles avec une source partagée.

### Transitoire (slice P0)

1. Tout `cynic source sync --apply` et `cynic source worktree --apply` retourne un rapport `blocked_unsupported_mutation` sans argv Git mutateur.
2. Toute couche source non résolue, inaccessible ou contradictoire impose `needs_review=true`, un unknown explicite, et interdit `no_op`.
3. Les plans existants peuvent rester observables; ils ne sont pas une autorité d'exécution.
4. Le skill Hermes de récupération devient une projection de la commande native read-only; aucune procédure shell de mutation ne reste canonique.

### Final

Un audit natif `cynic dev source-recovery-audit --json` assemble une observation versionnée de:

- remotes observés et leurs rôles déclarés/résolus;
- refs locales/distantes, upstream et divergence;
- worktrees, stashes, commits détachés et claims;
- candidats `preserve`, `review`, `metadata_only`, `blocked`.

Il ne mute jamais. Une promotion future est séparée: branche Forgejo exacte → SHA Builder → PR/revue humaine → merge explicite.

## Ownership

| Responsabilité | Propriétaire |
|---|---|
| schémas source/recovery, invariants fail-closed | `cynic-protocol` |
| observation Git argv injectée | `cynic-host` |
| admission/promotion future | `cynic-node` + kernel store |
| projection JSON | `cynic-cli` |
| procédure opérateur temporaire | skill Hermes, réduite quand CLI prouvée |

## Slices et preuves

### P0.1 — containment

- Test d'abord: observation avec remote Forgejo inaccessible ou rôle contradictoire ne peut jamais rendre `NoOp`.
- Test d'abord: `--apply` source/worktree retourne `blocked_unsupported_mutation` et n'appelle aucun runner mutateur.
- Corriger les surfaces source host/CLI sans nouvelle politique de merge.
- Preuve: tests ciblés + `cynic source status --json` ne certifie plus la convergence fausse.

### P0.2 — audit read-only

- Inventorier les types existants `source_worktree_inventory`; les étendre seulement si elles absorbent remotes/refs/stashes sans seconde autorité.
- Sinon créer le contrat pur minimal dans `cynic-protocol`, observer dans `cynic-host`, facade `cynic dev` dans `cynic-cli`.
- Tests de rôle distant inversé, ref locale sans upstream, worktree prunable avec HEAD non durable, stash non attribué.
- Preuve: JSON versionné + `cynic dev source-recovery-audit --json` sur ce dépôt.

### P1 — promotion séparée

- Aucun rebase/merge/push automatique.
- Contrat de proposition humaine contenant base/head Forgejo, plan digest, Builder proof, review et read-back Forgejo.

## Non-objectifs

- pas de fusion globale des branches;
- pas de synchronisation GitHub;
- pas de suppression de stash/worktree/branche;
- pas de nouveau script shell comme autorité;
- pas de mutation du checkout humain.
