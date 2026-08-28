# Audit : Privacy Ethereum Framework (PEF)

**Date** : 2026-08-27
**Scope** : Alignement code ↔ `objectifs.md` (référence fonctionnelle)
**Méthode** : Analyse statique du code synchronisé (branche `sync/stanislaz-v2-latest`, SHA `17c5906`)
**Sources** : `objectifs.md`, `mechanism_catalog.py`, `attack_models.py`, `privacy_metrics.py`, `metrics_engine.py`, `transaction_graph_runner.py`, `first_spy_runner.py`, `censorship_runner.py`, `dust_attack_runner.py`, `zk_bridge_runner.py`, `eclipse_runner.py`, `topology.py`

---

## 1. Dimensions P&H (Pfitzmann & Hansen 2010)

### 1.1 Confidentialité (CONF)

| Item | objectifs.md | Code | Alignement |
|------|-------------|------|------------|
| Définition | `CONF(tx) in {visible, hidden}` | `transaction_graph_runner._confidentiality()` retourne `"visible"` pour tous (l.278) | **PARTIEL** |
| Classification | Binaire | `mechanism_catalog.py` définit `confidentialité_class` statique par mécanisme (`sender_unlinkable`, `origin_obfuscated`, `recipient_private`, `compliant_anonymous`) | Déclaratif, pas empirique |
| Mesure | - | `zk_bridge_runner` introduit une gradation (0.0/0.25/1.0) | Simulé/statique |

**Verdict** : Pas de mesure empirique de confidentialité on-chain. La classification est déclarative (catalog) ou statique (zk_bridge). Aucun mécanisme ne mesure si un observateur externe peut réellement voir le montant/destinataire.

### 1.2 Anonymat (AS, H)

| Item | objectifs.md | Code | Alignement |
|------|-------------|------|------------|
| Définition | `AS = \|candidates\|`, `H = -sum p*log2(p)` | `privacy_metrics.py` : `compute_shannon_entropy()`, `compute_effective_anonymity_set_size()` | **OK** |
| Calcul AS | - | `transaction_graph_runner._candidate_count()` avec heuristiques mécanisme-spécifiques (l.198-245) | Empirique |
| Entropie | - | `shannon_entropy(candidate_count)` dans attack_models.py | **OK** |

**Verdict** : Bien couvert. Formules formelles implémentées. L'anonymity set a des heuristiques (relayer link, gas funding link) qui mériteraient d'être validées empiriquement.

### 1.3 Unlinkability (LP)

| Item | objectifs.md | Code | Alignement |
|------|-------------|------|------------|
| Définition | `LP(tx1,tx2) = P(same_user \| graph_features)` | `transaction_graph_runner._linking_probability()` (l.247-275) | **PARTIEL** |
| Timing | `LP` entre deposit/withdraw | `timing_attack_runner.py`, `temporal_intersection_runner.py` | **OK** |
| Heuristiques | - | Relayer link → LP=1.0, stealth → LP=0.0 (codé en dur) | Simulé |

**Verdict** : Couvert avec heuristiques. Certaines valeurs sont codées en dur plutôt que mesurées depuis les données réelles.

### 1.4 Unobservability (FSP)

| Item | objectifs.md | Code | Alignement |
|------|-------------|------|------------|
| Définition | `FSP = \|correctly attributed\| / \|observed\|` | `first_spy_runner.py` : precision/coverage avec spatial propagation delay | **OK** |
| Spatial | - | `topology.py` : propagation géographique 0-220ms, normalisation spy-N → node-N | **OK** |
| Classification | - | K-means filtering pour isoler les tx cibles | **OK** |

**Verdict** : La meilleure couverture. Inclut la propagation géographique réaliste et la normalisation des nœuds observateurs.

---

## 2. Métriques de performance

| Métrique | objectifs.md | Code | Alignement |
|----------|-------------|------|------------|
| Gas overhead | `(gas(tx) - gas(baseline)) / gas(baseline)` | `MetricsEngine.EXPECTED_PERFORMANCE_METRICS` inclut `gas_overhead_pct` | **OK** |
| Latence | `timestamp(inclusion) - timestamp(submission)` | `transaction_graph_runner._latency_values()` depuis les timestamps de bloc | **OK** |
| Throughput | `|confirmed txs| / W` | Capturé dans mechanisms, exposé dans metrics | **OK** |

**Verdict** : Pipeline complète. Baseline utilisée comme référence pour le surcoût.

---

## 3. Mécanismes

### 3.1 Couvertage

| Mécanisme | objectifs.md | Catalog | Script | Inclus dans rapport |
|-----------|-------------|---------|--------|-------------------|
| Baseline | ✓ | ✓ | ✓ | ✓ |
| Tornado Cash | ✓ | ✓ | ✓ | ✓ |
| Stealth address | ✓ | ✓ | ✓ | ✓ |
| Dandelion++ | ✓ | ✓ | ✓ | ✓ |
| Relay/proxy | ✓ | ✓ | ✓ | ✓ |
| Tornado + Dandelion | ✓ | ✓ | ✓ | **✗** |
| Relay + Stealth | ✓ | ✓ | ✓ | **✗** |
| Tornado + Dandelion + Stealth | ✓ | ✓ | ✓ | **✗** |
| Privacy Pools | - | ✓ | ✓ | ✓ |
| Mixer | - | ✓ | ✓ | ✓ |
| Tornado + Tor | - | ✓ | ✓ | **✗** |
| Ultimate Mixer | - | ✓ | ✓ | **✗** |

