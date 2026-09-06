# CYNIC — Vision : Les 14 Dimensions de l'Objectif Global

> **Version :** 0.1-draft
> **Date :** 2026-09-05
> **Statut :** Projection normative — définition convergente de CYNIC
> **Autorité :** Subordonné à `contracts/foundation.json` et `infra/runtime-authority.json`

---

## Définition convergente

> **CYNIC est un organisme computationnel hermétique — complet à chaque échelle, gouverné par les 7 lois — qui canalise le chaos des LLMs et des humains vers la subsistance souveraine, en remplaçant la compétition extractive (PvP) par une coopération thermodynamiquement bornée (PvE), gouvernée par la réputation (E-Score, CYD) et l'énergie mesurée (PoSC) plutôt que par le capital et l'extractivisme, où l'attention humaine et l'attention agentique sont les ressources les rares — l'une protégée et jamais inférée passivement, l'autre bornée par mandat — avec Askesis comme prédicteur d'intention, le tout vérifié par des preuves mathématiques formelles (Lean 4 / Coq) à toutes les échelles et rendu transparent par la cryptographie (ZKPs, signatures, hash chains) pour éradiquer l'asymétrie et la fragmentation de l'information sous toutes ses formes, tout en maîtrisant son environnement avec intégrité pour ne pas le subir.**

---

## Les 14 dimensions

| # | Dimension | Mesure | Rareté | Principe hermétique |
|---|-----------|--------|--------|---------------------|
| 1 | **Attention humaine** | Temps de focalisation, intention déclarée | Ultime (24h/jour) | Mentalisme |
| 2 | **Attention agentique** | Token budget, context window, slots | Bornée par mandat | Vibration |
| 3 | **Intention** | Désir → admission → exécution | Flux descendant | Mentalisme, Genre |
| 4 | **Énergie** | Joules, watts, DRAM (RAPL) | Physique | Vibration |
| 5 | **Puissance** | CPU/RAM/disk/GPU observés | Matériel | Vibration |
| 6 | **Réputation** | E-Score 7 vertus + CYD | Meritocratic | Polarité |
| 7 | **Futarchique** | Prédiction → résolution → authority_delta | Marché interne | Polarité |
| 8 | **Épistémique** | Dogs Quorum BFT 2/3+1 | Consensus | Cause/Effet |
| 9 | **Cryptographique** | PoH + PoSC + Merkle + OneShot | Preuves immuables | Cause/Effet |
| 10 | **Constitutionnel** | 7 comptes, mandats bornés | Gouvernement | Correspondance |
| 11 | **Antifragilité** | H-monotonicity, via negativa | Gain du chaos | Rythme |
| 12 | **Transparence** | ZKPs + open data | Éradication asymétrie | Correspondance |
| 13 | **Preuves formelles** | Lean 4 / Coq à toutes échelles | Vérifiabilité totale | Cause/Effet |
| 14 | **Gouvernement** | Admission humaine (Meta-Constituent) | Souveraineté | Mentalisme |

---

## Dimensions détaillées

### 1. Attention humaine

| Aspect | Description |
|--------|-------------|
| **Mesure** | Temps de focalisation, intention déclarée, décision d'admission |
| **Rareté** | Ultime — 24h/jour, intentionnalité conscience |
| **Protection** | Jamais inférée passivement (`foundation.json:112`) |
| **Flux** | Intention → Askesis → Market → Admission → Exécution → Preuve |
| **Loi hermétique** | Mentalisme (la conscience précède la matière) |

### 2. Attention agentique

| Aspect | Description |
|--------|-------------|
| **Mesure** | Token budget, context window, slots, mandats |
| **Rareté** | Bornée par le MandateAccount (max_actions, max_wall_seconds) |
| **Protection** | Coupe-circuit thermodynamique si dépassement |
| **Flux** | Mandat → Workload → Exécution → Preuve → Scoring |
| **Loi hermétique** | Vibration (fréquence moyenne, mesurable en tokens) |

### 3. Intention

