# CYNIC Net — Native Network Protocol Design

**Status**: `research` — Architectural specification, not yet implemented.
**Schema**: `cynic.network_protocol.v1`
**Author**: CYNIC Subagent (network protocol design)
**Date**: 2026-09-22
**Sources**: Nym mixnet, Tor, Lightning Network (BOLT 4/Sphinx), libp2p (Kademlia, DCUtR), WireGuard (Noise), KIRA distributed routing, Solana PoH.

---

## 1. Design Principles

| Principle | Rationale |
|-----------|-----------|
| **Sovereign by default** | No central directory authority, no SaaS control plane. All authority flows from operator-owned substrate. |
| **Privacy as a service** | Metadata protection (who, when, how much) is the product. Relay operators sell it; users buy it. |
| **Pay-per-use** | BTC/LN micropayments align incentives. Free-riding is economically irrational. |
| **Censorship resistance** | Pluggable transports (TCP/UDP/BT/radio) make the network hard to kill. |
| **Fail-closed on ambiguity** | Unknown routing state → drop, never guess. Dispute marks, not silent repair. |
| **Proof over assertion** | Every routing table update carries a PoH-attested epoch fingerprint and ≥2/3 quorum signatures. |

---

## 2. Layer Architecture

| Layer | Name | Mechanism | Incentive | Consensus | Attack Resistance |
|-------|------|-----------|-----------|-----------|------------------|
| **L7** Application | CYNIC Agent Bus | Typed messages over tunnel, bincode serialization | N/A (end-to-end) | Agent session protocol | Capability leases, bounded budgets |
| **L6** Presentation | Sphinx packets | Layered encryption (X25519 + ChaChaPoly), 5-hop circuits, constant 1366-byte cells | Per-hop HTLC fees | None (onion secrecy) | IND-CPA, unlinkability via blinded ephemeral keys |
| **L5** Session | Circuit lifecycle | Circuit build/teardown, PoH heartbeat, replay protection via nonce | Stake bonding for relay reputation | Epoch quorum (≥2/3) | Sybil: stake-weighted selection; DoS: rate limits |
| **L4** Transport | Multiplexed streams | QUIC (primary), TCP fallback, UDP hole-punch signaling | Bandwidth pricing per byte forwarded | None (transport-layer) | Connection migration, 0-RTT resumption |
| **L3** Network | Distributed routing | Kademlia DHT + source routing + PathID forwarding (KIRA-style) | DHT storage rewards (per epoch) | **PoH epoch fingerprint** + 2/3 quorum | Eclipse: k-bucket diversity; Poisoning: signed descriptors |
| **L2** Link | Peer-to-peer tunnels | WireGuard (Noise_IK handshake), cryptokey routing per peer | Stake-weighted relay selection | Fleet consensus evaluator | Mutual auth, forward secrecy, identity hiding |
| **L1** Physical | Pluggable transport | Ethernet/WiFi (QUIC), BLE L2CAP (mesh), LoRa/AX.25 (long-range, store-and-forward) | Radio operators earn per-packet relayed | Regional mesh gossip | Jamming resistance via FHSS on radio |

---

## 3. Onion Routing (Sphinx Protocol)

### 3.1 Circuit Topology

```
Client → [Entry Gateway] → [Mix L1] → [Mix L2] → [Mix L3] → [Exit] → Destination
   ↑           ↑              ↑           ↑           ↑         ↑
  Knows     Knows next     Knows next   Knows next  Knows next  Knows
  full      hop only       hop only     hop only    hop only    dest
  route
```

- **5 hops minimum** (configurable, 3+ recommended for mix privacy)
- **Entry Gateway**: Knows client IP, not destination. Bonded relay, high reputation.
- **Mix Layers**: Re-order, delay, pad packets. Earn per-packet fees.
- **Exit**: Knows destination, not client. Subject to legal pressure — incentivized via higher fees.

### 3.2 Sphinx Packet Format

```
┌──────────────────────────────────────────────────────────┐
│  Version (1B)  │  Ephemeral Pubkey (33B, X25519)         │
├──────────────────────────────────────────────────────────┤
│  Hop 1 Routing Info + HMAC (encrypted for Entry GW)      │
│  Hop 2 Routing Info + HMAC (encrypted for Mix L1)        │
│  Hop 3 Routing Info + HMAC (encrypted for Mix L2)        │
│  Hop 4 Routing Info + HMAC (encrypted for Mix L3)        │
│  Hop 5 Routing Info + HMAC (encrypted for Exit)          │
│  Payload (padded to 1300B total per packet)              │
├──────────────────────────────────────────────────────────┤
│  HMAC-SHA256 (32B) — integrity over full packet          │
└──────────────────────────────────────────────────────────┘
Total: 1366 bytes (constant — no positional information leak)
```

