# CYNIC Technical Debt Registry

**Approche** : bottom-up, par ROI cognitif, observabilité avant mutation
**Source d'autorité** : code observé, contracts/, infra/, runtime proof
**Règle** : chaque entrée = dette observée + impact + coût estimé + owner + status

---

## Index par catégorie

| Catégorie | Fichier | Entrées | Criticité max |
|-----------|---------|---------|---------------|
| **SPOF** | [spof.md](spof.md) | 7 | CRITIQUE |
| **Monoliths / SoC violations** | [monoliths.md](monoliths.md) | 12 | HIGH |
| **Ownership flou / missing owners** | [ownership.md](ownership.md) | 8 | HIGH |
| **Doublons code** | [duplication.md](duplication.md) | 6 | MEDIUM |
| **Prose noise à archiver** | [prose-archive.md](prose-archive.md) | 15 | LOW |
| **Config drift / missing contracts** | [config-drift.md](config-drift.md) | 5 | MEDIUM |
| **Tests / proof gaps** | [test-gaps.md](test-gaps.md) | 4 | MEDIUM |

---

## Métriques globales

- **Total entrées** : 57
- **CRITIQUE** : 7 (tous SPOF)
- **HIGH** : 20 (monoliths + ownership)
- **MEDIUM** : 15 (duplication + config + tests)
- **LOW** : 15 (prose archive)

---

## ROI cognitif (priorisation suggérée)

| Phase | Focus | Entrées | Effort estimé | ROI |
|-------|-------|---------|---------------|-----|
| **P0** | SPOF kernel (process, storage, REST, coord) | 4 | 2-3 sem | Évite cluster down total |
| **P1** | Split registry.rs + federation_runtime.rs | 2 | 3-4 j | Unlock distributed registry |
| **P1** | Split runtime_surface.rs (CLI) | 1 | 1-2 j | CLI thin, testable |
| **P2** | Split health_signal.rs (protocol) | 1 | 1-2 j | Policy séparée des types |
| **P2** | Move audit logic out of CLI | 3 | 2-3 j | SoC respecté |
| **P3** | Archive prose docs | 15 | 0.5 j | Cognitive load ↓ |
| **P3** | Dedup config drift | 5 | 1 j | Single source of truth |

---

## Règles de gestion

1. **Une entrée = un fichier** dans la catégorie correspondante
2. **Status** : `observed` → `planned` → `in_progress` → `done` → `verified`
3. **Preuve de résolution** : test + read-back observation (pas de claim sans proof)
4. **Owner** : crate + module + personne (si assigné)
5. **ROI** : `criticité × fréquence_impact / effort_jours`

---

## Navigation rapide

- **SPOF immédiats** → [spof.md#critique](spof.md#critique)
- **Monoliths > 400 lignes** → [monoliths.md#oversized](monoliths.md#oversized)
- **Ownership manquant** → [ownership.md#missing](ownership.md#missing)
- **Doublons actifs** → [duplication.md#active](duplication.md#active)