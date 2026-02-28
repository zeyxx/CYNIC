# CYNIC Dashboard — Minimal Interface

## Status
Le dashboard est accessible via l'API CYNIC:

```
GET http://localhost:8765/health
```

## Métriques affichées

### Consciousness
- Niveau actif (REFLEX/MICRO/MACRO/META)
- Cycles par niveau

### Learning
- États Q-table
- Total updates
- Pending flush

### Dogs
- 11 Dogs actifs: ANALYST, ARCHITECT, CARTOGRAPHER, CYNIC, DEPLOYER, GUARDIAN, JANITOR, ORACLE, SAGE, SCHOLAR, SCOUT

### Scheduler
- Workers par niveau
- Queue depths
- Timers health

## WebSocket Stream

```
ws://localhost:8765/ws/events
```

Stream en temps réel des événements CYNIC.
