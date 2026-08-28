# Audit : Architecture & Canonicité (PEF)

**Date** : 2026-08-28
**Scope** : Structure du codebase, séparation des responsabilités, dépendances, cohérence
**Méthode** : Analyse statique des imports, LOC, boundaries, patterns récurrents
**Base** : 216 fichiers Python, ~36k LOC (hors tests)

---

## Diagnostic synthétique

Le framework a grandi par accumulation fonctionnelle sans architecture imposée. Résultat : **7 crates Python, 216 fichiers, 36k LOC, 0 contrat de port formalisé, 0 test d'intégration end-to-end, 0 séparation mécanismes/attacks/metrics respectée.**

Le code "fonctionne" (les tests unitaires passent, le devnet tourne) mais **n'est pas maintenable, pas extensible, pas auditable** par un tiers.

---

## 1. Violations de séparation des responsabilités

### 1.1 Import cross-boundary (dépendances inverses)

**mechanisms → attacks** (6 violations) :

```python
# mechanisms/run_campaign.py
from attacks.adapters.env_nodes import DotenvObservationNodeProvider
from attacks.adapters.websocket_first_spy import WebsocketFirstSpyCapture
from attacks.application.first_spy_runner import FirstSpyRunner

# mechanisms/application/mechanism_campaign_runner.py
from attacks.domain.attack_models import TxGroundTruth
from attacks.adapters.mechanism_artifacts import collect_submission_times, collect_tx_hashes, collect_tx_origins

# mechanisms/ports/mechanism_ports.py
from attacks.domain.attack_models import FirstSpyReport
```

**Problème** : `mechanismes` ne devrait connaître ni `attacks` ni `metrics`. C'est le principe de l'hexagonal : les mécanismes produisent des événements, les attaques les consomment via des ports.

**attacks → mechanisms** (9 violations) :

```python
# attacks/application/first_spy_runner.py
from mechanisms.runtime.entropy import poisson_timer_from_env

# attacks/application/run_sybil_scenario.py
from mechanisms.application.mechanism_catalog import MechanismCatalog
from mechanisms.noise_generator import run_noise_generator
from mechanisms.runtime.warmup import wait_for_chain_ready

# attacks/adapters/rpc_probe_transactions.py
from mechanisms.runtime.entropy import poisson_timer_from_env

# attacks/adapters/mechanism_artifacts.py
from mechanisms.application.mechanism_catalog import MechanismCatalog

# attacks/domain/attack_models.py
from mechanisms.runtime.topology import propagation_delay_ms
```

**Problème** : Les attaques ne devraient pas dépendre de l'interne des mécanismes (catalog, runtime, noise generator). Elles devraient consommer des artefacts via un port (ArtifactStore).

### 1.2 Responsabilités mélangées

| Fichier | Responsabilités détectées |
|---|---|
| `metrics_engine.py` (1478 LOC) | Lecture, calcul, scoring, rendering, validation, historique |
| `attack_models.py` (832 LOC) | Types attacks + types first-spy + types transaction-graph + types timing + types dust + types eclipse + types web2 + types zk-bridge + types censorship + types sybil |
| `sybil_ml_spy.py` (771 LOC) | Features, modèle, entraînement, évaluation, persistance |
| `run_sybil_scenario.py` (578 LOC) | Orchestration, ground-truth, évaluation, rapport |

**Problème** : Un fichier = une responsabilité. Ici un fichier = un domaine entier.

---

## 2. Absence de ports & contrats formalisés

### 2.1 Ports existants (informels)

```
framework/ports/artifact_store.py        — FileArtifactStore (filesystem)
mechanisms/ports/mechanism_ports.py     — MechanismRunner, ResultStore, GitMetadataReader
attacks/ports/attack_ports.py           — AttackRunner (1 interface vide)
metrics/ports/metric_ports.py           — MetricsInputReader, MetricsReportWriter
```

**Problème** : Les ports sont des interfaces Python abstraites mais :
- Aucun n'est utilisé pour l'injection dans les runners concrets
- `AttackRunner` est une coquille vide (0 méthode)
- Les runners instancient directement les adapters concrets

### 2.2 Pas d'inversion de dépendance

```python
# mechanisms/tornado/strict_runner.py
from mechanisms.tornado.upstream_bridge import warm_upstream_bridge_persistent
from mechanisms.tornado.strict_models import TornadoStrictNote, TornadoMerkleProof
from mechanisms.tornado.strict_bundle import inspect_strict_bundle
from mechanisms.runtime.tx_campaign import MultiOriginTransactionRunner
from mechanisms.runtime.stealth_support import ...
from mechanisms.runtime.origins import MultiOriginRpcPool, OriginClient
from mechanisms.runtime.network_privacy import network_privacy_metadata
from mechanisms.runtime.console import verbose_console
from mechanisms.runtime.config import mechanism_tx_count
from mechanisms.runtime.anti_temporal import ...
from mechanisms.runtime.anti_dust import dust_isolation_metadata
```

