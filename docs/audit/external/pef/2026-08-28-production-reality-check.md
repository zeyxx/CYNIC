# Audit : Production Reality Check (PEF)

**Date** : 2026-08-28
**Scope** : Données de production réelles vs. revendications
**Méthode** : Analyse des artefacts JSON produits par `make run` sur devnet live
**Base** : `results/raw/baseline.json`, `results/mechanisms.json`, `results/metrics.json` (run 2026-08-28)

---

## Synthèse

La pipeline de production fonctionne et produit des artefacts **réels**. Les transactions sont signées, diffusées, incluses en bloc, et les receipts sont récupérées. Cependant, les métriques dérivées dans `metrics.json` ne reflètent qu'une fraction de ce qui est réellement mesuré sur le devnet.

---

## 1. Ce qui est RÉEL (vérifié sur devnet)

### 1.1 Transactions on-chain

```json
{
  "mechanism": "baseline",
  "tx_hashes": ["b530496d...", "9bf1c3bc...", ...],
  "tx_origins": {
    "b530496d...": "node-1",
    "9bf1c3bc...": "node-2",
    ...
  }
}
```

- **10 transactions** signées avec `TEST_PRIVATE_KEY_1`
- **10 confirmations** (receipts récupérées)
- **Origines distribuées** sur node-1 à node-10
- **12 nœuds** participants (Geth/Lighthouse)

### 1.2 Métriques de performance mesurées

| Métrique | Source | Valeur (baseline) |
|---|---|---|
| Latence | `timestamp(inclusion) - timestamp(submission)` | 3.45s |
| Gas utilisé | `receipt.gasUsed` | 21000 (standard ETH transfer) |
| Throughput | `tx_count / submission_window` | 0.54 tps |
| Block number | `web3.eth.block_number` | 35 → 45 (progression) |

### 1.3 Pipeline d'attaques (partiel)

- **First-spy** : 60 transactions de probe envoyées, observation WebSocket live sur 12 spy nodes
- **Transaction graph** : Analyse du graphe on-chain (adresses, edges, composantes)
- **Timing** : Corrélation temporelle deposit/withdraw (si applicable)

---

## 2. Ce qui est SIMULÉ (malgré le devnet live)

### 2.1 Métriques `assumed` dans `metrics.json`

```json
"metric_provenance": {
  "privacy.anonymity_set": {"source": "assumed", "basis": "baseline direct transfer has one sender"},
  "privacy.shannon_entropy": {"source": "assumed", "basis": "baseline direct transfer has no anonymity set"},
  "privacy.linking_probability": {"source": "assumed", "basis": "sender and recipient are directly observable"},
  "privacy.first_spy_precision": {"source": "assumed", "basis": "public mempool transit assumed observable until first-spy is measured"},
  "privacy.confidentiality": {"source": "assumed", "basis": "amount and recipient are public on-chain"}
}
```

**Problème** : Même avec un devnet live, le baseline reste `assumed` car :
- `transaction_graph_runner._confidentialité()` retourne `"visible"` en dur (corrigé dans le code mais pas dans les artefacts existants)
- `transaction_graph_runner._candidate_count()` retourne 1 pour baseline (hardcoded)
- `transaction_graph_runner._linking_probability()` retourne 1.0 pour baseline (hardcoded)

### 2.2 Métriques `modeled`

```json
"privacy.anonymity_set": {"source": "modeled", "basis": "number of deposits in fixed-size test cohort"}
```

**Problème** : L'anonymity set du mixer est modélisé sur la taille du cohort, pas mesuré sur les transactions réelles.

---

## 3. Décalage entre runtime et rapports

### 3.1 Ce que le runtime fait réellement

1. `make up` → Kurtosis déploie 12 nœuds, génère `.env`
2. `make run` → Pour chaque mécanisme :
   - Warmup (attendre que la chaîne produise des blocs)
   - Envoi de transactions signées
   - Attente des receipts
   - Écriture de `results/raw/<mechanism>.json`
3. `make metrics` → Lecture des artefacts, calcul des métriques, écriture de `results/metrics.json`

### 3.2 Ce que les rapports montrent

- **Performance** : Réelle (latence, gas, throughput mesurés on-chain)
- **Privacy** : Partiellement simulée (anonymity set, entropy, linking prob = assumed/modeled)
- **Adversaires** : Mixte (first-spy mesuré live, censorship/dust/eclipse simulés)

---

## 4. Revendications vs. Réalité

| Revendication README | Réalité |
|---|---|
| "Measured adversarial conditions" | **Vrai** pour first-spy, timing, transaction-graph (partiellement) |
| "Fully reproducible" | **Vrai** — Kurtosis + artefacts immuables + Merkle root |
| "Under active adversarial conditions" | **Partiel** — Seul first-spy est actif, les autres attaques sont passives ou simulées |
| "Quantify privacy on 4 formal dimensions" | **Partiel** — FSP mesuré, AS/LP/CONF assumés ou modélisés |

---

## 5. Points critiques pour la refactorisation

### 5.1 `_confidentiality()` doit utiliser le catalogue

```python
# Avant (hardcoded)
def _confidence(self, bundle):
    return "visible"

# Après (catalog-based)
def _confidentiality(self, bundle):
    return bundle.metadata.get("confidentiality_class", "visible")
```

### 5.2 `_candidate_count()` doit mesurer sur les données réelles

```python
# Avant (hardcoded)
if graph_profile == "baseline":
    return 1

# Après (mesuré)
return len(set(snapshot.sender for snapshot in snapshots))
```

### 5.3 `_linking_probability()` doit utiliser un algorithme réel

```python
# Avant (hardcoded)
if graph_profile in {"stealth", "relay", "relay_stealth"}:
    return 0.0

# Après (calculé depuis les features du graphe)
return compute_linking_probability(components, edges, addresses)
```

### 5.4 Tests d'intégration requis

```python
# tests/integration/test_baseline_live.py
@pytest.mark.integration
def test_baseline_produces_real_transactions():
    # Nécessite devnet — marqué @pytest.mark.integration
    result = run_mechanism("baseline")
    assert len(result.tx_hashes) == 10
    assert all(tx.startswith("0x") for tx in result.tx_hashes)
```

---

## 6. Verdict

| Aspect | Statut |
|---|---|
| Devnet live 12 nœuds | **OPÉRATIONNEL** |
| Transactions signées + confirmées | **RÉEL** |
| Métriques de performance | **RÉELLES** |
| First-spy measurement | **RÉEL** |
| Anonymity set / entropy / linking | **ASSUMÉ ou MODÉLISÉ** |
| Compositions multi-couche | **ABSENTES du rapport** |
| Confidentiality measurement | **HARDCODED** |

**Recommandation immédiate** : Appliquer les corrections 5.1-5.3 et relancer `make run` + `make metrics` pour valider que les artefacts reflètent des mesures réelles.

---

## 7. Artefacts de preuve

- `results/raw/baseline.json` — 10 tx hashes, origins distribuées
- `results/mechanisms.json` — Métriques de performance live
- `results/metrics.json` — Métriques privacy partiellement simulées
- `results/runs/mechanism-20260828T125348840411Z/` — Run immuable avec Merkle root
