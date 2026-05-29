<!-- RECONSTRUCTED 2026-05-29 — Original lost in home directory reset. Content approximate. -->

# Conditions Générales d'Utilisation — CYNIC API (Draft)

**Statut :** Brouillon — à valider avec un juriste
**Version :** 0.1

---

## 1. Objet

Les présentes CGU régissent l'utilisation de l'API CYNIC, un service de jugement automatisé de tokens et actifs numériques opéré par B&C SAS (en cours de constitution).

## 2. Description du service

CYNIC fournit des **verdicts** sur des stimuli (tokens, signaux sociaux, données de marché) via une API REST. Les verdicts sont :
- **HOWL** : le stimulus est jugé fiable par le consensus des modèles
- **BARK** : le stimulus est jugé suspect ou dangereux
- **EPOCHÉ** : les modèles sont en désaccord, jugement suspendu

## 3. Limitations fondamentales

### 3.1 Confiance plafonnée

**La confiance maximale d'un verdict CYNIC est de 61.8% (φ⁻¹).** Ceci est une propriété architecturale intentionnelle, pas une limitation.

### 3.2 Pas un conseil financier

Les verdicts CYNIC ne constituent en aucun cas un conseil en investissement, une recommandation d'achat ou de vente, ou un avis financier. L'utilisateur est seul responsable de ses décisions d'investissement.

### 3.3 Pas de garantie

CYNIC fournit un jugement basé sur l'état actuel de ses modèles et de sa mémoire (cristaux). Ce jugement peut être erroné. Aucune garantie de résultat n'est offerte.

## 4. Tiers de service

| Tier | Verdicts/jour | Prix | SLA |
|------|--------------|------|-----|
| Free | 10 | Gratuit | Best effort |
| Pro | 1 000 | 99€/mois | 99% uptime |
| Enterprise | Illimité | Sur devis | 99.9% uptime + support |

## 5. Données

### 5.1 Données envoyées

Les stimuli envoyés à l'API peuvent être stockés pour améliorer les modèles (cristaux). Les données sont stockées localement (infrastructure souveraine, zéro cloud).

### 5.2 Pas de revente

Les données des utilisateurs ne sont jamais revendues à des tiers.

## 6. Responsabilité

B&C SAS ne saurait être tenu responsable des pertes financières résultant de l'utilisation des verdicts CYNIC. La confiance plafonnée (§3.1) constitue un avertissement permanent intégré au produit.

## 7. Propriété intellectuelle

Le kernel CYNIC est distribué sous licence open source. L'API, les cristaux accumulés, les modèles entraînés et le dataset Hermes restent la propriété de B&C SAS.

## 8. Droit applicable

Droit français. Tribunaux compétents : Paris.

---

*DRAFT — À réviser par un avocat avant mise en production.*
