/**
 * Layer 3: Gossip Propagation Tests
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
  // Peer
  PeerStatus,
  createPeerInfo,
  PeerManager,
  // Protocol
  GossipProtocol,
  generateKeypair,
} from '../src/index.js';

import { GOSSIP_FANOUT } from '@cynic/core';

describe('Message Management', () => {
  let publicKey, privateKey;

  beforeEach(() => {
    const keypair = generateKeypair();
    publicKey = keypair.publicKey;
    privateKey = keypair.privateKey;
  });

  it('should generate unique message IDs', () => {
    const id1 = generateMessageId();
    const id2 = generateMessageId();
    assert.ok(id1.startsWith('msg_'));
    assert.notStrictEqual(id1, id2);
  });

  it('should create message with signature', () => {
    const message = createMessage({
      type: MessageType.BLOCK,
      payload: { slot: 1 },
      sender: publicKey,
      privateKey,
      ttl: 3,
    });

    assert.ok(message.id.startsWith('msg_'));
    assert.strictEqual(message.type, MessageType.BLOCK);
    assert.strictEqual(message.ttl, 3);
    assert.strictEqual(message.hops, 0);
    assert.ok(message.signature);
  });

  it('should verify message signature', () => {
    const message = createMessage({
      type: MessageType.JUDGMENT,
      payload: { verdict: 'WAG' },
      sender: publicKey,
      privateKey,
    });

    const valid = verifyMessage(message);
    assert.strictEqual(valid, true);
  });

  it('should reject tampered message', () => {
    const message = createMessage({
      type: MessageType.JUDGMENT,
      payload: { verdict: 'WAG' },
      sender: publicKey,
      privateKey,
    });

    // Tamper with payload
    message.payload.verdict = 'HOWL';

    const valid = verifyMessage(message);
    assert.strictEqual(valid, false);
  });

  it('should reject message without signature', () => {
    const message = {
      id: 'msg_123',
      type: MessageType.BLOCK,
      payload: {},
      sender: publicKey,
    };

    const valid = verifyMessage(message);
    assert.strictEqual(valid, false);
  });

  it('should check relay eligibility', () => {
    const relayable = createMessage({
      type: MessageType.BLOCK,
      payload: {},
      sender: publicKey,
      ttl: 2,
    });
    assert.strictEqual(shouldRelay(relayable), true);

    const expiredTTL = createMessage({
      type: MessageType.BLOCK,
      payload: {},
      sender: publicKey,
      ttl: 0,
    });
    assert.strictEqual(shouldRelay(expiredTTL), false);

    // Heartbeats ARE relayed (star topology fix: leaf nodes need heartbeats
    // from non-directly-connected peers via hub relay)
    const heartbeat = createMessage({
      type: MessageType.HEARTBEAT,
      payload: {},
      sender: publicKey,
      ttl: 1,
    });
    assert.strictEqual(shouldRelay(heartbeat), true);
  });

  it('should prepare message for relay', () => {
    const original = createMessage({
      type: MessageType.PATTERN,
      payload: { id: 'pat_1' },
      sender: publicKey,
      ttl: 3,
    });

    const relayed = prepareRelay(original);
    assert.strictEqual(relayed.ttl, 2);
    assert.strictEqual(relayed.hops, 1);
  });

  it('should create block message', () => {
    const block = { slot: 5, prev_hash: 'sha256:abc' };
    const message = createBlockMessage(block, publicKey, privateKey);

    assert.strictEqual(message.type, MessageType.BLOCK);
    assert.deepStrictEqual(message.payload, block);
  });

  it('should create sync request', () => {
    const request = createSyncRequest(10, publicKey, privateKey);

    assert.strictEqual(request.type, MessageType.SYNC_REQUEST);
    assert.strictEqual(request.payload.since_slot, 10);
    assert.strictEqual(request.ttl, 1); // No relay
  });

  it('should create sync response', () => {
    const blocks = [{ slot: 1 }, { slot: 2 }];
    const response = createSyncResponse(blocks, 'req_123', publicKey, privateKey);

    assert.strictEqual(response.type, MessageType.SYNC_RESPONSE);
    assert.strictEqual(response.payload.request_id, 'req_123');
    assert.strictEqual(response.payload.blocks.length, 2);
  });

  it('should create heartbeat (unsigned)', () => {
    const status = { height: 100, peers: 10 };
    const heartbeat = createHeartbeat(status, publicKey);

    assert.strictEqual(heartbeat.type, MessageType.HEARTBEAT);
    assert.strictEqual(heartbeat.ttl, 1);
    assert.ok(!heartbeat.signature); // Heartbeats are not signed
  });

  it('should create peer announcement', () => {
    const peerInfo = { address: 'localhost:8080', eScore: 75 };
    const announce = createPeerAnnounce(peerInfo, publicKey, privateKey);

    assert.strictEqual(announce.type, MessageType.PEER_ANNOUNCE);
    assert.strictEqual(announce.ttl, 2); // Limited relay
    assert.ok(announce.signature);
  });
});

describe('Peer Management', () => {
  it('should create peer info', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc123',
      address: 'localhost:8080',
      eScore: 75,
    });

    assert.ok(peer.id);
    assert.strictEqual(peer.publicKey, 'ed25519:abc123');
    assert.strictEqual(peer.address, 'localhost:8080');
    assert.strictEqual(peer.eScore, 75);
    assert.strictEqual(peer.status, PeerStatus.ACTIVE);
  });

  it('should have default eScore', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:def456',
      address: 'localhost:9000',
    });

    assert.strictEqual(peer.eScore, 50);
  });
});

describe('Peer Manager', () => {
  let manager;

  beforeEach(() => {
    manager = new PeerManager({ maxPeers: 10 });
  });

  it('should add peers', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
    });

    const added = manager.addPeer(peer);
    assert.strictEqual(added, true);

    const retrieved = manager.getPeer(peer.id);
    assert.strictEqual(retrieved.publicKey, peer.publicKey);
  });

  it('should update existing peer', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
      eScore: 50,
    });

    manager.addPeer(peer);

    // Update with new eScore
    const updated = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
      eScore: 75,
    });
    manager.addPeer(updated);

    const retrieved = manager.getPeer(peer.id);
    assert.strictEqual(retrieved.eScore, 75);
  });

  it('should remove peer', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
    });

    manager.addPeer(peer);
    manager.removePeer(peer.id);

    assert.strictEqual(manager.getPeer(peer.id), null);
  });

  it('should ban peer', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
    });

    manager.addPeer(peer);
    manager.banPeer(peer.id, 'malicious');

    assert.strictEqual(manager.getPeer(peer.id), null);

    // Cannot re-add banned peer
    const readded = manager.addPeer(peer);
    assert.strictEqual(readded, false);
  });

  it('should track message activity', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
    });

    manager.addPeer(peer);
    manager.updateActivity(peer.id, 'sent');
    manager.updateActivity(peer.id, 'received');
    manager.updateActivity(peer.id, 'received');

    const retrieved = manager.getPeer(peer.id);
    assert.strictEqual(retrieved.messagesSent, 1);
    assert.strictEqual(retrieved.messagesReceived, 2);
  });

  it('should record failures', () => {
    const peer = createPeerInfo({
      publicKey: 'ed25519:abc',
      address: 'localhost:8080',
    });

    manager.addPeer(peer);

    for (let i = 0; i < 5; i++) {
      manager.recordFailure(peer.id);
    }

    const retrieved = manager.getPeer(peer.id);
    assert.strictEqual(retrieved.status, PeerStatus.INACTIVE);
  });

  it('should get active peers', () => {
    for (let i = 0; i < 5; i++) {
      manager.addPeer(
        createPeerInfo({
          publicKey: `ed25519:peer${i}`,
          address: `localhost:${8080 + i}`,
        })
      );
    }

    const active = manager.getActivePeers();
    assert.strictEqual(active.length, 5);
  });

  it('should select gossip peers with fanout', () => {
    // Add more peers than fanout
    for (let i = 0; i < 20; i++) {
      manager.addPeer(
        createPeerInfo({
          publicKey: `ed25519:peer${i}`,
          address: `localhost:${8080 + i}`,
          eScore: 50 + i,
        })
      );
    }

    const selected = manager.selectGossipPeers(GOSSIP_FANOUT);
    assert.ok(selected.length <= GOSSIP_FANOUT);
  });

  it('should exclude specified peers from selection', () => {
    const peer1 = createPeerInfo({
      publicKey: 'ed25519:peer1',
      address: 'localhost:8080',
    });
    const peer2 = createPeerInfo({
      publicKey: 'ed25519:peer2',
      address: 'localhost:8081',
    });

    manager.addPeer(peer1);
    manager.addPeer(peer2);

    const selected = manager.selectGossipPeers(10, new Set([peer1.id]));
    assert.ok(!selected.some((p) => p.id === peer1.id));
  });

  it('should track seen messages', () => {
    const msgId = 'msg_123';

    assert.strictEqual(manager.hasSeenMessage(msgId), false);

    manager.markMessageSeen(msgId);
    assert.strictEqual(manager.hasSeenMessage(msgId), true);
  });

  it('should evict worst peer when at capacity', () => {
    // Fill to capacity
    for (let i = 0; i < 10; i++) {
      manager.addPeer(
        createPeerInfo({
          publicKey: `ed25519:peer${i}`,
          address: `localhost:${8080 + i}`,
          eScore: 50 + i * 5, // Increasing scores
        })
      );
    }

    const stats1 = manager.getStats();
    assert.strictEqual(stats1.total, 10);

    // Add one more (should evict lowest score)
    manager.addPeer(
      createPeerInfo({
        publicKey: 'ed25519:newpeer',
        address: 'localhost:9999',
        eScore: 100,
      })
    );

    const stats2 = manager.getStats();
    assert.strictEqual(stats2.total, 10);
  });

  it('should calculate hops for network size', () => {
    // With fanout of 13:
    // 1 node: 0 hops
    // 13 nodes: 1 hop
    // 169 nodes: 2 hops
    assert.strictEqual(PeerManager.calculateHops(1), 0);
    assert.ok(PeerManager.calculateHops(13) >= 1);
    assert.ok(PeerManager.calculateHops(169) >= 2);
  });

  it('should calculate propagation time', () => {
    const time = PeerManager.calculatePropagationTime(100);
    assert.ok(time > 0);
    assert.strictEqual(typeof time, 'number');
  });

  it('should get statistics', () => {
    manager.addPeer(
      createPeerInfo({
        publicKey: 'ed25519:peer1',
        address: 'localhost:8080',
        eScore: 60,
      })
    );
    manager.addPeer(
      createPeerInfo({
        publicKey: 'ed25519:peer2',
        address: 'localhost:8081',
        eScore: 80,
      })
    );

    const stats = manager.getStats();
    assert.strictEqual(stats.total, 2);
    assert.strictEqual(stats.active, 2);
    assert.strictEqual(stats.avgEScore, 70);
  });
});

describe('Gossip Protocol', () => {
  let protocol;
  let publicKey, privateKey;

  beforeEach(() => {
    const keypair = generateKeypair();
    publicKey = keypair.publicKey;
    privateKey = keypair.privateKey;

    protocol = new GossipProtocol({
      publicKey,
      privateKey,
      address: 'localhost:8080',
      onMessage: async () => {},
      sendFn: async () => {},
    });
  });

  it('should add peers', () => {
    protocol.addPeer({
      publicKey: 'ed25519:peer1',
      address: 'localhost:9000',
      eScore: 75,
    });

    const stats = protocol.getStats();
    assert.strictEqual(stats.total, 1);
  });

  it('should remove peers', () => {
    protocol.addPeer({
      publicKey: 'ed25519:peer1',
      address: 'localhost:9000',
    });

    const stats1 = protocol.getStats();
    assert.strictEqual(stats1.total, 1);

    // Get peer ID (hashed from public key)
    const peers = protocol.peerManager.getActivePeers();
    protocol.removePeer(peers[0].id);

    const stats2 = protocol.getStats();
    assert.strictEqual(stats2.total, 0);
  });

  it('should broadcast block', async () => {
    const sentMessages = [];
    protocol.sendFn = async (peer, message) => {
      sentMessages.push({ peer, message });
    };

    protocol.addPeer({
      publicKey: 'ed25519:peer1',
      address: 'localhost:9000',
    });

    const block = { slot: 1, prev_hash: 'sha256:abc' };
    const sent = await protocol.broadcastBlock(block);

    assert.strictEqual(sent, 1);
    assert.strictEqual(sentMessages[0].message.type, MessageType.BLOCK);
  });

  it('should broadcast judgment', async () => {
    const sentMessages = [];
    protocol.sendFn = async (peer, message) => {
      sentMessages.push({ peer, message });
    };

    protocol.addPeer({
      publicKey: 'ed25519:peer1',
      address: 'localhost:9000',
    });

    const judgment = { id: 'jdg_1', verdict: 'WAG' };
    await protocol.broadcastJudgment(judgment);

    assert.strictEqual(sentMessages[0].message.type, MessageType.JUDGMENT);
  });

  it('should broadcast pattern', async () => {
    const sentMessages = [];
    protocol.sendFn = async (peer, message) => {
      sentMessages.push({ peer, message });
    };

    protocol.addPeer({
      publicKey: 'ed25519:peer1',
      address: 'localhost:9000',
    });

    const pattern = { id: 'pat_1', strength: 0.8 };
    await protocol.broadcastPattern(pattern);

    assert.strictEqual(sentMessages[0].message.type, MessageType.PATTERN);
  });

  it('should handle incoming message', async () => {
    const handledMessages = [];
    protocol.onMessage = async (msg) => {
      handledMessages.push(msg);
    };

    // Create a valid signed message from another peer
    const otherKeypair = generateKeypair();
    const message = createMessage({
      type: MessageType.PATTERN,
      payload: { id: 'pat_1' },
      sender: otherKeypair.publicKey,
      privateKey: otherKeypair.privateKey,
      ttl: 2,
    });

    await protocol.handleMessage(message, 'peer123');

    assert.strictEqual(handledMessages.length, 1);
  });

  it('should not process duplicate messages', async () => {
    let handledCount = 0;
    protocol.onMessage = async () => {
      handledCount++;
    };

    const otherKeypair = generateKeypair();
    const message = createMessage({
      type: MessageType.PATTERN,
      payload: { id: 'pat_1' },
      sender: otherKeypair.publicKey,
      privateKey: otherKeypair.privateKey,
    });

    await protocol.handleMessage(message, 'peer123');
    await protocol.handleMessage(message, 'peer456'); // Same message ID

    assert.strictEqual(handledCount, 1);
  });

  it('should reject invalid signatures', async () => {
    const handledMessages = [];
    protocol.onMessage = async (msg) => {
      handledMessages.push(msg);
    };

    const message = {
      id: 'msg_fake',
      type: MessageType.BLOCK,
      payload: {},
      sender: 'ed25519:fakepeer',
      signature: 'invalid_signature',
      timestamp: Date.now(),
      ttl: 2,
      hops: 0,
    };

    await protocol.handleMessage(message, 'peer123');

    // Message should not be processed
    assert.strictEqual(handledMessages.length, 0);
  });

  it('should get stats with fanout', () => {
    const stats = protocol.getStats();

    assert.strictEqual(stats.fanout, GOSSIP_FANOUT);
    assert.strictEqual(typeof stats.estimatedReachTime, 'number');
  });
});
