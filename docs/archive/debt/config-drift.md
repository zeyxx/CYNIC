# Config Drift & Missing Contracts Registry — CYNIC

**Critère** : Configuration divergente entre fichiers, contrats manquants, sources de vérité multiples
**Référence** : AGENTS.md §2 "No hardcoding when dynamic contract exists" + §5 "Declarative desired state plus observed proof"

---

## DRIFT-001: Registry Variants — 4 Sources de Vérité
- **Fichiers** :
  1. `infra/registry.json` (18,833 chars) — **production registry** (autoritaire)
  2. `infra/registries/single-node.json` (3,484 chars) — bootstrap local
  3. `infra/registries/lab.json` (6,264 chars) — conformance lab
  4. `infra/registries/home-fiber.json` (non lu) — probable unused
- **Drift** : Champs communs (`runtime_artifacts`, `roles`, `nodes`) dupliqués, divergents
- **Canonique** : `infra/registry.json` = source unique, variants = projections générées
- **Action** : Générer single-node/lab/home-fiber depuis registry.json + params
- **Status** : `observed`

### Diffs observés (registry.json vs single-node.json)
| Champ | registry.json | single-node.json | Drift |
|-------|---------------|------------------|-------|
| `runtime_artifacts` | `[kernel, node, mcp, lxc, cynic]` | `[kernel, node, cynic]` | mcp/lxc manquants (normal pour single-node) |
| `nodes` | Multi-node | Single node | Subset |
| `roles` | Full | Minimal | Subset |

---

## DRIFT-002: Runtime Proof — 2 Versions
- **Fichiers** :
  1. `infra/runtime-proof.v1` (300 chars) — court
  2. `infra/runtime_proof.v1` (2,058 chars) — détaillé
- **Drift** : Même schéma (`cynic.runtime_proof.v1`), contenu différent
- **Canonique** : `runtime_proof.v1` (underscore) = version détaillée, `runtime-proof.v1` = deprecated
- **Action** : Supprimer `runtime-proof.v1`, garder `runtime_proof.v1`
- **Status** : `observed`

---

## DRIFT-003: Agent Exposure — Projections vs Authority
- **Fichiers** :
  1. `infra/agent-exposure.json` (22,820 chars) — **authoritative** (normatif)
  2. `infra/agent-projections/antigravity-mcp.json` (983 chars) — **SUPPRIMÉ** (MCP dead)
  3. `infra/agent-projections/codex-mcp.json` (909 chars) — **SUPPRIMÉ** (MCP dead)
- **Drift** : Projections MCP référencées dans agent-exposure.json (maintenant nettoyé)
- **Action** : Vérifier que `projection_targets` = `{}` (fait)
- **Status** : `resolved` (MCP cleaned)

---

## DRIFT-004: Surface Registry — Fragments vs Manifest
- **Fichiers** :
  1. `infra/surface-registry/manifest.json` (4,970 chars) — index
  2. `infra/surface-registry/fragments/*.json` (30+ fichiers) — surfaces individuelles
- **Drift** : Manifest liste surfaces, fragments les définissent — pas de validation cohérence
- **Canonique** : Manifest généré depuis fragments, ou fragments validés contre manifest
- **Action** : Ajouter validation `cynic dev surface-registry --verify`
- **Status** : `observed`

---

## DRIFT-005: Vertical Intents — Owner Crate Drift
- **Fichiers** :
  1. `infra/verticals/lifecycle/intent.json` — `owner_crate: "cynic-node"` + `runtime_artifacts` sans mcp (fixé)
  2. `infra/verticals/agent_exposure/intent.json` — `owner_crate: "cynic-cli"` (fixé depuis cynic-mcp)
- **Drift** : `agent_exposure` owner était `cynic-mcp` → corrigé vers `cynic-cli`
- **Action** : Audit tous les verticals pour owner_crate cohérent
- **Status** : `partial_resolved`

---

