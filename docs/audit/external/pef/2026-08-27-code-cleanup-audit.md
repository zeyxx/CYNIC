# Audit : Assainissement & Convergence (PEF)

**Date** : 2026-08-27
**Scope** : Code mort, duplications, placeholders, incohérences structurelles
**Base** : `attacks/`, `mechanisms/`, `metrics/` synchronisés (SHA `51217ce2`)

---

## 1. Code mort / Orphelins

### 1.1 `attacks/application/timing_attack_runner.py` (147 lignes)

**Statut** : ORPHELIN — importé par personne.

```bash
$ grep -r "timing_attack_runner\|TimingAttackRunner" --include="*.py" .
./attacks/application/timing_attack_runner.py:class TimingAnalyzerRunner:
```

`attacks/timing.py` importe `TimingAttackRunner` depuis `attacks/application/timing_runner.py`, pas depuis `timing_attack_runner.py`.

**Recommendation** : SUPPRIMER ou fusionner avec `timing_runner.py`.

### 1.2 `mechanisms/baseline/baseline.py`

**Statut** : ORPHELIN — pas d'imports trouvés.

Le mécanisme baseline est listé dans `mechanism_catalog.py` avec `script_path = mechanisms_dir / "baseline" / "baseline.py"` mais ce fichier n'est jamais exécuté. Le `mechanism_campaign_runner.py` appelle le runner via le port `MechanismRunner` qui est injecté. Le script baseline direct n'est pas utilisé.

**Recommendation** : Vérifier si c'est un stub à supprimer ou un runner alternatif à documenter.

### 1.3 Mécanismes non-référencés

Fichiers dans `mechanisms/` sans imports externes :

| Fichier | Statut |
|---|---|
| `mechanisms/dandelion/dandelion.py` | Orphelin |
| `mechanisms/mixer/mixer.py` | Orphelin (utilise tornado/runner.py via catalog) |
| `mechanisms/relay/relay.py` | Orphelin |
| `mechanisms/smoke.py` | Orphelin |
| `mechanisms/run_stage1.py` | Orphelin |
| `mechanisms/tornado/benchmark_strict.py` | Orphelin |
| `mechanisms/tornado/build_strict.py` | Orphelin |
| `mechanisms/tornado/check_strict_ready.py` | Orphelin |

Ces fichiers sont des CLI entrypoints alternatifs ou des scripts historiques. Le `mechanism_catalog.py` référence les bons chemins (`tornado/runner.py` pour mixer, `privacy_pools/privacy_pools.py` pour privacy_pools, etc.), mais certains mécanismes de base (dandelion, relay) n'ont pas de script actif.

**Recommendation** : Auditer chaque fichier orphelin et supprimer ou documenter.

---

## 2. Duplications

### 2.1 Entrypoints CLI (`attacks/*.py`)

Chaque attaque a :
- Un entrypoint CLI dans `attacks/<name>.py` (65-68 lignes)
- Un runner dans `attacks/application/<name>_runner.py` (150-330 lignes)

Les entrypoints sont quasi-identiques :
```python
parser = argparse.ArgumentParser(description="...")
parser.add_argument("--scenario", default="campaign")
parser.add_argument("--no-record", action="store_argument")
parser.add_argument("--json", action="store_true")
runner = SomeRunner(...)
report = runner.run(scenario=...)
```

**Impact** : 11 entrypoints × ~65 lignes = ~715 lignes de boilerplate.

**Recommendation** : Extraire un `cli_factory()` générique dans `attacks/run_<name>.py` ou utiliser `attacks/run_sweep.py` comme modèle unique.

### 2.2 Mécanismes expérimentaux vs. canoniques

| Mécanisme | Experimental | Canonique |
|---|---|---|
| mixer | `mechanisms/experimental/mixer/mixer.py` | `mechanisms/mixer/mixer.py` |
| tornado_cash | `mechanisms/experimental/tornado_cash/tornado_cash.py` | (via tornado/runner.py) |
| relay_stealth | `mechanisms/experimental/relay_stealth/relay_stealth.py` | — |
| stealth_direct | `mechanisms/experimental/stealth_direct/stealth_direct.py` | — |

Ces doublons expérimentaux sont des versions antérieures qui coexistent avec les versions canoniques.

**Recommendation** : Supprimer les versions expérimentales obsolètes ou les déplacer vers `archive/`.

---

## 3. Placeholders non implémentés

### 3.1 `_confidentiality()` dans `transaction_graph_runner.py`

