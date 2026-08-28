# Audit : Décalage Claims ↔ Production (PEF)

**Date** : 2026-08-27
**Scope** : README.md + INTRO_FRAMEWORK.md vs résultats réels (`results/metrics.json`, `results/mechanisms.json`)
**Méthode** : Croisement des déclarations publiques avec les artefacts de sortie et le code source

---

## Synthèse exécutive

Le framework déclare mesurer empiriquement la privacy sous attaque adversariale. **Environ 40% des métriques de privacy du rapport final sont `assumed` ou `modeled`, pas mesurées.** Les compositions multi-couche (périmètre déclaré clé) sont absentes du rapport final. Les attaques avancées (censorship, dust, eclipse, web2, zk_bridge) sont des simulations hors ligne ou des scores statiques binaires.

---

## 1. Cartographie Claims ↔ Réalité

| Claim (README / INTRO) | Production réelle (`results/metrics.json`) | Alignement |
|---|---|---|
| **"Measured adversarial conditions"** | Métriques privacy baseline = `assumed` (provenance: "baseline direct transfer has one sender") | **✗ MENSONGE** |
| **Confidentialité (CONF)** : "can an adversary observe amount/recipient?" | `confidentiality` = `visible` pour TOUS (hardcoded dans `transaction_graph_runner._confidentiality()` l.278) | **✗ MENSONGE** |
| **Anonymat (AS)** : "can an adversary identify sender?" | baseline: `assumed` ; mixer: `modeled` (test cohort size) ; relay/dandelion: `1` (hardcoded heuristic) | **PARTIEL** |
| **Unlinkability (LP)** : "can adversary link two txs?" | relay/stealth: `0.0` (hardcoded) ; relayer link: `1.0` (hardcoded) ; pas d'analyse de graphe réelle | **PARTIEL** |
| **Unobservability (FSP)** : "can adversary detect tx submitted?" | first-spy mesuré live (OK) mais unobservability = `1 - precision * coverage` (dérivation triviale) | **OK** |
| **First-Spy precision** | Mesuré live sur devnet 12-node avec spatial propagation delay | **OK** |
| **Sybil ML** | F1=0.186 (sous baseline 0.208) — README admet "classified 0 target transactions above threshold" | **✗ ÉCHEC** |
| **Transaction graph analysis** | Heuristiques statiques, pas d'analyse de graphe réelle sur les artefacts on-chain | **PARTIEL** |
| **Timing attack** | Latence mesurée (OK) ; linking prob = heuristique statique | **PARTIEL** |
| **Gas overhead** | Mesuré depuis `transaction_receipts` (baseline=0%, mixer=2767%) | **OK** |
| **Censorship** | `is_censorable` binaire (recipient ∈ known_contracts) ; delay multipliers codés en dur (2.0x, 5.0x) | **✗ SIMULATION** |
| **Dust** | Score binaire (0.0/0.9/1.0) basé sur metadata fields ; pas de dust envoyé on-chain | **✗ SIMULATION** |
| **Eclipse** | Simulation offline (vérifie `stem_hop_count` dans metadata) ; pas d'éclipse de nœud réelle | **✗ SIMULATION** |
| **Web2 correlation** | Simulation offline (pas de données web2 réelles) | **✗ SIMULATION** |
| **ZK-Bridge** | Score statique (0.0/0.25/1.0) basé sur `proof_backend` metadata | **✗ SIMULATION** |
| **Value Fingerprinting** | Score statique (0.0/0.9/1.0) basé sur metadata ; pas d'analyse empirique | **✗ SIMULATION** |
| **Compositions multi-couche** | **ABSENTES du rapport final** (filtrées par `DEFAULT_MECHANISM_METADATA`) | **✗ ABSENT** |
| **Scorecards SOTA** | Calculé mais inputs partiellement assumed/modeled → score biaisé | **PARTIEL** |

---

## 2. Preuve : analyse de provenance

### 2.1 Baseline (contrôle négatif)

```json
"metric_provenance": {
  "privacy.anonymity_set": {"source": "assumed", "basis": "baseline direct transfer has one sender"},
  "privacy.shannon_entropy": {"source": "assumed", "basis": "baseline direct transfer has no anonymity set"},
  "privacy.linking_probability": {"source": "assumed", "basis": "sender and recipient are directly observable"},
  "privacy.first_spy_precision": {"source": "assumed", "basis": "public mempool transit assumed observable until first-spy is measured"},
  "privacy.confidentiality": {"source": "assumed", "basis": "amount and recipient are public on-chain"}
}
```

