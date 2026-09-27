# Audit C — Proof Inventory

**Date :** 2026-08-20  
**Scope :** Tous les types de preuve — schémas, producteurs, consumers, fenêtres temporelles  
**Authority :** Read-only audit, no code modified  
**Constitution ref :** §8 Proof Obligations

---

## Files audited

| File | Lines | Role |
|---|---|---|
| `crates/cynic-protocol/src/evidence.rs` | 368 | EvidenceEnvelope, EvidenceRef, EvidenceLineage — primitives de preuve de contenu-adressable |
| `crates/cynic-protocol/src/build_proof.rs` | 19 | BuildProofCard — preuve de build |
| `crates/cynic-protocol/src/runtime/proofs.rs` | 221 | RuntimeProofBundle, RuntimeProof, RuntimeProofPersistenceRecord |
| `crates/cynic-protocol/src/temporal.rs` | 306 | TemporalProof, Epoch — preuve temporelle liée à un epoch logique |
| `crates/cynic-protocol/src/harness.rs` | 717 | HarnessEvidence, HarnessCellReport, HarnessMatrixReport |
| `crates/cynic-host/src/runtime_harness.rs` | 433 | RuntimeHarnessTargetAttestation |
| `crates/cynic-protocol/src/convergence.rs` | ~600+ | ConvergenceMatrixReport |
| `crates/cynic-protocol/src/audit.rs` | ~600+ | AuditReportV2 |
| `crates/cynic-host/src/kernel_tunnel.rs` | ~250 | TunnelProof |
| `crates/cynic-host/src/source_sync/types.rs` | ~150 | SyncProof |
| `crates/cynic-host/src/network_actuation.rs` | ~200 | NetworkActuationProofReport |
| `crates/cynic-host/src/kernel_proof_persistence.rs` | ~150 | KernelProofPersistenceReport, KernelPersistenceCommandProof |
| `crates/cynic-host/src/source_worktree.rs` | ~100 | SourceWorktreeCommandProof |
| `crates/cynic-protocol/src/agent_workload/workspace_types.rs` | ~150 | AgentWorkspaceApplyReport |
| `crates/cynic-protocol/src/runtime_takeover_activation.rs` | ~50 | RuntimeTakeoverAdmissionProof |
| `crates/cynic-protocol/src/federation_envelopes.rs` | ~150 | FederatedMeshProbeReport |
| `crates/cynic-protocol/src/node_activation.rs` | ~150 | ActivationProof |
| `crates/cynic-protocol/src/source_control.rs` | ~250 | SourceAdmissionReport |
| `crates/cynic-protocol/src/source_repository_bootstrap.rs` | ~100 | SourceRepositoryBootstrapReport |
| `crates/cynic-protocol/src/policy.rs` | ~150 | PolicyAdmissionReport |
| `crates/cynic-protocol/src/transport_security.rs` | ~150 | TransportSecurityReport |
| `crates/cynic-protocol/src/fleet_discovery.rs` | ~100 | FleetDiscoveryReport |
| `crates/cynic-protocol/src/outcomes.rs` | ~100 | KpiScoreReport |
| `crates/cynic-protocol/src/autoresearch.rs` | ~200 | AutoresearchReport |
| `crates/cynic-askesis/src/agent_loop.rs` | 422 | AgentLoopAuditReport |

---

## Questions & Answers

### 1. How many distinct proof types exist?

**Réponse : Au moins 28 types de preuve/rapports distincts.**

**Inventaire exhaustif :**

