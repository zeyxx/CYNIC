# CYNIC Protocol Evolution Map

**Date:** 2026-09-22
**Status:** Design specification
**Authority:** Subordinate to `hermes/plans/CYNIC_CONSTITUTION_V2.md` §2.8 (Scaling Vertical)
**Research basis:** TCP/IP → QUIC evolution, PBFT → HotStuff → Tower BFT, B.A.T.M.A.N./OLSR mesh routing, SWIM/HyParView gossip protocols

---

## Design Principle

> **Each scaling stage emerges a NEW network protocol — not merely "more nodes" running the same protocol.**

Protocol evolution mirrors biological metamorphosis: the larval form (SEED) is structurally and functionally distinct from the adult (FOREST). Each stage introduces a new protocol stack appropriate to its failure model, trust boundary, and scale. Protocols from prior stages are **strangled** (deprecated, then removed) — not carried forward as baggage.

---

## Stage 1 — SEED (1 node, 1 agent)

### Topology
**Loopback star.** Single physical node, single agent process. All communication traverses the kernel loopback interface or Unix domain sockets. No network interface is trusted or required.

### Consensus
**Single-authority (no BFT).** The sole node is the sole authority. State transitions are admitted without quorum — the node's local decision is final. Fencing tokens are still issued (monotonic counter) to detect stale writers after recovery, but no quorum check is needed.

*Rationale:* At n=1, Byzantine fault tolerance is meaningless — there is no second party to disagree with. The failure model is "the node dies," not "the node lies."

### Transport
**Unix Domain Sockets (UDS) + shared memory.** Zero network overhead. Agent ↔ kernel communication via `SOCK_SEQPACKET` UDS with `SCM_CREDENTIALS` for peer authentication. State snapshots via `mmap`'d files.

*Rationale:* TCP/IP stack is unnecessary overhead for same-host communication. UDS provides kernel-enforced credentials (PID/UID/GID) without TLS.

### Discovery
**Self-declaration.** The node reads its identity from a local configuration file (`infra/network/topology.json` → self-entry). No network discovery. The agent declares itself to the kernel via `cynic_agent_identify`; the kernel issues a session token.

### Failure Model
| Failure | Behavior |
|---------|----------|
| Agent process crash | Kernel detects via UDS disconnect; claims expire via TTL |
| Node power loss | Total unavailability (expected — no redundancy) |
| State corruption | Detected via checksums; recovery from last valid snapshot |
| Stale writer after recovery | Fencing token mismatch → write refused |

### Stress Test Criteria
- [ ] **Process restart recovery:** Agent SIGKILL'd → kernel reclaims claims within TTL; agent restarts, re-identifies, re-claims.
- [ ] **Idempotent replay:** Same transaction submitted twice → state unchanged (idempotency key dedup).
- [ ] **Snapshot integrity:** Corrupted state file → detected via SHA-256 digest, falls back to last valid snapshot.
- [ ] **Fencing token monotonicity:** Recovered agent with stale token → write refused until token sync.

---

## Stage 2 — SPROUT (1 node, 3–5 agents)

### Topology
**Multi-process star.** Kernel as central hub, agents as spokes. All inter-agent coordination flows through the kernel (no agent-to-agent direct channels). The kernel is the single coordination authority.

### Consensus
**Local quorum via kernel arbitration.** The kernel acts as the coordination authority (existing `cynic-protocol/src/coordination_scaling_harness.rs`). Agents request claims; kernel grants or denies based on scope overlap, budget, and fencing tokens. No distributed consensus — the kernel is the SSOT.

*Rationale:* Multiple agents on one node still share a single trust domain (the kernel). The kernel's local decision IS consensus — there is no network to disagree over.

### Transport
**TCP+TLS over loopback.** Agents connect to kernel via `127.0.0.1` with mutual TLS (certificate per agent, signed by fleet CA). Encrypted even on loopback — defense in depth against local privilege escalation.

*Rationale:* TLS provides agent identity authentication (not just PID-based UDS credentials). This prepares for Stage 3 where agents may run on different nodes.

### Discovery
**Static registry + kernel-issued tokens.** Agents are declared in `infra/registry.json` (agent_id, capabilities, credentials). On startup, agent authenticates via mTLS → kernel issues session token. No dynamic discovery — all agents are known a priori.

