# RAPPORT DE SESSION — knowledge-canon

## Identité unique

| champ | valeur |
|---|---|
| session_id | `20260929_194202_a40989` |
| branche | `agent/hermes/knowledge-canon` |
| HEAD | `b9f9b00c43e2990dea870740776ce1ec0e64542d` |
| worktree | `/home/user/cynic-restore-20260928` |
| remote | `ssh://forgejo@forgejo.synapse.cynic:22/jeanterre13/CYNIC.git` |
| builder | LXC 105 `cynic-builder`, `/var/lib/cynic/source/repositories/CYNIC` |
| état | **propre** — local == builder == remote |

---

## Résumé exécutif

**Phase C/D terminée** pour les générateurs sudoers non-réseau. **FCIS formalisé** dans les contrats normatifs. **4 générateurs corrigés ou supprimés**. **1 fail-open grave supprimé**. **1 mesure critique établie** (grant sudoers nu = n'importe quel argument).

**Reste ici** : 3 items, **tous réseau** — hors périmètre de cette session.

---

## Commits ce tour (20)

```
b9f9b00c fix(test): role is String not &str in bridge tests
7bf37e82 fix(bridge): validate staged sudoers before installing it
61a95966 docs(lxc): record that this membrane's bound is entirely inside the binary
54572b48 fix(sudoers): one escaper, applied by every renderer
4097162e fix(host-exec): delete the unbounded sudoers generator
1114c12e test(fcis): generalize the core-purity ratchet beyond cynic-network
7f8e9599 docs(contracts): FCIS is the architecture policy, not strict hexagonal
49e73f6d refactor(pve-helper): consume the unified capability catalog
e3297b91 fix(test): validate_sudoers is a ValidateSudoers variant, not Argv
df8c6d4b fix(pve-helper): role is String not &str; drop unused PathBuf import
272222aa fix(privilege): one DEFAULT_CATALOG_PATH, not two
92f69dab refactor(pve-helper): consume the unified capability catalog
c395a7e2 docs(design): record what a sudoers class actually bounds
0da81771 fix(privilege): bounded sudoers classes, colon escaping, validate-before-install
35eb9d34 refactor(privilege): transport_caps uses ParamKind, deletes TransportParamKind
20e705d1 fix(privilege): BranchName uses an allowlist, not a denylist
5b2f3bdd docs(design): mark Phase A/B done, C/D partial, record the sudoers lesson
45493bfb fix(privilege): sudoers names the binary, not the resolved argv
6d291f7a fix(catalog): remove --raw from forgejo-machine-token argv
a74d322a feat(privilege): sudoers generation from capability catalog
```

---

## Ce qui a été fait

### 1. FCIS formalisé dans les contrats normatifs

**Finding** : FCIS était formalisé **seulement** dans `infra/NETWORK_MYCELIUM_SPECIFICATION.md` §1.2 et enforcé par **un seul test** (`cynic-network`). Les contrats normatifs disaient encore `hexagonal (ports and adapters) with domain-driven design`. `grep -c FCIS contracts/pruning-policy.json` = **0**.

| fichier | changement |
|---|---|
| `contracts/pruning-policy.json` | `taxonomy.hexagonal_and_ddd` → **`taxonomy.fcis_and_ddd`** |
| `contracts/foundation.json` | Nouvelle hard boundary **`fcis_core_shell_separation`** |

La clause `known_gap` *« every adapter implements a declared port »* est **retirée avec sa raison** (FCIS ne l'exige pas).

### 2. Gate retravaillé pour FCIS

**Nouveau test** `a_pure_core_holds_no_io` — généralisation de `cynic-network/tests/architecture_boundary.rs` à tous les crates ayant adopté FCIS. Ratchet shrink-only.

**Mesure 2026-10-06** — crates qui ont coupé le shell sans nettoyer le core :

| crate | fichiers core avec I/O |
|---|---|
| `cynic-host` | 18 / 353 |
| `cynic-node` | 17 / 218 |
| `cynic-inference` | 9 / 112 |
| `cynic-secrets` | 1 / 4 |
| `cynic-build`, `cynic-domain-ops`, `cynic-intel`, `cynic-proxmox`, `cynic-release` | **0** |

### 3. Générateurs sudoers — 4 corrigés ou supprimés

| générateur | crate | statut |
|---|---|---|
| `catalog.rs` | `cynic-protocol` | ✅ unifié |
| `bounded_helper.rs` | `cynic-host` | ✅ escaper unifié |
| `host_exec_policy.rs` | `cynic-host` | ✅ **supprimé** (non borné) |
| `cynic-lxc/bundle.rs` | `cynic-lxc` | ✅ mesuré + verrouillé |
| `bridge.rs` | `cynic-host` | ✅ validate-before-install |
| `framework_bootstrap/scripts.rs` | `cynic-host` | ℹ️ **pas un générateur** — installe un sudoers pré-généré |
| `network_actuator_contract.rs` | `cynic-node` | ⛔ réseau |

### 4. Fail-open grave supprimé

`HostExecPolicyDocument::sudoers_rule()` : zéro appelant, zéro test, et il générait :
```
cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-host-exec
                                                    ^ AUCUN argument
```
**Toute sous-commande du binaire, y compris les mutantes, aurait tourné root.** Supprimé.

### 5. Mesure critique établie

**Un grant sudoers nu autorise n'importe quel argument.** Mesuré sur le builder :
```
cynic ALL=(root) NOPASSWD: /usr/bin/echo
→ sudo -n -u cynic sudo -n /usr/bin/echo REAL_ARGS_WORK
→ exécuté root, EXIT=0
```

La règle `--status` de `cynic-lxc` était **redondante**. Le bound est entièrement dans le binaire (`admit()` → `admitted_operation()`).

### 6. Bug P1 corrigé dans `bounded_helper`

`sudoers_rule()` ne échappait pas `:`. `escape_sudoers()` est maintenant `pub` et partagé.

### 7. Bug P2 corrigé dans `bridge.rs`

`install_commands_for_runtime` faisait **install AVANT validate**, et la validation ciblait le fichier **installé**.

---

## Tests finaux

| crate | résultat |
|---|---|
| `cynic-host --lib bridge` | **13/13** |
| `cynic-lxc --lib` | **5/5** |
| `cynic-lxc` architecture_boundary | **2/2** |
| `cynic-host --lib privilege` | **26/26** |
| `cynic-protocol --lib privilege` | **28/28** |
| `crate_topology_coherence` | **6/6** |

---

## Reste ouvert — 3 items, tous réseau

| item | blocage |
|---|---|
| `RegistryService` | Dernière collision concept-ownership, la branche réseau ne l'a pas encore réparée |
| `hexagonal_ddd_verifier` | La branche réseau l'a modifié (-145 lignes) depuis une base pré-FCIS |
| `network_actuator_contract.rs` | Générateur réseau |

---

## État des branches

### Branches réseau ouvertes

| branche | dernier commit | rôle |
|---|---|---|
| `network-convergence` | `ddea0d0f` 2026-10-06 | **agrégation** — contient 100% de `network-mycelium` + 10 commits |
| `network-mycelium-unification` | `a0baaa19` 2026-10-06 | tranche mycelium (contenue dans convergence) |
| `network-vocabulary` | `7cebbe6d` 2026-10-06 | tranche vocabulaire (contenue dans convergence) |

### Autres branches actives

| branche | dernier commit |
|---|---|
| `plane-taxonomy-v1` | `fc3b3899` 2026-10-06 |
| `core-extraction` | `a578b5b2` 2026-10-06 |
| `recovery/core-clean-20261006` | `f9145645` 2026-10-06 |

### `main`

Gelé depuis 2026-10-03 (`5efe23d1`). **Rien n'a mergé.**

---

## Recommandations pour la convergence

### Ordre causal

1. **`knowledge-canon` → `main`** — porte FCIS, 168 fichiers uniques, fondation
2. **`network-convergence` → `main`** — rebasé, 15 conflits (vocabulaire réseau)
3. **`core-extraction` → `main`** — +16 uniques
4. **`plane-taxonomy-v1` → `main`** — +13 uniques, merge-base ancien
5. **Archiver** `network-vocabulary` + `network-mycelium` (contenus dans `network-convergence`)

### Overlap mesuré

`knowledge-canon` ↔ `network-convergence` : **15 fichiers / 319 touchés**

```
Cargo.toml
crates/cynic-cli/{Cargo.toml, cli_args.rs, lib.rs, root_dispatch.rs}
crates/cynic-cli/src/network_surface/mesh.rs
crates/cynic-host/Cargo.toml
crates/cynic-node/src/{dns_activation_status,dns_bundle,fleet_inventory,
                       network_mesh_plan,network_mesh_plan/model,
                       network_shadow_proof,observe,provider_fallback_status}.rs
```

### Pourquoi `knowledge-canon` d'abord

Il porte les contrats normatifs FCIS. Si le réseau mergait avant, il imposerait ses `Cargo.toml`/`lib.rs` et il faudrait re-résoudre FCIS après.

---

## Métadonnées techniques

| champ | valeur |
|---|---|
| toolchain | root 1.95.0, JSON 1.96.0, CI `@stable`, réel 2, plafond 2 |
| crates | 50, L1=12, 8 binaires |
| `cynic-host` | 87 276 LOC |
| `cynic-node` | 54 774 LOC |
| `cynic-proxmox` | 1 509 LOC, L1, 4 deps |
| gates rouges pré-existants | `hexagonal_ddd_verifier` (5 collisions réseau), `cynic-node --lib` (3 tests), `cynic-cli --lib` (1 test) |
| builder | LXC 105, 120G rootfs (55%), 4 cores, 8G RAM, `jobs=1` |
| hyperviseur | `proxmox-forge` (pve), standalone, `pmxcfs` monté, `pvedaemon` actif |
| conteneurs | 100 organ-headscale, 101 organ-forgejo, 102 cynic-agent-host, 103 cynic-lxc-surrealdb, 105 cynic-builder |

---

## Signatures

- **Session** : `20260929_194202_a40989`
- **Branche** : `agent/hermes/knowledge-canon`
- **HEAD** : `b9f9b00c43e2990dea870740776ce1ec0e64542d`
- **Date** : 2026-10-07
- **État** : **prêt à mergé**
