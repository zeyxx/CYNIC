# Organ Anvil — Architecture Organique

**Type** : Organe sensoriel / réactif
**Rôle** : Perception du repo lifecycle, signal de décision, adaptation des gates
**Instance** : organ-anvil-hermes-agent
**Cron** : Toutes les 30 minutes (ou sur événement)

---

## Philosophie

Organ-anvil n'est pas un script de nettoyage — c'est un **organe sensoriel** qui perçoit l'état du repo comme un système nerveux perçoit l'environnement.

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
└── Coordonnées (MCP tools)

Handoff (.handoff.md) :
├── Journal de session
├── Décisions prises
└── Next steps

Gate state (.gate-passed) :
├── Timestamp du dernier check
└── État de la CI
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
- **Cohérence** : Les règles du repo s'appliquent toujours (pas de bypass sauf organ-anvil)
- **Isolation** : Chaque cortex a sa branche (pas de collision sur main)
- **Durabilité** : Push sur origin = sauvegarde externe

### 6. RISQUES & ANTI-PATTERNS

- **Triggers en cascade** : Ne pas lancer des actions qui déclenchent d'autres organes (boucle)
- **Auto-merge aveugle** : Ne pas merger sans validation humaine (pour l'instant)
- **Prune agressif** : Ne pas supprimer de branches avec travail non-poussé
- **Gate bypass** : Bypass seulement pour organ-anvil, pas pour les cortices

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

Organ-anvil ne remplace pas les cortices — il les complète :

```
Cortex (Gemini/Claude/Codex) :
├── Perçoit les données du domaine
├── Transforme en code/décisions
└── Structure dans des fichiers

Organ Anvil :
├── Perçoit l'état du repo
├── Transforme en signal de maintenance
└── Structure dans le lifecycle
```

**Handoff entre cortex et organ-anvil** :

1. Cortex termine → écrit dans `.handoff.md`
2. Organ-anvil lit `.handoff.md` → perçoit la fin de session
3. Organ-anvil sécurise → commit + push + cleanup
4. Organ-anvil écrit → état dans `.handoff.md`
5. Prochain cortex lit → contexte complet

---

## Métriques (Phase 2 readiness)

```
Métriques organ-anvil :
├── Repo health score (0-1)
│   ├── Dirty tree ratio
│   ├── Branch divergence count
│   ├── PR age average
│   └── Gate failure rate
├── Action count (commits, pushes, alerts)
├── Response time (perception → action)
└── Coordination failures (claims, conflicts)
```

**Phase 2 goal** : Wire organ-anvil au kernel via `/observe` → le repo health devient une métrique de l'organisme.

---

## Next Steps

1. ✅ Skill créé + cron actif
2. ✅ Registry entry + first run
3. [ ] Évoluer les gates vers adaptatives
4. [ ] Wire au kernel (Phase 2)
5. [ ] Ajouter métriques de repo health
6. [ ] Exposer une file `organ-anvil-proposal` pour review admin (approve/deny)
7. [ ] Déclencher la remédiation après approval admin
8. [ ] Intégrer avec organ-keep (backup)