### 3.2 Problème critique

`MetricsEngine._mechanism_rows()` filtre à la ligne 470 :
```python
if mechanism not in self.DEFAULT_MECHANISM_METADATA:
    continue
```

`DEFAULT_MECHANISM_METADATA` ne contient que 7 mécanismes (baseline, mixer, tornado_cash, stealth, dandelion, relay, privacy_pools). **Les compositions sont exclues de la matrice d'évaluation finale et du scorecard.** Elles existent dans le catalog mais sont invisibles dans le rapport.

---

## 4. Adversaires

| Adversaire | objectifs.md | Code | Alignement |
|------------|-------------|------|------------|
| First-Spy Estimator | ✓ | `first_spy_runner.py` | **OK** |
| Transaction Graph Analysis | ✓ | `transaction_graph_runner.py` | **OK** |
| Timing Attack | ✓ | `timing_attack_runner.py` | **OK** |
| Passif local | ✓ | first-spy avec N observers | **OK** |
| Passif global | ✓ | first-spy avec N observers | **OK** |
| Actif local | ✓ | Sybil ML avec feature extraction | **OK** |
| Actif global | ✓ | Sybil ML avec feature extraction | **OK** |
| Byzantine (f) | ✓ (documenté) | Aucun code | **✗** |
| Economique (rational/myopic) | ✓ (documenté) | Aucun code | **✗** |
| Quantique | ✓ (documenté) | Aucun code | **✗** |

---

## 5. Gaps structurels

### 5.1 Plausible deniability
- **objectifs.md** : "Cette propriété n'a pas encore de métrique implémentée et doit rester marquée comme gap"
- **Code** : Aucune tentative d'implémentation
- **Statut** : Conforme à la spécification (gap explicite)

### 5.2 GDPR Art. 17 (Droit à l'oubli)
- **objectifs.md** : "Cette propriété n'est pas mesurée par les mécanismes actuels"
- **Code** : Aucune tentative d'implémentation
- **Statut** : Conforme à la spécification (limite explicite)

### 5.3 Adversaires non-passifs
- **Byzantine (f)** : Documenté comme limite, aucun code
- **Economique** : Documenté comme limite, aucun code
- **Quantique** : Documenté comme limite, aucun code
- **Statut** : Conforme à la spécification

### 5.4 Compositions multi-couche
- **objectifs.md** : 3 compositions avec questions expérimentales claires ("privacy(X+Y) > somme des gains ?")
- **Code** : Les compositions existent dans le catalog et ont des scripts de mécanisme
- **Problème** :
  1. Exclues du rapport métrique final (filtre `DEFAULT_MECHANISM_METADATA`)
  2. Les attaques s'exécutent mécanisme-par-mécanisme, pas sur la composition comme unité
  3. Aucune métrique spécifique de composition (pas de mesure de "gain superieur/inferieur à la somme")
- **Statut** : **NON CONFORMES** — les questions expérimentales ne peuvent pas être répondues avec le pipeline actuel

### 5.5 Attaques simulées/offline
- **Eclipse** : Simulation offline (`eclipse_runner.py` analyse les artefacts, pas de live devnet)
- **Web2** : Simulation offline
- **Censorship** : Modèle probabiliste codé en dur (delay_p50=2.0x, delay_p80=5.0x)
- **Dust** : Score binaire (0.0/0.9/1.0) plutôt que continu
- **ZK-Bridge** : Score statique (0.0/0.25/1.0) plutôt que mesuré

---

## 6. Synthèse

### Couverture

| Catégorie | Score | Commentaire |
|-----------|-------|-------------|
| Dimensions P&H | 4/4 | Implémentées mais qualité inégale (FSP > AS ≈ LP > CONF) |
| Mécanismes | 8/8 | Tous listés + 4 supplémentaires |
| Compositions | 0/3 | Exclues du rapport final |
| Adversaires | 6/9 | 3 manquants (Byzantine, Economique, Quantique) |
| Métriques perf | 3/3 | Pipeline complète |

### Points d'action prioritaires

1. **P0 — Compositions exclues du rapport** : Ajouter les compositions à `DEFAULT_MECHANISM_METADATA` ou modifier le filtre pour les inclure dans la matrice d'évaluation
2. **P1 — Métriques de composition** : Implémenter une métrique qui mesure le gain de privacy d'une composition vs. la somme de ses parties
3. **P1 — Confidentialité empirique** : Remplacer la classification déclarative par une mesure réelle (analyse on-chain des valeurs/destinataires visibles)
4. **P2 — Dust/ZK-Bridge continus** : Remplacer les scores binaires par des métriques continues
5. **P2 — Eclipse/Censorship live** : Mesurer sur devnet plutôt que simuler

### Fichiers audités

- `mechanisms/application/mechanism_catalog.py`
- `attacks/domain/attack_models.py`
- `metrics/domain/privacy_metrics.py`
- `metrics/application/metrics_engine.py`
- `attacks/application/transaction_graph_runner.py`
- `attacks/application/first_spy_runner.py`
- `attacks/application/censorship_runner.py`
- `attacks/application/dust_attack_runner.py`
- `attacks/application/zk_bridge_runner.py`
- `attacks/application/eclipse_runner.py`
- `mechanisms/runtime/topology.py`
