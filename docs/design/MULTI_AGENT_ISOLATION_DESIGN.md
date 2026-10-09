# Isolation Multi-Agent — Design (V5 préfigurée)

Source: AGENTS.md §660-679, empirie session v1-v54, émergence détectée.

## 1. Constat (ce qui a cassé)

Pendant la session v1→V4e sur `agent/hermes/v1-debt-purge`:
- 4 commits d'autres agents ont atterri sur NOTRE branche pendant qu'on travaillait :
  `0928aeb6`, `f310c3cc`, `afd95d2b`, `cfa97eb1`
- `cargo check -p cynic-host` bloqué par `organic_opportunity` (RetentionPolicy::Default,
  SkillCandidate.hypothesis, PatternGcAssessment.windows) — dette d'un autre agent, pas la nôtre
- On a découvert les conflits au check, pas au claim
- Claims = advisory, pas enforce → les scopes se chevauchent silencieusement

## 2. Modèle d'isolation (worktree-per-agent)

### 2.1 Règle: 1 agent = 1 worktree

```
~/.workspace/CYNIC-<agent-name>/     ← worktree dédié
  .git (lien vers l'objet partagé)
  crates/, infra/, ...
  .agent-claims.json                 ← claims locaux (pas partagés)
```

Avantages:
- Fichiers d'un autre agent = absents de notre worktree (pas de `M`/`D` parasites)
- `cargo check` ne voit que nos fichiers + les leurs déjà commités sur la branche
- `git status` reflète uniquement NOTRE travail
- Suppression sans risque: `git worktree remove` si on casse tout

### 2.2 Cycle de vie

```
1. DÉBUT DE SESSION
   git worktree add --detach ~/.workspace/CYNIC-<agent> HEAD
   cd ~/.workspace/CYNIC-<agent>
   git checkout -b agent/<name>/<scope>   # si nouvelle branche
   git pull origin agent/<name>/<scope>   # si existante

2. PENDANT LE TRAVAIL
   - Claim local (.agent-claims.json du worktree)
   - Mutation uniquement dans notre worktree
   - Commit isolé, staged == file list

3. FIN DE SESSION
   git push origin agent/<name>/<scope>
   # worktree conservé pour prochaine session (pas de remove)
```

### 2.3 Vérification au démarrage (obligatoire)

Avant toute mutation, vérifier:
```bash
# 1. On est dans notre worktree
git rev-parse --show-toplevel | grep CYNIC-<agent-name>

# 2. Pas de fichiers d'autres agents dans git status
git status --short | grep -v "^ M crates/cynic-"   # adapter à nos paths

# 3. La branche est propre ou ne contient que NOS commits
git log --oneline --since="1 hour ago" --author="$(git config user.email)"
```

## 3. Claims par worktree

### 3.1 Stockage

```
~/.workspace/CYNIC-<agent>/
  .agent-claims.json     ← claims locaux, pas de conflit avec autres agents
```

### 3.2 Format (inchangé, schema v2)

```json
{
  "schema": "cynic.agent_claims.v2",
  "revision": 42,
  "claims": [
    {
      "claim_id": "claim_<agent>_<scope>_001",
      "agent_id": "hermes:main",
      "mode": "baseline",
      "scope": { "kind": "crate", "path": "crates/cynic-host" },
      "budget": { "actions": 20, "wall_seconds": 3600 },
      "consumed": { "actions": 0, "wall_seconds": 0 },
      "temporal": { "started_at_unix_s": ..., "ttl_seconds": 1800 },
      "priority": 10,
      "parent_claim_id": null,
      "status": "active"
    }
  ],
  "merge_queue": []
}
```

### 3.3 TTL / Heartbeat

- `ttl_seconds`: 1800 (30 min)
- Renouvellement: chaque commit incrémente `consumed.actions` et met à jour `temporal.started_at_unix_s`
- Expiration: si pas de renouvellement dans TTL → claim `expired` → libéré pour autres agents
- Application: un script `cynic agent heartbeat <claim_id>` (futur, V5)

## 4. Coordination post-isolation

Une fois l'isolation en place, la coordination devient possible:

### 4.1 Détection de conflit (futur)

```bash
cynic agent claim crates/cynic-host
# → vérifie .agent-claims.json de TOUS les worktrees
# → si conflit → refuse ou avertit
```

### 4.2 Merge queue (futur)

- Chaque agent push sa branche
- Merge bot vérifie:
  1. `cargo check` vert sur la branche
  2. Claims non conflictuels
  3. Tests passent
- Si conflit → notifie l'agent, attend résolution

### 4.3 Preemption cynic > baseline (inchangé)

- Un scheduler cynic peut préempter un claim baseline
- En pratique: pas encore actif (pas de scheduler natif)

## 5. Application immédiate (cette session)

### 5.1 Ce qu'on fait MAINTENANT

1. Créer un worktree isolé pour `hermes:main`
2. Y migrer le travail V4 finalisé
3. Ne PLUS travailler directement dans `/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC`

### 5.2 Commandes

```bash
# Créer le worktree isolé
git worktree add --detach ~/.workspace/CYNIC-hermes-main HEAD
cd ~/.workspace/CYNIC-hermes-main

# Vérifier qu'on est isolé
git rev-parse --show-toplevel
# → /home/user/.workspace/CYNIC-hermes-main

# Continuer le travail
git checkout agent/hermes/v1-debt-purge
```

### 5.3 Ce qu'on NE fait PAS

- Ne pas supprimer l'ancien worktree principal (les autres agents y travaillent peut-être)
- Ne pas forcer les autres agents à migrer (isolation progressive)
- Ne pas implémenter le scheduler natif maintenant (V5)

## 6. Limites connues

- Les worktrees partagent les objets Git (`.git/objects`) → pas de duplication de stockage
- Mais chaque worktree a son propre `index` et `HEAD` → isolation des fichiers de travail
- Si un agent force un `git push --force`, ça affecte tout le monde → confiance requise
- Le TTL/heartbeat n'est pas encore implémenté → les claims restent advisory

## 7. Prochaines étapes

1. **Immédiat**: migrer vers worktree isolé
2. **V5**: implémenter `cynic agent claim/heartbeat/release` avec enforce
3. **V5**: merge bot avec vérification `cargo check` + claims
4. **V5**: scheduler natif (mode `cynic`) avec preemption

Fichier: ce doc + `.hermes/plans/multi-agent-isolation.md` (exécution).