| # | Type | Crate/Module | Schema constant | Has time window |
|---|---|---|---|---|
| 1 | `BuildProofCard` | cynic-protocol/build_proof | `cynic.build_proof_card.v1` | Non (timestamp only) |
| 2 | `RuntimeProofBundle` | cynic-protocol/runtime/proofs | `cynic.runtime_proof_bundle.v1` | Via child proofs |
| 3 | `RuntimeProof` | cynic-protocol/runtime/proofs | `cinscription.runtime_proof.v1` | Oui (`observed_at_unix_ms`) |
| 4 | `RuntimeProofPersistenceRecord` | cynic-protocol/runtime/proofs | `cynic.runtime_proof_persistence.v1` | Non |
| 5 | `InferenceStatusProof` | cynic-protocol/runtime/proofs | `cynic.inference_status_proof.v1` | Non (vu les lignes 83+) |
| 6 | `TemporalProof` | cynic-protocol/temporal | `cynic.temporal.proof.v1` | Oui (epoch-based) |
| 7 | `EvidenceEnvelope` | cynic-protocol/evidence | `cynic.evidence.envelope.v1` | Oui (`observed_at_unix_ms`) |
| 8 | `HarnessEvidence` | cynic-protocol/harness | Aucun schema string | Non |
| 9 | `HarnessCellReport` | cynic-protocol/harness | `cynic.build.harness_cell_report.v1` | Non |
| 10 | `HarnessMatrixReport` | cynic-protocol/harness | `cynic.build.harness_matrix.v1` | Non |
| 11 | `RuntimeHarnessTargetAttestation` | cynic-host/runtime_harness | `cynic.runtime_harness.target_attestation.v2` | Oui (`observed_at`, `expires_at`) |
| 12 | `ConvergenceMatrixReport` | cynic-protocol/convergence | Dérivé | Non |
| 13 | `AuditReportV2` | cynic-protocol/audit | `cynic.audit_report.v2` | Non |
| 14 | `TunnelProof` | cynic-host/kernel_tunnel | Aucun champ schema visible | Non (`healthy: bool` only) |
| 15 | `SyncProof` | cynic-host/source_sync | Dérivé | Non |
| 16 | `NetworkActuationProofReport` | cynic-host/network_actuation | Dérivé | Non |
| 17 | `KernelProofPersistenceReport` | cynic-host/kernel_proof_persistence | Dérivé | Non |
| 18 | `SourceWorktreeCommandProof` | cynic-host/source_worktree | Dérivé | Non |
| 19 | `AgentWorkspaceApplyReport` | cynic-protocol/agent_workload | Dérivé | Non |
| 20 | `RuntimeTakeoverAdmissionProof` | cynic-protocol/runtime_takeover | Dérivé | Non |
| 21 | `FederatedMeshProbeReport` | cynic-protocol/federation | Dérivé | Non |
| 22 | `ActivationProof` | cynic-protocol/node_activation | Enum — schema dans chaque vari | Non |
| 23 | `SourceAdmissionReport` | cynic-protocol/source_control | Dérivé | Non |
| 24 | `SourceRepositoryBootstrapReport` | cynic-protocol/source_repo_bootstrap | Dérivé | Non |
| 25 | `PolicyAdmissionReport` | cynic-protocol/policy | Dérivé | Non |
| 26 | `TransportSecurityReport` | cynic-protocol/transport_security | Dérivé | Non |
| 27 | `FleetDiscoveryReport` | cynic-protocol/fleet_discovery | Dérivé | Non |
| 28 | `AgentLoopAuditReport` | cynic-askesis/agent_loop | `cynic.agent-learning-loop-audit.v2` | Non |

---

### 2. Which proofs have a schema string and which don't?

