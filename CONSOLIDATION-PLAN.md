# CYNIC CONSOLIDATION PLAN

##诊断 (Diagnostic)

Un engineer Anthropic verrait:
- **3 implémentations Python** qui font la même chose
- **Code JS dispersé** dans packages/node/ 
- **100+ fichiers de docs** dont la plupart sont contradictoires ou dépassés
- **0 deployment opérationnel** malgré 2 ans de travail

## Principes de Consolidation

### 1. UN SEUL codebase Python
On garde la meilleure version et on kill les autres.

### 2. Minimal viable
Tout ce qui n'est pas essential → DELETE

### 3. Docs minimalistes
Un seul README, un ARCHITECTURE.md, un CHANGELOG.md

### 4. Focus execution
Moins de plans, plus de code qui tourne.

---

##行动计划

### Phase 1: Audit (2h)
- [ ] Identifier les composants operational dans chaque version
- [ ] Identifier le code mort (duplicates, abandoned)
- [ ] Compter les lignes de code par composant

### Phase 2: Consolidation (4h)
- [ ] CHOISIR une base (probablement cynic-v1-python - plus complete)
- [ ] Merger les features des autres si useful
- [ ] Supprimer les 2 autres repertoires

### Phase 3: Nettoyage (2h)
- [ ] Supprimer docs/architecture/* (remplacer par 1 fichier)
- [ ] Supprimer docs/plans/* (on a assez planifié)
- [ ] Supprimer les MD duplicates a la racine

### Phase 4: Docker + Deploy (4h)
- [ ] Un seul Dockerfile fonctionnel
- [ ] Un seul docker-compose.yml
- [ ] Test de deployment

---

## Decision: Quelle base garder?

### Option A: cynic-v1-python
Avantages:
- Plus complet (adapters, dogs, judge, consensus, learning, storage, embeddings)
- wire event-bus deja implémenté
- 28 fichiers bien structurés

Inconvenients:
- Peut avoir du code legacy
- Pas certain que tout fonctionne

### Option B: cynic-v3  
Avantages:
- Plus recent
- Code plus clean?

Inconvenients:
- Incomplet
- Redondant avec v1

### Option C: Re-build from scratch
Avantages:
- Code propre, sans legacy
- On apprend de nos erreurs

Inconvenients:
- Perte de temps

---

## Recommandation

Garder **cynic-v1-python** comme base, mais:
1. Supprimer les dogs non-utiles (garder Guardian, Scout, Analyst, Oracle)
2. Supprimer storage/ si pas utilise
3. Supprimer embeddings/ si pas utilise  
4. Tester ce qui marche vraiment

En parallel: DELETE cynic-v3 et cynic-omniscient