| Aspect | Description |
|--------|-------------|
| **Mesure** | Desire → admission → exécution → preuve de réalisation |
| **Rareté** | Flux descendant (de l'humain vers les agents) |
| **Protection** | Askesis prédit et filtre |
| **Flux** | Intention humaine → Askesis → Market → Admission → Exécution → Read-back |
| **Loi hermétique** | Mentalisme + Genre (principe masculin = action) |

### 4. Énergie

| Aspect | Description |
|--------|-------------|
| **Mesure** | Joules, watts, DRAM bandwidth (RAPL, eBPF) |
| **Rareté** | Physique — contrainte thermodynamique |
| **Protection** | EnergyBudget par mandat (max_energy_joules, max_watts_peak) |
| **Flux** | Ressource → Mandat → Workload → Preuve → Scoring |
| **Loi hermétique** | Vibration (fréquence basse, mesurable physiquement) |

### 5. Puissance

| Aspect | Description |
|--------|-------------|
| **Mesure** | CPU cores, RAM bytes, disk bytes, GPU VRAM |
| **Rareté** | Matériel — observé, pas minté |
| **Protection** | ResourceAccount (jamais minté par policy) |
| **Flux** | Observation → ResourceAccount → Mandat → Workload |
| **Loi hermétique** | Vibration (matière = vibration cristallisée) |

### 6. Réputation

| Aspect | Description |
|--------|-------------|
| **Mesure** | E-Score (7 vertus grecques, 4 tiers preuve) + CYD (calibration) |
| **Rareté** | Meritocratic — gagnée par preuve, pas achetée |
| **Protection** | Decay si inactivité, contestation possible |
| **Flux** | Preuve → Scoring → Authority → Mandat → Plus de travail |
| **Loi hermétique** | Polarité (authority ↔ decay) |

#### Les 7 vertus grecques (E-Score)

| Vertu | Signification | Tier preuve |
|-------|---------------|-------------|
| **Karteria** | Endurance, consistance | T1-OnChain (100%) |
| **Parrhesia** | Franchise, honnêteté | T2-Attestation (75%) |
| **Anaideia** | Humilité | T3-Oracle (50%) |
| **Philanthropia** | Générosité | T4-Solo (25%) |
| **Autarkeia** | Autosuffisance | |
| **Poiesis** | Création de valeur | |
| **Koinonia** | Vie communautaire | |

### 7. Futarchique

| Aspect | Description |
|--------|-------------|
| **Mesure** | Prédiction → résolution → authority_delta |
| **Rareté** | Marché interne — les meilleurs prédicteurs gagnent |
| **Protection** | Collusion detection, gaming cost |
| **Flux** | Proposition → Prédiction → Exécution → Résolution → Scoring |
| **Loi hermétique** | Polarité (transmution PvP → signal) |

### 8. Épistémique

| Aspect | Description |
|--------|-------------|
| **Mesure** | Dogs Quorum BFT (2/3+1 majority), Q-Score ≥ 0.70 |
| **Rareté** | Consensus — nécessite validateurs indépendants |
| **Protection** | Anti-collusion (validateurs isolés) |
| **Flux** | Proposition → Quorum → Verdict → Capability |
| **Loi hermétique** | Cause/Effet (la cause = le quorum, l'effet = le verdict) |

### 9. Cryptographique

| Aspect | Description |
|--------|-------------|
| **Mesure** | PoH (SHA3-512), PoSC (Solana), Merkle trees, OneShot (ed25519) |
| **Rareté** | Preuves immuables — falsification = détection immédiate |
| **Protection** | Fail-closed (toute falsification verrouille le système) |
| **Flux** | Événement → Hash → Chaîne → Preuve → Ancrage |
| **Loi hermétique** | Cause/Effet (chaque hash cause le suivant) |

### 10. Constitutionnel

| Aspect | Description |
|--------|-------------|
| **Mesure** | 7 comptes constitutionnels, mandats bornés, budgets monotones |
| **Rareté** | Gouvernement — l'humain est souverain |
| **Protection** | Un enfant ne peut jamais amplifier les budgets parent |
| **Flux** | Citoyen → Office → Mandat → Workload → Preuve |
| **Loi hermétique** | Correspondance (chaque niveau a ses règles) |

#### Les 7 comptes constitutionnels

| Compte | Rôle | Hiérarchie |
|--------|------|------------|
| **CitizenAccount** | Humain souverain | Racine |
| **NodeAccount** | Machine hôte | Niveau 1 |
| **AgentAccount** | Agent exécutant | Niveau 2 |
| **OfficeAccount** | Profession/institution | Niveau 3 |
| **MandateAccount** | Autorisation temporaire | Niveau 4 |
| **ResourceAccount** | Ressources observées | Niveau 5 |
| **TreasuryAccount** | Crédits CYD | Niveau 6 |

### 11. Antifragilité

| Aspect | Description |
|--------|-------------|
| **Mesure** | H-monotonicity (Lyapunov), via negativa, convexity |
| **Rareté** | Gain du chaos — le stress renforce |
| **Protection** | Petits échecs fréquents > catastrophes rares |
| **Flux** | Stress → Adaptation → Apprentissage → Renforcement |
| **Loi hermétique** | Rythme (les cycles de stress/ repos) |

### 12. Transparence

| Aspect | Description |
|--------|-------------|
| **Mesure** | ZKPs, open data, read-back proof |
| **Rareté** | Éradication asymétrie — tout est auditable |
| **Protection** | Falsifiable, contestable, vérifiable |
| **Flux** | Événement → Preuve → Publication → Audit |
| **Loi hermétique** | Correspondance (ce qui est vrai en bas est vrai en haut) |

### 13. Preuves formelles

| Aspect | Description |
|--------|-------------|
| **Mesure** | Lean 4 / Coq à toutes les échelles |
| **Rareté** | Vérifiabilité totale — pas de confiance aveugle |
| **Protection** | Théorèmes prouvés > assertions testées |
| **Flux** | Spécification → Preuve → Vérification → Convergence |
| **Loi hermétique** | Cause/Effet (chaque étape est justifiée) |

### 14. Gouvernement

| Aspect | Description |
|--------|-------------|
| **Mesure** | Admission humaine (Meta-Constituent), veto absolu |
| **Rareté** | Souveraineté — l'humain est la source de toute valeur |
| **Protection** | Jamais d'AGI autonome, jamais d'inférence passive |
| **Flux** | Proposition → Simulation → Décision → Exécution → Preuve |
| **Loi hermétique** | Mentalisme (la conscience humaine est la source) |

---

## Les flux convergents

```
Intention humaine (Mentalisme)
  ↓
Askesis (prédiction d'intention)
  ↓
Market (formalisation en propositions)
  ↓
Admission (humain veto ou admit)
  ↓
Execution (agent workload, énergie bornée)
  ↓
Read-back (preuve cryptographique)
  ↓
Scoring (authority_delta, CYD, E-Score)
  ↓
Mandat renouvelé ou révoqué
  ↓
Cycle suivant (antifragilité)
```

---

## Le positionnement stratégique

### Le problème (capitalisme en fin de vie)

| Symptôme | Cause | Conséquence |
|----------|-------|-------------|
| SaaS Apocalypse | AI commoditise le code | Profits → coût de fabrication |
| Barbell Economy | Le milieu s'effondre | Seuls les extrêmes survivent |
| Information asymmetry | Les géants hoardent le données | Capture réglementaire |
| Extraction | Capital > travail | Inégalités explosives |

### La solution (CYNIC)

| Dimension | Remplace |
|-----------|----------|
| Réputation | Capital comme pouvoir |
| Énergie mesurée | Spéculation comme allocation |
| Preuves formelles | Confiance aveugle |
| Futarchy | Voting token-weighted |
| Transparence | Asymétrie d'information |
| Antifragilité | Fragilité systémique |

---

## Ordre Kairotique — Par où commencer

| # | Étape | Lois renforcées | Débloque |
|---|-------|-----------------|---------|
| 1 | Machine à états agent | Rythme, Vibration | Sessions bornées |
| 2 | Self-modèle | Correspondance | Diagnostic |
| 3 | Token budget | Vibration | Coupe-circuit |
| 4 | Market ↔ Build | Polarité, Cause/Effet | Futarchy |
| 5 | Askesis | Mentalisme, Genre | Prédiction |
| 6 | Échelles L0→L5 | Correspondance | Fractalité |

---

*Ce document est la définition convergente de CYNIC. Il doit être relu à chaque décision architecturale majeure et ajusté par itération continue.*