### 3.3 Key Derivation Per Hop

1. Client generates ephemeral keypair `(eph_priv, eph_pub)`
2. For each hop `i`: `shared_secret_i = X25519(eph_priv, hop_i_static_pub)`
3. `blinding_factor_i = SHA256(eph_pub || shared_secret_i)`
4. Next ephemeral: `eph_priv_{i+1} = eph_priv_i * blinding_factor_i mod n`
5. Derive keys via HKDF: `rho_i` (payload encryption), `hmac_i` (integrity), `filler_i` (padding)

Each hop re-blinds the ephemeral key for its successor. Successive nodes cannot correlate packets by key value.

### 3.4 Per-Hop Processing

```
Receive packet:
  1. Compute shared_secret = X25519(my_static_priv, packet.eph_pub)
  2. Derive rho_key, hmac_key via HKDF(shared_secret)
  3. Verify HMAC over (version || eph_pub || hops_data) → drop if invalid
  4. Decrypt my layer → extract (next_hop_addr, my_hmac_tag, instructions)
  5. Shift hops_data left by my layer size
  6. Generate filler bytes via PRF(shared_secret) → pad to 1300B
  7. Blind eph_pub for next hop
  8. Recompute HMAC for outgoing packet
  9. Forward to next_hop_addr
```

---

## 4. Economic Incentives (BTC/LN)

### 4.1 Micropayment Model: HTLC-per-Hop

Each circuit carries a chain of Hash Time-Locked Contracts (HTLCs), one per relay hop:

```
Client creates:  payment_hash = SHA256(payment_preimage)

Circuit payment chain:
  Client ──HTLC(amount+fees*4)──→ Entry GW
                                      │ HTLC(amount+fees*3)
                                      ▼
                                   Mix L1
                                      │ HTLC(amount+fees*2)
                                      ▼
                                   Mix L2
                                      │ HTLC(amount+fees*1)
                                      ▼
                                   Mix L3
                                      │ HTLC(amount)
                                      ▼
                                    Exit
```

- **Fee structure**: Each hop charges `base_fee + fee_rate * amount`. Exit charges premium.
- **Settlement**: When Exit receives the preimage, it claims its HTLC. Each hop upstream claims its own HTLC using the same preimage, propagating backward.
- **Timeouts**: Each HTLC has decreasing `cltv_expiry` (e.g., Entry GW: +100 blocks, Exit: +20 blocks). Late settlement → stake slashing.

### 4.2 Stake-Bonded Reputation

Relays bond satoshis into on-chain timelock contracts to signal capacity:

| Stake Tier | Bond (sats) | Selection Weight | Epoch Reward Share |
|------------|-------------|------------------|---------------------|
| Probationary | 10k | 1x | 1x |
| Standard | 100k | 5x | 3x |
| Trusted | 1M | 25x | 10x |
| Backbone | 10M | 100x | 50x |

**Selection probability** ∝ `bond * uptime_score * latency_score * capacity_score`

**Slashing conditions**:
- Misrouting (correlatable packet modification)
- Extended downtime (>50% of epoch)
- Invalid PoH attestation
- Eclipse attack participation (proven by witness attestations)

### 4.3 Epoch-Based Reward Distribution

Inspired by Nym mixmining, adapted for BTC/LN:

- **Epoch duration**: ~60 minutes (PoH-timed)
- **Active set**: Top N relays by reputation score selected per epoch
- **Standby set**: Next M relays (incentivize spare capacity)
- **Reward pool**: Transaction fees from that epoch + small inflation subsidy (bootstrapping reserve)
- **Reward split**: 80% operator / 20% delegators (stake pool participants)

---

## 5. PoH-Based Consensus for Routing Table

### 5.1 Problem

Tor uses 9 hardcoded directory authorities — a central trust anchor. CYNIC has none. How do all nodes agree on the set of active relays without a central source?

### 5.2 Solution: VDF-Based Epoch Consensus

Each epoch, nodes attest to the routing table state using Proof of History:

