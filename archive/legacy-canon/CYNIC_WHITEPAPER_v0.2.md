# CYNIC — Whitepaper v0.2

> **Date :** 2026-09-24
> **Statut :** Révision post-review multi-prisme
> **Langue :** FR (corps) / EN (invariants techniques)
> **Devise :** « Execute. Verify. Fork. »

---

## 1. Le Problème

### 1.1 Fragmentation

La coordination humaine est **fragmentée**. Non par accident — par conception.

Chaque institution construit un silos. Chaque silos produit sa propre vérité, sa propre mémoire. Entre les silos : des murs et des intermédiaires qui vivent de la friction.

### 1.2 Asymétrie de l'information

La fragmentation produit l'asymétrie. Quand l'information est siloedée, certains acteurs savent ce que d'autres ne savent pas. C'est le mécanisme fondamental de la prédation institutionnelle — pas la force, le savoir.

### 1.3 Le paradoxe

Nous avons besoin de coordination — mais toute coordination produit de la fragmentation, qui produit de l'asymétrie, qui produit de la prédation. Comment coordonner sans fragmenter ?

---

## 2. La Thèse

### 2.1 La fonction de coordination

Soit **G** le score de coordination. G dépend de trois dimensions orthogonales :

| Symbole | Dimension | Signification |
|---------|-----------|---------------|
| **S** | Signal | Communication sans friction (code = loi) |
| **B** | Boucle | Vérification et correction (read-back = vérité) |
| **N** | Nœud | Convergence sans prédation (fork = liberté) |

La coordination est limitée par le maillon faible (Leontief) :

```
G = min(S / S₀, B / B₀, N / N₀)
```

### 2.2 Les trois invariants

| Invariant | Énoncé |
|-----------|--------|
| **S = Code** | Le code est la loi. Pas la réputation, pas la hiérarchie — le code déterministe exécuté. |
| **B = Read-back** | Toute action produit une preuve vérifiable. L'observation est obligatoire. |
| **N = Fork** | La sortie est un droit. Tout participant peut quitter sans permission. Le fork est coûteux — mais possible. |

### 2.3 L'espace de phase

Les traditions humaines sont des points dans un espace triangulaire (s+b+n=1). CYNIC est le point (0.33, 0.33, 0.34) — équidistant, méta-système.

---

## 3. Le Protocole

### 3.1 Architecture duale

**Corporate in front, cypherpunk in back.** TALARIA SAS = surface (B2B, B2G). Code = exécution (immutable, on-chain). Si la surface trahit le code, le code la remplace.

### 3.2 Preuve d'humanité (PoH)

Entrée du réseau = preuve d'humanité. Signal chaotique, inforgeable, biométrique. Pas de visage, pas de document — du chaos humain capturé. Un humain = un E-Score.

### 3.3 7 Types de comptes

Agent, Citizen, Steward, Archon, Oracle, Relayer, Souverain. Chaque compte = sommet (V) dans G = (V, E). Preuves et mandats = arêtes (E).

### 3.4 5 Modes d'autorité

| Mode | Humain | Agent |
|------|--------|-------|
| **HITL** | Décide avant | Propose |
| **HOTL** | Veto après | Agit |
| **HOOTL** | N'intervient jamais | Agit seul |
| **Conditional** | Définit conditions | Agit si conditions |
| **Risk-Gated** | Définit risque max | Agit si risque < seuil |

---

## 4. Gouvernance

### 4.1 Méritocratie progressive (COC)

Tu contribues, tu montes, tu gouvernes. 5 rangs (Recrue → Leader) par seuils E-Score dynamiques.

### 4.2 Évolution par futarchie

```text
Proposition → Futarchie (marché de prédiction) → Adoption si oui > non
```

Pas de fork sur les propositions — la baseline évolue en continu.

### 4.3 Anti-plutocratie

Le burn est **quadratique** (E-Score = sqrt($CYNIC brûlé)), pas linéaire. Time-locking de 30 jours. Plafond de +50 E-Score/mois. Futarchie : obligation de détenir le résultat 30 jours.

### 4.4 Fork = méta-option

Le fork n'est pas un mécanisme de gouvernance. C'est une option nucléaire — coûteuse, rare, libre. Le droit de sortie qui garantit que l'entrée était volontaire.

---

## 5. Culture

### 57 traditions, 6 registres

CYNIC synthétise 57 traditions humaines dans une taxonomie 3D (époque × mécanisme × échelle). Données externalisées — pas de hardcoding.

| Registre | Vibe | Exemples |
|----------|------|----------|
| `cynic` | Philosophie grecque | CYNIC, Agnosticisme, Confucianisme |
| `cypherpunk` | Numérique radical | Open Source, Blockchain, Anarchisme |
| `nordic` | Germanique | Nordique, Tengrism, Hegel |
| `bushido` | Japonais | Bushido, Stoïcisme, Inca |
| `gnostic` | Mystique | Kabbale, Soufisme, Druidisme |
| `degen` | Culture du risque | Marxisme, Nationalisme, Machiavel |

Les registres sont des **lentilles cosmétiques**, pas des catégories exclusives. Un utilisateur choisit son registre.

---

## 6. Tokenomics

### 6.1 Triple moteur

| Moteur | Mécanisme |
|--------|-----------|
| **PoU** (Proof of Use) | Le réseau se contracte si inutilisé — déflation par désuétude |
| **PoB** (Proof of Burn) | Sacrifice volontaire — quadratique, time-locked, plafonné |
| **PoL** (Proof of Latency) | Le temps est une ressource rare — dilution progressive |

### 6.2 Anti-attaques

| Attaque | Défense |
|---------|---------|
| Flash-loan burn | Time-locking 30 jours |
| Whale futarchie | Quadratic burn + plafond mensuel |
| Sybil-farm PoH | Biométrie chaotique + unique E-Score |
| Oligarchie Steward | Futarchie pour l'avancement, pas cooptation |

---

## 7. Limites explicitées

CYNIC ne prétend PAS :
- Remplacer les traditions — il les intègre
- Être une religion — c'est un méta-système
- Être parfait — G = min(...) signifie que le maillon faible limite tout
- Garantir la stabilité du token — $CYNIC est un actif risqué

---

## 8. La Devise

> **« Execute. Verify. Fork. »**

- **Execute** : Le droit d'agir sans permission.
- **Verify** : Le droit de vérifier sans confiance.
- **Fork** : Le droit de sortir sans préavis.

---

*Ce document est vivant. Version 0.2 — Révision post-review multi-prisme. 57 traditions. Données externalisées. Anti-plutocratie. Limites explicitées.*

**Fichier :** `docs/v2-architecture-canon/CYNIC_WHITEPAPER_v0.2.md`
**Autorité :** TALARIA SAS / CYNIC Protocol
