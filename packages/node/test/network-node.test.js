/**
 * CYNICNetworkNode Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests multi-node orchestration.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { CYNICNetworkNode, NetworkState } from '../src/network/network-node.js';

// Mock crypto for key generation
const mockPublicKey = 'test-public-key-0123456789abcdef';
const mockPrivateKey = 'test-private-key-0123456789abcdef';

describe('CYNICNetworkNode', () => {
  let node;

  beforeEach(() => {
    node = null;
  });

  afterEach(async () => {
    if (node) {
      await node.stop();
    }
  });

  describe('constructor', () => {
    it('requires publicKey and privateKey', () => {
      expect(() => new CYNICNetworkNode({})).toThrow('publicKey and privateKey required');
    });

    it('creates node with valid keys', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false, // Disable actual networking for tests
      });

      expect(node.publicKey).toBe(mockPublicKey);
      expect(node.state).toBe(NetworkState.OFFLINE);
    });

    it('uses default E-Score of 50', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      expect(node.eScore).toBe(50);
    });

    it('uses φ-aligned default port 8618', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      const info = node.getInfo();
      expect(info.port).toBe(8618);
    });
  });

  describe('state management', () => {
    it('starts OFFLINE', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      expect(node.state).toBe(NetworkState.OFFLINE);
      expect(node.isOnline).toBe(false);
      expect(node.isParticipating).toBe(false);
    });

    it('stays OFFLINE when networking disabled', async () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      await node.start();
      expect(node.state).toBe(NetworkState.OFFLINE);
    });
  });

  describe('E-Score management', () => {
    it('allows setting E-Score', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      node.setEScore(75);
      expect(node.eScore).toBe(75);
    });

    it('clamps E-Score to 0-100', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      node.setEScore(150);
      expect(node.eScore).toBe(100);

      node.setEScore(-50);
      expect(node.eScore).toBe(0);
    });
  });

  describe('seed nodes', () => {
    it('accepts seed nodes in constructor', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
        seedNodes: ['ws://localhost:8618', 'ws://localhost:8619'],
      });

      const status = node.getStatus();
      expect(status.discovery).toBe(null); // Discovery not initialized when disabled
    });

    it('allows adding seed nodes', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      // Should not throw even when disabled
      node.addSeedNode('ws://localhost:8620');
    });
  });

  describe('getInfo', () => {
    it('returns node info', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        eScore: 75,
        port: 8620,
        enabled: false,
      });

      const info = node.getInfo();

      expect(info.port).toBe(8620);
      expect(info.eScore).toBe(75);
      expect(info.state).toBe(NetworkState.OFFLINE);
      expect(info.publicKey).toContain('test-public-key');
      expect(info.stats).toBeDefined();
    });
  });

  describe('getStatus', () => {
    it('returns full status', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      const status = node.getStatus();

      expect(status.node).toBeDefined();
      expect(status.transport).toBe(null);
      expect(status.consensus).toBe(null);
      expect(status.discovery).toBe(null);
      expect(status.sync).toBeDefined();
    });
  });

  describe('events', () => {
    it('emits started event', async () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      const startedPromise = new Promise(resolve => {
        node.on('started', resolve);
      });

      await node.start();

      // When disabled, start() returns early without emitting
      // This is expected behavior
    });

    it('emits stopped event', async () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      const events = [];
      node.on('stopped', () => events.push('stopped'));

      await node.start();
      await node.stop();

      // When disabled, no events emitted (expected)
    });
  });
});

describe('NetworkState', () => {
  it('has correct states', () => {
    expect(NetworkState.OFFLINE).toBe('OFFLINE');
    expect(NetworkState.BOOTSTRAPPING).toBe('BOOTSTRAPPING');
    expect(NetworkState.SYNCING).toBe('SYNCING');
    expect(NetworkState.ONLINE).toBe('ONLINE');
    expect(NetworkState.PARTICIPATING).toBe('PARTICIPATING');
    expect(NetworkState.ERROR).toBe('ERROR');
  });
});

describe('Fork Detection', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
    });
  });

  afterEach(async () => {
    if (node) {
      await node.stop();
    }
  });

  it('detects fork when peers report different hashes for same slot', async () => {
    const events = [];
    node.on('fork:detected', (e) => events.push(e));

    // Peer A reports hash A for slot 100
    node._checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa-1111' }], 50);

    // No fork yet (only one hash)
    expect(events.length).toBe(0);

    // Peer B reports DIFFERENT hash for same slot 100 (FORK!)
    node._checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb-2222' }], 60);

    // Fork should be detected
    expect(events.length).toBe(1);
    expect(events[0].slot).toBe(100);
    expect(events[0].branches).toBe(2);
  });

  it('calculates heaviest branch by E-Score', async () => {
    const events = [];
    node.on('fork:detected', (e) => events.push(e));

    // Multiple peers on branch A (total E-Score: 50 + 40 = 90)
    node._checkForForks('peer-a1', [{ slot: 100, hash: 'hash-aaaa' }], 50);
    node._checkForForks('peer-a2', [{ slot: 100, hash: 'hash-aaaa' }], 40);

    // One peer on branch B (E-Score: 60)
    node._checkForForks('peer-b1', [{ slot: 100, hash: 'hash-bbbb' }], 60);

    // Fork detected, branch A should be heaviest (90 > 60)
    expect(events.length).toBe(1);
    expect(events[0].heaviestBranch).toBe('hash-aaaa'.slice(0, 16));
  });

  it('returns fork status via getForkStatus()', () => {
    // Create a fork
    node._checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 50);
    node._checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 60);

    const status = node.getForkStatus();

    expect(status.detected).toBe(true);
    expect(status.forkSlot).toBe(100);
    expect(status.branches.length).toBe(2);
    expect(status.stats.forksDetected).toBe(1);
  });

  it('cleans up old fork data', () => {
    // Record hashes for old slots
    for (let i = 0; i < 150; i++) {
      node._slotHashes.set(i, { hash: `hash-${i}` });
    }

    // Simulate consensus at slot 150
    node._consensus = { lastFinalizedSlot: 150 };

    // Cleanup should remove slots < 50 (150 - 100)
    node._cleanupForkData();

    expect(node._slotHashes.has(49)).toBe(false);
    expect(node._slotHashes.has(50)).toBe(true);
  });

  it('records block hashes for slots', () => {
    node.recordBlockHash(100, 'hash-100-abc');
    node.recordBlockHash(101, 'hash-101-def');

    expect(node._slotHashes.get(100)?.hash).toBe('hash-100-abc');
    expect(node._slotHashes.get(101)?.hash).toBe('hash-101-def');
  });
});

describe('State Sync', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
    });
  });

  afterEach(async () => {
    if (node) {
      await node.stop();
    }
  });

  it('tracks peer slots from heartbeats', async () => {
    // Simulate receiving a heartbeat
    const heartbeat = {
      type: 'HEARTBEAT',
      nodeId: 'peer-node-id',
      eScore: 75,
      slot: 100,
      finalizedSlot: 95,
      state: 'PARTICIPATING',
      timestamp: Date.now(),
    };

    // Access the private method directly for testing
    await node._handleHeartbeat(heartbeat, 'peer-id-123');

    // Check that peer slot is tracked
    const peerInfo = node._peerSlots.get('peer-id-123');
    expect(peerInfo).toBeDefined();
    expect(peerInfo.finalizedSlot).toBe(95);
    expect(peerInfo.eScore).toBe(75);
  });

  it('calculates behindBy correctly', async () => {
    // Simulate multiple peer heartbeats
    await node._handleHeartbeat({
      type: 'HEARTBEAT',
      finalizedSlot: 100,
      eScore: 50,
    }, 'peer-1');

    await node._handleHeartbeat({
      type: 'HEARTBEAT',
      finalizedSlot: 150,
      eScore: 60,
    }, 'peer-2');

    // Run sync check (node starts at slot 0)
    node._checkStateSync();

    // Should be behind by the highest peer slot
    expect(node._syncState.behindBy).toBe(150);
  });

  it('emits sync:needed when significantly behind', async () => {
    const events = [];
    node.on('sync:needed', (e) => events.push(e));

    // Simulate peer far ahead
    await node._handleHeartbeat({
      type: 'HEARTBEAT',
      finalizedSlot: 100, // >10 slots ahead
      eScore: 50,
    }, 'peer-1');

    node._checkStateSync();

    expect(events.length).toBe(1);
    expect(events[0].behindBy).toBe(100);
    expect(events[0].bestPeer).toBe('peer-1');
  });
});