```
Epoch N consensus flow:
  1. Leader (rotating, stake-weighted lottery) proposes routing_table_root = Merkle root of all relay descriptors
  2. Each relay computes VDF(input=prev_epoch_fingerprint || routing_table_root) → epoch_proof
  3. Relays broadcast: NodeEpochAttestation { epoch_n, routing_table_root, epoch_proof, vdf_output }
  4. Collect attestations until ≥2/3 of stake-weighted supermajority agrees
  5. Finalize epoch: epoch_fingerprint = SHA256(N || routing_table_root || epoch_proof || quorum_bitfield)
```

### 5.3 Epoch Fingerprint Computation

```
compute_epoch_fingerprint(epoch_n, routing_table_root, binary_sha256, stake_merkle_root):
  SHA256(epoch_n.to_be_bytes()
       || routing_table_root
       || binary_sha256
       || stake_merkle_root)
```

This binds:
- **Routing table** (which relays are active)
- **Software version** (fork detection)
- **Stake distribution** (who can validate)

### 5.4 Consensus States

| State | Condition | Action |
|-------|-----------|--------|
| `Synchronized` | ≥2/3 agree on epoch fingerprint | Normal operation |
| `DegradedClockSkew` | Nodes exceed max skew (e.g., 5s) | Flag skewed nodes, warn operator |
| `Partitioned` | Multiple epoch branches detected | Quorum intersection analysis, healing protocol |
| `Transitioning` | <2/3 but converging | Retry, increase timeout |
| `Fork` | Different binary_sha256 across quorum | Halt, alert — possible software divergence |

### 5.5 Comparison to Existing CYNIC Consensus

The existing `cynic-consensus` crate (`evaluate_fleet_consensus`) already implements Solana-style Tower BFT with ≥2/3 quorum. CYNIC Net reuses this evaluator but adds:
- VDF-based timestamping (PoH) to prevent long-range attacks
- Merkle root of routing table (not just binary SHA)
- Stake-weighted voting (not 1-node-1-vote)

---

## 6. NAT Traversal

### 6.1 Strategy: libp2p DCUtR + Relay Fallback

| Step | Mechanism | Success Rate |
|------|-----------|--------------|
| 1 | STUN/AutoNAT for public address discovery | ~95% (cone NAT) |
| 2 | DCUtR hole punching via relay synchronization | 83% (port-restricted cone), 40% (symmetric) |
| 3 | Circuit relay (TURN-style) fallback | ~100% (via relay) |
| 4 | QUIC connection migration after punch | Maintained across IP changes |

### 6.2 DCUtR Protocol

```
Peer A (NAT'd) ←──relay──→ Relay ←──relay──→ Peer B (public)

  1. Both peers connected to Relay
  2. A requests hole punch to B via Relay
  3. Relay tells B: "A wants to punch, here's A's observed public addr"
  4. Relay tells A: "B is ready, here's B's addr, punch now"
  5. A and B simultaneously send UDP packets to each other's public addr
  6. NAT pinholes open (transient inbound rules)
  7. Direct QUIC connection forms, relay drops signaling role
  8. Connection migrates to direct path; relay notified to clean up
```

### 6.3 ICE-Like Transport Negotiation

Over the DHT, peers exchange `TransportOffer`:
```json
{
  "peer_id": "...",
  "transports": ["quic", "tcp", "webrtc"],
  "public_addrs": ["1.2.3.4:2428", "[2001::1]:2428"],
  "nat_type": "port_restricted_cone",
  "wireguard_pubkey": "...",
  "preferred_transport": "quic"
}
```

Selection priority: QUIC > TCP > WebRTC > relay.

---

## 7. Pluggable Transport

### 7.1 Transport Abstraction

```rust
pub trait PluggableTransport: Send + Sync {
    async fn dial(&self, addr: &Multiaddr) -> Result<Box<dyn SecureStream>, TransportError>;
    async fn listen(&self, addr: &Multiaddr) -> Result<Box<dyn SecureListener>, TransportError>;
    fn protocols(&self) -> Vec<&'static str>;
}
```

### 7.2 Transport Implementations

| Transport | Encrypted | Multiplexed | NAT Traverse | Use Case |
|-----------|-----------|-------------|--------------|----------|
| QUIC | Yes (TLS 1.3) | Yes | Yes (DCUtR) | Default, internet |
| TCP+Noise | Yes | No (needs stream mux) | Partial | Legacy fallback |
| WebRTC | Yes (DTLS) | Yes | Yes (ICE) | Browser/Electron |
| BLE L2CAP | Yes (AES-CCM) | No | N/A (broadcast) | Local mesh, IoT |
| LoRa/AX.25 | Optional (AES-CTR) | No | N/A | Long-range, rural |
| Radio (FSK) | Optional | No | N/A | Disaster, protest |

