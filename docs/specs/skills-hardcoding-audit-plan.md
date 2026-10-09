# CYNIC Skills Hardcoding Audit — Plan de Correction

## Résumé

| Skill | Hardcoding | Sévérité | Status |
|---|---|---|---|
| `cynic-runtime-diagnostics` | `10.10.10.103`, `127.0.0.1:3030`, `/etc/cynic/`, `/var/lib/cynic` | **Critique** | À corriger |
| `cynic-crate-repair` | `10.10.10.119` (builder), `10.10.10.40:8080` | **Haute** | À corriger |
| `cynic-builder-runtime` | Forgejo webhook, builder IPs | **Haute** | À corriger |
| `cynic-infra-diagnostics` | `10.10.10.103`, `127.0.0.1:3030`, `/etc/cynic/` | **Critique** | À corriger |
| `cynic-infrastructure` | `127.0.0.1:3030`, `/etc/cynic/inference.toml` | **Moyenne** | À corriger |
| `infra-ops-as-code` | `/etc/cynic/secrets/`, `/var/lib/cynic/`, `127.0.0.1:9147` | **Moyenne** | À corriger |

## Principe

**AGENTS.md invariant #2** : "No hardcoding when a dynamic contract exists"
**AGENTS.md invariant #3** : "Dynamic detection before specialization"

**Règle** : Si CYNIC expose une surface de découverte (CLI/MCP), **aucune skill ne doit contourner ça** avec des IPs/ports/chemins codés en dur.

## Surfaces CYNIC de remplacement

| Au lieu de... | Surface CYNIC |
|---|---|
| `10.10.10.103` (SurrealDB) | `cynic runtime storage-capabilities --json` |
| `127.0.0.1:3030` (kernel REST) | `cynic runtime status --json` |
| `10.10.10.119` (builder) | `cynic runtime registry --json` |
| `/etc/cynic/secrets/*` | Non exposé (sécurité) ou `cynic runtime storage-capabilities` |
| `/var/lib/cynic` | `cynic runtime registry --json` |
| `/etc/cynic/inference.toml` | `cynic runtime inference-discover --json` |

## Plan d'action

### Phase 1 — cynic-runtime-diagnostics (Critique)
- Remplacer `10.10.10.103` par `cynic runtime storage-capabilities --json`
- Remplacer `127.0.0.1:3030` par `cynic runtime status --json`
- Remplacer `/etc/cynic/secrets/` par "Utiliser `cynic runtime storage-capabilities` pour identifier le storage"
- Remplacer `192.168.100.2:3030` par "Obtenir via `cynic runtime registry --json`"

### Phase 2 — cynic-crate-repair (Haute)
- Remplacer `10.10.10.119` par `cynic runtime registry --json | jq '.nodes[] | select(.role == "build_runner")'`
- Remplacer `10.10.10.40:8080` par `cynic runtime registry --json`

### Phase 3 — cynic-builder-runtime (Haute)
- Remplacer les références Forgejo/builder par `cynic runtime registry --json`

### Phase 4 — cynic-infra-diagnostics (Critique)
- Remplacer `10.10.10.103` par `cynic runtime storage-capabilities`
- Remplacer `127.0.0.1:3030` par `cynic runtime status`

### Phase 5 — cynic-infrastructure (Moyenne)
- Remplacer `127.0.0.1:3030` par `cynic runtime status`
- Remplacer `/etc/cynic/inference.toml` par `cynic runtime inference-discover`

### Phase 6 — infra-ops-as-code (Moyenne)
- Documenter que les secrets ne sont jamais codés en dur dans les skills
- Référer à `cynic runtime storage-capabilities` pour la découverte

## Validation

Après correction, vérifier avec :
```bash
grep -r "10\.10\.10\.\|192\.168\.\|/etc/cynic\|/var/lib/cynic\|127\.0\.0\.1:" /home/user/.hermes/skills/cynic*.md
```
Aucun résultat attendu (sauf commentaires explicatifs).

---

*Plan créé le 2026-08-20. Exécution en cours.*
