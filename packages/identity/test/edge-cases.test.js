/**
 * @cynic/identity Edge Cases Tests
 *
 * "Edge cases reveal the truth" - κυνικός
 *
 * Additional tests for edge cases, error handling, and coverage gaps.
 *
 * @module @cynic/identity/test/edge-cases
 */

import { test, describe, beforeEach } from 'node:test';
import assert from 'node:assert';
import fs from 'fs';
import path from 'path';
import os from 'os';

import {
  // Key management
  KeyManager,
  createKeyManager,
  generateKeypair,
  deriveNodeId,

  // Node identity
  NodeIdentity,
  createNodeIdentity,
  IdentityStatus,

  // Reputation
  ReputationGraph,
  createReputationGraph,
  TrustLevel,
  TRUST_DECAY_RATE,
  MAX_PROPAGATION_DEPTH,

  // E-Score 7D
  EScore7DCalculator,
  createEScore7DCalculator,
  calculateEScore7D,
  ESCORE_7D_DIMENSIONS,
  ESCORE_7D_TOTAL_WEIGHT,
} from '../src/index.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// KEY MANAGER EDGE CASES
// =============================================================================

describe('KeyManager Edge Cases', () => {
  test('load without keyfile throws', async () => {
    const km = createKeyManager(); // No keyfile

    await assert.rejects(
      async () => km.load(),
      { message: 'No keyfile configured' }
    );
  });

  test('save without keyfile throws', async () => {
    const km = createKeyManager();
    await km.generate(false);

    await assert.rejects(
      async () => km.save(),
      { message: 'No keyfile configured' }
    );
  });

  test('save without keys throws', async () => {
    const tmpDir = os.tmpdir();
    const keyfile = path.join(tmpDir, `cynic-test-nokeys-${Date.now()}.json`);
    const km = createKeyManager({ keyfile });

    await assert.rejects(
      async () => km.save(),
      { message: 'No keypair to save' }
    );
  });

  test('sign without private key throws', () => {
    const km = createKeyManager();

    assert.throws(
      () => km.sign('hello'),
      { message: 'No private key loaded' }
    );
  });

  test('verify without public key throws', () => {
    const km = createKeyManager();

    assert.throws(
      () => km.verify('hello', 'invalid_signature'),
      { message: 'No public key provided' }
    );
  });

  test('verify with invalid signature returns false', async () => {
    const km = createKeyManager();
    await km.generate(false);

    const valid = km.verify('hello', 'invalid_hex_signature');
    assert.strictEqual(valid, false);
  });

  test('constructor with pre-loaded keys', () => {
    const keypair = generateKeypair();
    const km = createKeyManager({
      publicKey: keypair.publicKey,
      privateKey: keypair.privateKey,
    });

    assert.ok(km.hasKeys);
    assert.ok(km.nodeId);
    assert.strictEqual(km.nodeId.length, 32);
  });

  test('formattedKey returns prefixed key', async () => {
    const km = createKeyManager();
    await km.generate(false);

    assert.ok(km.formattedKey.startsWith('ed25519:'));
    assert.strictEqual(km.formattedKey, `ed25519:${km.publicKey}`);
  });

  test('formattedKey returns null without keys', () => {
    const km = createKeyManager();
    assert.strictEqual(km.formattedKey, null);
  });

  test('loadOrGenerate loads if file exists', async () => {
    const tmpDir = os.tmpdir();
    const keyfile = path.join(tmpDir, `cynic-test-lor-${Date.now()}.json`);

    try {
      // First create a keyfile
      const km1 = createKeyManager({ keyfile });
      await km1.generate(true);
      const nodeId1 = km1.nodeId;

      // Now loadOrGenerate should load
      const km2 = createKeyManager({ keyfile });
      const wasLoaded = await km2.loadOrGenerate();

      assert.strictEqual(wasLoaded, true);
      assert.strictEqual(km2.nodeId, nodeId1);
    } finally {
      if (fs.existsSync(keyfile)) {
        fs.unlinkSync(keyfile);
      }
    }
  });

  test('loadOrGenerate generates if file missing', async () => {
    const tmpDir = os.tmpdir();
    const keyfile = path.join(tmpDir, `cynic-test-missing-${Date.now()}.json`);

    const km = createKeyManager({ keyfile });
    const wasLoaded = await km.loadOrGenerate();

    assert.strictEqual(wasLoaded, false);
    assert.ok(km.hasKeys);

    // Cleanup
    if (fs.existsSync(keyfile)) {
      fs.unlinkSync(keyfile);
    }
  });

  test('loadOrGenerate without keyfile generates', async () => {
    const km = createKeyManager();
    const wasLoaded = await km.loadOrGenerate();

    assert.strictEqual(wasLoaded, false);
    assert.ok(km.hasKeys);
  });

  test('clear removes all keys', async () => {
    const km = createKeyManager();
    await km.generate(false);

    assert.ok(km.hasKeys);

    km.clear();

    assert.strictEqual(km.hasKeys, false);
    assert.strictEqual(km.publicKey, null);
    assert.strictEqual(km.nodeId, null);
  });

  test('sign handles Buffer input', async () => {
    const km = createKeyManager();
    await km.generate(false);

    const data = Buffer.from('hello world');
    const sig = km.sign(data);

    assert.ok(sig);
    assert.ok(km.verify(data, sig));
  });

  test('load with invalid keyfile throws', async () => {
    const tmpDir = os.tmpdir();
    const keyfile = path.join(tmpDir, `cynic-test-invalid-${Date.now()}.json`);

    try {
      // Create invalid keyfile
      fs.writeFileSync(keyfile, JSON.stringify({ foo: 'bar' }));

      const km = createKeyManager({ keyfile });
      await assert.rejects(
        async () => km.load(),
        { message: /Invalid keyfile format/ }
      );
    } finally {
      if (fs.existsSync(keyfile)) {
        fs.unlinkSync(keyfile);
      }
    }
  });
});