### 7.3 Transport Selection

Client and relay negotiate via multiaddr:
```
/ip4/1.2.3.4/udp/2428/quic
/ip6/2001::1/tcp/2428
/l2cap/aa:bb:cc:dd:ee:ff
/lora/915MHz/10.4kHz
```

First common protocol wins. Falls back to relay if direct transport unavailable.

---

## 8. Decentralized Directory (No Central Authorities)

### 8.1 Kademlia DHT for Relay Discovery

The DHT stores three record types:

| Key | Value | TTL |
|-----|-------|-----|
| `SHA256(relay_id)` | RelayDescriptor { pubkey, addrs, stake_txid, fee_policy, uptime_proof } | 24h |
| `SHA256("active_set:" + epoch)` | List of top-N relay IDs | epoch duration |
| `SHA256("bootstrap")` | List of long-lived seed nodes | Permanent |

### 8.2 Relay Descriptor

```json
{
  "schema": "cynic.relay_descriptor.v1",
  "relay_id": "ed25519_pubkey_hex",
  "public_key_x25519": "onion_curve_pubkey_hex",
  "addresses": ["/ip4/1.2.3.4/udp/2428/quic", "/onion3/.../tcp/443"],
  "stake": {
    "txid": "...",
    "vout": 0,
    "amount_sats": 100000,
    "lock_until_block": 900000,
    "slash_conditions": ["invalid_attestation", "downtime_50pct"]
  },
  "fee_policy": {
    "base_fee_msat": 1000,
    "fee_rate_millionths": 100,
    "min_htlc_msat": 1000,
    "max_cltv_delta": 2016
  },
  "capacity_mbps": 1000,
  "software_version": "cynic-net 0.1.0",
  "epoch_attestation": { "epoch": 1234, "fingerprint": "...", "proof": "..." },
  "signature": "ed25519_sig_over_all_above"
}
```

### 8.3 Bootstrap Mechanism

Phase 0 (initial): 3-5 long-lived seed nodes (operated by CYNIC maintainers), hardcoded in binary.
Phase 1 (growth): Seeds dissolved after 6 months; discovery via gossip + DHT only.
Phase 2 (mature): New nodes find peers via DHT bootstrap query to well-known relay IDs.

---

## 9. Attack Resistance

| Attack | Mitigation | Mechanism |
|--------|------------|-----------|
| **Sybil** | Stake bonding | Each relay must lock satoshis. Adversary pays linear cost per identity. |
| **Eclipse** | DHT diversity + k-bucket constraints | Routing table entries from diverse /16 prefixes. Random XOR-distance queries. |
| **Traffic analysis** | Poisson timing + cover traffic + constant-size packets | Sphinx 1366B cells. Dummy packets injected at 10-20% rate. |
| **Routing table poisoning** | ≥2/3 quorum + epoch fingerprinting | Invalid descriptor = no attestation = no selection. |
| **DoS** | Per-peer rate limiting + reputation decay | Sliding window; low-rep peers deprioritized. |
| **Man-in-the-middle** | Noise mutual auth + Sphinx HMAC | Static key pairs, no certificates. HMAC over full packet. |
| **Timing correlation** | Mixing latency (50-200ms per hop) | Batch processing, re-ordering within batch. |
| **Exit snooping** | End-to-end TLS (inside tunnel) | Exit sees only ciphertext to destination. |
| **Long-range attack** | VDF-based epoch sequencing | Can't retro-compute VDF; epoch history is immutable. |
| **Censorship** | Pluggable transports (radio/BLE) | Block internet traffic, still have local mesh. |

---

## 10. Integration with CYNIC Runtime

### 10.1 Crate Layout

```
crates/
  cynic-net/           ← New: CYNIC Net core
    src/
      lib.rs           ← Re-exports
      sphinx.rs        ← Sphinx packet construct/parse
      circuit.rs       ← Circuit lifecycle manager
      onion.rs         ← Onion key derivation
      routing/         ← L3 distributed routing
        mod.rs
        dht.rs         ← Kademlia DHT for relay discovery
        consensus.rs   ← PoH epoch consensus
        source_route.rs← PathID-based forwarding (KIRA-style)
      transport/       ← L1 pluggable transports
        mod.rs
        quic.rs
        tcp.rs
        ble.rs
        radio.rs
      incentive/       ← BTC/LN micropayments
        mod.rs
        htlc.rs
        stake.rs
        reputation.rs
      nat_traversal.rs ← DCUtR + ICE
```