### Failure Model
| Failure | Behavior |
|---------|----------|
| Agent crash | Heartbeat timeout → kernel expires claim, reclaims scope |
| Kernel crash | All agents lose coordination; agents enter read-only mode; kernel restarts, agents reconnect |
| Claim conflict | Two agents claim same scope → first-wins; second blocked with `ClaimConflict` |
| Budget exhaustion | Agent's budget depleted → all further mutations refused until new claim |
| TLS cert expiry | Agent cannot authenticate → `blocked_identity_invalid` |

### Stress Test Criteria
- [ ] **Agent crash recovery:** Agent A holds claim → SIGKILL → Agent B can claim same scope after TTL expiry.
- [ ] **Claim preemption:** Cynic-mode agent vs baseline-mode agent → Cynic preempts, baseline gets 300s grace.
- [ ] **Kernel restart:** Kernel SIGKILL'd → all agents detect disconnect → kernel restarts → agents re-authenticate and re-establish claims within 30s.
- [ ] **Budget monotonicity:** Parent claim budget = 100; children consume 60 + 50 → second child refused (would exceed parent).

---

## Stage 3 — SAPLING (3–7 nodes)

### Topology
**Full-mesh overlay.** All nodes are peers. WireGuard tunnel (`10.20.0.0/24`) provides encrypted full-mesh connectivity. Each node maintains a direct encrypted channel to every other node. Maximum 7 nodes → 21 pairwise tunnels (trivial for WireGuard).

*Rationale:* Full-mesh is optimal for small n — minimal latency, no routing protocol needed, every node has complete network knowledge. At n=7, O(n²) = 49 tunnel contexts is negligible.

### Consensus
**PBFT (Practical Byzantine Fault Tolerance).** 3f+1 tolerance: with 4 nodes, tolerate 1 Byzantine fault; with 7 nodes, tolerate 2. Three-phase protocol: PRE-PREPARE → PREPARE → COMMIT. View-change on leader failure.

*Rationale:* First stage requiring true BFT. PBFT is well-understood, proven for n ≤ ~20, and provides immediate finality (no probabilistic confirmation). The O(n²) message complexity is acceptable at n ≤ 7.

### Transport
**QUIC over WireGuard.** Each pairwise WireGuard tunnel carries QUIC connections. QUIC provides:
- Stream multiplexing (control channel + data channel on one connection)
- 0-RTT connection resumption (fast recovery after transient failure)
- Encrypted transport headers (prevents middlebox ossification)
- Connection migration (survives IP changes during NAT rebinding)

*Rationale:* QUIC replaces TCP for inter-node communication. The key innovation is **encrypted transport headers** — preventing the ossification that plagues TCP. Built atop WireGuard for defense-in-depth (double encryption: WireGuard for network-level, QUIC for transport-level).

### Discovery
**SWIM gossip + registry bootstrap.** Nodes bootstrap from `infra/network/topology.json` (known peers). After bootstrap, SWIM (Scalable Weakly-consistent Infection-style Membership) protocol maintains liveness:
- Each node probes k random peers per interval
- Direct probe (Ping/Ack) → suspect if timeout
- indirect probe (PingReq via third party) → confirm or clear suspicion
- Gossip disseminates membership changes (Alive/Suspect/Dead)

*Rationale:* SWIM provides decentralized failure detection with no single point of failure. At n ≤ 7, gossip converges in <2s. The suspicion mechanism prevents false positives from transient network jitter.

### Failure Model
| Failure | Behavior |
|---------|----------|
| Node crash | SWIM detects within 3s; PBFT view-change if crashed node was leader |
| Byzantine node (f ≤ 2) | PBFT tolerates; conflicting messages detected via quorum certificates |
| Network partition | Minority partition (≤ f nodes) → read-only; majority (≥ 2f+1) → continues |
| WireGuard tunnel failure | QUIC connection fails over to alternate path (if available) or marks peer unreachable |
| Leader crash | View-change protocol elects new leader; O(n³) message complexity acceptable at n ≤ 7 |

### Stress Test Criteria
- [ ] **Byzantine tolerance:** 1 node sends conflicting PRE-PREPARE messages → detected via quorum mismatch; node isolated.
- [ ] **Network partition:** 7-node cluster split 4/3 → majority (4) continues committing; minority (3) enters read-only.
- [ ] **Leader crash recovery:** Leader SIGKILL'd → view-change completes within 5s; new leader resumes consensus.
- [ ] **SWIM convergence:** Node joins cluster → all nodes aware within 3 gossip rounds (<10s).
- [ ] **QUIC 0-RTT:** Node restarts → resumes QUIC connections with 0-RTT data (no handshake latency).

