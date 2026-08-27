# Test & Proof Gaps Registry — CYNIC

**Critère** : Manque de tests unitaires, tests d'intégration, read-back proof, validation gates
**Référence** : AGENTS.md "Execution Standard" + "Runtime Harness" + "Validation"

---

## GAP-001: Registry Writer — Aucun Test de Preuve
- **Fichier** : `cynic-host/src/registry.rs` (fonctions `write_registry*`)
- **Manquant** :
  - Test unitaire : write atomique + read-back vérifie contenu
  - Test intégration : concurrent writes + lease admission
  - Proof gate : `cynic dev registry write --verify` produit proof JSON
- **Criticité** : CRITIQUE (OWN-001 + SPOF-004)
- **Effort** : 2-3j
- **Status** : `observed`

---

## GAP-002: Kernel HA — Zero Tests
- **Composant** : Leader election, hot standby, state sync (n'existent pas)
- **Manquant** : Tout — pas de contrat, pas d'impl, pas de tests
- **Criticité** : CRITIQUE (SPOF-001)
- **Effort** : 2-3sem (avec impl)
- **Status** : `observed`

---

## GAP-003: Storage HA — Zero Tests
- **Composant** : SurrealDB cluster / etcd / replication (n'existent pas)
- **Manquant** : Tout
- **Criticité** : CRITIQUE (SPOF-002)
- **Effort** : 1-2sem (avec impl)
- **Status** : `observed`

---

## GAP-004: Network Actuation — Pas de Read-Back Proof
- **Fichiers** : `cynic-host/src/network_actuation.rs`, `cynic-node/src/network_mesh_apply_plan.rs`
- **Manquant** :
  - Preflight → plan → apply → **observe post-apply** → compare desired vs observed
  - Rollback test : apply fails → rollback → observe = pre-state
  - Proof JSON émis à chaque étape
- **Criticité** : HIGH (DUP-002 + OWN-007)
- **Effort** : 3-5j
- **Status** : `observed`

---

## GAP-005: Disk Observation — Pas de Fleet Proof
- **Fichiers** : `cynic-host/src/fleet_disk_observer.rs`
- **Manquant** :
  - Test : multi-node disk report → fleet rollup → proof cohérence
  - Test : node down → fleet report marque `unknown` pas `healthy`
  - Proof schema : `cynic.fleet_disk.v1` validation
- **Criticité** : MEDIUM
- **Effort** : 2-3j
- **Status** : `observed`

---

## GAP-006: Agent Session/Workload — Pas de Test Completion Envelope
- **Fichiers** : `cynic-protocol/src/agent_workload/`, `cynic-kernel/src/bootstrap/agent_session.rs`
- **Manquant** :
  - Test : delegation → execution → completion envelope émis
  - Test : budget enforcement (actions, wall_time, child_count)
  - Test : parent failure → child recovery / fence
  - Proof : `cynic.agent_completion_envelope.v1` validation
- **Criticité** : HIGH (OWN-005)
- **Effort** : 1-2sem
- **Status** : `observed`

---

## GAP-007: Federation Runtime — Pas de Test Emergence State
- **Fichier** : `cynic-host/src/federation_runtime.rs` (fix récent `emergence_state`)
- **Manquant** :
  - Test : `MeshPeerObserver::observe_mesh()` produit `EmergenceState` valide
  - Test : peer count → role distribution → phase computation
  - Test : network partition → emergence state degrade gracieusement
- **Criticité** : MEDIUM (fix récent non testé)
- **Effort** : 1-2j
- **Status** : `observed`

---

## GAP-008: Peer Resolver — Tests Incomplets
- **Fichier** : `cynic-protocol/src/peer_resolver.rs` + `cynic-host/src/peer_resolver_mesh.rs`
- **Manquant** :
  - Test : `resolve_peers_filtered` avec divers filtres (role, health, region)
  - Test : `SignedPeerEndpoint` signature verification (valid/invalid/expired)
  - Test : `fallback_peers()` return vec![] (fail-closed) — **déjà fixé mais pas testé**
- **Criticité** : MEDIUM
- **Effort** : 1-2j
- **Status** : `observed`

---

## GAP-009: Health Signal — Pas de Test Classification/Threshold
- **Fichier** : `cynic-protocol/src/health_signal.rs` (1466 lignes)
- **Manquant** :
  - Test : `classify_signal()` pour chaque `SignalKind`
  - Test : `evaluate_thresholds()` boundaries (warn/critical/emergency)
  - Test : `aggregate_signals()` multi-node → fleet health
  - Property test : thresholds monotonic, no gaps
- **Criticité** : MEDIUM (MONO-016)
- **Effort** : 2-3j
- **Status** : `observed`

---

## GAP-010: Runtime Harness — Pas de Conformance Suite
- **Manquant** : `cynic runtime harness` executable contract
- **Requis** (AGENTS.md) :
  - `kind`: lab/conformance/staging/canary/production
  - `authority`: test/conformance/rc/canary/prod
  - `isolation_boundary`: sandbox/VM/staging/canary/prod
  - `target_scope`: host/node/fleet
  - `mutates_host_network`: bool
- **Actuel** : `cynic runtime harness --json` existe mais vide/partiel
- **Criticité** : HIGH (foundation pour tout test HA)
- **Effort** : 1-2sem
- **Status** : `observed`

---

## GAP-011: Bootstrap Chain — Pas de Test Intégrité
- **Fichier** : `cynic-kernel/src/infra/boot/seed_integrity_chain.rs` (si existe)
- **Manquant** :
  - Test : seed valide → chain verified = true
  - Test : seed corrompu → chain verified = false + proof
  - Test : multi-seed quorum validation (n'existe pas)
- **Criticité** : MEDIUM (SPOF-005)
- **Effort** : 1-2j
- **Status** : `observed`

---

## GAP-012: Inference Router — Pas de Test Failover [RÉSOLU]
- **Fichier** : `cynic-inference/src/application/routing_policy.rs` + `cynic-kernel/src/api/rest/inference_router.rs`
- **Statut** : `RÉSOLU`. Tests unitaires implémentés et validés :
  - `test_primary_model_down_fallback_selected` : bascule automatique si le modèle primaire est indisponible / critique
  - `test_gpu_oom_cpu_fallback_selected` : bascule vers le profil CPU en cas de contrainte VRAM / OOM
  - `test_tool_use_constraint_respected_during_failover` : respect strict des contraintes de capacités pendant le failover
- **Criticité** : RÉSOLU (tests automatisés)
---

## Pattern Test Requis (AGENTS.md)

```rust
// 1. Unit test — pure function
#[test]
fn test_classify_signal_cpu_pressure() {
    let signal = HealthSignal::cpu_pressure(0.95);
    assert_eq!(classify_signal(&signal), Severity::Critical);
}

// 2. Integration test — port + adapter
#[tokio::test]
async fn test_disk_observation_local_adapter() {
    let adapter = LocalDiskAdapter::new();
    let report = adapter.observe_disk().await.unwrap();
    assert!(report.mounts.iter().any(|m| m.available > 0));
    // Read-back proof
    let proof = verify_disk_report(&report).await.unwrap();
    assert!(proof.verified);
}

// 3. Read-back proof test — desired → observe → reconcile → apply → observe → verify
#[tokio::test]
async fn test_network_apply_readback() {
    let desired = NetworkPlan { ... };
    let pre = observe_network().await.unwrap();
    let plan = reconcile_network(&desired, &pre).unwrap();
    apply_network(&plan).await.unwrap();
    let post = observe_network().await.unwrap();
    let proof = verify_network_convergence(&desired, &post).await.unwrap();
    assert!(proof.converged);
}

// 4. Harness test — conformance gate
#[tokio::test]
async fn test_conformance_single_node() {
    let harness = RuntimeHarness::lab().await.unwrap();
    let report = harness.run_conformance().await.unwrap();
    assert!(report.all_gates_passed());
}
```

---

## Couverture Actuelle (estimation)

| Crate | Unit Tests | Integration Tests | Read-Back Proof Tests | Harness Tests |
|-------|------------|-------------------|----------------------|---------------|
| cynic-protocol | ~60% | ~10% | 0% | 0% |
| cynic-host | ~30% | ~20% | ~5% | 0% |
| cynic-node | ~20% | ~10% | 0% | 0% |
| cynic-kernel | ~10% | ~5% | 0% | 0% |
| cynic-cli | ~5% | 0% | 0% | 0% |
| cynic-inference | ~15% | ~5% | 0% | 0% |

---

## Priorisation

| Entrée | Criticité | Effort | ROI | Ordre |
|--------|-----------|--------|-----|-------|
| GAP-001 (Registry Writer Proof) | CRITIQUE | 2-3j | 10/10 | P1-1 |
| GAP-004 (Network Actuation Proof) | HIGH | 3-5j | 8/10 | P1-2 |
| GAP-006 (Agent Completion Envelope) | HIGH | 1-2sem | 7/10 | P2-1 |
| GAP-007 (Emergence State Test) | MEDIUM | 1-2j | 6/10 | P2-2 |
| GAP-008 (Peer Resolver Tests) | MEDIUM | 1-2j | 6/10 | P2-3 |
| GAP-009 (Health Signal Tests) | MEDIUM | 2-3j | 5/10 | P2-4 |
| GAP-005 (Fleet Disk Proof) | MEDIUM | 2-3j | 5/10 | P3-1 |
| GAP-010 (Runtime Harness) | HIGH | 1-2sem | 9/10 | P0-FOUNDATION |
| GAP-011 (Bootstrap Chain Test) | MEDIUM | 1-2j | 4/10 | P3-2 |
| GAP-012 (Inference Failover Test) | MEDIUM | 2-3j | 4/10 | P3-3 |
| GAP-002/003 (HA Tests) | CRITIQUE | avec impl | - | P0-SPOF |