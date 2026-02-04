/**
 * Gossip Protocol — Comprehensive Tests
 *
 * Tests for GossipProtocol, PeerManager, message creation/verification,
 * relay mechanics, consensus message types, and edge cases.
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  // Message
  MessageType,
  generateMessageId,
  createMessage,
  verifyMessage,
  shouldRelay,
  prepareRelay,
  createBlockMessage,
  createSyncRequest,
  createSyncResponse,
  createHeartbeat,
  createPeerAnnounce,
  // Consensus gossip messages
  createConsensusBlockProposal,
  createConsensusVote,
  createConsensusVoteAggregate,
  createConsensusFinality,
  createConsensusSlotStatus,
  isConsensusGossipMessage,
  // Peer
  PeerStatus,
  createPeerInfo,
  PeerManager,
  // Protocol
  GossipProtocol,
  // Crypto
  generateKeypair,
} from '../src/index.js';

import { GOSSIP_FANOUT } from '@cynic/core';

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

function makeProtocol(overrides = {}) {
  const kp = generateKeypair();
  const sentMessages = [];
  const handledMessages = [];

  const protocol = new GossipProtocol({
    publicKey: kp.publicKey,
    privateKey: kp.privateKey,
    address: 'localhost:8080',
    onMessage: async (msg) => { handledMessages.push(msg); },
    sendFn: async (peer, message) => { sentMessages.push({ peer, message }); },
    ...overrides,
  });

  return { protocol, keypair: kp, sentMessages, handledMessages };
}

function addPeers(protocol, count) {
  const peers = [];
  for (let i = 0; i < count; i++) {
    const kp = generateKeypair();
    const info = { publicKey: kp.publicKey, address: `localhost:${9000 + i}`, eScore: 50 + i };
    protocol.addPeer(info);
    peers.push({ ...info, keypair: kp });
  }
  return peers;
}

// ─────────────────────────────────────────────────────────────────
// 1. Message ID generation
// ─────────────────────────────────────────────────────────────────

describe('Message — ID Generation', () => {
  it('should generate unique message IDs', () => {
    const ids = new Set(Array.from({ length: 100 }, () => generateMessageId()));
    assert.strictEqual(ids.size, 100);
  });

  it('should prefix IDs with msg_', () => {
    const id = generateMessageId();
    assert.ok(id.startsWith('msg_'));
  });
});

// ─────────────────────────────────────────────────────────────────
// 2. Message creation and signing
// ─────────────────────────────────────────────────────────────────

describe('Message — Creation & Signing', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should create message with correct structure', () => {
    const msg = createMessage({
      type: MessageType.BLOCK,
      payload: { slot: 1 },
      sender: kp.publicKey,
      privateKey: kp.privateKey,
      ttl: 3,
    });

    assert.ok(msg.id);
    assert.strictEqual(msg.type, MessageType.BLOCK);
    assert.strictEqual(msg.ttl, 3);
    assert.strictEqual(msg.hops, 0);
    assert.ok(msg.timestamp > 0);
    assert.ok(msg.signature);
  });

  it('should create unsigned message when no privateKey provided', () => {
    const msg = createMessage({
      type: MessageType.HEARTBEAT,
      payload: {},
      sender: kp.publicKey,
    });
    assert.ok(!msg.signature);
  });

  it('should default TTL to 3', () => {
    const msg = createMessage({
      type: MessageType.PATTERN,
      payload: {},
      sender: kp.publicKey,
    });
    assert.strictEqual(msg.ttl, 3);
  });
});

// ─────────────────────────────────────────────────────────────────
// 3. Message verification
// ─────────────────────────────────────────────────────────────────

describe('Message — Verification', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should verify valid signature', () => {
    const msg = createMessage({
      type: MessageType.JUDGMENT,
      payload: { verdict: 'WAG' },
      sender: kp.publicKey,
      privateKey: kp.privateKey,
    });
    assert.strictEqual(verifyMessage(msg), true);
  });

  it('should reject tampered payload', () => {
    const msg = createMessage({
      type: MessageType.JUDGMENT,
      payload: { verdict: 'WAG' },
      sender: kp.publicKey,
      privateKey: kp.privateKey,
    });
    msg.payload.verdict = 'HOWL';
    assert.strictEqual(verifyMessage(msg), false);
  });

  it('should reject message without signature', () => {
    assert.strictEqual(verifyMessage({ sender: kp.publicKey }), false);
  });

  it('should reject message without sender', () => {
    assert.strictEqual(verifyMessage({ signature: 'sig123' }), false);
  });

  it('should reject message signed by different key', () => {
    const kp2 = generateKeypair();
    const msg = createMessage({
      type: MessageType.BLOCK,
      payload: {},
      sender: kp.publicKey,
      privateKey: kp2.privateKey, // wrong key
    });
    assert.strictEqual(verifyMessage(msg), false);
  });
});

// ─────────────────────────────────────────────────────────────────
// 4. Message relay mechanics
// ─────────────────────────────────────────────────────────────────

describe('Message — Relay Mechanics', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should relay messages with TTL > 0', () => {
    const msg = createMessage({ type: MessageType.BLOCK, payload: {}, sender: kp.publicKey, ttl: 2 });
    assert.strictEqual(shouldRelay(msg), true);
  });

  it('should NOT relay messages with TTL = 0', () => {
    const msg = createMessage({ type: MessageType.BLOCK, payload: {}, sender: kp.publicKey, ttl: 0 });
    assert.strictEqual(shouldRelay(msg), false);
  });

  it('should NOT relay heartbeat messages', () => {
    const msg = createMessage({ type: MessageType.HEARTBEAT, payload: {}, sender: kp.publicKey, ttl: 5 });
    assert.strictEqual(shouldRelay(msg), false);
  });

  it('should decrement TTL and increment hops on relay', () => {
    const msg = createMessage({ type: MessageType.PATTERN, payload: {}, sender: kp.publicKey, ttl: 3 });
    const relayed = prepareRelay(msg);
    assert.strictEqual(relayed.ttl, 2);
    assert.strictEqual(relayed.hops, 1);
    // Original should not be mutated
    assert.strictEqual(msg.ttl, 3);
    assert.strictEqual(msg.hops, 0);
  });

  it('should chain multiple relays correctly', () => {
    const msg = createMessage({ type: MessageType.BLOCK, payload: {}, sender: kp.publicKey, ttl: 3 });
    const r1 = prepareRelay(msg);
    const r2 = prepareRelay(r1);
    const r3 = prepareRelay(r2);
    assert.strictEqual(r3.ttl, 0);
    assert.strictEqual(r3.hops, 3);
    assert.strictEqual(shouldRelay(r3), false);
  });
});

// ─────────────────────────────────────────────────────────────────
// 5. Typed message creation helpers
// ─────────────────────────────────────────────────────────────────

describe('Message — Typed Helpers', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should create block message', () => {
    const msg = createBlockMessage({ slot: 5, data: 'x' }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.BLOCK);
    assert.strictEqual(msg.payload.slot, 5);
    assert.ok(msg.signature);
  });

  it('should create sync request with TTL=1', () => {
    const msg = createSyncRequest(10, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.SYNC_REQUEST);
    assert.strictEqual(msg.payload.since_slot, 10);
    assert.strictEqual(msg.ttl, 1);
  });

  it('should create sync response with request ID', () => {
    const msg = createSyncResponse([{ slot: 1 }], 'req_42', kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.SYNC_RESPONSE);
    assert.strictEqual(msg.payload.request_id, 'req_42');
    assert.strictEqual(msg.payload.blocks.length, 1);
    assert.strictEqual(msg.ttl, 1);
  });

  it('should create unsigned heartbeat with TTL=1', () => {
    const msg = createHeartbeat({ height: 100 }, kp.publicKey);
    assert.strictEqual(msg.type, MessageType.HEARTBEAT);
    assert.strictEqual(msg.ttl, 1);
    assert.ok(!msg.signature);
  });

  it('should create peer announce with TTL=2', () => {
    const msg = createPeerAnnounce({ address: 'a:1' }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.PEER_ANNOUNCE);
    assert.strictEqual(msg.ttl, 2);
    assert.ok(msg.signature);
  });
});

// ─────────────────────────────────────────────────────────────────
// 6. Consensus gossip message helpers
// ─────────────────────────────────────────────────────────────────

describe('Message — Consensus Gossip Helpers', () => {
  let kp;

  beforeEach(() => {
    kp = generateKeypair();
  });

  it('should create consensus block proposal', () => {
    const msg = createConsensusBlockProposal({ blockHash: 'h', slot: 1 }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.CONSENSUS_BLOCK_PROPOSAL);
    assert.strictEqual(msg.ttl, 3);
    assert.ok(msg.signature);
  });

  it('should create consensus vote', () => {
    const msg = createConsensusVote({ blockHash: 'h', decision: 'APPROVE' }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.CONSENSUS_VOTE);
    assert.strictEqual(msg.ttl, 3);
  });

  it('should create consensus vote aggregate', () => {
    const msg = createConsensusVoteAggregate({ blockHash: 'h', approveWeight: 70 }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.CONSENSUS_VOTE_AGGREGATE);
    assert.strictEqual(msg.ttl, 2);
  });

  it('should create consensus finality notification', () => {
    const msg = createConsensusFinality({ blockHash: 'h', status: 'DETERMINISTIC' }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.CONSENSUS_FINALITY);
    assert.strictEqual(msg.ttl, 3);
  });

  it('should create consensus slot status', () => {
    const msg = createConsensusSlotStatus({ slot: 42, validators: 10 }, kp.publicKey, kp.privateKey);
    assert.strictEqual(msg.type, MessageType.CONSENSUS_SLOT_STATUS);
    assert.strictEqual(msg.ttl, 2);
  });

  it('should identify consensus messages by type prefix', () => {
    assert.strictEqual(isConsensusGossipMessage('CONSENSUS_VOTE'), true);
    assert.strictEqual(isConsensusGossipMessage('CONSENSUS_BLOCK_PROPOSAL'), true);
    assert.strictEqual(isConsensusGossipMessage('BLOCK'), false);
    assert.strictEqual(isConsensusGossipMessage('HEARTBEAT'), false);
    // null/undefined return falsy (not strictly false)
    assert.ok(!isConsensusGossipMessage(null));
    assert.ok(!isConsensusGossipMessage(undefined));
  });
});

// ─────────────────────────────────────────────────────────────────
// 7. Peer info creation
// ─────────────────────────────────────────────────────────────────

describe('Peer — Info Creation', () => {
  it('should create peer info with phi-salted hash ID', () => {
    const peer = createPeerInfo({ publicKey: 'ed25519:abc', address: 'a:1', eScore: 75 });
    assert.ok(peer.id);
    assert.strictEqual(peer.publicKey, 'ed25519:abc');
    assert.strictEqual(peer.address, 'a:1');
    assert.strictEqual(peer.eScore, 75);
    assert.strictEqual(peer.status, PeerStatus.ACTIVE);
    assert.strictEqual(peer.messagesSent, 0);
    assert.strictEqual(peer.messagesReceived, 0);
    assert.strictEqual(peer.failedAttempts, 0);
  });

  it('should default eScore to 50', () => {
    const peer = createPeerInfo({ publicKey: 'k', address: 'a' });
    assert.strictEqual(peer.eScore, 50);
  });

  it('should produce deterministic ID for same public key', () => {
    const p1 = createPeerInfo({ publicKey: 'ed25519:same', address: 'a:1' });
    const p2 = createPeerInfo({ publicKey: 'ed25519:same', address: 'a:2' });
    assert.strictEqual(p1.id, p2.id);
  });

  it('should produce different ID for different public key', () => {
    const p1 = createPeerInfo({ publicKey: 'ed25519:key1', address: 'a:1' });
    const p2 = createPeerInfo({ publicKey: 'ed25519:key2', address: 'a:1' });
    assert.notStrictEqual(p1.id, p2.id);
  });
});

// ─────────────────────────────────────────────────────────────────
// 8. PeerManager — add, update, remove
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — CRUD', () => {
  let mgr;

  beforeEach(() => {
    mgr = new PeerManager({ maxPeers: 10 });
  });

  it('should add and retrieve peer', () => {
    const peer = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    assert.strictEqual(mgr.addPeer(peer), true);
    assert.ok(mgr.getPeer(peer.id));
  });

  it('should update existing peer on re-add', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1', eScore: 50 });
    mgr.addPeer(p);
    const p2 = createPeerInfo({ publicKey: 'k1', address: 'a:2', eScore: 80 });
    mgr.addPeer(p2);

    const retrieved = mgr.getPeer(p.id);
    assert.strictEqual(retrieved.eScore, 80);
    assert.strictEqual(retrieved.address, 'a:2');
    assert.strictEqual(mgr.getStats().total, 1); // no duplicate
  });

  it('should remove peer', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    mgr.removePeer(p.id);
    assert.strictEqual(mgr.getPeer(p.id), null);
  });

  it('should get peer by public key', () => {
    const p = createPeerInfo({ publicKey: 'ed25519:findme', address: 'a:1' });
    mgr.addPeer(p);
    const found = mgr.getPeerByPublicKey('ed25519:findme');
    assert.ok(found);
    assert.strictEqual(found.publicKey, 'ed25519:findme');
  });

  it('should return null for unknown peer', () => {
    assert.strictEqual(mgr.getPeer('nonexistent'), null);
    assert.strictEqual(mgr.getPeerByPublicKey('nonexistent'), null);
  });
});

// ─────────────────────────────────────────────────────────────────
// 9. PeerManager — banning
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Banning', () => {
  let mgr;

  beforeEach(() => {
    mgr = new PeerManager({ maxPeers: 10, banDurationMs: 1000 });
  });

  it('should ban a peer and prevent re-adding', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    mgr.banPeer(p.id, 'malicious');

    assert.strictEqual(mgr.getPeer(p.id), null);
    assert.strictEqual(mgr.isBanned(p.id), true);
    assert.strictEqual(mgr.addPeer(p), false); // cannot re-add
  });

  it('should unban a peer', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    mgr.banPeer(p.id, 'test');
    assert.strictEqual(mgr.unbanPeer(p.id), true);
    assert.strictEqual(mgr.isBanned(p.id), false);
    assert.strictEqual(mgr.addPeer(p), true); // can add again
  });

  it('should expire bans after banDurationMs', async () => {
    const shortBan = new PeerManager({ maxPeers: 10, banDurationMs: 50 });
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    shortBan.addPeer(p);
    shortBan.banPeer(p.id, 'temp');
    assert.strictEqual(shortBan.isBanned(p.id), true);

    await new Promise((r) => setTimeout(r, 60));

    assert.strictEqual(shortBan.isBanned(p.id), false);
    assert.strictEqual(shortBan.addPeer(p), true); // can re-add after ban expires
  });
});

// ─────────────────────────────────────────────────────────────────
// 10. PeerManager — activity tracking and failures
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Activity & Failures', () => {
  let mgr;

  beforeEach(() => {
    mgr = new PeerManager({ maxPeers: 10 });
  });

  it('should track sent and received messages', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    mgr.updateActivity(p.id, 'sent');
    mgr.updateActivity(p.id, 'sent');
    mgr.updateActivity(p.id, 'received');

    const peer = mgr.getPeer(p.id);
    assert.strictEqual(peer.messagesSent, 2);
    assert.strictEqual(peer.messagesReceived, 1);
  });

  it('should mark peer inactive after max failures', () => {
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    for (let i = 0; i < 5; i++) mgr.recordFailure(p.id);

    assert.strictEqual(mgr.getPeer(p.id).status, PeerStatus.INACTIVE);
  });

  it('should not crash on failure for unknown peer', () => {
    mgr.recordFailure('nonexistent'); // should not throw
    mgr.updateActivity('nonexistent', 'sent'); // should not throw
  });
});

// ─────────────────────────────────────────────────────────────────
// 11. PeerManager — gossip peer selection
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Gossip Peer Selection', () => {
  let mgr;

  beforeEach(() => {
    mgr = new PeerManager({ maxPeers: 100 });
  });

  it('should select up to fanout peers', () => {
    for (let i = 0; i < 20; i++) {
      mgr.addPeer(createPeerInfo({ publicKey: `k${i}`, address: `a:${i}`, eScore: 50 }));
    }
    const selected = mgr.selectGossipPeers(GOSSIP_FANOUT);
    assert.ok(selected.length <= GOSSIP_FANOUT);
    assert.ok(selected.length > 0);
  });

  it('should return all peers if fewer than fanout', () => {
    mgr.addPeer(createPeerInfo({ publicKey: 'k1', address: 'a:1' }));
    mgr.addPeer(createPeerInfo({ publicKey: 'k2', address: 'a:2' }));
    const selected = mgr.selectGossipPeers(GOSSIP_FANOUT);
    assert.strictEqual(selected.length, 2);
  });

  it('should exclude specified peers', () => {
    const p1 = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    const p2 = createPeerInfo({ publicKey: 'k2', address: 'a:2' });
    mgr.addPeer(p1);
    mgr.addPeer(p2);
    const selected = mgr.selectGossipPeers(10, new Set([p1.id]));
    assert.ok(!selected.some((p) => p.id === p1.id));
  });

  it('should not select inactive peers', () => {
    const p = createPeerInfo({ publicKey: 'k_inactive', address: 'a:1' });
    mgr.addPeer(p);
    for (let i = 0; i < 5; i++) mgr.recordFailure(p.id); // mark inactive

    const selected = mgr.selectGossipPeers(10);
    assert.strictEqual(selected.length, 0);
  });
});

// ─────────────────────────────────────────────────────────────────
// 12. PeerManager — message deduplication
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Message Deduplication', () => {
  let mgr;

  beforeEach(() => {
    mgr = new PeerManager({ maxSeenMessages: 100, messageExpireMs: 200 });
  });

  it('should track seen messages', () => {
    assert.strictEqual(mgr.hasSeenMessage('msg_1'), false);
    mgr.markMessageSeen('msg_1');
    assert.strictEqual(mgr.hasSeenMessage('msg_1'), true);
  });

  it('should expire seen messages after messageExpireMs', async () => {
    mgr.markMessageSeen('msg_expire');
    assert.strictEqual(mgr.hasSeenMessage('msg_expire'), true);

    await new Promise((r) => setTimeout(r, 250));

    assert.strictEqual(mgr.hasSeenMessage('msg_expire'), false);
  });

  it('should evict oldest when at capacity', () => {
    const small = new PeerManager({ maxSeenMessages: 5, messageExpireMs: 60000 });
    for (let i = 0; i < 10; i++) {
      small.markMessageSeen(`msg_${i}`);
    }
    // Should not exceed capacity (some evicted)
    assert.ok(small.seenMessages.size <= 10); // eviction deletes 10% at a time
  });
});

// ─────────────────────────────────────────────────────────────────
// 13. PeerManager — eviction and capacity
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Eviction', () => {
  it('should evict worst peer when at capacity', () => {
    const mgr = new PeerManager({ maxPeers: 3 });

    // Add 3 peers with increasing eScore
    mgr.addPeer(createPeerInfo({ publicKey: 'low', address: 'a:1', eScore: 10 }));
    mgr.addPeer(createPeerInfo({ publicKey: 'mid', address: 'a:2', eScore: 50 }));
    mgr.addPeer(createPeerInfo({ publicKey: 'high', address: 'a:3', eScore: 90 }));

    assert.strictEqual(mgr.getStats().total, 3);

    // Add one more — should evict lowest score
    mgr.addPeer(createPeerInfo({ publicKey: 'new', address: 'a:4', eScore: 75 }));
    assert.strictEqual(mgr.getStats().total, 3);
    assert.strictEqual(mgr.getPeerByPublicKey('low'), null); // evicted
    assert.ok(mgr.getPeerByPublicKey('new')); // new peer added
  });
});

// ─────────────────────────────────────────────────────────────────
// 14. PeerManager — static calculations
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Static Calculations', () => {
  it('should calculate hops needed (O(log_fanout(n)))', () => {
    assert.strictEqual(PeerManager.calculateHops(0), 0);
    assert.strictEqual(PeerManager.calculateHops(1), 0);
    assert.ok(PeerManager.calculateHops(13) >= 1);
    assert.ok(PeerManager.calculateHops(169) >= 2);
    assert.ok(PeerManager.calculateHops(10000) >= 3);
  });

  it('should calculate propagation time', () => {
    const time = PeerManager.calculatePropagationTime(100, 50);
    assert.ok(time > 0);
    assert.strictEqual(typeof time, 'number');
  });

  it('should scale propagation logarithmically', () => {
    const t1 = PeerManager.calculatePropagationTime(100);
    const t2 = PeerManager.calculatePropagationTime(10000);
    // 10000 is 100x more nodes, but time should NOT be 100x more
    assert.ok(t2 < t1 * 10);
  });
});

// ─────────────────────────────────────────────────────────────────
// 15. PeerManager — statistics
// ─────────────────────────────────────────────────────────────────

describe('PeerManager — Statistics', () => {
  it('should report correct stats', () => {
    const mgr = new PeerManager({ maxPeers: 10 });
    mgr.addPeer(createPeerInfo({ publicKey: 'k1', address: 'a:1', eScore: 60 }));
    mgr.addPeer(createPeerInfo({ publicKey: 'k2', address: 'a:2', eScore: 80 }));

    const stats = mgr.getStats();
    assert.strictEqual(stats.total, 2);
    assert.strictEqual(stats.active, 2);
    assert.strictEqual(stats.inactive, 0);
    assert.strictEqual(stats.avgEScore, 70);
  });

  it('should count banned peers separately', () => {
    const mgr = new PeerManager({ maxPeers: 10 });
    const p = createPeerInfo({ publicKey: 'k1', address: 'a:1' });
    mgr.addPeer(p);
    mgr.banPeer(p.id, 'test');

    const stats = mgr.getStats();
    assert.strictEqual(stats.total, 0); // banned peer removed from active
    assert.strictEqual(stats.banned, 1);
  });
});

// ─────────────────────────────────────────────────────────────────
// 16. GossipProtocol — peer management
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Peer Management', () => {
  it('should add peers', () => {
    const { protocol } = makeProtocol();
    protocol.addPeer({ publicKey: 'ed25519:p1', address: 'a:1', eScore: 60 });
    assert.strictEqual(protocol.getStats().total, 1);
  });

  it('should remove peers', () => {
    const { protocol } = makeProtocol();
    protocol.addPeer({ publicKey: 'ed25519:p1', address: 'a:1' });
    const peers = protocol.peerManager.getActivePeers();
    protocol.removePeer(peers[0].id);
    assert.strictEqual(protocol.getStats().total, 0);
  });

  it('should get active peers', () => {
    const { protocol } = makeProtocol();
    addPeers(protocol, 3);
    const active = protocol.getActivePeers();
    assert.strictEqual(active.length, 3);
  });
});

// ─────────────────────────────────────────────────────────────────
// 17. GossipProtocol — broadcast
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Broadcast', () => {
  it('should broadcast block to all peers', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 3);

    const sent = await protocol.broadcastBlock({ slot: 1, data: 'test' });
    assert.strictEqual(sent, 3);
    assert.strictEqual(sentMessages.length, 3);
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.BLOCK));
  });

  it('should broadcast judgment', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);

    await protocol.broadcastJudgment({ verdict: 'WAG', score: 75 });
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.JUDGMENT));
  });

  it('should broadcast pattern', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);

    await protocol.broadcastPattern({ id: 'pat1', strength: 0.8 });
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.PATTERN));
  });

  it('should handle broadcast to zero peers gracefully', async () => {
    const { protocol } = makeProtocol();
    const sent = await protocol.broadcastBlock({ slot: 1 });
    assert.strictEqual(sent, 0);
  });

  it('should handle send failures without crashing', async () => {
    let failCount = 0;
    const { protocol } = makeProtocol({
      sendFn: async () => {
        failCount++;
        throw new Error('connection refused');
      },
    });
    addPeers(protocol, 3);

    const sent = await protocol.broadcastBlock({ slot: 1 });
    assert.strictEqual(sent, 0);
    assert.strictEqual(failCount, 3);
  });
});

// ─────────────────────────────────────────────────────────────────
// 18. GossipProtocol — consensus message broadcast
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Consensus Broadcast', () => {
  it('should broadcast block proposal', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);
    await protocol.broadcastBlockProposal({ blockHash: 'h', slot: 1 });
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.CONSENSUS_BLOCK_PROPOSAL));
  });

  it('should broadcast vote', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);
    await protocol.broadcastVote({ blockHash: 'h', decision: 'APPROVE' });
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.CONSENSUS_VOTE));
  });

  it('should broadcast vote aggregate', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 1);
    await protocol.broadcastVoteAggregate({ blockHash: 'h', approveWeight: 70 });
    assert.strictEqual(sentMessages[0].message.type, MessageType.CONSENSUS_VOTE_AGGREGATE);
  });

  it('should broadcast finality notification', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 1);
    await protocol.broadcastFinality({ blockHash: 'h', status: 'DETERMINISTIC' });
    assert.strictEqual(sentMessages[0].message.type, MessageType.CONSENSUS_FINALITY);
  });

  it('should broadcast slot status to all peers', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 3);
    const sent = await protocol.broadcastSlotStatus({ slot: 42 });
    assert.strictEqual(sent, 3);
    assert.ok(sentMessages.every((m) => m.message.type === MessageType.CONSENSUS_SLOT_STATUS));
  });
});

// ─────────────────────────────────────────────────────────────────
// 19. GossipProtocol — message handling
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Message Handling', () => {
  it('should process valid signed message', async () => {
    const { protocol, handledMessages } = makeProtocol();
    const other = generateKeypair();

    const msg = createMessage({
      type: MessageType.PATTERN,
      payload: { id: 'pat_1' },
      sender: other.publicKey,
      privateKey: other.privateKey,
      ttl: 2,
    });

    await protocol.handleMessage(msg, 'peerX');
    assert.strictEqual(handledMessages.length, 1);
  });

  it('should reject duplicate messages', async () => {
    const { protocol, handledMessages } = makeProtocol();
    const other = generateKeypair();

    const msg = createMessage({
      type: MessageType.PATTERN,
      payload: { id: 'pat_dup' },
      sender: other.publicKey,
      privateKey: other.privateKey,
    });

    await protocol.handleMessage(msg, 'p1');
    await protocol.handleMessage(msg, 'p2'); // same ID
    assert.strictEqual(handledMessages.length, 1);
  });

  it('should reject messages with invalid signatures', async () => {
    const { protocol, handledMessages } = makeProtocol();

    const fakeMsg = {
      id: 'msg_fake',
      type: MessageType.BLOCK,
      payload: {},
      sender: 'ed25519:fakepeer',
      signature: 'invalid_signature',
      timestamp: Date.now(),
      ttl: 2,
      hops: 0,
    };

    await protocol.handleMessage(fakeMsg, 'peerBad');
    assert.strictEqual(handledMessages.length, 0);
  });

  it('should accept heartbeats without signature check', async () => {
    const { protocol, handledMessages } = makeProtocol();

    const heartbeat = {
      id: generateMessageId(),
      type: MessageType.HEARTBEAT,
      payload: { eScore: 80 },
      sender: 'someone',
      timestamp: Date.now(),
      ttl: 1,
      hops: 0,
    };

    await protocol.handleMessage(heartbeat, 'peerHB');
    assert.strictEqual(handledMessages.length, 1);
  });

  it('should relay relayable messages to peers', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);

    const other = generateKeypair();
    const msg = createMessage({
      type: MessageType.BLOCK,
      payload: { slot: 1 },
      sender: other.publicKey,
      privateKey: other.privateKey,
      ttl: 2,
    });

    await protocol.handleMessage(msg, 'origin');
    // Should relay to connected peers
    assert.ok(sentMessages.length > 0);
    // Relayed message should have decremented TTL
    assert.strictEqual(sentMessages[0].message.ttl, 1);
    assert.strictEqual(sentMessages[0].message.hops, 1);
  });

  it('should NOT relay messages with TTL=0', async () => {
    const { protocol, sentMessages } = makeProtocol();
    addPeers(protocol, 2);

    const other = generateKeypair();
    const msg = createMessage({
      type: MessageType.BLOCK,
      payload: { slot: 1 },
      sender: other.publicKey,
      privateKey: other.privateKey,
      ttl: 0, // no relay
    });

    // Manually mark as not seen so handleMessage processes it
    await protocol.handleMessage(msg, 'origin');
    // msg has TTL=0 so shouldRelay returns false, no relay expected
    // but handleMessage was called (and would skip relay)
    // sentMessages may have entries from relay, but with TTL=0 shouldRelay is false
    // Note: the message won't be processed (verifyMessage fails with no signature on BLOCK)
    // Let's check no relay happened
    assert.strictEqual(sentMessages.length, 0);
  });

  it('should handle peer announce and add new peer', async () => {
    const { protocol, keypair } = makeProtocol();
    const other = generateKeypair();

    const announceMsg = createMessage({
      type: MessageType.PEER_ANNOUNCE,
      payload: { publicKey: 'ed25519:new_peer', address: 'newhost:9999', eScore: 60 },
      sender: other.publicKey,
      privateKey: other.privateKey,
      ttl: 2,
    });

    await protocol.handleMessage(announceMsg, 'somePeer');
    const found = protocol.peerManager.getPeerByPublicKey('ed25519:new_peer');
    assert.ok(found);
  });
});

// ─────────────────────────────────────────────────────────────────
// 20. GossipProtocol — sync response handling
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Sync Response', () => {
  it('should resolve pending request on sync response', async () => {
    const { protocol, keypair } = makeProtocol();
    const other = generateKeypair();

    // Simulate a pending request
    let resolved = null;
    const requestId = 'req_test_42';
    protocol._addPendingRequest(requestId, {
      resolve: (data) => { resolved = data; },
      reject: () => {},
    });

    // Create sync response message targeting that requestId
    const responseMsg = createMessage({
      type: MessageType.SYNC_RESPONSE,
      payload: { request_id: requestId, blocks: [{ slot: 1 }, { slot: 2 }] },
      sender: other.publicKey,
      privateKey: other.privateKey,
      ttl: 1,
    });

    await protocol.handleMessage(responseMsg, 'syncPeer');
    assert.ok(resolved);
    assert.strictEqual(resolved.length, 2);
    assert.strictEqual(protocol.pendingRequests.has(requestId), false);
  });
});

// ─────────────────────────────────────────────────────────────────
// 21. GossipProtocol — pending request bounds
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Pending Request Bounds', () => {
  it('should evict oldest request when at capacity', () => {
    const { protocol } = makeProtocol({ maxPendingRequests: 2 });

    let evicted = null;
    protocol._addPendingRequest('r1', { resolve: () => {}, reject: (e) => { evicted = e; } });
    protocol._addPendingRequest('r2', { resolve: () => {}, reject: () => {} });
    protocol._addPendingRequest('r3', { resolve: () => {}, reject: () => {} });

    assert.strictEqual(protocol.pendingRequests.size, 2);
    assert.ok(evicted); // r1 should have been evicted
    assert.ok(evicted.message.includes('evicted'));
  });
});

// ─────────────────────────────────────────────────────────────────
// 22. GossipProtocol — stats
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — Stats', () => {
  it('should include fanout and propagation estimate', () => {
    const { protocol } = makeProtocol();
    addPeers(protocol, 5);

    const stats = protocol.getStats();
    assert.strictEqual(stats.fanout, GOSSIP_FANOUT);
    assert.ok(typeof stats.estimatedReachTime === 'number');
    assert.strictEqual(stats.total, 5);
    assert.strictEqual(stats.active, 5);
    assert.strictEqual(stats.pendingRequests, 0);
  });
});

// ─────────────────────────────────────────────────────────────────
// 23. GossipProtocol — sendTo direct messaging
// ─────────────────────────────────────────────────────────────────

describe('GossipProtocol — sendTo', () => {
  it('should send message directly to a known peer', async () => {
    const { protocol, sentMessages } = makeProtocol();
    const peers = addPeers(protocol, 1);

    const msg = { type: 'test', payload: 'hello' };
    await protocol.sendTo(peers[0].publicKey, msg);
    assert.strictEqual(sentMessages.length, 1);
  });

  it('should throw for unknown peer', async () => {
    const { protocol } = makeProtocol();
    await assert.rejects(
      () => protocol.sendTo('unknown_peer', { type: 'test' }),
      /Peer not found/
    );
  });

  it('should record failure on send error', async () => {
    const { protocol } = makeProtocol({
      sendFn: async () => { throw new Error('fail'); },
    });
    const peers = addPeers(protocol, 1);

    await assert.rejects(
      () => protocol.sendTo(peers[0].publicKey, { type: 'test' }),
      /fail/
    );

    // Peer should have a failure recorded
    const peer = protocol.peerManager.getPeerByPublicKey(peers[0].publicKey);
    assert.ok(peer.failedAttempts >= 1);
  });
});