**Avec schema string explicite (sécurisé pour l'identification) :**
- BuildProofCard (`cynic.build_proof_card.v1`)
- RuntimeProofBundle (`cynic.runtime_proof_bundle.v1`)
- RuntimeProof (`cynic.runtime_proof.v1`)
- RuntimeProofPersistenceRecord (`cynic.runtime_proof_persistence.v1`)
- InferenceStatusProof (`cynic.inference_status_proof.v1`)
- TemporalProof (`cynic.temporal.proof.v1`)
- EvidenceEnvelope (`cynic.evidence.envelope.v1`)
- HarnessCellReport (`cynic.build.harness_cell_report.v1`)
- HarnessMatrixReport (`cynic.build.harness_matrix.v1`)
- RuntimeHarnessTargetAttestation (`cynic.runtime_harness.target_attestation.v2`)
- AgentLoopAuditReport (`cynic.agent-learning-loop-audit.v2`)
- AuditReportV2 (`cynic.audit_report.v2`)

**Sans schema string explicite (identification fragile) :**
- `HarnessEvidence` — a `reference: String` mais pas de champ `schema`
- `TunnelProof` — structure opaque avec `healthy: bool`, pas de schema
- `SyncProof`, `NetworkActuationProofReport`, `KernelProofPersistenceReport`, etc. — leurs schemas sont dérivés du nom de type ou d'un autre champ, pas d'une constante `schema`

**Conséquence :** Les preuves sans schema string ne peuvent pas être identifiées de manière fiable par un consumer externe. Si tu reçois un JSON, impossible de savoir si c'est un `TunnelProof` ou un autre type sans conventions supplémentaires.

---

### 3. Which proofs carry a time window and which are timeless?

**Avec fenêtre temporelle vérifiable (bornes observed/expiry) :**
- `RuntimeProof` — `observed_at_unix_ms: u128`
- `TemporalProof` — `epoch` + `observed_at_epoch`
- `RuntimeHarnessTargetAttestation` — `observed_at_unix_ms` + `expires_at_unix_ms` + validation d'expiration
- `EvidenceEnvelope` — `observed_at_unix_ms: u128`

**Sans fenêtre temporelle (timeless ou approximatif) :**
- `BuildProofCard` — `timestamp: String` (ISO8601), pas de champ `expires_at`
- `HarnessEvidence` — aucun champ temporel
- `HarnessCellReport` / `HarnessMatrixReport` — aucun champ temporel
- `TunnelProof` — aucun champ temporel (un instantané `healthy`)
- `SyncProof` — aucun champ temporel visible
- `AgentLoopAuditReport` — aucun champ temporel

**Conséquence :** Les preuves timeless ne peuvent pas être invalidées par le temps. Un `HarnessCellReport` produit il y a 6 mois est considéré comme valide aujourd'hui. C'est un problème pour la conformance — une preuve de conformation expire, même si le code n'a pas changé.

---

### 4. Is there any proof that is consumed by a higher layer but produced by the same layer (self-attestation) ?

**Réponse : Oui, potentiellement dans le build scheduler.**

**Cas suspect :** `cynic-node/src/usecases/build_scheduler/mod.rs:165-172`  
Le scheduler appelle `persist_success_proofs()` puis vérifie `expected_proofs.all(|proof| evidence_refs.contains(proof))`. Le scheduler est à la fois le producteur (via persist) et le vérificateur (via la boucle de complétion).

**Cas suspect :** `cynic-host/src/kernel_proof_persistence.rs`  
Le kernel persiste ses propres preuves et les rapporte via `KernelProofPersistenceReport`. Le producteur et le consumer sont dans le même module kernel.

**Cas légitime mais à surveiller :** `cynic-host/src/runtime_metrics.rs`  
Les métriques sont produites par le host et consommées par le même host pour le rapport. Mais ce n'est pas une preuve de mutation — c'est une observation.

**Gap :** Pas de règle explicite "une preuve ne doit pas être auto-attestée". La constitution §8 dit "produite par la couche la plus basse qui possède le fait", mais aucune vérification statique n'empêche un module de produire et consommer sa propre preuve.

---

### 5. Can any proof type be extended without breaking existing consumers (forward-compat) ?

**Réponse : Non, pas de manière uniforme.**

**Preuves forward-compatibles (probables) :**
- `EvidenceEnvelope` — utilise `#[serde(default)]` sur optionnels, `#[serde(flatten)]` possible
- `RuntimeProofBundle` — contient `Vec<RuntimeProof>`, extension par ajout de preuve possible
- Les rapports avec `#[serde(deny_unknown_fields)]` sont **cassants** pour les consumers

**Preuves non-forward-compatibles (cassantes) :**
- Les enums `#[serde(tag = "status", rename_all = "snake_case")]` comme `LeaseMutationOutcome` — ajouter une variante casse les consumers qui n'ont pas `#[serde(other)]`
- Les structs avec `#[serde(deny_unknown_fields)]` — rejettent tout champ inconnu
- Les `HarnessCellReport` / `HarnessMatrixReport` — leur structure est fixe, pas de mécanisme d'extension

**Gap :** Pas de politique de schema evolution unifiée. Certaines preuves peuvent être étendues (EvidenceEnvelope), d'autres cassent les consumers (enums sans `Unknown`). Pas de guideline dans AGENTS.md sur la forward-compat des preuves.

---

## Summary of gaps

| # | Gap | Severity | Constitution ref |
|---|---|---|---|
| 1 | 28+ types de preuve distincts, sans registry unifié | Medium | §8 Proof Obligations |
| 2 | 12+ types sans schema string explicite | High | §8 Proof Obligations |
| 3 | 15+ types timeless (sans fenêtre temporelle) | Medium | §8 Proof Obligations |
| 4 | Build scheduler produit et consomme ses propres preuves (auto-attestation) | High | §8 Proof Obligations |
| 5 | Pas de politique forward-compat unifiée | Medium | §8 Proof Obligations |
| 6 | Pas de type `ProofEnvelope` unifié pour envelopper n'importe quelle preuve | Medium | §8 Proof Obligations |

---

## Recommendations for U4 (Proof Registry)

1. Créer un `ProofEnvelope` unifié dans `cynic-protocol` avec : `schema`, `produced_at_unix_ms`, `producer_id`, `evidence_refs`, `payload: Value`
2. Exiger un `schema` string sur chaque type de preuve (refuser les types sans schema)
3. Ajouter une fenêtre temporelle (`observed_at_unix_ms`, `expires_at_unix_ms: Option<u64>`) à tous les rapports de conformance (pas aux preuves instantanées comme TunnelProof)
4. Interdire l'auto-attestation dans le build scheduler — séparer le producteur (observer) du consumer (scheduler)
5. Adopter une règle de schema evolution : jamais retirer un champ, toujours `#[serde(default)]` sur les nouveaux champs, `#[serde(other)]` sur les nouvelles variantes d'enum
6. Créer un registry des types de preuve (catalogue) pour que le meta-harness puisse les agréger sans matching exhaustif