---

## Stage 4 — TREE (7–50 nodes)

### Topology
**Hierarchical tree with gossip overlays.** Nodes organized into clusters of 7–10 nodes (PBFT clusters). Cluster heads form an upper-tier tree. Intra-cluster: full-mesh (PBFT). Inter-cluster: tree routing with cluster-head representatives.

```
         Cluster Head A
        /       |       \
   Cluster1  Cluster2  Cluster3
   (7 nodes) (7 nodes) (7 nodes)
```

*Rationale:* Full-mesh PBFT doesn't scale beyond ~20 nodes (O(n²) message complexity). Hierarchical decomposition bounds per-cluster size while allowing fleet-scale growth. Tree topology minimizes inter-cluster hops (O(log n) diameter).

### Consensus
**HotStuff-derived (linear communication, pipelined).** Each PBFT cluster runs HotStuff internally:
- Linear message complexity (O(n) per decision via threshold signatures)
- Pipelined: PREPARE of block N overlaps with PRE-COMMIT of block N-1
- Responsive: leader drives consensus at actual network speed (not max delay)
- Frequent leader rotation (every block) → censorship resistance

Cross-cluster consensus: **two-phase commit** coordinated by cluster heads. Cluster head proposes → peer cluster heads PREPARE → COMMIT if all agree.

*Rationale:* HotStuff's linear communication is essential at n=50. Pipelining maximizes throughput. Leader rotation prevents any single node from becoming a bottleneck or censorship point.

### Transport
**QUIC with priority channels.** Single QUIC connection per peer pair, multiplexed into:
- **Control channel (high priority):** Consensus messages, heartbeats, view-change
- **Data channel (normal priority):** State replication, evidence sync, gossip
- **Bulk channel (low priority):** Snapshot transfer, forensic logs

*Rationale:* QUIC's native stream multiplexing maps directly to priority separation. Control messages are never blocked by bulk data transfer. This prevents the head-of-line blocking that plagues TCP.

### Discovery
**HyParView (active + passive partial views).** Each node maintains:
- **Active view** (~log₂(n) + 1 ≈ 6–7 peers): Symmetric TCP connections for gossip transport
- **Passive view** (~6 × log₂(n) ≈ 36–42 peers): Candidate reservoir for replacing failed active links
- **Periodic shuffle:** Every 30s, exchange random peers between active/passive views to heal partitions

*Rationale:* HyParView solves the fundamental problem of random peer sampling — under churn, random views drift and partitions form. The active/passive separation ensures gossip transport (active) is always connected while the passive view provides repair candidates. At n=50, HyParView recovers from 90% node failure in ~4 rounds.

### Failure Model
| Failure | Behavior |
|---------|----------|
| Node crash (within cluster) | HotStuff view-change; HyParView repairs active view from passive reservoir |
| Cluster head failure | New cluster head elected via intra-cluster consensus; upper-tier tree re-rooted |
| Cluster partition | Each sub-cluster continues independently; merge on healing via anti-entropy |
| Leader rotation stall | Responsiveness guarantee: if leader doesn't advance, replicas timeout and rotate |
| Mass churn (up to 90%) | HyParView shuffle reconnects fragments; consensus resumes when quorum restored |
| Byzantine cluster head | Detected via cross-cluster quorum mismatch; head replaced via intra-cluster vote |

### Stress Test Criteria
- [ ] **Cluster reconfiguration:** Add/remove node to 7-node cluster → HotStuff adapts without downtime.
- [ ] **Leader rotation under load:** 50 nodes, 1000 TPS → leader rotates every block, no throughput drop >20%.
- [ ] **Cross-cluster atomicity:** Transaction spanning 3 clusters → either all commit or all abort (2PC).
- [ ] **Partition healing:** 50-node cluster splits 25/25 → each side continues; merge on reconnection via HyParView shuffle.
- [ ] **Byzantine cluster head:** 1 cluster head sends conflicting cross-cluster proposals → detected via quorum; head replaced within 10s.

---

## Stage 5 — FOREST (50+ nodes)