### 10.2 Dependencies

```toml
[dependencies]
# Existing CYNIC
cynic-protocol = { path = "../cynic-protocol" }
cynic-consensus = { path = "../cynic-consensus" }

# Crypto
x25519-dalek = "2"
chacha20poly1305 = "0.10"
sha2 = "0.10"
hkdf = "0.12"

# Networking
quinn = "0.11"           ← QUIC
libp2p = "0.54"          ← Kademlia, DCUtR, noise, multiplex

# Bitcoin/LN
bitcoin = "0.32"
lightning = "0.0.125"    ← LDK for HTLC management

# Serialization
bincode = "1"
serde = "1"
```

### 10.3 Integration Points

| Existing CYNIC Component | How CYNIC Net Uses It |
|--------------------------|----------------------|
| `cynic-consensus` | `evaluate_fleet_consensus` becomes the epoch quorum checker. PoH attestation is a new `NodeEpochAttestation` variant. |
| `cynic-protocol::network` | `NetworkMeshPlan`, `MeshProviderBinding` extended with `CynicNetRole::OnionRelay`. |
| `cynic-agent-bus` | Agent messages tunneled over Sphinx circuits for private cross-node communication. |
| `cynic-network` | `NetworkDiscoveryProbePort` extended with `probe_onion_latency()` for relay quality measurement. |

---

## 11. Comparison to Existing Systems

| Feature | Tor | Nym | Lightning | CYNIC Net |
|---------|-----|-----|-----------|------------|
| Onion routing | Yes (3 hops) | Yes (5 hops, mixnet) | Yes (Sphinx, LN) | Yes (5 hops, Sphinx) |
| Incentives | Volunteer only | NYM token | BTC/LN | **BTC/LN (native)** |
| Consensus | Central directory authorities | NyxD validators (Cosmos SDK) | None (gossip) | **PoH + 2/3 quorum (no central authority)** |
| NAT traversal | Pluggable transports | Gateway relay | Channel peer discovery | **DCUtR + relay fallback** |
| Pluggable transport | Yes (obfs4, snowflake) | No (fixed) | No | **Yes (TCP/UDP/BT/radio)** |
| Stake/sybil resistance | None | NYM bond | Channel capacity | **BTC timelock bond** |
| Privacy model | Circuit-based | Mixnet (timing + cover) | Source-routed payments | **Mixnet + source-routed HTLCs** |

---

## 12. Open Work / Risks

| Item | Status | Risk |
|------|--------|------|
| VDF implementation (PoH) | Needs research (RSA-group vs. Wesolowski/Pietrzak) | VDF ASIC advantage could centralize epoch leadership |
| HTLC-per-hop fee market | Needs simulation | Fee volatility may price out small users |
| BLE/LoRa transport | Needs hardware testing | Regulatory (radio spectrum), low bandwidth |
| Stake slashing on Bitcoin | Needs OP_CTV or relay-enforced covenant | Limited script expressiveness; may need federated slashing |
| Epoch leader rotation | Needs stake-weighted VRF lottery | Long-range attack on leader selection |
| Cover traffic overhead | Needs tuning | 10-20% overhead may deter cost-sensitive users |

---

## 13. Next Steps (Implementation Order)

1. **Sphinx crate** — Packet construct/parse + unit tests (pure crypto, no network)
2. **DHT relay discovery** — libp2p Kademlia adapter, relay descriptor storage
3. **Circuit build** — 5-hop circuit over mock transport (testnet)
4. **HTLC incentive** — LDK integration, per-hop payment channel
5. **PoH epoch consensus** — Extend `cynic-consensus` with VDF, epoch fingerprint
6. **NAT traversal** — DCUtR + QUIC + relay fallback
7. **Pluggable transports** — TCP fallback, BLE, radio (hardware-dependent)
8. **Integration** — Agent bus over Sphinx circuits, full CYNIC Net stack
9. **Mainnet launch** — Bootstrap with seed nodes, dissolve after 6 months

---

*End of design. Ready for review, critique, and implementation.*