// =============================================================================
// NODE IDENTITY EDGE CASES
// =============================================================================

describe('NodeIdentity Edge Cases', () => {
  test('sign before initialize throws', () => {
    const identity = createNodeIdentity();

    assert.throws(
      () => identity.sign('hello'),
      { message: /not initialized/i }
    );
  });

  test('signObject before initialize throws', () => {
    const identity = createNodeIdentity();

    assert.throws(
      () => identity.signObject({ foo: 'bar' }),
      { message: /not initialized/i }
    );
  });

  test('getStats before initialize throws', () => {
    const identity = createNodeIdentity();

    assert.throws(
      () => identity.getStats(),
      { message: /not initialized/i }
    );
  });

  test('exportPublic before initialize throws', () => {
    const identity = createNodeIdentity();

    assert.throws(
      () => identity.exportPublic(),
      { message: /not initialized/i }
    );
  });

  test('createAttestation before initialize throws', () => {
    const identity = createNodeIdentity();

    assert.throws(
      () => identity.createAttestation(),
      { message: /not initialized/i }
    );
  });

  test('status transitions to VERIFIED on burn', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    assert.strictEqual(identity.status, IdentityStatus.EPHEMERAL);

    // Record burn with tx signature
    await identity.recordBurn(1e6, 'tx_signature_123');

    assert.strictEqual(identity.status, IdentityStatus.VERIFIED);
  });

  test('addTag and removeTag work correctly', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    assert.strictEqual(identity.metadata.tags.length, 0);

    identity.addTag('validator');
    identity.addTag('premium');
    assert.strictEqual(identity.metadata.tags.length, 2);
    assert.ok(identity.metadata.tags.includes('validator'));

    // Adding same tag twice doesn't duplicate
    identity.addTag('validator');
    assert.strictEqual(identity.metadata.tags.length, 2);

    identity.removeTag('validator');
    assert.strictEqual(identity.metadata.tags.length, 1);
    assert.ok(!identity.metadata.tags.includes('validator'));

    // Removing non-existent tag is safe
    identity.removeTag('nonexistent');
    assert.strictEqual(identity.metadata.tags.length, 1);
  });

  test('markOffline updates E-Score calc', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    // Record some uptime
    identity.heartbeat();
    identity.heartbeat();

    // Mark offline
    identity.markOffline();

    // Should still work
    assert.ok(identity.eScore >= 0);
  });

  test('isTrusted property works', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    // New identity should not be trusted (E-Score too low)
    assert.strictEqual(identity.isTrusted, false);
  });

  test('getStats returns complete info', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    identity.heartbeat();
    await identity.recordBurn(1e6);
    identity.recordJudgment('j1', true);

    const stats = identity.getStats();

    assert.ok(stats.nodeId);
    assert.ok(stats.publicKey);
    assert.ok(stats.status);
    assert.ok(stats.metadata);
    assert.ok(stats.eScore);
    assert.ok(stats.stats);
  });

  test('verifyAttestation with null returns false', () => {
    assert.strictEqual(NodeIdentity.verifyAttestation(null), false);
  });

  test('verifyAttestation with missing fields returns false', () => {
    assert.strictEqual(NodeIdentity.verifyAttestation({}), false);
    assert.strictEqual(NodeIdentity.verifyAttestation({ signature: 'foo' }), false);
    assert.strictEqual(NodeIdentity.verifyAttestation({ publicKey: 'bar' }), false);
  });

  test('verifyAttestation with expired attestation returns false', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    // Create attestation with very short TTL
    const attestation = identity.createAttestation(1); // 1ms TTL

    // Wait for expiration
    await new Promise(r => setTimeout(r, 10));

    assert.strictEqual(NodeIdentity.verifyAttestation(attestation), false);
  });

  test('verifyAttestation with invalid signature returns false', async () => {
    const identity = createNodeIdentity();
    await identity.initialize();

    const attestation = identity.createAttestation();
    attestation.signature = 'invalid_signature';

    assert.strictEqual(NodeIdentity.verifyAttestation(attestation), false);
  });

  test('save and load with datafile', async () => {
    const tmpDir = os.tmpdir();
    const keyfile = path.join(tmpDir, `cynic-node-key-${Date.now()}.json`);
    const datafile = path.join(tmpDir, `cynic-node-data-${Date.now()}.json`);

    try {
      const identity1 = createNodeIdentity({ keyfile, datafile });
      await identity1.initialize();

      identity1.addTag('test');
      identity1.heartbeat();
      await identity1.recordBurn(5e6);
      await identity1.save();

      // Create new identity and load
      const identity2 = createNodeIdentity({ keyfile, datafile });
      await identity2.initialize();

      assert.strictEqual(identity2.nodeId, identity1.nodeId);
      assert.ok(identity2.metadata.tags.includes('test'));
    } finally {
      if (fs.existsSync(keyfile)) fs.unlinkSync(keyfile);
      if (fs.existsSync(datafile)) fs.unlinkSync(datafile);
    }
  });
});

