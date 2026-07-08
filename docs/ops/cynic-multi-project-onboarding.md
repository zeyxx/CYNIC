# CYNIC — Onboarding multi-projets (VPS ou local)

Ce guide décrit la méthode pour brancher un nouveau projet sur CYNIC sans
reconstruire le framework.

## Étape 1 — Délimiter le noyau

- Identifier les entités et ports stables du projet.
- Refuser le couplage direct `project -> cynic` dans les modules d’organe.
- Ajouter les contracts dans la couche `ports`.

## Étape 2 — Injecter les dépendances

- Créer un adapter pour chaque besoin externe:
  - `ProjectStateAdapter`
  - `ProjectSecretAdapter`
  - `ProjectTransportAdapter`
- Brancher ces adapters dans le bootstrap de démarrage.

## Étape 3 — Externaliser la config

- Garder la config runtime dans une source d’environnement, pas dans le code.
- Utiliser un fichier projet déclaré (`project.config.*`) + `.env` + secrets manager.

## Étape 4 — Déploiement VPS

- Déployer CYNIC kernel + infra minimal de base.
- Brancher l’organe projet via sa fiche d’instance.
- Vérifier la health via `/coord/who` et `/health`.
- Valider que l’observabilité écrit en JSONL dans l’espace prévu.

## Étape 5 — Cycle d’acceptation

- `make gate-0`
- test de projection MCP locale
- vérification mempool/task d’activation
- run smoke test end-to-end de l’organe projet

## Anti-patterns à bannir

- hardcoder `localhost` ou identifiants dans des classes de domaine.
- créer un `singleton` global de config mutable.
- faire des dépendances circulaires entre organes.
- mélanger règles métier et I/O (DB/HTTP/system call).

## Résultat attendu

Le même dépôt CYNIC peut servir à N projets:
- en changeant uniquement `project adapters` + config,
- sans toucher au noyau commun ni à l’orchestration centrale.

