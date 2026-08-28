# ADR-001: Architecture Hexagonale

**Date** : 2026-08-28
**Statut** : PROPOSÉ
**Contexte** : Le codebase PEF (216 fichiers, 36k LOC) a grandi par accumulation sans architecture imposée. Résultat : 73 imports cross-crate, dépendances circulaires implicites, 0 test d'intégration, impossible de tester un domaine isolément.

---

## Décision

Adopter une architecture hexagonale (ports & adapters) avec 3 domaines purs + 1 socle partagé.

## Architecture Cible

```
privacy-ethereum-framework/
├── framework/                    ← Socle partagé (types, ports, utilitaires)
│   ├── domain/                   ← Types purs partagés (RunManifest, ArtifactKind, etc.)
│   ├── ports/                    ← Interfaces abstraites (ArtifactStore, GitMetadataReader)
│   └── adapters/                 ← Adapters concrets partagés (filesystem, pydantic)
│
├── domains/
│   ├── mechanisms/               ← Domaine pur (aucune dépendance externe)
│   │   ├── model/                ← Types mécanismes (MechanismSpec, MechanismResult)
│   │   ├── ports/                ← Interfaces (MechanismRunner, ResultStore)
│   │   └── services/             ← Logique pure (catalog, validation)
│   │
│   ├── attacks/                  ← Domaine pur (aucune dépendance externe)
│   │   ├── model/                ← Types attaques (FirstSpyReport, AttackConfig)
│   │   ├── ports/                ← Interfaces (AttackRunner, ObservationProvider)
│   │   └── services/             ← Logique pure (analysis, scoring)
│   │
│   └── metrics/                  ← Domaine pur (aucune dépendance externe)
│       ├── model/                ← Types métriques (PrivacyMetricsVector, Scorecard)
│       ├── ports/                ← Interfaces (MetricsCalculator, Scorer)
│       └── services/             ← Logique pure (computation, aggregation)
│
├── adapters/
│   ├── infrastructure/           ← Adapters concrets (Web3, Kurtosis, Docker)
│   ├── persistence/              ← Stockage (JSON stores, Merkle tree, slides export)
│   └── ui/                       ← Interfaces utilisateur (CLI, Web UI)
│
├── application/
│   └── orchestration/            ← Orchestration (injection dépendances)
│       ├── campaign_runner.py    ← Campagne mécanismes
│       ├── attack_orchestrator.py← Campagne attaques
│       └── metrics_pipeline.py   ← Pipeline métriques
│
└── tests/
    ├── unit/                     ← Tests par domaine (mocked ports)
    ├── integration/              ← Tests avec vrais adapters (devnet)
    └── contract/                 ← Tests de ports (injection)
```

## Flux de Dépendance

```
                    ┌─────────────────────────────────────┐
                    │         application/                │
                    │    (orchestration + injection)      │
                    └──────────┬──────────────────────────┘
                               │ injecte
                    ┌──────────▼──────────────────────────┐
                    │          adapters/                  │
                    │   (infrastructure + persistence)    │
                    └──────────┬──────────────────────────┘
                               │ implémente
                    ┌──────────▼──────────────────────────┐
                    │  framework/ports/ + domains/*/ports/│
                    │       (interfaces abstraites)       │
                    └──────────┬──────────────────────────┘
                               │ définit
                    ┌──────────▼──────────────────────────┐
                    │      domains/*/model/ + services/   │
                    │         (logique pure)              │
                    └─────────────────────────────────────┘
```

**Règle d'or** : Un domaine ne dépend que de `framework/` et de lui-même. Jamais d'un autre domaine.

## Plan de Migration

### Phase 1 — Socle partagé (framework/)
- [ ] Déplacer `framework/domain/` (types purs : RunManifest, ArtifactKind, etc.)
- [ ] Définir les ports abstraits dans `framework/ports/`
- [ ] Déplacer les adapters concrets dans `framework/adapters/`

### Phase 2 — Séparation des domaines
- [ ] Extraire `domains/mechanismes/` (model + ports + services)
- [ ] Extraire `domains/attacks/` (model + ports + services)
- [ ] Extraire `domains/metrics/` (model + ports + services)
- [ ] Supprimer les imports cross-crate (73 → 0)

### Phase 3 — Injection de dépendances
- [ ] Créer `application/orchestration/` avec injection via ports
- [ ] Remplacer les instanciations directes par des ports injectés
- [ ] Tests de contrat sur les ports

### Phase 4 — Tests d'intégration
- [ ] Marquer les tests unitaires vs integration (`@pytest.mark.integration`)
- [ ] Tests end-to-end avec devnet éphémère
- [ ] CI avec séparation unit/integration

## Conséquences

**Positives :**
- Testabilité : chaque domaine testable isolément avec des mocks
- Maintenabilité : ajout d'un mécanisme = 1 fichier dans `domains/mechanismes/`
- Auditabilité : séparation claire entre mesuré/simulé
- Extensibilité : nouveau provider (autre que Kurtosis) = nouvel adapter

**Négatives :**
- Refactorisation lourde (~200 fichiers à déplacer)
- Courbe d'apprentissage pour les contributeurs
- Verbosité accrue (ports + adapters vs instanciation directe)

## Références

- Audit architecture : `docs/audit/external/pef/2026-08-28-architecture-audit.md`
- Audit production : `docs/audit/external/pef/2026-08-28-production-reality-check.md`