// =============================================================================
// REPUTATION GRAPH EDGE CASES
// =============================================================================

describe('ReputationGraph Edge Cases', () => {
  let graph;

  beforeEach(() => {
    graph = createReputationGraph();
  });

  test('getTrust with self returns STRONG', () => {
    const trust = graph.getTrust('A', 'A');
    assert.strictEqual(trust, TrustLevel.STRONG);
  });

  test('findTrustPath with self returns single element', () => {
    const path = graph.findTrustPath('A', 'A');
    assert.deepStrictEqual(path, ['A']);
  });

  test('removeTrust returns false for non-existent', () => {
    const removed = graph.removeTrust('A', 'B');
    assert.strictEqual(removed, false);
  });

  test('removeTrust updates stats correctly', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    assert.strictEqual(graph.stats.totalRelations, 1);
    assert.strictEqual(graph.stats.trustRelations, 1);

    graph.removeTrust('A', 'B');
    assert.strictEqual(graph.stats.totalRelations, 0);
    assert.strictEqual(graph.stats.trustRelations, 0);
  });

  test('removeTrust from distrust updates stats', () => {
    graph.setTrust('A', 'B', TrustLevel.DISTRUST);
    assert.strictEqual(graph.stats.distrustRelations, 1);

    graph.removeTrust('A', 'B');
    assert.strictEqual(graph.stats.distrustRelations, 0);
  });

  test('setTrust normalizes level to [-1, 1]', () => {
    graph.setTrust('A', 'B', 5); // Above max
    const trust1 = graph.getDirectTrust('A', 'B');
    assert.ok(trust1 <= 1);

    graph.setTrust('A', 'C', -5); // Below min
    const trust2 = graph.getDirectTrust('A', 'C');
    assert.ok(trust2 >= -1);
  });

  test('getReputation without perspective (global)', () => {
    graph.setTrust('A', 'X', TrustLevel.STRONG);
    graph.setTrust('B', 'X', TrustLevel.MODERATE);

    const rep = graph.getReputation('X'); // No perspective

    assert.ok(rep.score);
    assert.ok(rep.trust !== undefined);
    assert.strictEqual(rep.trusters, 2);
    assert.strictEqual(rep.source, 'global');
  });

  test('getReputation for node with no trusters', () => {
    const rep = graph.getReputation('lonely_node');

    assert.ok(rep.score >= 0);
    assert.strictEqual(rep.trust, TrustLevel.UNKNOWN);
    assert.strictEqual(rep.trusters, 0);
    assert.strictEqual(rep.source, 'escore_only');
  });

  test('getReputation uses custom getEScore', () => {
    const customGraph = createReputationGraph({
      getEScore: (nodeId) => {
        if (nodeId === 'high') return 90;
        if (nodeId === 'low') return 10;
        return 50;
      },
    });

    const highRep = customGraph.getReputation('high');
    const lowRep = customGraph.getReputation('low');

    assert.strictEqual(highRep.score, 90);
    assert.strictEqual(lowRep.score, 10);
  });

  test('getTrust with maxDepth=1 only returns direct', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    graph.setTrust('B', 'C', TrustLevel.STRONG);

    const trust = graph.getTrust('A', 'C', 1);
    assert.strictEqual(trust, TrustLevel.UNKNOWN);
  });

  test('transitive trust stops at distrust', () => {
    graph.setTrust('A', 'B', TrustLevel.DISTRUST);
    graph.setTrust('B', 'C', TrustLevel.STRONG);

    const trust = graph.getTrust('A', 'C');
    assert.strictEqual(trust, TrustLevel.UNKNOWN);
  });

  test('transitive trust decays per hop', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    graph.setTrust('B', 'C', TrustLevel.STRONG);

    const trustAtoC = graph.getTrust('A', 'C');

    // Should decay: direct trust from B to C is 1.0, then φ⁻¹ decay applied
    // Result depends on algorithm: 1.0 * 1.0 * φ⁻¹ or (1.0 * φ⁻¹) * 1.0 * φ⁻¹
    // The important thing is it's positive and less than 1
    assert.ok(trustAtoC > 0, 'should have positive transitive trust');
    assert.ok(trustAtoC < 1, 'should decay from full trust');
    assert.ok(trustAtoC <= PHI_INV + 0.01, 'should not exceed one hop decay');
  });

  test('deep transitive trust has more decay', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    graph.setTrust('B', 'C', TrustLevel.STRONG);
    graph.setTrust('C', 'D', TrustLevel.STRONG);

    const trustAtoD = graph.getTrust('A', 'D');

    // Should be approximately φ⁻¹ * φ⁻¹ = φ⁻² ≈ 0.382
    assert.ok(trustAtoD > 0);
    assert.ok(trustAtoD < PHI_INV);
  });

  test('getTrustedBy with minTrust filter', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    graph.setTrust('A', 'C', TrustLevel.WEAK);
    graph.setTrust('A', 'D', TrustLevel.DISTRUST);

    const trusted = graph.getTrustedBy('A', 0.5);

    // Only B should pass (STRONG = 1.0)
    assert.strictEqual(trusted.length, 1);
    assert.strictEqual(trusted[0].nodeId, 'B');
  });

  test('getTrusters with minTrust filter', () => {
    graph.setTrust('X', 'A', TrustLevel.STRONG);
    graph.setTrust('Y', 'A', TrustLevel.WEAK);
    graph.setTrust('Z', 'A', TrustLevel.DISTRUST);

    const trusters = graph.getTrusters('A', 0.5);

    // Only X should pass
    assert.strictEqual(trusters.length, 1);
    assert.strictEqual(trusters[0].nodeId, 'X');
  });

  test('clear removes all data', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);
    graph.setTrust('B', 'C', TrustLevel.MODERATE);

    graph.clear();

    assert.strictEqual(graph.stats.totalRelations, 0);
    assert.strictEqual(graph.getDirectTrust('A', 'B'), TrustLevel.UNKNOWN);
  });

  test('trust decays over time', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);

    // Get relation and manipulate timestamp to simulate time passing
    const relation = graph.graph.get('A').get('B');
    relation.timestamp = Date.now() - (24 * 60 * 60 * 1000); // 1 day ago

    const decayedTrust = graph.getDirectTrust('A', 'B');

    // Should be approximately 1.0 * 0.618 = 0.618
    assert.ok(decayedTrust < 1.0);
    assert.ok(Math.abs(decayedTrust - TRUST_DECAY_RATE) < 0.01);
  });

  test('export includes all relations', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG, 'reason1');
    graph.setTrust('B', 'C', TrustLevel.MODERATE, 'reason2');

    const exported = graph.export();

    assert.strictEqual(exported.relations.length, 2);
    assert.ok(exported.exportedAt);
  });

  test('import preserves timestamps', () => {
    graph.setTrust('A', 'B', TrustLevel.STRONG);

    // Manipulate timestamp
    const originalTimestamp = Date.now() - 1000000;
    graph.graph.get('A').get('B').timestamp = originalTimestamp;

    const exported = graph.export();

    const newGraph = createReputationGraph();
    newGraph.import(exported);

    const relation = newGraph.graph.get('A').get('B');
    assert.strictEqual(relation.timestamp, originalTimestamp);
  });

  test('constants are correct', () => {
    assert.ok(Math.abs(TRUST_DECAY_RATE - PHI_INV) < 0.001);
    assert.strictEqual(MAX_PROPAGATION_DEPTH, 3);
  });
});