### Topology
**Mesh of clusters (B.A.T.M.A.N.-style originator flooding).** No single hierarchy. Clusters discover each other via originator message flooding:
- Each cluster periodically broadcasts its "originator message" (cluster ID, head ID, sequence number, capabilities)
- Nodes forward originator messages to neighbors, decrementing TTL
- Best path to each cluster determined by originator message count and link quality (ETX metric)

*Rationale:* At 50+ nodes, static hierarchy becomes a bottleneck and SPOF. B.A.T.M.A.N.'s distance-vector approach scales to hundreds of nodes without global routing tables. Each node only knows "the next hop toward cluster X" — not the full path.

### Consensus
**Tower BFT with Proof-of-History (PoH).** Derived from Solana's Tower BFT:
- **PoH (Proof of History):** Sequential SHA-256 hash chain encoding time into the ledger. Each hash incorporates the previous hash + counter. Provides a cryptographic clock — validators agree on event ordering without communication.
- **Tower BFT:** Each validator maintains a vote tower (stack of prior votes). Lockout doubles with each consecutive vote: 1→2→4→8→...→2³² slots. At 32 votes, maximum lockout (de facto finality).
- **Leader schedule:** Computed at epoch start; leaders rotate every 4 slots (≈1.6s at 400ms slots).

*Rationale:* Tower BFT reduces PBFT's O(n²) message complexity to O(n) by using PoH as a pre-consensus clock. Validators don't negotiate time — they verify it. The exponential lockout provides economic finality: a validator voting on a fork that loses faces exponentially increasing penalty (slashing).

### Transport
**QUIC + CYNIC pluggable transport.** QUIC as the base transport, extended with:
- **Pluggable congestion control:** Experiment with BBR, COPA, or custom CYNIC congestion control without kernel upgrades
- **Connection migration:** Nodes change IP (WiFi ↔ cellular, NAT rebinding) — connections survive via Connection ID
- **0-RTT session resumption:** New nodes join and immediately send data (no handshake latency)
- **Unreliable datagram extension (RFC 9221):** For gossip and consensus messages where retransmission is harmful (stale votes)

*Rationale:* QUIC's userspace implementation enables rapid protocol evolution — new congestion control algorithms deploy without OS upgrades. The unreliable datagram extension is critical for consensus: retransmitting a stale vote is worse than dropping it.

### Discovery
**Plumtree epidemic broadcast + DHT.** Two-layer discovery:
1. **Plumtree (epidemic broadcast):** Efficient broadcast to all nodes. Eager push (spanning tree) for first delivery + lazy push (IHAVE/IWANT) for redundancy. Achieves <500ms P50 broadcast latency.
2. **DHT (Distributed Hash Table):** Content-addressed discovery. Nodes and resources identified by cryptographic hash. `find_node(hash)` routes through the DHT to the responsible node.

*Rationale:* Plumtree provides efficient broadcast for consensus and governance messages. DHT provides scalable lookup for specific nodes/resources without flooding. Together: broadcast for "everyone needs to know," DHT for "I need to find X."

### Failure Model
| Failure | Behavior |
|---------|----------|
| Mass churn (90% node failure) | Remaining 10% with sufficient stake continue; Tower BFT lockout ensures finality of pre-crash state |
| Network partition | Each partition continues independently; on healing, longest-chain (highest lockout) wins; minority fork discarded |
| Byzantine minority (< n/3) | Tower BFT tolerates; conflicting votes detected via quorum; slashing penalizes equivocation |
| Leader absence | Skipped slots filled with virtual PoH ticks; next leader resumes; no stall |
| Cluster isolation | Isolated cluster continues; on reconnection, anti-entropy sync via Merkle tree comparison |
| Epoch boundary failure | New epoch leader schedule computed deterministically from stake weights; no single point of failure |

### Stress Test Criteria
- [ ] **90% node failure recovery:** 100-node cluster → 90 nodes SIGKILL'd → remaining 10% with sufficient stake continue producing blocks.
- [ ] **Partition healing:** 100-node cluster splits 60/40 → both sides continue; on reconnection, minority fork (40) discarded; state converges within 60s.
- [ ] **Finality under load:** 5000 TPS, 100 nodes → Tower BFT reaches 32-vote lockout (finality) within 60s.
- [ ] **Leader rotation at scale:** 100 nodes, leader rotates every 1.6s → no single leader handles >1% of total blocks per epoch.
- [ ] **Plumtree broadcast:** 100 nodes, 1MB broadcast message → all nodes receive within 2s (P99).
- [ ] **DHT lookup latency:** 100 nodes, random node lookup → <500ms (P50) via DHT routing.
- [ ] **Epoch transition:** 100 nodes, epoch boundary → new leader schedule computed and distributed within 1 slot (400ms).