**100% assumed**. Aucune mesure réelle. Le README déclare "measured adversarial conditions" mais le contrôle est purement déclaratif.

### 2.2 Mixer (Tornado-style)

```json
"privacy.anonymity_set": {"source": "modeled", "basis": "number of deposits in fixed-size test cohort"},
"privacy.shannon_entropy": {"source": "modeled", "basis": "log2(test cohort size)"},
"privacy.first_spy_precision": {"source": "assumed", "basis": "public mempool transit assumed observable until first-spy is measured"}
```

**Anonymity set modeled, pas mesuré**. La taille de l'anonymity set = nombre de dépôts dans le test, pas l'ensemble réel des transactions confondues observé on-chain.

### 2.3 Mécanismes réseau (relay, dandelion, stealth)

```
relay: anonymity_set=1, shannon_entropy=0.0, linking_probability=0.0
dandelion: anonymity_set=1, shannon_entropy=0.0, linking_probability=1.0
stealth: anonymity_set=1, shannon_entropy=0.0, linking_probability=0.0
```

**Toutes les valeurs sont des heuristiques codées en dur** dans `transaction_graph_runner.py` :
- relay → AS=1 (l.244)
- dandelion → AS=1 (l.203-204)
- stealth → AS=1 si `withdrawal_hashes` présent (l.234-238)
- relay_stealth → LP=1.0 si withdrawal_hashes (l.270-272)

Pas d'analyse réelle du graphe de transactions.

---

## 3. Compositions : absentes du rapport

### 3.1 Le claim

> "Composition 1 : Tornado Cash + Dandelion++"
> "Composition 2 : Relay/proxy + Stealth address"
> "Composition 3 : Tornado Cash + Dandelion++ + Stealth address"
> — `objectifs.md` et README.md

Questions expérimentales déclarées :
- "Est-ce que privacy(X+Y) > somme des gains ?"
- "Obtient-on une couverture complète ou une fuite subsiste-t-elle ?"

### 3.2 La réalité

```python
# metrics_engine.py l.470
if mechanism not in self.DEFAULT_MECHANISM_METADATA:
    continue
```

```python
# metrics_engine.py l.65-107
DEFAULT_MECHANISM_METADATA = {
    "baseline": {...}, "mixer": {...}, "tornado_cash": {...},
    "stealth": {...}, "dandelion": {...}, "relay": {...}, "privacy_pools": {...}
}
```

Les 5 compositions du catalog (`tornado_tor`, `ultimate_mixer`, `relay_stealth`, `tornado_dandelion`, `tornado_dandelion_stealth`) sont **exclues** du rapport métrique final et du leaderboard SOTA.

Les artefacts bruts existent (`results/tornado_dandelion.json`, `results/tornado_dandelion_stealth.json`, `results/relay_stealth.json`) mais ne remontent pas dans la scorecard.

**Aucune métrique de composition** (gain vs somme des parties) n'est calculée.

---

## 4. Attaques avancées : simulations hors ligne

### 4.1 Censorship

**Claim** : "Vérifie si le mécanisme est vulnérable aux attaques de censure (ex : un validateur pourrait rejeter les preuves ZK suspectes)"

**Code** (`censorship_runner.py`) :
```python
# l.111-113
is_censorable = False
censorship_vector = None

# l.116-128
known_contracts = set()
if "contract_address" in bundle.raw:
    known_contracts.add(bundle.raw["contract_address"].lower())
for snapshot in snapshots:
    if snapshot.recipient and snapshot.recipient.lower() in known_contracts:
        is_censorable = True
        censorship_vector = "Public Contract Destination (OFAC Blacklist)"
        break

# l.132-137
if is_censorable:
    delay_p50 = 1.0 / (1.0 - 0.5)  # 2.0x
    delay_p80 = 1.0 / (1.0 - 0.8)  # 5.0x
```

Aucune censure réelle sur le devnet. Les multiplicateurs sont des constantes mathématiques (probabilité de censure de 50% et 80%), pas des observations.

### 4.2 Dust

**Claim** : "Envoie de petits montants ("dust") vers des adresses pour tracer les mouvements de fonds ultérieurs"

