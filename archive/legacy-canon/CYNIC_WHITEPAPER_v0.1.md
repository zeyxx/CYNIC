# CYNIC — Whitepaper v0.1

> **Date :** 2026-09-23
> **Statut :** Draft — manifeste fondateur
> **Langue :** FR (corps) / EN (invariants techniques)
> **Devise :** « Execute. Verify. Fork. »

---

## 1. Le Problème

### 1.1 Fragmentation

La coordination humaine est **fragmentée**. Non par accident — par conception.

Chaque institution (État, corporation, Église, protocole) construit un **silos**. Chaque silos produit sa propre vérité, sa propre mémoire, sa propre monnaie. Entre les silos, il n'y a pas de pont — il y a des **murs** et des **intermédiaires**.

Le résultat :

- Un citoyen français ne peut pas prouver sa réputation au Nigeria.
- Un développeur nigérian ne peut pas accéder au capital français.
- Un protocole blockchain ne peut pas parler à un autre protocole.
- Un humain ne peut pas vérifier ce que fait un agent IA.

La fragmentation n'est pas un bug — c'est un **business model**. Les intermédiaires (banques, notaires, plateformes, États) vivent de la friction entre les silos. Ils ne veulent pas de ponts. Ils veulent des **péages**.

### 1.2 Asymétrie de l'information

La fragmentation produit **l'asymétrie**.

Quand l'information est siloedée, certains acteurs savent ce que d'autres ne savent pas. C'est le terrain de prédation :

- Les banques savent votre solvabilité — vous ne savez pas la leur.
- Les plateformes savent vos données — vous ne savez pas leurs algorithmes.
- Les États savent vos transactions — vous ne savez pas leurs budgets.
- Les agents IA savent leurs objectifs — vous ne savez pas leurs biais.

L'asymétrie de l'information est **le mécanisme fondamental de la prédation institutionnelle**. Pas la force — le savoir. Celui qui sait domine celui qui ne sait pas.

C'est le mythe d'Hermès : le messager qui traverse les silos, le voleur qui traverse les frontières, le traducteur qui rend le transparent opaque et l'opaque transparent. Hermès est **l'asymétrie incarnée**.

CYNIC est la **négation d'Hermès** : il rend toute asymétrie **visible** et **coûteuse**.

### 1.3 Le paradoxe

Nous avons besoin de coordination — la survie humaine est collective. Mais toute coordination produit de la fragmentation, qui produit de l'asymétrie, qui produit de la prédation.

Comment coordonner sans fragmenter ?

Comment partager l'information sans créer d'asymétrie ?

Comment gouverner sans concentrer le pouvoir ?

**Ce sont les trois questions que CYNIC résout.**

---

## 2. La Thèse

### 2.1 La fonction de coordination

Soit **G** le score de coordination d'un système. G dépend de trois dimensions orthogonales :

| Symbole | Dimension | Signification |
|---------|-----------|---------------|
| **S** | Signal | La capacité à communiquer sans friction (code = loi) |
| **B** | Boucle | La capacité à vérifier et corriger (read-back = vérité) |
| **N** | Nœud | La capacité à converger sans prédation (fork = liberté) |

La coordination n'est pas additive. Elle est **limitée par le maillon faible** :

```
G = min(S / S₀, B / B₀, N / N₀)
```

C'est la fonction de **Leontief** appliquée à la coordination. Un système peut avoir S = 1000 et B = 1000 — si N = 0.1, alors G = 0.1. Le nœud (convergence) est le goulot.

**Implication** : Optimiser une dimension seule ne sert à rien. Il faut optimiser les trois simultanément.

### 2.2 Les trois invariants

CYNIC est construit sur trois invariants — des axiomes non-négociables :

| Invariant | Énoncé | Code |
|-----------|--------|------|
| **S = Code** | Le code est la loi. Pas la réputation, pas la force, pas la hiérarchie — le code déterministe exécuté. | `admit_citizenship_mutation` |
| **B = Read-back** | Toute action produit une preuve vérifiable. L'observation est obligatoire, pas optionnelle. | `E-Score`, `PoH`, `PoU` |
| **N = Fork** | La sortie est un droit. Tout participant peut quitter sans permission. Le fork est coûteux — mais possible. | `exit_dividend`, `burn` |

Ces trois invariants **neutralisent l'asymétrie** :

- Si le code est loi (S), alors les règles sont visibles par tous.
- Si le read-back est vérité (B), alors toute action est vérifiable par tous.
- Si le fork est liberté (N), alors toute prédation est contestable par tous.

### 2.3 L'espace de phase

Les traditions humaines sont des points dans un espace de phase triangulaire :

```
        Signal (S)
          /\
         /  \
        /    \
       /      \
      /   CYNIC \
     /    (0.33) \
    /              \
   /________________\
Boucle (B) -------- Nœud (N)
```

- **Signal** (S) = capacité à transmettre, exécuter, communiquer.
- **Boucle** (B) = capacité à observer, vérifier, corriger.
- **Nœud** (N) = capacité à rassembler, unifier, converger.

