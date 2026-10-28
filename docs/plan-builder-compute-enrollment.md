# Plan d'enrollment et de séparation des rôles — CYNIC builder/compute

## Problèmes identifiés

1. `cynic-builder` nommé comme s'il était le seul builder → confusion.
2. `cynic-compute-01` a plusieurs rôles mais son enrollment n'est pas complet.
3. SSH configuré pour `titouan` → agents lancent des processus sous ce user → conflits et incohérences avec `cynic`.
4. `cynic` comme binaire standalone n'est pas formalisé.

## Plan

### Phase 1 — Renommage `cynic-builder` → `cynic-builder-01`

Surfaces à mettre à jour :
- `infra/registry.json` — node id, name, service names.
- `infra/runtime-authority.json` — références au builder.
- `infra/registry.json` — réservations IP (10.10.10.119).
- Code Rust — références au builder dans les crates (search_files).
- Docs — références dans les docs.
- Systemd units — si présents.

### Phase 2 — Formalisation de l'enrollment de compute-01

- Cloner le repo CYNIC sur compute-01 (SSH titouan).
- Installer les toolchains (Rust, Node) si absents.
- Créer le user `cynic` (séparé de titouan).
- Configurer SSH pour `cynic` (pas titouan).
- Formaliser la cérémonie d'enrollment (registry, runtime-authority).
- Vérifier les health endpoints.

### Phase 3 — Formalisation de `cynic` comme binaire standalone

- Le binaire `cynic` doit tourner sous le user `cynic`, pas `titouan`.
- Les agents doivent utiliser `cynic` comme binaire standalone, pas des processus ad-hoc sous titouan.
- Séparation des processus : pas de conflit entre titouan (humain) et cynic (agent).
- Formaliser les permissions et les paths (CYNIC root sous /home/cynic ou /opt/cynic).

### Phase 4 — Granularité ciblée

- `cynic-builder-01` (10.10.10.119) → builds légers uniquement.
- `cynic-compute-01` (192.168.0.25) → builds lourds, benchmarks, vérification.
- Éviter la contention parallèle.

## Actions immédiates

1. Renommer `cynic-builder` → `cynic-builder-01` sur toutes les surfaces.
2. Cloner CYNIC sur compute-01 et installer toolchains.
3. Créer le user `cynic` sur compute-01.
4. Configurer SSH pour `cynic`.
5. Formaliser l'enrollment dans le registry.

## Risques

- Renommage sur toutes les surfaces = risque de casse si références manquées.
- Création du user `cynic` = changement de permissions sur compute-01.
- Enrollment incomplet = risque de perte de service.

## Validation

- `cynic audit all` doit montrer host_observed = true après enrollment.
- `cynic git branch status` doit être propre.
- Les health endpoints de compute-01 doivent répondre.
