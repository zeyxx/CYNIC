# Organ Anvil - Architecture Data-Driven

## Structure des fichiers

```
infra/organ-anvil/
├── schema.json      # Définition du schema (version 1.0.0)
├── state.json       # État actuel (overwrite complet)
├── audit.jsonl      # Journal immutable (append-only)
├── poh.json         # Historique append-only des snapshots repo
└── dashboard.html   # Interface opérateur (autonome)
```

## Principes ACID

### Atomicité
- `state.json` est écris en entier ou pas du tout (pas de partial write)
- `audit.jsonl` chaque ligne est un événement atomique
- `reports.jsonl` chaque ligne est un rapport JSON complet append-only

### Cohérence
- Schema versionné dans `schema.json`
- Tous les champs attendus sont présents dans `state.json`

### Isolation
- Un seul processus écrit `state.json` à la fois (pas de concurrence)
- `audit.jsonl` est append-only (pas de conflit d'écriture)
- `reports.jsonl` est append-only; aucun rapport JSON ne doit être réécrit en place

### Durabilité
- Les fichiers sont persistés sur disque après chaque run
- Pas de données en mémoire volatile

## Gestion de l'évolution

### Schema Evolution
1. Nouveau champ → Ajout dans `schema.json` avec version incrémentée
2. Migration → Script de migration dans `scripts/organ-anvil-migrate.sh`
3. Compatibilité backward → Les anciens champs sont conservés, les nouveaux ajoutés avec valeur par défaut

### State Evolution
- `state.json` est un snapshot complet - pas de merge partiel
- Si un champ manque → valeur par défaut du schema
- Si un champ inconnu est présent → ignoré (forward compatible)

### Audit Evolution
- `audit.jsonl` est immutable - jamais modifié
- `reports.jsonl` est immutable - chaque rapport JSON devient une ligne de dataset historique
- Nouveau format d'entry → Nouvelle version dans le champ `details`
- Anciennes entries restent lisibles (backward compatible)

## Interface Opérateur

### Dashboard (dashboard.html)
- Lecture de `state.json` et `audit.jsonl`
- Affichage visuel du health score
- Liste des branches, worktrees, stashes
- Dernières 10 entries d'audit
- Alertes visuelles (couleurs)

### Alertes
- Écrites dans `state.json.alerts`
- Format: `{"severity": "warning|critical", "message": "...", "action_needed": "..."}`
- Consommées par le dashboard
- `documented PUSH_FORCE fallback missing from pre-push hook` est critique: le protocole annonce un échappatoire de gate, mais le hook installé ne l'implémente pas.

## Monitoring

### Health Score (0-100)
- 100 = Repo clean, pas de branches orphelines, pas de stashes
- <70 = Attention (dirty tree, branches divergentes)
- <50 = Action nécessaire (cleanup immédiat)

### Métriques
- `run_count`: Nombre de perceptions
- `last_perception`: Dernière mise à jour
- `branches_local/remote`: Compteur de branches
- `prs_open`: PRs ouverts (si gh CLI dispo)
- `worktrees`: Worktrees actifs
- `stashes`: Stashes en attente
- `gate_markers`: Présence + mtime de `.gate-0`, `.gate-1`, `.gate-2`, `.gate-passed`
- `push_force_supported`: Vérifie si le hook pre-push implémente le fallback documenté
- `repo-health`: Diagnostic JSON non-mutant qui croise branches locales/remotes, PR ouvertes, stashes, worktree, gates et bus de coordination

## Workflow

1. **Perception** → `bash scripts/organ-anvil.sh state`
2. **Décision** → Hermes Agent lit state.json et décide
3. **Action** → Scripts ou MCP tools
4. **Audit** → `bash scripts/organ-anvil.sh audit "action" '{"details":...}' "outcome"`
5. **Signal** → `bash scripts/organ-anvil.sh signal` emits compact JSON for cortices/Hermes consumers
6. **Triage** → `bash scripts/organ-anvil.sh triage` emits non-mutating scope diagnosis for dirty worktrees
7. **Repo Health** → `bash scripts/organ-anvil.sh repo-health` emits non-mutating JSON radar for branches/PRs/stashes/gates/coord
8. **Branch Report** → `bash scripts/organ-anvil.sh branch-report <branch> --save` appends an immutable JSON report to `reports.jsonl`
9. **Rapport** → Dashboard HTML + handoff.md

## Évolutivité

- Ajouter de nouveaux capteurs dans `scripts/organ-anvil.sh`
- Ajouter de nouvelles règles dans le prompt du cron
- Ajouter de nouvelles métriques dans `state.json`
- Ajouter de nouveaux panneaux dans `dashboard.html`