```python
# l.277-278
def _confidentiality(self, bundle: MechanismArtifactBundle) -> str:
    return "visible"
```

**Impact** : 100% des mécanismes ont `confidentiality = visible` dans le rapport final. Le catalog définit des `confidentialité_class` différentes (`sender_unlinkable`, `recipient_private`, etc.) mais elles ne sont pas utilisées dans le transaction graph.

**Recommendation** : Utiliser `bundle.metadata.get("confidentiality_class")` ou implémenter une mesure réelle.

### 3.2 `_candidate_count()` heuristiques

```python
# l.203-204
if graph_profile == "dandelion":
    return 1
# l.243-244
if graph_profile == "relay":
    return 1
```

**Impact** : anonymity_set = 1 pour relay et dandelion, quelles que soient les observations réelles.

**Recommendation** : Mesurer depuis les données on-chain (destinataires uniques, nb de fragments, etc.).

### 3.3 `_linking_probability()` codée en dur

```python
# l.270-274
if graph_profile in {"stealth", "relay", "relay_stealth"}:
    return 0.0
```

**Impact** : LP = 0.0 pour tous les mécanismes stealth/relay, sans mesure réelle.

**Recommendation** : Implémenter un algorithme de linking basé sur les features du graphe.

---

## 4. Incohérences structurelles

### 4.1 `DEFAULT_MECHANISM_METADATA` exclut les compositions

```python
# metrics_engine.py l.65-107
DEFAULT_MECHANISM_METADATA = {
    "baseline": ..., "mixer": ..., "tornado_cash": ...,
    "stealth": ..., "dandelion": ..., "relay": ..., "privacy_pools": ...
}
```

Les 5 compositions (`tornado_tor`, `ultimate_mixer`, `relay_stealth`, `tornado_dandelion`, `tornado_dandelion_stealth`) sont dans `mechanism_catalog.py` mais pas dans `DEFAULT_MECHANISM_METADATA`.

**Impact** : Les compositions sont invisibles dans le rapport métrique final et le leaderboard SOTA.

**Recommendation** : Étendre `DEFAULT_MECHANISM_METADATA` ou supprimer le filtre.

### 4.2 `_mechanism_rows()` ignore les compositions

```python
# metrics_engine.py l.470
if mechanism not in self.DEFAULT_MECHANISM_METADATA:
    continue
```

Même problème que 4.1.

### 4.3 Incohérence schema_version

Certains modules utilisent `"privacy-ethereum-framework.run.v1"` (ex: `metrics_engine.py`), d'autres `"1.0.0"` (ex: `timing_attack_runner.py`).

**Recommendation** : Unifier vers un seul schéma canonique.

### 4.4 `timing_attack_runner.py` vs `timing_runner.py`

| Fichier | Classe | Importé par |
|---|---|---|
| `attacks/application/timing_runner.py` | `TimingAttackRunner` | `attacks/timing.py` |
| `attacks/application/timing_attack_runner.py` | `TimingAnalyzerRunner` | **personne** |

Deux implémentations différentes de timing attack. L'une est active, l'autre est morte.

---

## 5. Résumé des actions

| Priorité | Action | Fichiers impactés | LOC à supprimer |
|---|---|---|---|
| **P0** | Supprimer `timing_attack_runner.py` (orphelin) | 1 fichier | -147 |
| **P0** | Étendre `DEFAULT_MECHANISM_METADATA` aux compositions | `metrics_engine.py` | — |
| **P1** | Supprimer/oublier les mécanismes expérimentaux obsolètes | `mechanisms/experimental/` | ~500 |
| **P1** | Corriger `_confidentiality()` pour utiliser `confidentiality_class` | `transaction_graph_runner.py` | — |
| **P1** | Corriger `_candidate_count()` et `_linking_probability()` pour mesures réelles | `transaction_graph_runner.py` | — |
| **P2** | Extraire CLI boilerplate dans un factory | `attacks/*.py` (11 fichiers) | ~500 |
| **P2** | Unifier `schema_version` | tous les fichiers attackers/mechanisms | — |
| **P2** | Supprimer les mécanismes orphelins (`dandelion.py`, `relay.py`, `smoke.py`, etc.) | `mechanisms/` | ~300 |

---

## 6. Impact sur la couverture tests

Avant toute suppression, vérifier :

```bash
$ grep -r "timing_attack_runner\|TimingAnalyzerRunner" tests/
$ grep -r "baseline/baseline\|dandelion/dandelion" tests/
```

Si les tests couvrent les versions orphelines, il faut les supprimer aussi (ou les mettre à jour).