**Code** (`dust_attack_runner.py`) :
- Vérifie `withdrawal_hashes`, `recovered_keys_count`, `recipient_mode` dans les metadata
- Retourne un score binaire (0.0, 0.9, 1.0)
- Aucun dust n'est envoyé on-chain

### 4.3 Eclipse

**Claim** : "Isoler un nœud/coin jusqu'à ce que ses transactions passent par un relais contrôlé par l'attaquant"

**Code** (`eclipse_runner.py`) :
- Vérifie `stem_hop_count` et `overlay_mechanism` dans les metadata
- Aucune éclipse de nœud réelle sur le devnet
- Simulation purement déclarative

### 4.4 Web2 Correlation

**Claim** : "Corèle les données d'identités Web2 (ex : KYC d'exchange) avec les transactions on-chain"

**Code** : Aucun appel API, aucune donnée Web2. Simulation offline basée sur les metadata.

### 4.5 ZK-Bridge

**Claim** : "Exploite la transparence des bridges ZK (annonces publiques des proofs) pour corréler les entrées/sorties"

**Code** (`zk_bridge_runner.py`) :
- Score statique (0.0/0.25/1.0) basé sur `proof_backend` et `equivalence_target` metadata
- Aucune corrélation de bridge réelle

---

## 5. Scorecards SOTA : calcul valide, inputs biaisés

Le calcul du `harmonic_mean`, `privacy_score`, `performance_score` est mathématiquement correct (vu dans `sota_scorer.py`). Mais les inputs sont pollués :

| Input | Source réelle | Impact |
|---|---|---|
| `anonymity_set` | assumed/modeled/hardcoded | Privacy score artificiellement basse |
| `shannon_entropy` | assumed/modeled | Idem |
| `linking_probability` | hardcoded heuristics | Non représentatif |
| `first_spy_precision` | assumed pour baseline/mixer | Baseline devrait être 1.0 mesuré, pas assumed |
| `confidentiality` | statique (catalog) | Pas de mesure réelle |

---

## 6. Red-team : auto-évaluation

Le fichier `results/red-team.json` contient un verdict d'alignement threat-prisms. Le red-team ne teste pas la sécurité du framework, il valide la cohérence des métadonnées de provenance. C'est une vérification de forme, pas de fond.

---

## 7. Verdict

### Ce que PEF fait réellement bien

- Déploiement devnet 12-node reproductible (Kurtosis)
- Mesure de latence/throughput on-chain
- Premier-spy measurement live avec spatial propagation
- Pipeline de campagne reproductible avec artefacts immuables
- Traçabilité de provenance (chaque métrique déclare sa source)

### Ce que PEF ne fait pas (mais déclare faire)

- Mesurer empiriquement la confidentialité (c'est une classification déclarative)
- Mesurer l'anonymity set réel (c'est un modèle ou une heuristique)
- Mesurer la linking probability réelle (c'est du code dur)
- Exécuter les attaques avancées sur devnet (c'est de la simulation offline)
- Évaluer les compositions multi-couche (c'est exclut du rapport)
- Démontrer un signal Sybil ML (F1 < baseline)

### Impact sur la crédibilité scientifique

Si ces scorecards étaient présentés dans un rapport académique ou professionnel sans transparence sur le ratio assumed/measured, c'est **trompeuse**. La section "Honest framing" du README limite correctement les claims (pas de nation-state resistance), mais ne mentionne pas que 40% des métriques privacy sont assumed/modeled.

---

## 8. Recommandations

### P0 — Transparence

Ajouter dans README.md et INTRO_FRAMEWORK.md un tableau de couverture :
- X métriques measured_live
- Y métriques derived
- Z métriques modeled/assumed
- Compositions : absentes du rapport

### P1 — Inclure les compositions

Supprimer le filtre `DEFAULT_MECHANISM_METADATA` ou l'étendre aux compositions.

### P2 — Mesurer réellement

- Confidentialité : analyser les txs on-chain pour déterminer si montant/destinataire sont visibles
- Anonymity set : mesurer l'ensemble réel des transactions confondues (pas la taille du cohort)
- Linking probability : implémenter un vrai algorithm de graphe (pas des heuristiques)

### P3 — Attaques live

- Censorship : implémenter un validateur qui rejette certaines txs
- Dust : envoyer du dust on-chain et observer le spend
- Eclipse : isoler un nœud et observer le comportement