**Problème** : 10 imports directs vers du concret. Aucun port injecté. Impossible de tester `strict_runner.py` isolément.

---

## 3. Duplications & incohérences

### 3.1 Entrypoints CLI dupliqués

11 fichiers `attacks/*.py` avec la même structure :
```python
parser = argparse.ArgumentParser(description="...")
parser.add_argument("--scenario", default="campaign")
parser.add_argument("--no-record", action="store_true")
parser.add_argument("--json", action="store_true")
runner = SomeRunner(...)
report = runner.run(scenario=...)
```

**Impact** : ~715 lignes de boilerplate. Aucune factorisation.

### 3.2 Mécanismes expérimentaux vs. canoniques

| Mécanisme | Experimental | Canonique | Script actif |
|---|---|---|---|
| mixer | `experimental/mixer/mixer.py` | `mixer/mixer.py` | `tornado/runner.py` (via catalog) |
| tornado_cash | `experimental/tornado_cash/tornado_cash.py` | — | `tornado/runner.py` |
| relay_stealth | `experimental/relay_stealth/relay_stealth.py` | — | — |
| stealth_direct | `experimental/stealth_direct/stealth_direct.py` | — | — |
| tornado_dandelion | `experimental/tornado_dandelion/tornado_dandelion.py` | — | — |
| ultimate_mixer | `experimental/ultimate_mixer/ultimate_mixer.py` | — | — |

**Problème** : Les scripts canoniques (baseline, dandelion, relay, stealth, mixer) ont été supprimés du tracking mais sont restaurés à la demande. Les mécanismes expérimentaux sont archivés mais toujours dans le repo.

### 3.3 Types dupliqués

| Type | Fichier 1 | Fichier 2 |
|---|---|---|
| `TimingMechanismReport` | `attacks/domain/attack_models.py` | `attacks/application/timing_attack_runner.py` (supprimé) |
| `FirstSpyReport` | `attacks/domain/attack_models.py` | `mechanisms/ports/mechanism_ports.py` (importé) |
| `TxGroundTruth` | `attacks/domain/attack_models.py` | `mechanisms/application/mechanism_campaign_runner.py` (importé) |

---

## 4. Tests : unitaires uniquement, 0 intégration

### 4.1 Couverture réelle

| Type | Fichiers | Couverture |
|---|---|---|
| Tests unitaires | 42 fichiers | Logique Python isolée |
| Tests d'intégration | 0 | Aucun |
| Tests end-to-end | 0 | Aucun |
| Tests de contrat (ports) | 0 | Aucun |

### 4.2 Tests qui nécessitent un devnet

```python
# tests/test_contracts.py — nécessite web3 + devnet
# tests/test_first_spy_development.py — nécessite WebSocket live
# tests/test_sybil_calibration.py — nécessite observations live
```

Ces tests ne passent que sur un devnet actif. Aucun mock, aucun fixture, aucun test de contrat sur les ports.

### 4.3 Tests de régression fragiles

```python
# tests/test_network_timing_regression.py
source = open("/home/s/privacy-ethereum-framework/...").read()
assert "time.sleep(0.25)" not in source
```

Tests basés sur des chemins absolus d'un autre environnement. Cassés par design.

---

## 5. Structure de dossiers incohérente

```
attacks/
├── adapters/          ← OK
├── application/       ← OK
├── domain/            ← OK
├── ports/             ← OK (mais vide)
├── censorship.py      ← entrypoint CLI
├── dust_attack.py     ← entrypoint CLI
├── eclipse_attack.py  ← entrypoint CLI
├── timing.py          ← entrypoint CLI
├── tx_graph.py        ← entrypoint CLI
├── value_fingerprint.py ← entrypoint CLI
├── web2_correlation.py ← entrypoint CLI
├── zk_bridge_correlation.py ← entrypoint CLI
├── run_first_spy.py   ← entrypoint CLI
├── run_first_spy_with_noise.py ← entrypoint CLI
├── run_sweep.py       ← entrypoint CLI
├── p2p_dos_amplification.py ← module standalone
└── temporal_intersection.py ← entrypoint CLI
```

**Problème** : Mélange entrypoints CLI + modules standalone + adapters + domain + application dans le même namespace. Pas de séparation `cli/` / `lib/`.