// =============================================================================
// E-SCORE 7D EDGE CASES
// =============================================================================

describe('EScore7DCalculator Edge Cases', () => {
  test('zero burns stay at zero', () => {
    const calc = createEScore7DCalculator();

    assert.strictEqual(calc.totalBurned, 0);

    const result = calc.calculate();
    assert.strictEqual(result.breakdown.burn, 0);
  });

  test('burns accumulate correctly', () => {
    const calc = createEScore7DCalculator();

    calc.recordBurn(1000);
    calc.recordBurn(2000);
    assert.strictEqual(calc.totalBurned, 3000);
  });

  test('recordBlock increments counter', () => {
    const calc = createEScore7DCalculator();

    calc.recordBlock();
    calc.recordBlock();
    calc.recordBlock();

    assert.strictEqual(calc.blocksProcessed, 3);
  });

  test('recordIssue increments counter', () => {
    const calc = createEScore7DCalculator();

    calc.recordIssue();
    calc.recordIssue();

    assert.strictEqual(calc.issues, 2);
  });

  test('setContentQualityScore updates value', () => {
    const calc = createEScore7DCalculator();

    calc.setContentQualityScore(0.9);
    assert.strictEqual(calc.contentQualityScore, 0.9);

    // Clamp to [0, 1]
    calc.setContentQualityScore(1.5);
    assert.strictEqual(calc.contentQualityScore, 1);

    calc.setContentQualityScore(-0.5);
    assert.strictEqual(calc.contentQualityScore, 0);
  });

  test('all dimensions contribute to score', () => {
    // Use calculateEScore7D directly with all data
    const result = calculateEScore7D({
      burn: { totalBurned: 1e9 },
      build: { commits: 10, prs: 5, issues: 3 },
      judge: { agreementCount: 10, totalJudgments: 10, contentQualityScore: 0.8 },
      run: { uptimeSeconds: 3600, expectedUptimeSeconds: 3600, blocksProcessed: 100 },
      social: { qualityScore: 0.9, contentCount: 10, relevanceScore: 0.9 },
      graph: {
        trustReceived: 0.8,
        transitiveScore: 0.7,
        trustedByCount: 10,
        networkSize: 100,
      },
      hold: { holdings: 1e9 },
    });

    // All breakdown dimensions should be > 0
    assert.ok(result.breakdown.burn > 0, 'burn should contribute');
    assert.ok(result.breakdown.build > 0, 'build should contribute');
    assert.ok(result.breakdown.judge > 0, 'judge should contribute');
    assert.ok(result.breakdown.run > 0, 'run should contribute');
    assert.ok(result.breakdown.social > 0, 'social should contribute');
    assert.ok(result.breakdown.graph > 0, 'graph should contribute');
    assert.ok(result.breakdown.hold > 0, 'hold should contribute');
  });

  test('total weight constant is accurate', () => {
    // Sum all dimension weights
    let totalWeight = 0;
    for (const dim of Object.values(ESCORE_7D_DIMENSIONS)) {
      totalWeight += dim.weight;
    }

    assert.ok(Math.abs(totalWeight - ESCORE_7D_TOTAL_WEIGHT) < 0.001);
  });

  test('empty calculateEScore7D returns 0', () => {
    const result = calculateEScore7D({});
    assert.strictEqual(result.score, 0);
    assert.strictEqual(result.trustLevel, 'OBSERVER');
  });
});

// =============================================================================
// DERIVE NODE ID EDGE CASES
// =============================================================================

describe('deriveNodeId Edge Cases', () => {
  test('different keys produce different IDs', () => {
    const keypair1 = generateKeypair();
    const keypair2 = generateKeypair();

    const id1 = deriveNodeId(keypair1.publicKey);
    const id2 = deriveNodeId(keypair2.publicKey);

    assert.notStrictEqual(id1, id2);
  });

  test('ID is always 32 hex chars', () => {
    for (let i = 0; i < 10; i++) {
      const keypair = generateKeypair();
      const id = deriveNodeId(keypair.publicKey);

      assert.strictEqual(id.length, 32);
      assert.ok(/^[0-9a-f]+$/.test(id));
    }
  });
});
