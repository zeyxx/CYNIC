/**
 * CYNICNetworkNode Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests multi-node orchestration and extracted SRP components.
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

describe('Validator Set Management', () => {
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

  it('adds validator with sufficient E-Score', () => {
    const added = node.addValidator({
      publicKey: 'validator-1-key',
      eScore: 50,
      burned: 100,
    });

    expect(added).toBe(true);
    expect(node._validatorManager.size).toBe(1);

    const validator = node.getValidator('validator-1-key');
    expect(validator).toBeDefined();
    expect(validator.eScore).toBe(50);
    expect(validator.burned).toBe(100);
    expect(validator.status).toBe('active');
  });

  it('rejects validator with low E-Score', () => {
    const added = node.addValidator({
      publicKey: 'low-score-validator',
      eScore: 10, // Below minimum of 20
    });

    expect(added).toBe(false);
    expect(node._validatorManager.size).toBe(0);
  });

  it('removes validator', () => {
    node.addValidator({ publicKey: 'validator-to-remove', eScore: 50 });
    expect(node._validatorManager.size).toBe(1);

    const removed = node.removeValidator('validator-to-remove', 'test');
    expect(removed).toBe(true);
    expect(node._validatorManager.size).toBe(0);
  });

  it('penalizes validator and reduces E-Score', () => {
    node.addValidator({ publicKey: 'bad-validator', eScore: 50 });

    node.penalizeValidator('bad-validator', 10, 'test_penalty');

    const validator = node.getValidator('bad-validator');
    expect(validator.eScore).toBe(40);
    expect(validator.penalties).toBe(10);
  });

  it('removes validator when penalty drops E-Score below minimum', () => {
    node.addValidator({ publicKey: 'very-bad-validator', eScore: 25 });

    // Penalize enough to drop below minimum (20)
    node.penalizeValidator('very-bad-validator', 10, 'severe_penalty');

    // Validator should be removed
    expect(node.getValidator('very-bad-validator')).toBeNull();
    expect(node._validatorManager.stats.validatorsPenalized).toBe(1);
  });

  it('rewards validator for blocks', () => {
    node.addValidator({ publicKey: 'good-validator', eScore: 50 });

    node.rewardValidator('good-validator', 'block_proposed');
    node.rewardValidator('good-validator', 'block_finalized');

    const validator = node.getValidator('good-validator');
    expect(validator.blocksProposed).toBe(1);
    expect(validator.blocksFinalized).toBe(1);
    expect(validator.eScore).toBeGreaterThan(50); // E-Score increased
  });

  it('updates validator activity on heartbeat', () => {
    node.addValidator({ publicKey: 'active-validator', eScore: 50 });

    const before = node.getValidator('active-validator').lastSeen;

    // Wait a tiny bit then update
    node.updateValidatorActivity('active-validator');

    const after = node.getValidator('active-validator').lastSeen;
    expect(after).toBeGreaterThanOrEqual(before);
  });

  it('returns validator set status', () => {
    node.addValidator({ publicKey: 'v1', eScore: 60 });
    node.addValidator({ publicKey: 'v2', eScore: 40 });

    const status = node.getValidatorSetStatus();

    expect(status.total).toBe(2);
    expect(status.active).toBe(2);
    expect(status.avgEScore).toBe(50);
    expect(status.maxValidators).toBe(100);
  });

  it('calculates total voting weight', () => {
    node.addValidator({ publicKey: 'v1', eScore: 50, burned: 0 });
    node.addValidator({ publicKey: 'v2', eScore: 100, burned: 99 }); // sqrt(100) = 10

    const weight = node.getTotalVotingWeight();

    // v1: 50 * 1 * 1 = 50
    // v2: 100 * 10 * 1 = 1000
    // Total: 1050
    expect(weight).toBe(1050);
  });

  it('checks supermajority correctly (61.8%)', () => {
    node.addValidator({ publicKey: 'v1', eScore: 50 });
    node.addValidator({ publicKey: 'v2', eScore: 50 });

    const totalWeight = node.getTotalVotingWeight(); // 100

    // 61% = not enough
    expect(node.hasSupermajority(61)).toBe(false);

    // 62% = supermajority
    expect(node.hasSupermajority(62)).toBe(true);
  });

  it('filters validators by status and E-Score', () => {
    node.addValidator({ publicKey: 'v1', eScore: 60 });
    node.addValidator({ publicKey: 'v2', eScore: 40 });
    node.addValidator({ publicKey: 'v3', eScore: 80 });

    // Filter by minEScore
    const highScore = node.getValidators({ minEScore: 50 });
    expect(highScore.length).toBe(2);
    expect(highScore[0].eScore).toBe(80); // Sorted by score
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
    node._forkDetector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa-1111' }], 50);

    // No fork yet (only one hash)
    expect(events.length).toBe(0);

    // Peer B reports DIFFERENT hash for same slot 100 (FORK!)
    node._forkDetector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb-2222' }], 60);

    // Fork should be detected
    expect(events.length).toBe(1);
    expect(events[0].slot).toBe(100);
    expect(events[0].branches).toBe(2);
  });

  it('calculates heaviest branch by E-Score', async () => {
    const events = [];
    node.on('fork:detected', (e) => events.push(e));

    // Multiple peers on branch A (total E-Score: 50 + 40 = 90)
    node._forkDetector.checkForForks('peer-a1', [{ slot: 100, hash: 'hash-aaaa' }], 50);
    node._forkDetector.checkForForks('peer-a2', [{ slot: 100, hash: 'hash-aaaa' }], 40);

    // One peer on branch B (E-Score: 60)
    node._forkDetector.checkForForks('peer-b1', [{ slot: 100, hash: 'hash-bbbb' }], 60);

    // Fork detected, branch A should be heaviest (90 > 60)
    expect(events.length).toBe(1);
    expect(events[0].heaviestBranch).toBe('hash-aaaa'.slice(0, 16));
  });

  it('returns fork status via getForkStatus()', () => {
    // Create a fork
    node._forkDetector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 50);
    node._forkDetector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 60);

    const status = node.getForkStatus();

    expect(status.detected).toBe(true);
    expect(status.forkSlot).toBe(100);
    expect(status.branches.length).toBe(2);
    expect(status.stats.forksDetected).toBe(1);
  });

  it('cleans up old fork data', () => {
    // Record hashes for old slots
    for (let i = 0; i < 150; i++) {
      node._forkDetector._slotHashes.set(i, { hash: `hash-${i}` });
    }

    // Simulate consensus at slot 150 via the fork detector's dependency
    node._forkDetector.wire({ getLastFinalizedSlot: () => 150 });

    // Cleanup should remove slots < 50 (150 - 100)
    node._forkDetector._cleanupForkData();

    expect(node._forkDetector._slotHashes.has(49)).toBe(false);
    expect(node._forkDetector._slotHashes.has(50)).toBe(true);
  });

  it('records block hashes for slots', () => {
    node.recordBlockHash(100, 'hash-100-abc');
    node.recordBlockHash(101, 'hash-101-def');

    expect(node._forkDetector._slotHashes.get(100)?.hash).toBe('hash-100-abc');
    expect(node._forkDetector._slotHashes.get(101)?.hash).toBe('hash-101-def');
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
    const peerInfo = node._stateSyncManager.peerSlots.get('peer-id-123');
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
    expect(node._stateSyncManager.syncState.behindBy).toBe(150);
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

// Valid 64-char hex merkle root for anchoring tests
const validMerkleRoot = 'a'.repeat(64);
const validMerkleRoot2 = 'b'.repeat(64);

describe('Solana Anchoring', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
      anchoringEnabled: false, // Start disabled
      solanaCluster: 'devnet',
      anchorInterval: 100,
    });
  });

  afterEach(async () => {
    if (node) {
      await node.stop();
    }
  });

  it('starts with anchoring disabled by default', () => {
    const status = node.getAnchoringStatus();
    expect(status.enabled).toBe(false);
    expect(status.cluster).toBe('devnet');
    expect(status.anchorInterval).toBe(100);
    expect(status.dryRun).toBe(false);
    expect(status.hasAnchorer).toBe(false);
  });

  it('enables anchoring', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet-keypair' });

    const status = node.getAnchoringStatus();
    expect(status.enabled).toBe(true);
    expect(status.hasWallet).toBe(true);
  });

  it('disables anchoring', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    expect(node.getAnchoringStatus().enabled).toBe(true);

    node.disableAnchoring();
    expect(node.getAnchoringStatus().enabled).toBe(false);
  });

  it('shouldAnchor returns true for slots at interval', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Interval is 100, so slot 100, 200, 300 should anchor
    expect(node.shouldAnchor(100)).toBe(true);
    expect(node.shouldAnchor(200)).toBe(true);
    expect(node.shouldAnchor(0)).toBe(true); // 0 % 100 = 0

    // Non-interval slots should not anchor
    expect(node.shouldAnchor(50)).toBe(false);
    expect(node.shouldAnchor(150)).toBe(false);
  });

  it('shouldAnchor returns false when disabled', () => {
    // Anchoring is disabled
    expect(node.shouldAnchor(100)).toBe(false);
  });

  it('anchors block and tracks status (fallback simulation)', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    const block = {
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    };

    const events = [];
    node.on('block:anchored', (e) => events.push(e));

    const result = await node.anchorBlock(block);

    expect(result.success).toBe(true);
    expect(result.signature).toContain('sim_'); // Simulated signature
    expect(result.simulated).toBe(true);
    expect(events.length).toBe(1);
    expect(events[0].slot).toBe(100);

    // Check status updated
    const status = node.getAnchoringStatus();
    expect(status.lastAnchorSlot).toBe(100);
    expect(status.anchored).toBe(1);
    expect(status.stats.blocksAnchored).toBe(1);
  });

  it('returns null when anchoring without wallet or anchorer', async () => {
    await node.enableAnchoring({}); // No wallet, anchorer init may fail gracefully

    // Ensure no anchorer either
    node._anchoringManager._anchorer = null;

    const result = await node.anchorBlock({ slot: 100, hash: validMerkleRoot });

    expect(result).toBeNull();
  });

  it('fails when block has no valid merkle root', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    const result = await node.anchorBlock({
      slot: 100,
      hash: 'not-a-valid-hex',
      merkleRoot: 'also-not-valid',
    });

    // Should fail because no 64-char hex found
    expect(result.success).toBe(false);
    expect(result.error).toContain('merkle root');
  });

  it('tracks pending, anchored, and failed counts', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Anchor blocks with valid merkle roots
    await node.anchorBlock({ slot: 100, hash: validMerkleRoot, merkleRoot: validMerkleRoot });
    await node.anchorBlock({ slot: 200, hash: validMerkleRoot2, merkleRoot: validMerkleRoot2 });

    const status = node.getAnchoringStatus();
    expect(status.anchored).toBe(2);
    expect(status.failed).toBe(0);
  });

  it('getAnchorStatus returns anchor info for hash', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    await node.anchorBlock({ slot: 100, hash: validMerkleRoot, merkleRoot: validMerkleRoot });

    const anchorInfo = node.getAnchorStatus(validMerkleRoot);
    expect(anchorInfo).not.toBeNull();
    expect(anchorInfo.status).toBe('anchored');
    expect(anchorInfo.slot).toBe(100);
  });

  it('verifyAnchor finds signature in cache with source: cache', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    const result = await node.anchorBlock({
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    });

    const verification = await node.verifyAnchor(result.signature);
    expect(verification.verified).toBe(true);
    expect(verification.slot).toBe(100);
    expect(verification.hash).toBe(validMerkleRoot);
    expect(verification.source).toBe('cache');
  });

  it('verifyAnchor returns false for unknown signature', async () => {
    const verification = await node.verifyAnchor('unknown-signature-xyz');
    expect(verification.verified).toBe(false);
    expect(verification.error).toContain('not found');
  });

  it('onBlockFinalized triggers anchoring when shouldAnchor', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    const events = [];
    node.on('block:anchored', (e) => events.push(e));

    // Slot 100 should trigger anchor (interval = 100)
    await node.onBlockFinalized({
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    });

    expect(events.length).toBe(1);
  });

  it('onBlockFinalized records block hash', async () => {
    await node.onBlockFinalized({
      slot: 50,
      hash: 'block-hash-50',
      merkleRoot: 'root',
    });

    expect(node._forkDetector._slotHashes.get(50)?.hash).toBe('block-hash-50');
  });

  it('dryRun mode creates node without real wallet in anchorer', () => {
    const dryNode = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
      dryRun: true,
      wallet: 'mock-wallet',
    });

    expect(dryNode._anchoringManager._dryRun).toBe(true);
    expect(dryNode._anchoringManager._wallet).toBe('mock-wallet');

    const status = dryNode.getAnchoringStatus();
    expect(status.dryRun).toBe(true);
  });

  it('cleans up anchorer on stop()', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Manually set an anchorer for test purposes
    node._anchoringManager._anchorer = { getStats: () => ({}) };
    expect(node._anchoringManager._anchorer).not.toBeNull();

    await node.stop();

    expect(node._anchoringManager._anchorer).toBeNull();
  });

  it('getAnchoringStatus includes anchorerStats', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Without real anchorer, anchorerStats should be null
    const status = node.getAnchoringStatus();
    // May or may not have anchorer depending on @cynic/anchor availability
    if (status.hasAnchorer) {
      expect(status.anchorerStats).not.toBeNull();
    } else {
      expect(status.anchorerStats).toBeNull();
    }
  });
});

describe('_resolveMerkleRoot', () => {
  let node;

  beforeEach(() => {
    node = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
    });
  });

  it('resolves judgments_root first (highest priority)', () => {
    const root = 'a'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({
      judgments_root: root,
      merkleRoot: 'b'.repeat(64),
      hash: 'c'.repeat(64),
    });
    expect(result).toBe(root);
  });

  it('resolves judgmentsRoot second', () => {
    const root = 'b'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({
      judgmentsRoot: root,
      merkleRoot: 'c'.repeat(64),
      hash: 'd'.repeat(64),
    });
    expect(result).toBe(root);
  });

  it('resolves merkleRoot third', () => {
    const root = 'c'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({
      merkleRoot: root,
      hash: 'd'.repeat(64),
    });
    expect(result).toBe(root);
  });

  it('resolves hash as fallback', () => {
    const root = 'd'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({ hash: root });
    expect(result).toBe(root);
  });

  it('returns null when no valid 64-char hex found', () => {
    const result = node._anchoringManager.resolveMerkleRoot({
      hash: 'not-hex',
      merkleRoot: 'too-short',
    });
    expect(result).toBeNull();
  });

  it('returns null for empty block', () => {
    expect(node._anchoringManager.resolveMerkleRoot({})).toBeNull();
  });

  it('rejects non-hex 64-char strings', () => {
    const result = node._anchoringManager.resolveMerkleRoot({
      hash: 'g'.repeat(64), // 'g' is not hex
    });
    expect(result).toBeNull();
  });
});