---

## 6. Résumé des anti-patterns

| Anti-pattern | Sévérité | Impact |
|---|---|---|
| Imports cross-boundary (mécanismes ↔ attaques) | **CRITIQUE** | Impossible de tester isolément, impossible d'extaquer un domaine |
| Fichiers > 1000 LOC | **ÉLEVÉ** | Non-maintenable, non-revuevable |
| 0 injection de dépendances | **ÉLEVÉ** | 0 testabilité, 0 modularité |
| 0 test d'intégration | **ÉLEVÉ** | Régressions invisibles |
| Entrypoints CLI dupliqués | **MOYEN** | ~715 LOC boilerplate |
| Types dupliqués entre domaines | **MOYEN** | Incohérence, synchronisation manuelle |
| Mécanismes expérimentaux archivés mais trackés | **FAIBLE** | Confusion |
| Tests avec chemins absolus | **FAILE** | Cassés hors environnement d'origine |

---

## 7. Architecture cible (proposition)

### 7.1 Séparation hexagonale

```
pef/
├── domains/
│   ├── mechanisms/          ← Pure domain (no deps)
│   │   ├── model/           ← Types, events
│   │   ├── ports/           ← Interfaces (MechanismRunner, ArtifactStore)
│   │   └── services/        ← Domain services (pure logic)
│   ├── attacks/             ← Pure domain (no deps)
│   │   ├── model/
│   │   ├── ports/           ← Interfaces (AttackRunner, ObservationProvider)
│   │   └── services/
│   └── metrics/             ← Pure domain (no deps)
│       ├── model/
│       ├── ports/           ← Interfaces (MetricsCalculator, Scorer)
│       └── services/
├── adapters/
│   ├── infrastructure/      ← Web3, FileSystem, Kurtosis, Docker
│   ├── ui/                  ← CLI, Web UI
│   └── persistence/         ← JSON stores, Merkle tree
├── application/
│   ├── orchestration/       ← Campaign runner, job manager
│   └── config/              ← Env, .env, network.yaml
└── tests/
    ├── unit/                ← Tests par domaine (mocked)
    ├── integration/         ← Tests avec vrais adapters (devnet)
    └── contract/            ← Tests de ports (injection)
```

### 7.2 Flux de dépendance

```
orchestration → ports (interfaces) ← adapters (concrets)
                     ↑
               domains (pure)
```

- Les domaines ne dépendent que d'eux-mêmes
- Les adapters implémentent les ports définis par les domaines
- L'orchestration injecte les adapters via les ports

### 7.3 Contrats de test

```python
# tests/contracts/test_mechanism_port.py
class FakeMechanismRunner(MechanismRunner):
    def run(self, spec: MechanismSpec) -> MechanismResult:
        return MechanismResult(...)

# tests/integration/test_baseline_live.py
# Nécessite devnet — marqué @pytest.mark.integration
```

---

## 8. Plan de convergence

### Phase 1 — Stabilisation (courant)
- [ ] Supprimer les imports cross-boundary (6 mechanisms→attacks, 9 attacks→mechanisms)
- [ ] Extraire CLI boilerplate dans un factory
- [ ] Scinder `attack_models.py` en un type par fichier
- [ ] Scinder `metrics_engine.py` en calculateur + scorer + renderer

### Phase 2 — Formalisation
- [ ] Définir les ports comme interfaces abstraites (ABC)
- [ ] Injecter les adapters via les ports dans tous les runners
- [ ] Ajouter des tests de contrat sur les ports

### Phase 3 — Intégration
- [ ] Tests d'intégration avec devnet (marqués `@pytest.mark.integration`)
- [ ] CI avec devnet éphémère (GitHub Actions + Kurtosis)
- [ ] Tests end-to-end : `make run` → `make metrics` → `make red-team`

### Phase 4 — Extraction
- [ ] Extraire `framework/` comme crate réutilisable
- [ ] Extraire `mechanism_catalog.json` comme source de vérité externe
- [ ] Documenter l'architecture cible (ADR)

---

## 9. Verdict

Le framework est **fonctionnel mais pas canonique**. Il marche parce que les développeurs connaient les implicites. Un nouvel contributeur ne peut pas :
- Comprendre la séparation des responsabilités
- Tester un domaine isolément
- Ajouter un mécanisme sans toucher à 5 fichiers
- Ajouter une attaque sans toucher à 3 domaines

**Recommandation** : Avant toute nouvelle fonctionnalité, figer l'architecture cible (ADR-001) et refactorer par couches. Sinon chaque ajout augmente la dette technique de manière exponentielle.
