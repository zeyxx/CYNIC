# SESSION HANDOFF — Kernel Organ Extraction

**Session précédente** : `agent/hermes/supply-chain-low-layer-hardening` → `2eea86ba`
**Session suivante** : `agent/hermes/kernel-organ-decoupling`

---

## 1. État actuel

### cynic-inference (propre, livré)
| Couche | Purity | Tests |
|--------|--------|-------|
| `domain/` | ✅ 100% pure | 5/5 arch_boundary |
| `infra/` | ✅ I/O confinée | ✅ |
| `application/` | ⚠️ 13 direct adapter imports | À extraire plus tard |
| `organ/` | ⚠️ God object legacy | Pas de tests unitaires |

### cynic-kernel (dette)
54 fichiers dépendent de `organ/` :
- `domain/organ.rs` (93 LOC) — interfaces métier dans le kernel
- `organ/mod.rs` — ré-exporte `cynic_inference::organ::*`
- 16 fichiers infra, 11 API, 4 bootstrap, 2 senses

---

## 2. Objectif de la session

Sortir `organ/` du kernel. Le kernel = bus d'observabilité, pas d'orchestration métier.

**Cible** :
```
cynic-kernel (bus) → imports → cynic-inference-runtime (orchestration)
                                 ↑
                           implémente OrganPort
```

**Anti-cible** : Ne pas casser les endpoints REST existants (`/health`, `/capabilities`).

---

## 3. Plan proposé

### Phase 1 — Inventaire contractuel (30 min)
1. Lister les méthodes de `OrganPort` utilisées par le kernel
2. Identifier si l'implémentation `InferenceOrgan` est la seule
3. Vérifier les endpoints REST qui exposent `OrganHealth`

### Phase 2 — Strangler pattern (2h minimum)
1. Déplacer `cynic-kernel/src/domain/organ.rs` → `cynic-inference/src/ports/organ_port.rs`
2. Garder un ré-export dans le kernel pour compatibilité (`pub use cynic_inference::ports::organ_port`)
3. Mettre à jour les 54 imports

### Phase 3 — Tests
1. `cargo test -p cynic-kernel` → 0 régression
2. `cargo test -p cynic-inference` → organ/ tests passent
3. Builder (10.10.10.119) vérifié

### Phase 4 — Nettoyage
1. Supprimer `cynic-kernel/src/organ/mod.rs` (ré-export inutile)
2. Supprimer `cynic-inference/src/organ/` (si l'orchestration moderne dans `application/` couvre)
3. Architecture boundary test pour le kernel

---

## 4. Fichiers à modifier (identifiés)

| Fichier | Action |
|---------|--------|
| `cynic-kernel/src/domain/organ.rs` | Déplacer → `cynic-inference/src/ports/organ_port.rs` |
| `cynic-kernel/src/organ/mod.rs` | Ré-écrire comme façade |
| `cynic-kernel/src/infra/tasks/lifecycle.rs` | Remplacer `Option<Arc<InferenceOrgan>>` par `Arc<dyn OrganPort>` |
| `cynic-kernel/src/infra/tasks/runtime_loops/probes.rs` | Idem |
| `cynic-kernel/src/api/rest/health.rs` | Adapter le type de retour |
| 48 autres fichiers | Imports `crate::organ::*` → `crate::ports::organ_port::*` |

---

## 5. Contraintes

- **Builder** : `ssh cynic-builder` (rsync vers `/tmp/cynic-src/`)
- **Cortex** : rustc ICE sur `cynic-host` — ne PAS builder localement
- **Agents parallèles** : Un autre agent travaille sur `cynic-askesis`, `cynic-domain-ops`, `cynic-mcp` — ne pas toucher à ces crates
- **Commits** : Un commit par phase, `git add chemins/explicites`, jamais `git add -A`
- **Branche** : Créer `agent/hermes/kernel-organ-decoupling` depuis HEAD

---

## 6. Acceptance criteria

- [ ] `cargo check -p cynic-kernel` → 0 erreurs
- [ ] `cargo check -p cynic-inference` → 0 erreurs
- [ ] `cargo test -p cynic-kernel` → 0 régression
- [ ] `cynic-kernel/src/organ/` supprimé
- [ ] `cynic-kernel/src/domain/organ.rs` supprimé (remplacé par ré-export façade)
- [ ] Architecture boundary test pour le kernel ajouté