## MISSING-001: Kernel HA Contract
- **Manquant** : `contracts/kernel_ha.json` ou `infra/kernel_ha.json`
- **Requis** : Leader election config, standby count, state sync policy, failover timeout
- **Actuel** : Rien — kernel assume single process
- **Priorité** : CRITIQUE (SPOF-001)

---

## MISSING-002: Storage HA Contract
- **Manquant** : `contracts/storage_ha.json`
- **Requis** : SurrealDB cluster config / etcd endpoints / replication factor / backup policy
- **Actuel** : `StorageConfig` dans kernel config seulement
- **Priorité** : CRITIQUE (SPOF-002)

---

## MISSING-003: Registry Writer Contract
- **Manquant** : `contracts/registry_writer.json`
- **Requis** : Principal owner, lease requirements, admission policy, audit log schema
- **Actuel** : Pas de contrat — n'importe qui peut écrire
- **Priorité** : CRITIQUE (OWN-001)

---

## MISSING-004: Network Actuation Delegation Contract
- **Manquant** : `contracts/network_delegation.json`
- **Requis** : Host→Node delegation schema, apply/rollback proof, privilege boundary
- **Actuel** : Implicite dans code (DUP-002)
- **Priorité** : HIGH (DUP-002 + OWN-007)

---

## MISSING-005: Agent Session Authority Contract
- **Manquant** : `contracts/agent_session_authority.json`
- **Requis** : Session depth rules, capability monotonicity, completion envelope schema
- **Actuel** : Partiel dans `cynic-protocol/src/agent_coordination.rs` + `agent_workload/`
- **Priorité** : HIGH (OWN-005)

---

## Validation Drift (script suggéré)

```bash
#!/bin/bash
# verify_config_drift.sh

echo "=== Registry Variants Drift ==="
jq -r '.runtime_artifacts[]' infra/registry.json | sort > /tmp/registry_artifacts.txt
jq -r '.runtime_artifacts[]' infra/registries/single-node.json | sort > /tmp/single_artifacts.txt
diff -u /tmp/registry_artifacts.txt /tmp/single_artifacts.txt

echo "=== Runtime Proof Versions ==="
diff -u infra/runtime-proof.v1 infra/runtime_proof.v1

echo "=== Vertical Owner Crates ==="
for f in infra/verticals/*/intent.json; do
    echo "$f: $(jq -r '.owner_crate' "$f")"
done

echo "=== Surface Registry Consistency ==="
jq -r '.surfaces[].id' infra/surface-registry/manifest.json | sort > /tmp/manifest_surfaces.txt
ls infra/surface-registry/fragments/*.json | xargs -I{} basename {} .json | sort > /tmp/fragment_surfaces.txt
diff -u /tmp/manifest_surfaces.txt /tmp/fragment_surfaces.txt
```

---

## Priorisation

| Entrée | Criticité | Effort | ROI | Ordre |
|--------|-----------|--------|-----|-------|
| DRIFT-001 (Registry variants) | HIGH | 2-3j | 8/10 | P1-1 |
| DRIFT-002 (Runtime proof) | LOW | 0.5j | 3/10 | P3-1 |
| DRIFT-004 (Surface registry) | MEDIUM | 1-2j | 5/10 | P2-1 |
| DRIFT-005 (Vertical intents) | MEDIUM | 1j | 4/10 | P2-2 |
| MISSING-001 (Kernel HA) | CRITIQUE | 2-3sem | 10/10 | P0-SPOF |
| MISSING-002 (Storage HA) | CRITIQUE | 1-2sem | 10/10 | P0-SPOF |
| MISSING-003 (Registry Writer) | CRITIQUE | 3-5j | 9/10 | P1-2 |
| MISSING-004 (Network Delegation) | HIGH | 3-5j | 7/10 | P2-3 |
| MISSING-005 (Agent Session) | HIGH | 1-2sem | 6/10 | P2-4 |