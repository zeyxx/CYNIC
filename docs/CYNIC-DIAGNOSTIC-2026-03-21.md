# CYNIC Diagnostic Complet — 2026-03-21

*Radiographie du projet basée sur 3 audits simultanés + 2 sessions de crystallize-truth*

## 1. Le Crystal Loop est INERTE

Le feedback loop (verdict → observe → crystallize → inject → meilleur verdict) n'a JAMAIS produit un crystal mature.

- 20 crystals en DB, tous `Decaying`, domaine `workflow`
- Confidences : 0.09 à 0.33 — loin du seuil 0.618
- `format_crystal_context` filtre correctement pour `Crystallized`/`Canonical` → en trouve ZÉRO
- Les Dogs reçoivent ZÉRO crystal context → CYNIC opère en stateless

**Root cause :** Le seuil φ⁻¹ (0.618) est structurellement inatteignable :
- Q-Scores chess : max ~0.57 (Sicilian Defense)
- Running mean de 0.57 ne converge jamais vers 0.618
- Workflow frequency ratios : structurellement exclus par design (comment dans aggregate_observations)

**Le code est correct. L'architecture est correcte. Le seuil est mal calibré.**

Options :
1. Abaisser le seuil de cristallisation (ex: 0.50 au lieu de 0.618)
2. Seed manuellement des crystals pour valider le chemin d'injection
3. Router du contenu à plus haute confiance à travers le judge

## 2. Benchmark Chess Actuel

| Position | Verdict | Q-Total | Fidelity | PHI | Verify | Culture | Burn | Sovereignty |
|---|---|---|---|---|---|---|---|---|
| Sicilian Defense | **Howl** | 0.572 | 0.609 | 0.618 | 0.559 | 0.618 | 0.500 | 0.540 |
| Scholar's Mate | **Bark** | 0.182 | 0.075 | 0.125 | 0.254 | 0.305 | 0.200 | 0.250 |
| Fool's Mate | **Bark** | 0.079 | 0.075 | 0.075 | 0.075 | 0.075 | 0.100 | 0.075 |

Ordering correct. Sicilian = Howl borderline (0.572 vs threshold 0.528).
Scholar's Mate anomaly détectée sur VERIFY (spread 0.568 > φ⁻²).

## 3. Module Stability (47 fichiers, ~10,891 lignes)

### STABLE (35% — à geler)
circuit_breaker, config, verdict_cache, deterministic dog, inference dog,
task_health, health_loop, remediation, pipeline, embedding, chat, summarization,
observe, response, probe/*

### UNSTABLE (53% — fix magnets)
| Rang | Fichier | Commits (6j) | Problème |
|------|---------|-------------|----------|
| 1 | main.rs | 30 | Hub de câblage — chaque feature atterrit ici |
| 2 | storage/surreal.rs | 22 | SurrealDB 3.x turbulence + SQL bugs |
| 3 | api/mcp/mod.rs | 21 | Plus gros fichier, tout collide |
| 4 | api/rest/health.rs | 17 | Face observable — status codes, auth |
| 5 | judge.rs | 15 | Cœur du pipeline — seuils, anomalies |
| 6 | domain/usage.rs | 9 | Flush path, cost tracking |
| 7 | domain/ccm.rs | 8 | SQL co-occurrence, dedup |

### DEAD (3% — à brûler)
- `domain/temporal.rs` — 211 lignes, `dog_scores[i % 7]` relabelé "Past/Future"
- `InferenceRequest/Response/BackendCapability/MockBackend::infer()` — ~120 lignes MCTS-era

## 4. Le Cycle Fix-and-Shift

```
50 commits en 6 jours : 28 fix, 7 feat, 4 docs, 3 refactor, 2 test, 2 burn
Fix/feat ratio : 4:1
```

6 causes simultanées :
1. Pas de tests E2E — chaque fix casse l'aval
2. Surface trop large — bugs découverts en parallèle
3. Vision change — hackathon → solidité → compound → OS → agents
4. Phase post-hackathon (normal)
5. 2 sessions sans scope = réactif
6. AI drift — Claude optimise "busy" au lieu de "productive" sans guidage humain

## 5. Ce Qui DOIT Être Fait (MUST)

### MUST 1 : Débloquer le crystal loop
Le seuil 0.618 empêche toute cristallisation. Sans crystals, CYNIC = wrapper stateless.
Action : calibrer le seuil OU seeder des crystals test pour valider l'injection.

### MUST 2 : Brûler le code mort
331 lignes de dead architecture (temporal fake + MCTS types). Rule 21 : no dead architecture.

### MUST 3 : Stabiliser les fix magnets
main.rs (30 commits) et mcp/mod.rs (21 commits) sont trop gros et attirent les fixes.
Extraire le câblage de main.rs. Découper mcp/mod.rs.

### MUST 4 : WebSocket /ws
Seule pièce manquante pour l'observabilité (détection model drift, session collision, push events).

### MUST 5 : Protocole ILC (Isolated Learning Cycle)
OBSERVE → CLAIM (worktree) → CHANGE → PROVE (make check + dry-run merge) → RELEASE.
Pas de PROVE = pas de RELEASE. Gate dur.

## 6. Ce Qui PEUT Attendre (WANT)

- `cynic run` launcher (sucre syntaxique)
- Capability manifests (pas nécessaire pour 4 agents de confiance)
- Scheduler/preemption (pas un OS)
- WASM sandboxing (pas de code untrusted aujourd'hui)
- API versioning (pas de consommateurs externes)
- IPv6 (design for, deploy later)

## 7. Ordre d'exécution

1. **Burn dead code** (temporal + MCTS types) — 1 heure, réduit le bruit
2. **Calibrer le crystal seuil** — débloquer le feedback loop, PUIS valider A/B
3. **WebSocket /ws** — observabilité temps réel
4. **Découper main.rs et mcp/mod.rs** — réduire les fix magnets
5. **Adopter ILC** — chaque session = worktree + objectif + prove gate
