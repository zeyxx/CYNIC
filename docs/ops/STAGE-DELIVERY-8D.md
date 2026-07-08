# Stage Delivery — Cadence 8 dimensions

Ce document fixe la cadence minimale pour développer avec une bonne stabilité
et une vélocité mesurable.

## Dimensions à couvrir (1 itération complète = 8 dimensions)

1. **Portabilité**
   - Vérifie les dépendances de chemins dans les surfaces runtime/source.
   - Source d’évidence: `infra/agent-exposure.json`, `infra/registry.json`, projections d'organes.

2. **MCP / Lifecycle**
   - Vérifie les outils clés dans `cynic-coord`.
   - Vérifie les profils de lifecycle (`gate-2`, `mcp-reload`, `deploy`) dans le registre.

3. **Réseau**
   - Vérifie le contrat réseau dans `infra/registry.json`.
   - Vérifie l’absence de valeurs IP littérales non-placeholder dans les champs réseau du registre.

4. **Services**
   - Vérifie la présence des services attendus (`docs/ops/expected-services.txt`).
   - Vérifie que les unités existent dans le repo.

5. **Fleet**
   - Vérifie la santé déclarative du registre (`nodes`, `status`, `aliases`, unique).

6. **Abstraction**
   - Mesure la dette "shim" de compatibilité.
   - Encourage la concentration des comportements canonaux dans `organs/...` + `crates/...`.

7. **Simplicité**
   - Vérifie la couverture des `MANIFEST.yaml` et des entrypoints par organ.

8. **Transparence + Gouvernance**
   - Vérifie la présence des preuves observables (observe/health/policy docs + AGENTS).

## Commande d’exécution

```bash
make stage-readiness ITERATIONS=8
```

Arguments optionnels:

- `ITERATIONS=n` : plan de distribution sur n itérations (8 dimensions = 1 cycle complet).
- Avec `n > 8`, le plan reconduit les dimensions dans l’ordre dans un cycle strict de 8.
- `JSON=1` : sortie JSON.
- `MAX_WARN=K` : active un mode strict (`--max-warn=K`) pour faire échouer en CI locale au-delà d'un nombre max de `WARN`.
- `MAX_WARN` n'est pas requis; sans lui, les `WARN` restent non bloquants.

Exemple:

```bash
make stage-readiness ITERATIONS=12 JSON=1
MAX_WARN=0 make stage-readiness ITERATIONS=8
make stage-readiness-strict ITERATIONS=8
```

Pour CI locale:

```bash
make stage-readiness-strict ITERATIONS=12
echo "make_exit=$?"

# Vérification canonique sans variante ad hoc:
make stage-readiness-strict ITERATIONS=12
echo "make_exit=$?"
```

La cible `stage-readiness-strict` force `MAX_WARN=0` (aucun WARN accepté).
`make stage-readiness-strict` renvoie typiquement `2` via l'agrégation de Make quand `warn_limit_exceeded` est atteint.

## Interprétation

- `PASS`: dimension stabilisée pour cette itération.
- `WARN`: dimension à améliorer (pas bloquante pour le build, mais à traiter avant livraison de stage).
- `FAIL`: dimension bloquante pour la progression confiable.

Pour la régularité du stage:

- Faire au minimum un passage `make stage-readiness` à la fin de chaque cycle de
  travail.
- Un résultat `FAIL` impose une action corrective avant la prochaine itération.
- Un résultat `WARN` est consigné dans la tâche suivante avec date butoir.

## Pourquoi ce format fonctionne bien pour le stage

- Il maintient les contraintes de **fiabilité** (pas d’érosion silencieuse de l’infrastructure),
  la **stabilité** (contrôle de lifecycle, services, fleet),
  et la **vélocité** (plan d’itérations simple, réutilisable, observable).

Règle de livraison:

- Les `PASS` doivent rester stables par dimension.
- Les `WARN` alimentent automatiquement le plan d'itération suivant.
- Une `FAIL` empêche la livraison si non corrigée avant la release.

### Règle de scaling

- Une base de `8` dimensions définit la granularité.
- `N = 8` couvre un cycle complet.
- `N = 16` couvre deux cycles complets.
- `N = 12` couvre 1 cycle complet + 4 itérations du cycle suivant.
