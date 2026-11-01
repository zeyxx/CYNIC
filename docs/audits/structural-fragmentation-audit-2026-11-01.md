# Structural Fragmentation Audit — CYNIC

Date: 2026-11-01
Scope: crates/* (excluding target and .superpowers rescues)
Focus: misplaced adapters, primitive obsession, fragmented stores/transports, duplicate domain concepts.

## 1. Misplaced Adapters Masquerading as Use Cases

### 1.1 HTTP server in `usecases/`
- Location: `crates/cynic-node/src/usecases/builder_http.rs`
- Flaw: Axum router, handlers (`/health`, `/webhook/forgejo`, `/exec`, `/admin/*`), filesystem reads, HMAC verification, process control (`std::process::exit`) all live in a `usecases/` module. This is an adapter (imperative shell), not a use case. Violates FCIS/Hexagonal: use cases must be pure orchestration; I/O belongs in `adapters/`.
- Proposal: move to `crates/cynic-node/src/adapters/builder_http.rs` (or `services/builder_http.rs`) and keep only pure decision helpers (`submit_action_to_canonical`, `decide_builder_restart`, `shape_*`) in `usecases/` or `domain/`.

### 1.2 I/O inside other `usecases/`
Files in `crates/cynic-node/src/usecases/` importing `std::fs`, `std::process::Command`, `tokio::fs`, `axum`, `reqwest`:
- `build_pull.rs`, `build_scheduler/prepare.rs`, `build_scheduler/execute.rs`, `build_scheduler/mod.rs`, `build_admission.rs`, `storage_lifecycle_uc.rs`, `source_admission_uc.rs`, `node_enrollment_lifecycle_uc.rs`, `external_project_audit_uc.rs`, `network_security_audit_uc.rs`, `observe_node_uc.rs`, `federation_membership_uc.rs`, `network_sync_uc.rs`, `node_network_recovery_uc.rs`, `federated_mesh_resolver.rs`, `coordination_observation_bridge.rs`, `notification_outbox_uc.rs`, `builder_http.rs`.
- Violates Zero-I/O Core / Hexagonal: use cases must call ports; filesystem/process/network side effects must be behind adapters.
- Proposal: split each into pure use-case orchestrator + adapter; inject ports; move direct I/O to `adapters/`.

## 2. Primitive Obsession & Stringly-Typed Protocols

### 2.1 Build submission action/role as `String`
- Location: `crates/cynic-node/src/usecases/builder_http.rs` `AgentBuildSubmission.action: String`, `desired_role: "build_runner".into()`, `AgentBuildSubmission` fields; `submit_action_to_canonical(&str)`.
- Violates DDD: actions/roles should be typed enums in `cynic-protocol`/`cynic-build`, not raw strings mapped at runtime.
- Proposal: define `BuildAction` and `BuildRole` enums (serde rename) in `cynic-build`/`cynic-protocol`; parse at boundary; remove string mapping.

### 2.2 HTTP boundary DTOs duplicated
- Location: `ExecResponse` in both `crates/cynic-build/src/adapters/http.rs` and `crates/cynic-node/src/usecases/builder_http.rs`; `ForgejoPayload`/`ForgejoCommit` in both `crates/cynic-vcs` and `cynic-node`.
- Proposal: canonicalize transport DTOs in `cynic-node-protocol` or `cynic-protocol`; adapters reuse.

### 2.3 Lifecycle kinds as substring matching
- Location: `builder_http.rs` `evidence_lifecycle_kind` matches substrings in strings.
- Proposal: typed `EvidenceKind` enum on `EvidenceEnvelope`; no stringly matching.

## 3. Fragmented Stores & Transports

### 3.1 Artifact transport vs publication
- Ports: `RuntimeArtifactTransportPort` (`crates/cynic-node/src/ports/runtime_artifact_transport_port.rs`) and `ReleaseArtifactPublicationPort` (`crates/cynic-node/src/ports/release_publication_port.rs`).
- Adapters: `http_runtime_artifact_transport.rs`, `http_release_artifact_publication.rs`, plus `builder_client`/`HttpExternalBuildAdapter`.
- Overlap: both move artifacts over HTTP and compute digests; separate ports/dtos cause drift.
- Proposal: unify under one `ArtifactTransportPort` with operation enum (Stage/Publish) or a shared `HttpArtifactTransport` adapter; keep release-specific signing separate.

### 3.2 Evidence stores
- `LocalContentAddressedEvidenceStore` in `cynic-host` is used directly by `cynic-node` builder HTTP (`state.evidence`).
- Multiple evidence store ports (`ContentAddressedEvidenceStorePort`) vs direct concrete type.
- Proposal: depend only on port trait; remove direct concrete store in AppState; provide via DI.

### 3.3 Builder client fragmentation
- `crates/cynic-build/src/builder_client.rs` (resolve_builder_url_with_health) and `HttpExternalBuildAdapter` and builder HTTP `/exec`.
- Proposal: consolidate builder resolution + transport in one adapter module.

## 4. Duplicate Domain Concepts Across Crates

The following concepts are defined in multiple crates and should be canonicalized (prefer `cynic-protocol` or `cynic-types` as SSOT; others become type aliases/re-exports):

- `AgentBudgets`: `cynic-cli/src/mask/types.rs`, `cynic-protocol/src/agent_citizenship.rs`
- `AgentSessionRecord`: `cynic-protocol/src/agent_session/types.rs`, `cynic-surreal/src/domain/session.rs`
- `AllowanceGrant`: `cynic-consensus/src/peer/accounts.rs`, `cynic-protocol/src/allowance.rs`
- `AuditSummary`: `cynic-cli/src/node_host_audit.rs`, `cynic-protocol/src/audit.rs`
- `Capability`: `cynic-discovery/src/types.rs`, `cynic-protocol/src/federation_authz.rs`, `cynic-protocol/src/privilege/catalog.rs`
- `Claim`/`ClaimStatus`: `cynic-audit-engine`, `cynic-knowledge`, `cynic-protocol`
- `CommandOutput`: `cynic-node/src/ports/build_runner_adapter.rs`, `cynic-release/src/external_deploy/deploy.rs`
- `Credential`/`CredentialType`: `cynic-host/src/credential.rs`, `cynic-vcs/src/ports.rs`
- `CynicPaths`: `cynic-knowledge/src/adapters/rust_ast.rs`, `cynic-types/src/paths.rs`
- `DiscoveredService`: `cynic-inference/src/ods_discovery.rs`, `cynic-network/src/topology/discovery.rs`
- `EndpointObservation`: `cynic-host/src/endpoint_observation.rs`, `cynic-vcs/src/ports.rs`
- `ExecResponse`: `cynic-build/src/adapters/http.rs`, `cynic-node/src/usecases/builder_http.rs`
- `ForgejoCommit`/`ForgejoPayload`: `cynic-vcs`, `cynic-node`
- `HeadscaleNode`: `cynic-network`, `cynic-node`
- `HostExecRequest`: `cynic-host`, `cynic-node`
- `MemorySecretProvider`: `cynic-domain-ops`, `cynic-secrets`
- `NetworkActuator*`: `cynic-network`, `cynic-node`
- `NetworkPathEvidence`/`TransportScope`/`TransportSecurityReport`: `cynic-network`, `cynic-protocol`
- `NetworkPolicy`: 4 crates
- `Node`/`Registry*`: many
- `Observation`: `cynic-cli`, `cynic-inference`, `cynic-protocol`
- `PeerState`: `cynic-consensus`, `cynic-node-protocol`
- `PlacementDecision`: `cynic-build`, `cynic-protocol`
- `RegistryLifecycle`/`RegistryNetworkPolicy`/`RegistrySource*`: `cynic-host`, `cynic-node`
- `RuntimeBasket*`: `cynic-host`, `cynic-node`, `cynic-protocol`
- `SourceAdmissionReport`/`SourceChange*`/`SourceMatterClass`/`SourcePromotionStage`: `cynic-protocol`, `cynic-vcs`
- `StorageDecision`/`StorageObservation`: `cynic-node`, `cynic-proxmox`
- `TransportSecurity*`: `cynic-network`, `cynic-protocol`

Full list: 91 duplicate struct/enum names across distinct crates (generated by audit script). Priority: concepts appearing in both L0 (`cynic-protocol`/`cynic-types`) and higher crates should be hoisted to L0 and re-exported.

## 5. Recommended Unification Plan

1. Move `builder_http.rs` adapter out of `usecases/`; keep pure helpers.
2. Introduce typed `BuildAction`, `BuildRole`, `EvidenceKind` in protocol/build; parse at boundaries.
3. Unify artifact transport port + HTTP adapter; inject evidence store via port.
4. Hoist duplicate concepts to `cynic-protocol`/`cynic-types`; replace duplicates with re-exports.
5. Add architecture boundary tests to prevent regression (I/O in usecases, duplicate definitions).

## 6. Verification
- `make audit-crate` / `cynic audit all` should be run after refactors.
- No code changes made by this audit (report only).
