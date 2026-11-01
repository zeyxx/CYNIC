# CYNIC vs AI Bus — Comparaison et leçons

Date : 2026-10-27

## Résumé

CYNIC et AI Bus (Robinho100) sont deux systèmes de coordination agentique avec des philosophies différentes.

- **CYNIC** : multi-agents, multi-hosts, Solana, fleet, contrats machine-readable, DAG d'isolation. Large mais peu fonctionnel en production.
- **AI Bus** : focalisé sur la collaboration Codex CLI + Claude Code CLI via MCP localhost. Plus étroit mais plus mature et validé.

## État réel de CYNIC

### Ce qui est avancé en surface
- 47 crates, 16 contrats machine-readable, DAG d'isolation avec 47 tests.
- Pipeline data-centric (capture → canonical) avec adapters multi-sources.
- Kanban, claims, scopes, worktrees.
- Audit, debt ledger, convergence.

### Ce qui n'est pas fonctionnel en pratique
- Pas de métriques mesurées en production.
- Pas d'observabilité/logging réel.
- Pas de modèle local intégré.
- Pas de context efficiency (FULL/DIFF/REFERENCE).
- Le pipeline data-centric n'a pas de couche model/projection.
- Peu de choses sont réellement utilisées au quotidien.

### Gaps identifiés (vs AI Bus)
1. **Stagnation detection** : CYNIC n'a pas de détection de ping-pong entre agents.
2. **Workspace Sync Guard** : CYNIC fait de la discipline Git manuelle, pas de prepare/publish automatisé.
3. **LOCAL worker** : CYNIC n'a pas de modèle local pour tâches LOW/MEDIUM.
4. **Context efficiency** : CYNIC transmet tout le contexte à chaque tour, pas de reconstruction prouvable.
5. **Guardrails formalisés** : CYNIC a des guardrails mais moins formalisés que ai-bus.

## Leçons d'AI Bus applicables à CYNIC

1. **Guardrails stricts et mesurables** : deadline, max turns, alternation, stagnation. Chaque garde doit être testable et mesurable.
2. **Workspace Sync Guard** : prepare (vérif Git) + publish (commit atomique, pas de `git add .`). Aligné avec la discipline CYNIC.
3. **LOCAL worker** : modèle local pour tâches LOW/MEDIUM avec routing déterministe. Réduit la dépendance aux modèles frontier.
4. **Context efficiency** : FULL/DIFF/REFERENCE avec reconstruction prouvable. Évite de transmettre tout le contexte à chaque tour.
5. **Observabilité** : ai-bus a des métriques, des logs, des tests E2E. CYNIC manque de tout ça.

## Recommandations pour CYNIC

1. **Prioriser la fonctionnalité sur la surface** : moins de crates, plus de choses qui marchent.
2. **Ajouter des métriques mesurées** : temps de réponse, taux de succès, stagnation count.
3. **Ajouter de l'observabilité** : logs structurés, traces, dashboards.
4. **Absorber les patterns de ai-bus** : guardrails, sync guard, context efficiency.
5. **Réduire la complexité** : CYNIC est en voie de dégraissage. Continuer.

## Conclusion

CYNIC a une architecture ambitieuse mais manque de fonctionnalité réelle. AI Bus est plus étroit mais plus mature et validé. CYNIC devrait absorber les patterns de ai-bus (guardrails, sync guard, observabilité) tout en réduisant sa complexité.
