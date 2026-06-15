# Organ Forge — Architecture Organique

**Type** : Organe sensoriel / réactif
**Rôle** : Perception du repo lifecycle, orchestration registry-driven, adaptation des gates
**Instance** : déclarée dans `infra/registry.json`
**Cron** : Toutes les 30 minutes (ou sur événement)

---

## Philosophie

Organ-forge n'est pas un script de nettoyage — c'est un **organe sensoriel** qui perçoit l'état du repo comme un système nerveux perçoit l'environnement.

Le repo est le système nerveux de l'organisme :
- Branches = pensées en cours
- PRs = décisions en attente
- Commits = souvenirs
- Dirty trees = travail actif
- Divergences = conflits cognitifs

L'organe perçoit ces signaux et adapte ses règles en conséquence.

---

## Architecture Data-Centric

### 1. PERCEPTION (INPUT)

L'organe perçoit l'état du repo via des capteurs :

```
Capteurs :
├── git status → dirty tree (travail en cours)
├── git branch → branches locales/remote (pensées)
├── git fetch → synchronisation (communication)
├── gh pr list → PRs ouverts (décisions)
├── git log → historique (mémoire)
└── cynic_coord_who → agents actifs (coordination)
```

**Données perçues** :
- Branches orphelines → signal de fin de session
- PRs stagnants > N jours → signal de blocage
- Dirty tree → signal de travail actif non-sécurisé
- Divergences (+/- commits) → signal de conflit
- Claims actifs → signal de coordination

### 2. TRANSFORMATION (TRAITEMENT)

Les données brutes → signal de décision :

```
Signal → Décision :
├── Orpheline + pushable → Push automatique
├── Orpheline + conflict → Alerter cortex
├── PR vert + >3 jours → Suggestion merge
├── PR rouge + >7 jours → Suggestion close
├── Dirty tree → Commit + push (sécurisation)
├── Divergence 2-way → Alerter, ne pas résoudre
└── Claim stale + >1h → Release automatique
```

**Règles adaptatives** :
- Fréquence des scans adapte selon l'activité (plus fréquent si activité élevée)
- Seuil de "stale" adapte selon le cycle de développement
- Gates s'adaptent selon l'échelle de l'organisme

### 3. STRUCTURATION (MODÉLISATION)

L'état perçu est structuré dans :

```
Registry (infra/registry.json) :
├── Organes actifs + statut
├── Dépendances entre organes
├── Politique `lifecycle.kernel_release`
├── Rôles dynamiques: build runner, kernel primary, orchestrateur
└── Preuve fleet Tailscale requise avant dispatch release

Gate state (.gate-passed) :
├── Timestamp du dernier check
└── État des checks cortex locaux

Kernel mempool (/agent-tasks) :
├── Requêtes de validation release (`gate-2-request`)
├── Requêtes build/deploy (`deploy-request`)
└── Résultats produits par l'exécuteur registry lifecycle
```

### 4. ANALYSE & COMPRÉHENSION

L'organe détecte des patterns :

```
Patterns :
├── Branches divergentes → Besoin de coordination
├── PRs ouverts sans review → Goulot d'étranglement
├── Sessions courtes + beaucoup de branches → Exploration
├── Sessions longues + peu de commits → Blocage
└── Claims non-released → Agent bloqué ou mort
```

### 5. FIABILITÉ (SURVIE SYSTÈME)

Principes ACID adaptés au repo :

- **Atomicité** : Un commit est soit complet, soit absent (pas de partial commit)
- **Cohérence** : Les règles du repo s'appliquent toujours ; pas de bypass cortex pour les markers release
- **Isolation** : Chaque cortex a sa branche (pas de collision sur main)
- **Durabilité** : Push sur origin = sauvegarde externe

### 6. RISQUES & ANTI-PATTERNS

- **Triggers en cascade** : Ne pas lancer des actions qui déclenchent d'autres organes (boucle)
- **Auto-merge aveugle** : Ne pas merger sans validation humaine (pour l'instant)
- **Prune agressif** : Ne pas supprimer de branches avec travail non-poussé
- **Gate bypass** : Pas de marker release écrit par les cortices ; le Level 2 appartient à l'exécuteur du lifecycle registre

---

## Évolution des Gates

Les gates actuelles sont bureaucratiques — elles s'exécutent toujours, même quand l'organisme est calme.

**Gates adaptatives** :

```
Échelle de l'organisme → Intensité des gates :

Petit (< 5 organes) :
├── Pre-commit : Secret check + basic fmt
├── Pre-push : Config drift + grep rules
└── CI : Full build + test + clippy

Moyen (5-15 organes) :
├── Pre-commit : Secret + fmt + lint
├── Pre-push : Full check
└── CI : Full + integration tests

Grand (>15 organes) :
├── Pre-commit : Fast checks only
├── Pre-push : Incremental checks
└── CI : Parallel + full suite
```

**Adaptation automatique** :
- Si >3 PRs ouverts → Relâcher les gates sur les branches feature
- Si 0 PRs ouverts → Resserer les gates (moins de pression)
- Si activité > 10 commits/jour → Gates incrémentales
- Si activité < 2 commits/jour → Gates complètes

---

## Coordination avec les Cortices

Organ-forge ne remplace pas les cortices — il les complète :

```
Cortex (Gemini/Claude/Codex) :
├── Perçoit les données du domaine
├── Transforme en code/décisions
└── Structure dans des fichiers

Organ Forge :
├── Perçoit l'état du repo
├── Transforme en signal de lifecycle
└── Structure dans le registre et le mempool kernel
```

**Passage cortex → organ-forge** :

1. Cortex observe/claim via kernel.
2. Cortex demande `gate-2-request` ou `deploy-request`.
3. Organ-forge lit `lifecycle.kernel_release.request_profiles`.
4. Organ-forge résout les intents et targets depuis `infra/registry.json`.
5. Organ-forge poste/traite la tâche dans `/agent-tasks`.
6. Le résultat revient au kernel, pas dans un journal markdown.

Les clients machine doivent appeler directement :

```
python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers
python3 organs/forge/src/entrypoints/cli.py delegate-profile <profile> --json
```

Ces sorties sont des objets JSON uniques. Les targets Makefile restent des
wrappers lisibles par un opérateur et peuvent retourner le code d'erreur de
`make` quand le lifecycle est bloqué.

---

## Métriques (Phase 2 readiness)

```
Métriques organ-forge :
├── Repo health score (0-1)
│   ├── Dirty tree ratio
│   ├── Branch divergence count
│   ├── PR age average
│   └── Gate failure rate
├── Action count (commits, pushes, alerts)
├── Response time (perception → action)
└── Coordination failures (claims, conflicts)
```

**Phase 2 goal** : Wire organ-forge au kernel via `/observe` → le repo health devient une métrique de l'organisme.

---

## Next Steps

1. ✅ Skill créé + cron actif
2. ✅ Registry entry + first run
3. [ ] Évoluer les gates vers adaptatives
4. [ ] Wire au kernel (Phase 2)
5. [ ] Ajouter métriques de repo health
6. [ ] Exposer une file `organ-forge-proposal` pour review admin (approve/deny)
7. [ ] Déclencher la remédiation après approval admin
8. [ ] Intégrer avec organ-keep (backup)