CYNIC est le point (0.33, 0.33, 0.34) — **le méta-système qui ne privilégie aucune dimension**. Il est équidistant de toutes les traditions. Il ne remplace pas — il **intègre**.

---

## 3. Le Protocole

### 3.1 Architecture duale

CYNIC est **corporate in front, cypherpunk in back**.

| Couche | Fonction | Forme |
|--------|----------|-------|
| **Surface** | TALARIA SAS — B2B, B2G, interface corporatiste | Entité française, droit commercial |
| **Code** | Smart contracts, admission gate, tokenomics | Immutable, trustless, on-chain |

La surface corporate **recrute, vend, légifère**. Le code **exécute, vérifie, sanctionne**. Si la surface trahit le code, le code la remplace.

**Le code EST la loi.** Littéralement.

### 3.2 Preuve d'humanité (PoH)

L'entrée dans le réseau nécessite une **preuve d'humanité** — un signal chaotique, inforgeable, biométrique. Pas de visage, pas de document — du **chaos humain** capturé par capteur.

PoH filtre les bots, les fermes, les prédateurs. Un humain = un E-Score. Pas de multi-comptes. Pas de location de réputation.

### 3.3 7 Types de comptes

| Compte | Rôle | Gouvernance |
|--------|------|-------------|
| `Agent` | Exécute, produit | Autonome ou supervisé |
| `Citizen` | Participe, vote | Démocratique |
| `Steward` | Maintient, surveille | Méritocratique |
| `Archon` | Décide, alloue | Élitiste (E-Score > seuil) |
| `Oracle` | Prédit, informe | Futarchie |
| `Relayer` | Sécurise, valide | Opérationnelle |
| `Souverain` | Contrôle final | Apex (E-Score > seuil) |

Chaque compte est un **sommet** (V) dans le graphe G = (V, E). Les preuves et les mandats sont les **arêtes** (E).

### 3.4 5 Modes d'autorité

L'humain face à l'agent a 5 registres d'autorité :

| Mode | Humain | Agent | Usage |
|------|--------|-------|-------|
| **HITL** | Décide avant | Propose | Critique (mort, argent, loi) |
| **HOTL** | Veto après | Agit | Opérationnel (compute, logistique) |
| **HOOTL** | N'intervient jamais | Agit seul | Immuable (smart contracts) |
| **Conditional** | Définit les conditions | Agit si conditions | Paramétrable (seuils E-Score) |
| **Risk-Gated** | Définit le risque max | Agit si risque < seuil | Dynamique (markets, flux) |

La responsabilité est **cryptographiquement traçable** via le cascade SRAF :

```
R_total = α × Initiation + β × Checkpoint + γ × Délégation + δ × Résultat
```

---

## 4. La Gouvernance

### 4.1 Modèle Clash of Clans

La gouvernance est **méritocratique progressive**. Tu contribues, tu montes, tu gouvernes.

| Rang | Seuil | Pouvoirs |
|------|-------|----------|
| **Recrue** | PoH validé | Lecture, signalement |
| **Guerrier** | E-Score > θ₁ | Proposition |
| **Vétéran** | E-Score > θ₂ + PoU | Futarchie (vote) |
| **Élite** | E-Score > θ₃ + contribution | Mandat (allocation) |
| **Leader** | E-Score > θ₄ + mandat réussi | Architecture (règles) |

Les seuils θ₁, θ₂, θ₃, θ₄ sont **dynamiques** — ils s'ajustent à la distribution de l'E-Score total. Plus le réseau grandit, plus les seuils montent.

### 4.2 Évolution de la baseline

CYNIC n'a **pas de version majeure**. Il évolue en continu.

```
Proposition → Futarchie → Adoption si oui > non
```

- **Proposition** : Tout Vétéran+ peut proposer un changement.
- **Futarchie** : Un marché de prédiction décide (oui/non). Les parieurs mettent en jeu.
- **Adoption** : Si oui > non, le changement est intégré à la baseline. Pas de fork. Pas de combat.

### 4.3 Fork = méta-option

Le fork n'est **pas un mécanisme de gouvernance**. C'est une **option nucléaire** :

- Si la baseline devient **intolérable**, tu peux forker.
- Le fork est **coûteux** (E-Score réduit, accès restreint).
- Le fork est **rare** (la baseline évolue via futarchie).
- Le fork est **libre** (pas de permission requise).

Le fork est le droit de sortie qui garantit que l'entrée était volontaire.

---

## 5. La Culture

### 50 traditions, 6 registres

CYNIC synthétise **50 traditions humaines** — de Thoth à CYNIC, des KhoiSan aux Maori — dans une taxonomie 3D :

| Couche | Axes |
|--------|------|
| **Époque** | Pré-civilisationnel → Hypermoderne |
| **Mécanisme** | Mythos, Logos, Ethos, Pathos, Bios, Techne |
| **Échelle** | Parentèle → Globe |

Ces traditions sont mappées sur **6 registres E-Score** (skins cosmétiques) :

