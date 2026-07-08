# CYNIC — Portabilité multi-projets (Fondation)

Objectif
Ce document pose la règle minimale pour que CYNIC soit porté sur `N` projets sans
réécriture de coeur.

## 1) Principe central

Le noyau de CYNIC doit rester agnostique au projet applicatif.
Les intégrations spécifiques (`auth`, stockage local, UI, opérateurs métiers) passent
par des adaptateurs explicitement injectés.

## 2) Règle d'inversion

- Les modules `cynic-core` / `organs` ne lisent aucune donnée métier applicative.
- Les dépendances projet spécifiques sont construites au bord d'entrée
  (bootstrap/adapter layer).
- Les contrats publics sont stables: API de bus, observabilité, lifecycle, registry.

## 3) Topologie cible

Pour chaque projet utilisant CYNIC, on doit retrouver les couches:

1. `core` — protocoles, types, policy primitives, erreurs.
2. `ports` — contracts d’intégration (storage, secrets, identité, transport).
3. `adapters` — implémentations projet-spécifiques.
4. `organ` — cellules opérationnelles autonomes.
5. `runtime` — services, tâches, config, état minimal.

## 4) Contrat d’implémentation (minimum à fournir par projet)

- `project.config` (ou équivalent) décrit l’environnement cible.
- un adaptateur de persistance (`ProjectStateStore`).
- un adaptateur de secret vault (`SecretProvider`).
- un adaptateur kernel/mesh endpoint.
- une stratégie d’autorisations/permissions projet.

## 5) Ce qui ne doit jamais être codé en dur

- URLs, tokens, hosts, clés d’API.
- noms d’instances ou roles de nœuds production.
- chemins de repo internes (`/home/user/...`) en dur.

## 6) Indicateur de maturité

Un projet est portable quand:
- la compilation du cœur CYNIC passe sans dépendance applicative,
- les adaptateurs projet sont remplaçables par d’autres implémentations,
- le déploiement (démo locale / VPS) ne change que la couche `adapters` et config.