---

## Evolution Summary Table

| Stage | Nodes | Topology | Consensus | Transport | Discovery | Failure Model | Stress Test |
|-------|-------|----------|-----------|-----------|-----------|---------------|-------------|
| **SEED** | 1 | Loopback star | Single-authority (no BFT) | Unix Domain Sockets + mmap | Self-declaration (local config) | Single point of failure (node death) | Process restart recovery, idempotent replay, snapshot integrity |
| **SPROUT** | 1 (3–5 agents) | Multi-process star | Local quorum (kernel arbitration) | TCP+TLS over loopback | Static registry + kernel tokens | Kernel SPOF, agent crash, claim conflict | Agent crash recovery, claim preemption, kernel restart, budget monotonicity |
| **SAPLING** | 3–7 | Full-mesh overlay | PBFT (3f+1, 3-phase) | QUIC over WireGuard | SWIM gossip + registry bootstrap | Byzantine (f ≤ 2), network partition, leader crash | Byzantine tolerance, partition (4/3), leader crash recovery, SWIM convergence, QUIC 0-RTT |
| **TREE** | 7–50 | Hierarchical tree + gossip clusters | HotStuff (linear, pipelined) + 2PC cross-cluster | QUIC with priority channels | HyParView (active/passive views) | Cluster head failure, mass churn (90%), Byzantine head | Cluster reconfig, leader rotation under load, cross-cluster atomicity, partition healing |
| **FOREST** | 50+ | Mesh of clusters (B.A.T.M.A.N.) | Tower BFT + PoH (exponential lockout finality) | QUIC + pluggable transport + unreliable datagrams | Plumtree broadcast + DHT | Mass churn (90%), partition, Byzantine minority, leader absence | 90% failure recovery, partition healing, finality under load, leader rotation at scale, broadcast/DHT latency |

---

## Protocol Strangling Rules

1. **Each stage's protocol is self-contained.** FOREST nodes do not run PBFT "for compatibility with SAPLING nodes." They run Tower BFT. Period.

2. **Transition is strangle, not big-bang.** When upgrading from SPROUT to SAPLING:
   - Deploy SAPLING protocol alongside SPROUT
   - Migrate agents one-by-one to SAPLING transport
   - Once all agents migrated, SPROUT protocol is deprecated (read-only)
   - After one release cycle with no SPROUT traffic, SPROUT code is removed

3. **Schema versioning is mandatory.** Every protocol message carries a `schema` field (e.g., `"cynic.consensus.hotstuff.v1"`). Receivers MUST reject unknown schemas. This enables protocol evolution without ambiguity.

4. **No protocol downgrade attacks.** Nodes track the minimum protocol version in the network. A node attempting to join with an older protocol version is rejected until it upgrades.

---

## Research References

| Domain | Reference | Key Insight Applied |
|--------|-----------|---------------------|
| Transport evolution | QUIC (RFC 9000, Langley et al. 2017) | Encrypted headers prevent ossification; stream multiplexing eliminates head-of-line blocking; 0-RTT for fast recovery |
| BFT evolution | HotStuff (Yin et al. 2019) | Linear message complexity via threshold signatures; pipelining for throughput; responsiveness to actual network speed |
| BFT evolution | Tower BFT (Solana) | PoH as pre-consensus clock; exponential lockout for economic finality; leader rotation for censorship resistance |
| Mesh routing | B.A.T.M.A.N. (open-mesh.org) | Distance-vector via originator messages; no global routing table; scales to hundreds of nodes |
| Mesh routing | OLSR (RFC 3626) | Multi-Point Relays for efficient flooding; link quality (ETX) metric for path selection |
| Gossip membership | SWIM (Das et al. 2002) | Decentralized failure detection; suspicion mechanism prevents false positives; gossip dissemination |
| Gossip membership | HyParView (Leitão et al. 2007) | Active/passive view separation; partition healing via shuffle; recovers from 90% failure |
| Epidemic broadcast | Plumtree (Leitão et al. 2007) | Eager/lazy hybrid; spanning tree for first delivery + IHAVE/IWANT for redundancy |

---

*This document is executable prose. Law lives in Rust types + tests + running code. Transpose, compile, test, merge.*