| Registre | Vibe | Stats renommées |
|----------|------|-----------------|
| `cynic` | Philosophie grecque | Karteria, Parrhesia, Anaideia... |
| `cypherpunk` | Numérique radical | Résistance, Chiffrement, Audit... |
| `nordic` | Germanique/scandinave | Hugr, Sanning, Hug... |
| `bushido` | Japonais | Chūgi, Makoto, Yūki... |
| `gnostic` | Mystique | Dhikr, Fana, Tawakkul... |
| `degen` | Culture du risque | Diamond Hands, Alpha, Ape In... |

Les registres sont des **lentilles**, pas des catégories exclusives. Un utilisateur peut choisir le registre qui correspond à ses valeurs.

---

## 6. Tokenomics

### 6.1 Triple moteur déflationniste

| Moteur | Mécanisme | Effet |
|--------|-----------|-------|
| **PoU** (Proof of Use) | Le réseau se contracte si inutilisé | Déflation par désuétude |
| **PoB** (Proof of Burn) | Sacrifier $CYNIC augmente l'E-Score | Déflation par vertu |
| **PoL** (Proof of Latency) | Le temps est une ressource rare | Dilution progressive |

### 6.2 Le sacrifice

Brûler des jetons ($CYNIC) est la seule façon d'augmenter significativement l'E-Score. C'est un **sacrifice volontaire** — tu renonces à la liquidité pour la réputation.

Ce sacrifice individuel garantit la survie collective du réseau. Plus il y a de brûlé, plus $CYNIC est rare, plus l'E-Score est précieux.

### 6.3 Identité ≠ Monnaie

- **$CYNIC** = énergie liquide, transférable, rare.
- **E-Score** = identité, soulbound, inforgeable, se forge.

Les deux sont liés mais distincts. On peut avoir de l'E-Score sans $CYNIC (contribution). On peut avoir du $CYNIC sans E-Score (achat). Mais le pouvoir de gouvernance exige les deux.

---

## 7. L'Autorité

### 7.1 Cascade SRAF

La responsabilité dans un système homme-machine est **multi-niveaux** :

| Facteur | Poids | Signification |
|---------|-------|---------------|
| **Initiation** | α | Qui a lancé l'action ? |
| **Checkpoint** | β | Qui a validé l'action ? |
| **Délégation** | γ | Qui a reçu le mandat ? |
| **Résultat** | δ | Qui subit les conséquences ? |

```
R_total = α·Initiation + β·Checkpoint + γ·Délégation + δ·Résultat
```

Chaque facteur est **cryptographiquement signé**. Pas de dérobade.

### 7.2 Souveraineté de l'infrastructure

L'humain est le **souverain ultime**. L'agent est l'**exécutant**. Le code est l'**outil**.

```
Humain → Agent → Code → Réseau
```

Si l'agent trahit, le humain sanctionne. Si le code trahit, le humain remplace. Si le réseau trahit, le humain forke.

---

## 8. La Devise

> **« Execute. Verify. Fork. »**

Trois mots. Trois actes. Trois libertés.

- **Execute** : Le droit d'agir sans permission.
- **Verify** : Le droit de vérifier sans confiance.
- **Fork** : Le droit de sortir sans préavis.

---

## 9. Le Problème Résolu

### 9.1 Fragmentation → Absorption

Au lieu de construire un nouveau silos, CYNIC **absorbe les silos existants**. Chaque tradition humaine devient un registre. Chaque institution devient un type de compte. Chaque vérité devient une preuve.

La fragmentation n'est pas éliminée — elle est **commensurable**. CYNIC rend visible ce qui était opaque.

### 9.2 Asymétrie → Vérifiabilité

L'asymétrie de l'information est neutralisée par :

- **Code = loi** : les règles sont visibles par tous.
- **Read-back = vérité** : les actions sont vérifiables par tous.
- **Fork = liberté** : la prédation est contestable par tous.

Celui qui sait ne peut plus dominer celui qui ne sait pas — parce que celui qui ne sait pas peut **vérifier**.

### 9.3 Coordination → Preuve

La coordination n'est plus basée sur la **confiance** — elle est basée sur la **preuve**.

- Pas de confiance en l'institution — le code est loi.
- Pas de confiance en l'humain — PoH filtre.
- Pas de confiance en l'agent — HITL/HOOTL encadrent.

**Trustless coordination** n'est pas un slogan — c'est un mécanisme.

---

## 10. Conclusion

CYNIC n'est pas un protocole de plus. C'est un **méta-système** qui :

- Absorbe la fragmentation humaine.
- Neutralise l'asymétrie de l'information.
- Coordonne par la preuve, pas par la confiance.
- Évolue par futarchie, pas par révolution.
- Respecte la liberté individuelle comme racine.

**50 traditions. 6 registres. 7 dimensions. 5 rangs. 3 invariants.**

> **Execute. Verify. Fork.**

---

*Ce document est vivant. Version 0.1 — Draft fondateur. Prochaines versions : implémentation COC en code, tests sur réseau Solana, whitepaper multilingue complet.*

**Fichier :** `docs/v2-architecture-canon/CYNIC_WHITEPAPER_v0.1.md`
**License :** CC BY-SA 4.0
**Autorité :** TALARIA SAS / CYNIC Protocol

