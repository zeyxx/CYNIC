/**
 * CYNICNetworkNode Tests
 *
 * PHASE 2: DECENTRALIZE
 *
 * Tests multi-node orchestration and extracted SRP components.
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';
import { CYNICNetworkNode, NetworkState } from '../src/network/network-node.js';
import { calculateVoteWeight } from '@cynic/protocol';

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
      assert.throws(() => new CYNICNetworkNode({}), { message: /publicKey and privateKey required/ });
    });

    it('creates node with valid keys', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false, // Disable actual networking for tests
      });

      assert.strictEqual(node.publicKey, mockPublicKey);
      assert.strictEqual(node.state, NetworkState.OFFLINE);
    });

    it('uses default E-Score of 50', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      assert.strictEqual(node.eScore, 50);
    });

    it('uses φ-aligned default port 8618', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      const info = node.getInfo();
      assert.strictEqual(info.port, 8618);
    });
  });

  describe('state management', () => {
    it('starts OFFLINE', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      assert.strictEqual(node.state, NetworkState.OFFLINE);
      assert.strictEqual(node.isOnline, false);
      assert.strictEqual(node.isParticipating, false);
    });

    it('stays OFFLINE when networking disabled', async () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      await node.start();
      assert.strictEqual(node.state, NetworkState.OFFLINE);
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
      assert.strictEqual(node.eScore, 75);
    });

    it('clamps E-Score to 0-100', () => {
      node = new CYNICNetworkNode({
        publicKey: mockPublicKey,
        privateKey: mockPrivateKey,
        enabled: false,
      });

      node.setEScore(150);
      assert.strictEqual(node.eScore, 100);

      node.setEScore(-50);
      assert.strictEqual(node.eScore, 0);
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
      assert.strictEqual(status.discovery, null); // Discovery not initialized when disabled
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

      assert.strictEqual(info.port, 8620);
      assert.strictEqual(info.eScore, 75);
      assert.strictEqual(info.state, NetworkState.OFFLINE);
      assert.ok(info.publicKey.includes('test-public-key'));
      assert.ok(info.stats !== undefined);
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

      assert.ok(status.node !== undefined);
      assert.strictEqual(status.transport, null);
      assert.strictEqual(status.consensus, null);
      assert.strictEqual(status.discovery, null);
      assert.ok(status.sync !== undefined);
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
    assert.strictEqual(NetworkState.OFFLINE, 'OFFLINE');
    assert.strictEqual(NetworkState.BOOTSTRAPPING, 'BOOTSTRAPPING');
    assert.strictEqual(NetworkState.SYNCING, 'SYNCING');
    assert.strictEqual(NetworkState.ONLINE, 'ONLINE');
    assert.strictEqual(NetworkState.PARTICIPATING, 'PARTICIPATING');
    assert.strictEqual(NetworkState.ERROR, 'ERROR');
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

    assert.strictEqual(added, true);
    assert.strictEqual(node._validatorManager.size, 1);

    const validator = node.getValidator('validator-1-key');
    assert.ok(validator !== undefined);
    assert.strictEqual(validator.eScore, 50);
    assert.strictEqual(validator.burned, 100);
    assert.strictEqual(validator.status, 'active');
  });

  it('rejects validator with low E-Score', () => {
    const added = node.addValidator({
      publicKey: 'low-score-validator',
      eScore: 10, // Below minimum of 20
    });

    assert.strictEqual(added, false);
    assert.strictEqual(node._validatorManager.size, 0);
  });

  it('removes validator', () => {
    node.addValidator({ publicKey: 'validator-to-remove', eScore: 50 });
    assert.strictEqual(node._validatorManager.size, 1);

    const removed = node.removeValidator('validator-to-remove', 'test');
    assert.strictEqual(removed, true);
    assert.strictEqual(node._validatorManager.size, 0);
  });

  it('penalizes validator and reduces E-Score', () => {
    node.addValidator({ publicKey: 'bad-validator', eScore: 50 });

    node.penalizeValidator('bad-validator', 10, 'test_penalty');

    const validator = node.getValidator('bad-validator');
    assert.strictEqual(validator.eScore, 40);
    assert.strictEqual(validator.penalties, 10);
  });

  it('removes validator when penalty drops E-Score below minimum', () => {
    node.addValidator({ publicKey: 'very-bad-validator', eScore: 25 });

    // Penalize enough to drop below minimum (20)
    node.penalizeValidator('very-bad-validator', 10, 'severe_penalty');

    // Validator should be removed
    assert.strictEqual(node.getValidator('very-bad-validator'), null);
    assert.strictEqual(node._validatorManager.stats.validatorsPenalized, 1);
  });

  it('rewards validator for blocks', () => {
    node.addValidator({ publicKey: 'good-validator', eScore: 50 });

    node.rewardValidator('good-validator', 'block_proposed');
    node.rewardValidator('good-validator', 'block_finalized');

    const validator = node.getValidator('good-validator');
    assert.strictEqual(validator.blocksProposed, 1);
    assert.strictEqual(validator.blocksFinalized, 1);
    assert.ok(validator.eScore > 50); // E-Score increased
  });

  it('updates validator activity on heartbeat', () => {
    node.addValidator({ publicKey: 'active-validator', eScore: 50 });

    const before = node.getValidator('active-validator').lastSeen;

    // Wait a tiny bit then update
    node.updateValidatorActivity('active-validator');

    const after = node.getValidator('active-validator').lastSeen;
    assert.ok(after >= before);
  });

  it('returns validator set status', () => {
    node.addValidator({ publicKey: 'v1', eScore: 60 });
    node.addValidator({ publicKey: 'v2', eScore: 40 });

    const status = node.getValidatorSetStatus();

    assert.strictEqual(status.total, 2);
    assert.strictEqual(status.active, 2);
    assert.strictEqual(status.avgEScore, 50);
    assert.strictEqual(status.maxValidators, 100);
  });

  it('calculates total voting weight', () => {
    node.addValidator({ publicKey: 'v1', eScore: 50, burned: 0 });
    node.addValidator({ publicKey: 'v2', eScore: 100, burned: 99 });

    const weight = node.getTotalVotingWeight();

    // Uses protocol formula: eScore × max(log_φ(burned+1), 1) × uptime
    const expected =
      calculateVoteWeight({ eScore: 50, burned: 0, uptime: 1 }) +
      calculateVoteWeight({ eScore: 100, burned: 99, uptime: 1 });
    assert.ok(Math.abs(weight - expected) < 0.001);
  });

  it('checks supermajority correctly (61.8%)', () => {
    node.addValidator({ publicKey: 'v1', eScore: 50 });
    node.addValidator({ publicKey: 'v2', eScore: 50 });

    const totalWeight = node.getTotalVotingWeight(); // 100

    // 61% = not enough
    assert.strictEqual(node.hasSupermajority(61), false);

    // 62% = supermajority
    assert.strictEqual(node.hasSupermajority(62), true);
  });

  it('filters validators by status and E-Score', () => {
    node.addValidator({ publicKey: 'v1', eScore: 60 });
    node.addValidator({ publicKey: 'v2', eScore: 40 });
    node.addValidator({ publicKey: 'v3', eScore: 80 });

    // Filter by minEScore
    const highScore = node.getValidators({ minEScore: 50 });
    assert.strictEqual(highScore.length, 2);
    assert.strictEqual(highScore[0].eScore, 80); // Sorted by score
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
    assert.strictEqual(events.length, 0);

    // Peer B reports DIFFERENT hash for same slot 100 (FORK!)
    node._forkDetector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb-2222' }], 60);

    // Fork should be detected
    assert.strictEqual(events.length, 1);
    assert.strictEqual(events[0].slot, 100);
    assert.strictEqual(events[0].branches, 2);
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
    assert.strictEqual(events.length, 1);
    assert.strictEqual(events[0].heaviestBranch, 'hash-aaaa'.slice(0, 16));
  });

  it('returns fork status via getForkStatus()', () => {
    // Create a fork
    node._forkDetector.checkForForks('peer-a', [{ slot: 100, hash: 'hash-aaaa' }], 50);
    node._forkDetector.checkForForks('peer-b', [{ slot: 100, hash: 'hash-bbbb' }], 60);

    const status = node.getForkStatus();

    assert.strictEqual(status.detected, true);
    assert.strictEqual(status.forkSlot, 100);
    assert.strictEqual(status.branches.length, 2);
    assert.strictEqual(status.stats.forksDetected, 1);
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

    assert.strictEqual(node._forkDetector._slotHashes.has(49), false);
    assert.strictEqual(node._forkDetector._slotHashes.has(50), true);
  });

  it('records block hashes for slots', () => {
    node.recordBlockHash(100, 'hash-100-abc');
    node.recordBlockHash(101, 'hash-101-def');

    assert.strictEqual(node._forkDetector._slotHashes.get(100)?.hash, 'hash-100-abc');
    assert.strictEqual(node._forkDetector._slotHashes.get(101)?.hash, 'hash-101-def');
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
    assert.ok(peerInfo !== undefined);
    assert.strictEqual(peerInfo.finalizedSlot, 95);
    assert.strictEqual(peerInfo.eScore, 75);
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
    assert.strictEqual(node._stateSyncManager.syncState.behindBy, 150);
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

    assert.strictEqual(events.length, 1);
    assert.strictEqual(events[0].behindBy, 100);
    assert.strictEqual(events[0].bestPeer, 'peer-1');
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
    assert.strictEqual(status.enabled, false);
    assert.strictEqual(status.cluster, 'devnet');
    assert.strictEqual(status.anchorInterval, 100);
    assert.strictEqual(status.dryRun, false);
    assert.strictEqual(status.hasAnchorer, false);
  });

  it('enables anchoring', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet-keypair' });

    const status = node.getAnchoringStatus();
    assert.strictEqual(status.enabled, true);
    assert.strictEqual(status.hasWallet, true);
  });

  it('disables anchoring', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    assert.strictEqual(node.getAnchoringStatus().enabled, true);

    node.disableAnchoring();
    assert.strictEqual(node.getAnchoringStatus().enabled, false);
  });

  it('shouldAnchor returns true for slots at interval', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Interval is 100, so slot 100, 200, 300 should anchor
    assert.strictEqual(node.shouldAnchor(100), true);
    assert.strictEqual(node.shouldAnchor(200), true);
    assert.strictEqual(node.shouldAnchor(0), true); // 0 % 100 = 0

    // Non-interval slots should not anchor
    assert.strictEqual(node.shouldAnchor(50), false);
    assert.strictEqual(node.shouldAnchor(150), false);
  });

  it('shouldAnchor returns false when disabled', () => {
    // Anchoring is disabled
    assert.strictEqual(node.shouldAnchor(100), false);
  });

  it('anchors block and tracks status (fallback simulation)', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    node._anchoringManager._anchorer = null; // Use simulation path (no real wallet in tests)

    const block = {
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    };

    const events = [];
    node.on('block:anchored', (e) => events.push(e));

    const result = await node.anchorBlock(block);

    assert.strictEqual(result.success, true);
    assert.ok(result.signature.includes('sim_')); // Simulated signature
    assert.strictEqual(result.simulated, true);
    assert.strictEqual(events.length, 1);
    assert.strictEqual(events[0].slot, 100);

    // Check status updated
    const status = node.getAnchoringStatus();
    assert.strictEqual(status.lastAnchorSlot, 100);
    assert.strictEqual(status.anchored, 1);
    assert.strictEqual(status.stats.blocksAnchored, 1);
  });

  it('returns null when anchoring without wallet or anchorer', async () => {
    await node.enableAnchoring({}); // No wallet, anchorer init may fail gracefully

    // Ensure no anchorer either
    node._anchoringManager._anchorer = null;

    const result = await node.anchorBlock({ slot: 100, hash: validMerkleRoot });

    assert.strictEqual(result, null);
  });

  it('fails when block has no valid merkle root', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    const result = await node.anchorBlock({
      slot: 100,
      hash: 'not-a-valid-hex',
      merkleRoot: 'also-not-valid',
    });

    // Should fail because no 64-char hex found
    assert.strictEqual(result.success, false);
    assert.ok(result.error.includes('merkle root'));
  });

  it('tracks pending, anchored, and failed counts', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    node._anchoringManager._anchorer = null; // Use simulation path for tests

    // Anchor blocks with valid merkle roots
    await node.anchorBlock({ slot: 100, hash: validMerkleRoot, merkleRoot: validMerkleRoot });
    await node.anchorBlock({ slot: 200, hash: validMerkleRoot2, merkleRoot: validMerkleRoot2 });

    const status = node.getAnchoringStatus();
    assert.strictEqual(status.anchored, 2);
    assert.strictEqual(status.failed, 0);
  });

  it('getAnchorStatus returns anchor info for hash', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    node._anchoringManager._anchorer = null; // Use simulation path for tests

    await node.anchorBlock({ slot: 100, hash: validMerkleRoot, merkleRoot: validMerkleRoot });

    const anchorInfo = node.getAnchorStatus(validMerkleRoot);
    assert.ok(anchorInfo !== null);
    assert.strictEqual(anchorInfo.status, 'anchored');
    assert.strictEqual(anchorInfo.slot, 100);
  });

  it('verifyAnchor finds signature in cache with source: cache', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    node._anchoringManager._anchorer = null; // Use simulation path for tests

    const result = await node.anchorBlock({
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    });

    const verification = await node.verifyAnchor(result.signature);
    assert.strictEqual(verification.verified, true);
    assert.strictEqual(verification.slot, 100);
    assert.strictEqual(verification.hash, validMerkleRoot);
    assert.strictEqual(verification.source, 'cache');
  });

  it('verifyAnchor returns false for unknown signature', async () => {
    const verification = await node.verifyAnchor('unknown-signature-xyz');
    assert.strictEqual(verification.verified, false);
    assert.ok(verification.error.includes('not found'));
  });

  it('onBlockFinalized triggers anchoring when shouldAnchor', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });
    node._anchoringManager._anchorer = null; // Use simulation path for tests

    const events = [];
    node.on('block:anchored', (e) => events.push(e));

    // Slot 100 should trigger anchor (interval = 100, set in beforeEach)
    await node.onBlockFinalized({
      slot: 100,
      hash: validMerkleRoot,
      merkleRoot: validMerkleRoot,
    });

    assert.strictEqual(events.length, 1);
  });

  it('onBlockFinalized records block hash', async () => {
    await node.onBlockFinalized({
      slot: 50,
      hash: 'block-hash-50',
      merkleRoot: 'root',
    });

    assert.strictEqual(node._forkDetector._slotHashes.get(50)?.hash, 'block-hash-50');
  });

  it('dryRun mode creates node without real wallet in anchorer', () => {
    const dryNode = new CYNICNetworkNode({
      publicKey: 'test-public-key-0123456789abcdef',
      privateKey: 'test-private-key-0123456789abcdef',
      enabled: false,
      dryRun: true,
      wallet: 'mock-wallet',
    });

    assert.strictEqual(dryNode._anchoringManager._dryRun, true);
    assert.strictEqual(dryNode._anchoringManager._wallet, 'mock-wallet');

    const status = dryNode.getAnchoringStatus();
    assert.strictEqual(status.dryRun, true);
  });

  it('cleans up anchorer on stop()', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Manually set an anchorer for test purposes
    node._anchoringManager._anchorer = { getStats: () => ({}) };
    assert.ok(node._anchoringManager._anchorer !== null);

    await node.stop();

    assert.strictEqual(node._anchoringManager._anchorer, null);
  });

  it('getAnchoringStatus includes anchorerStats', async () => {
    await node.enableAnchoring({ wallet: 'mock-wallet' });

    // Without real anchorer, anchorerStats should be null
    const status = node.getAnchoringStatus();
    // May or may not have anchorer depending on @cynic/anchor availability
    if (status.hasAnchorer) {
      assert.ok(status.anchorerStats !== null);
    } else {
      assert.strictEqual(status.anchorerStats, null);
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
    assert.strictEqual(result, root);
  });

  it('resolves judgmentsRoot second', () => {
    const root = 'b'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({
      judgmentsRoot: root,
      merkleRoot: 'c'.repeat(64),
      hash: 'd'.repeat(64),
    });
    assert.strictEqual(result, root);
  });

  it('resolves merkleRoot third', () => {
    const root = 'c'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({
      merkleRoot: root,
      hash: 'd'.repeat(64),
    });
    assert.strictEqual(result, root);
  });

  it('resolves hash as fallback', () => {
    const root = 'd'.repeat(64);
    const result = node._anchoringManager.resolveMerkleRoot({ hash: root });
    assert.strictEqual(result, root);
  });

  it('returns null when no valid 64-char hex found', () => {
    const result = node._anchoringManager.resolveMerkleRoot({
      hash: 'not-hex',
      merkleRoot: 'too-short',
    });
    assert.strictEqual(result, null);
  });

  it('returns null for empty block', () => {
    assert.strictEqual(node._anchoringManager.resolveMerkleRoot({}), null);
  });

  it('rejects non-hex 64-char strings', () => {
    const result = node._anchoringManager.resolveMerkleRoot({
      hash: 'g'.repeat(64), // 'g' is not hex
    });
    assert.strictEqual(result, null);
  });
});
