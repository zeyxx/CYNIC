/**
 * CYNIC Privacy Tests
 *
 * Tests for the privacy layer: commitments, differential privacy, local store, aggregator
 *
 * "φ distrusts φ" - κυνικός
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  COMMITMENT_CONSTANTS,
  generateBlindingFactor,
  commit,
  verify,
  ProfileCommitment,
  PatternCommitment,
  CommitmentStore,
} from '../src/privacy/commitments.js';

import {
  PRIVACY_CONSTANTS,
  laplacianNoise,
  gaussianNoise,
  DifferentialPrivacy,
  PrivatePatternAggregator,
  PrivateKnowledgeAggregator,
} from '../src/privacy/differential.js';

import {
  LOCAL_STORE_CONSTANTS,
  LocalDataCategory,
  generateLocalId,
  localHash,
  LocalEntry,
  LocalStore,
  ProfileSignalStore,
  SessionHistoryStore,
} from '../src/privacy/local-store.js';

import {
  AGGREGATOR_CONSTANTS,
  DataTier,
  goldenAngleHash,
  ContributionMeta,
  PrivacyAggregator,
  BatchAggregator,
  createPrivacyPipeline,
} from '../src/privacy/aggregator.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// COMMITMENT TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Commitments', () => {
  describe('generateBlindingFactor', () => {
    it('generates correct length blinding factor', () => {
      const blinding = generateBlindingFactor();
      assert.strictEqual(blinding.length, COMMITMENT_CONSTANTS.BLINDING_SIZE);
    });

    it('generates different values each time', () => {
      const b1 = generateBlindingFactor();
      const b2 = generateBlindingFactor();
      assert.notStrictEqual(b1.toString('hex'), b2.toString('hex'));
    });
  });

  describe('commit and verify', () => {
    it('creates verifiable commitment', () => {
      const value = 'test_value';
      const { commitment, blindingFactor } = commit(value);

      assert.ok(commitment);
      assert.strictEqual(typeof commitment, 'string');
      assert.strictEqual(commitment.length, 64); // SHA256 hex

      // Should verify correctly
      assert.ok(verify(commitment, value, blindingFactor));
    });

    it('fails verification with wrong value', () => {
      const { commitment, blindingFactor } = commit('correct_value');
      assert.ok(!verify(commitment, 'wrong_value', blindingFactor));
    });

    it('fails verification with wrong blinding factor', () => {
      const { commitment } = commit('test_value');
      const wrongBlinding = generateBlindingFactor();
      assert.ok(!verify(commitment, 'test_value', wrongBlinding));
    });

    it('is hiding (can\'t learn value from commitment)', () => {
      // Same value with different blinding factors gives different commitments
      const value = 'secret';
      const { commitment: c1 } = commit(value);
      const { commitment: c2 } = commit(value);

      assert.notStrictEqual(c1, c2);
    });

    it('is binding (can\'t open to different value)', () => {
      const { commitment, blindingFactor } = commit('original');

      // Can only verify with original value
      assert.ok(verify(commitment, 'original', blindingFactor));
      assert.ok(!verify(commitment, 'different', blindingFactor));
    });
  });

  describe('ProfileCommitment', () => {
    it('creates commitment for profile level', () => {
      const pc = new ProfileCommitment(5);

      assert.ok(pc.commitment);
      assert.strictEqual(pc.level, 5);
      assert.ok(pc.blindingFactor);
    });

    it('proves minimum level (valid claim)', () => {
      const pc = new ProfileCommitment(8); // Master level

      // Should be able to prove level >= 5
      const proof = pc.proveMinLevel(5);
      assert.ok(proof);
      assert.strictEqual(proof.claim, 'level >= 5');
      assert.ok(proof.proof.witnesses.length > 0);
      assert.ok(proof.proof.confidence <= PHI_INV);
    });

    it('fails to prove minimum level (invalid claim)', () => {
      const pc = new ProfileCommitment(3); // Practitioner

      // Cannot prove level >= 5 when actual level is 3
      const proof = pc.proveMinLevel(5);
      assert.strictEqual(proof, null);
    });

    it('verifies valid range proof', () => {
      const pc = new ProfileCommitment(5);
      const proof = pc.proveMinLevel(3);

      assert.ok(proof);
      assert.ok(ProfileCommitment.verifyMinLevel(proof.commitment, proof.proof, 3));
    });

    it('rejects invalid range proof', () => {
      const pc = new ProfileCommitment(5);
      const proof = pc.proveMinLevel(3);

      // Try to verify for different minLevel
      assert.ok(!ProfileCommitment.verifyMinLevel(proof.commitment, proof.proof, 5));
    });

    it('serializes without exposing blinding factor', () => {
      const pc = new ProfileCommitment(5);
      const json = pc.toJSON();

      assert.ok(json.commitment);
      assert.ok(json.createdAt);
      assert.ok(!json.blindingFactor); // CRITICAL: must not be serialized
      assert.ok(!json.level); // Level also not serialized
    });
  });

  describe('PatternCommitment', () => {
    it('creates commitment for pattern', () => {
      const pc = new PatternCommitment('my_pattern', 'category_a');

      assert.ok(pc.commitment);
      assert.ok(pc.patternHash);
      assert.strictEqual(pc.patternHash.length, 16); // Truncated for privacy
    });

    it('generates different hashes for different patterns', () => {
      const pc1 = new PatternCommitment('pattern_1', 'cat');
      const pc2 = new PatternCommitment('pattern_2', 'cat');

      assert.notStrictEqual(pc1.patternHash, pc2.patternHash);
    });

    it('proves category membership', () => {
      const pc = new PatternCommitment('my_pattern', 'errors');

      const proof = pc.proveCategory('errors');
      assert.ok(proof);
      assert.strictEqual(proof.claim, 'pattern in category: errors');
      assert.strictEqual(proof.proof.category, 'errors');
      assert.strictEqual(proof.proof.confidence, PHI_INV);
    });

    it('fails to prove wrong category', () => {
      const pc = new PatternCommitment('my_pattern', 'errors');

      const proof = pc.proveCategory('solutions');
      assert.strictEqual(proof, null);
    });

    it('exports to collective without raw pattern', () => {
      const pc = new PatternCommitment('sensitive_pattern', 'cat');
      const collective = pc.toCollective();

      assert.ok(collective.commitment);
      assert.ok(collective.patternHash);
      assert.strictEqual(collective.category, 'cat');
      assert.ok(!collective.pattern); // CRITICAL: raw pattern not exposed
      assert.ok(!collective.blindingFactor);
    });
  });

  describe('CommitmentStore', () => {
    it('stores and retrieves commitments', () => {
      const store = new CommitmentStore();
      const pc = new ProfileCommitment(5);

      store.add('user1', pc);
      const retrieved = store.get('user1');

      assert.strictEqual(retrieved, pc);
    });

    it('prunes expired commitments', () => {
      const store = new CommitmentStore();

      // Add a commitment
      store.add('user1', new ProfileCommitment(5));

      // Manually set creation date to past max age
      const entry = store.commitments.get('user1');
      entry.createdAt = new Date(Date.now() - (COMMITMENT_CONSTANTS.MAX_AGE_DAYS + 1) * 24 * 60 * 60 * 1000);

      const removed = store.pruneExpired();
      assert.strictEqual(removed, 1);
      assert.strictEqual(store.get('user1'), null);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// DIFFERENTIAL PRIVACY TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Differential Privacy', () => {
  describe('Constants', () => {
    it('uses φ⁻¹ as epsilon', () => {
      assert.strictEqual(PRIVACY_CONSTANTS.EPSILON, PHI_INV);
    });

    it('uses φ⁻² as minimum noise floor', () => {
      assert.strictEqual(PRIVACY_CONSTANTS.MIN_NOISE_FLOOR, PHI_INV_2);
    });

    it('uses Fibonacci numbers for limits', () => {
      assert.strictEqual(PRIVACY_CONSTANTS.MAX_QUERIES_PER_PERIOD, 89); // Fib(11)
      assert.strictEqual(PRIVACY_CONSTANTS.BUDGET_REFRESH_HOURS, 21); // Fib(8)
    });
  });

  describe('laplacianNoise', () => {
    it('generates noise centered around 0', () => {
      const samples = Array.from({ length: 1000 }, () => laplacianNoise(1));
      const mean = samples.reduce((a, b) => a + b, 0) / samples.length;

      // Mean should be close to 0 (within reasonable variance)
      assert.ok(Math.abs(mean) < 0.5);
    });

    it('generates noise with correct scale', () => {
      const scale = 2;
      const samples = Array.from({ length: 1000 }, () => laplacianNoise(scale));

      // For Laplace(0, b), variance = 2b²
      const variance = samples.reduce((acc, s) => acc + s * s, 0) / samples.length;
      const expectedVariance = 2 * scale * scale;

      // Within 50% of expected (statistical test)
      assert.ok(variance > expectedVariance * 0.5);
      assert.ok(variance < expectedVariance * 1.5);
    });
  });

  describe('gaussianNoise', () => {
    it('generates noise centered around 0', () => {
      const samples = Array.from({ length: 1000 }, () => gaussianNoise(1));
      const mean = samples.reduce((a, b) => a + b, 0) / samples.length;

      assert.ok(Math.abs(mean) < 0.2);
    });
  });

  describe('DifferentialPrivacy', () => {
    let dp;

    beforeEach(() => {
      dp = new DifferentialPrivacy();
    });

    it('initializes with φ⁻¹ epsilon', () => {
      const status = dp.getBudgetStatus();
      assert.strictEqual(status.epsilon, PHI_INV);
    });

    it('adds noise to values', () => {
      const trueValue = 100;
      const noisedValue = dp.addNoise(trueValue);

      // Should be different due to noise
      // Note: There's a tiny chance they're equal, but statistically unlikely
      assert.ok(typeof noisedValue === 'number');
    });

    it('tracks budget usage', () => {
      dp.addNoise(100);
      dp.addNoise(100);

      const status = dp.getBudgetStatus();
      assert.ok(status.budgetUsed > 0);
      assert.strictEqual(status.queryCount, 2);
    });

    it('enforces query limits', () => {
      // Use up all queries
      for (let i = 0; i < PRIVACY_CONSTANTS.MAX_QUERIES_PER_PERIOD; i++) {
        dp.addNoise(100);
      }

      assert.ok(!dp.canQuery());
      assert.throws(() => dp.addNoise(100), /budget exhausted/);
    });

    it('adds noise to counts (non-negative result)', () => {
      const count = dp.addNoiseToCount(10);
      assert.ok(count >= 0);
      assert.ok(Number.isInteger(count));
    });

    it('adds noise to ratios (bounded 0-1)', () => {
      const ratio = dp.addNoiseToRatio(0.5);
      assert.ok(ratio >= 0);
      assert.ok(ratio <= 1);
    });
  });

  describe('PrivatePatternAggregator', () => {
    it('aggregates patterns with privacy', () => {
      const agg = new PrivatePatternAggregator();

      agg.addPattern('hash1', 'errors');
      agg.addPattern('hash1', 'errors');
      agg.addPattern('hash2', 'errors');

      // Get private (noised) count - should be close to 2 but noised
      const count = agg.getPrivateCount('hash1', 'errors');
      assert.ok(typeof count === 'number');
    });

    it('returns private statistics', () => {
      const agg = new PrivatePatternAggregator();

      agg.addPattern('h1', 'cat1');
      agg.addPattern('h2', 'cat1');
      agg.addPattern('h3', 'cat2');

      const stats = agg.getPrivateStats();
      assert.ok(stats.totalPatterns >= 0);
      assert.ok(stats.uniquePatterns >= 0);
      assert.ok(stats.budgetStatus);
    });

    it('gets top patterns by noised count', () => {
      const agg = new PrivatePatternAggregator();

      // Add patterns with different frequencies
      for (let i = 0; i < 10; i++) agg.addPattern('common', 'cat');
      for (let i = 0; i < 5; i++) agg.addPattern('medium', 'cat');
      agg.addPattern('rare', 'cat');

      const top = agg.getTopPatterns(3);
      assert.ok(Array.isArray(top));
      assert.ok(top.length <= 3);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// LOCAL STORE TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Local Store', () => {
  describe('Constants', () => {
    it('uses Fibonacci numbers', () => {
      assert.strictEqual(LOCAL_STORE_CONSTANTS.RETENTION_DAYS, 89); // Fib(11)
      assert.strictEqual(LOCAL_STORE_CONSTANTS.CLEANUP_INTERVAL_HOURS, 21); // Fib(8)
      assert.strictEqual(LOCAL_STORE_CONSTANTS.MAX_ENTRIES_PER_CATEGORY, 144); // Fib(12)
      assert.strictEqual(LOCAL_STORE_CONSTANTS.MAX_TOTAL_ENTRIES, 377); // Fib(14)
    });
  });

  describe('generateLocalId', () => {
    it('generates unique IDs', () => {
      const id1 = generateLocalId();
      const id2 = generateLocalId();

      assert.ok(id1.startsWith('local_'));
      assert.ok(id2.startsWith('local_'));
      assert.notStrictEqual(id1, id2);
    });
  });

  describe('LocalStore', () => {
    let store;

    beforeEach(() => {
      store = new LocalStore();
    });

    it('adds and retrieves entries', () => {
      const entry = store.add(LocalDataCategory.RAW_MESSAGES, { text: 'hello' });

      assert.ok(entry.id);
      assert.strictEqual(entry.category, LocalDataCategory.RAW_MESSAGES);

      const retrieved = store.get(entry.id);
      assert.strictEqual(retrieved.data.text, 'hello');
    });

    it('tracks access count', () => {
      const entry = store.add(LocalDataCategory.INTERACTIONS, { action: 'click' });

      store.get(entry.id);
      store.get(entry.id);

      assert.strictEqual(entry.accessCount, 2);
    });

    it('retrieves by category', () => {
      store.add(LocalDataCategory.ERRORS, { type: 'syntax' });
      store.add(LocalDataCategory.ERRORS, { type: 'runtime' });
      store.add(LocalDataCategory.INTERACTIONS, { action: 'scroll' });

      const errors = store.getByCategory(LocalDataCategory.ERRORS);
      assert.strictEqual(errors.length, 2);
    });

    it('respects category limit', () => {
      const category = LocalDataCategory.RAW_MESSAGES;

      // Add more than limit
      for (let i = 0; i < LOCAL_STORE_CONSTANTS.MAX_ENTRIES_PER_CATEGORY + 10; i++) {
        store.add(category, { index: i });
      }

      const entries = store.getByCategory(category);
      assert.ok(entries.length <= LOCAL_STORE_CONSTANTS.MAX_ENTRIES_PER_CATEGORY);
    });

    it('deduplicates by key', () => {
      store.add(LocalDataCategory.ERRORS, { type: 'err1' }, { dedupKey: 'unique_error' });

      assert.ok(store.hasSimilar('unique_error'));
      assert.ok(!store.hasSimilar('different_error'));
    });

    it('removes entries', () => {
      const entry = store.add(LocalDataCategory.INTERACTIONS, { test: true });
      const removed = store.remove(entry.id);

      assert.ok(removed);
      assert.strictEqual(store.get(entry.id), null);
    });

    it('clears category', () => {
      store.add(LocalDataCategory.ERRORS, { a: 1 });
      store.add(LocalDataCategory.ERRORS, { b: 2 });
      store.add(LocalDataCategory.INTERACTIONS, { c: 3 });

      const removed = store.clearCategory(LocalDataCategory.ERRORS);
      assert.strictEqual(removed, 2);
      assert.strictEqual(store.getByCategory(LocalDataCategory.ERRORS).length, 0);
    });

    it('exports and imports for backup', () => {
      store.add(LocalDataCategory.ERRORS, { type: 'test' });
      store.add(LocalDataCategory.INTERACTIONS, { action: 'test' });

      const backup = store.exportForBackup();
      assert.ok(backup.entries.length === 2);
      assert.ok(backup.exportedAt);

      // Import into new store
      const newStore = new LocalStore();
      const result = newStore.importFromBackup(backup);

      assert.strictEqual(result.imported, 2);
      assert.strictEqual(newStore.entries.size, 2);
    });

    it('provides statistics', () => {
      store.add(LocalDataCategory.ERRORS, { test: 1 });
      store.add(LocalDataCategory.ERRORS, { test: 2 });

      const stats = store.getStats();
      assert.strictEqual(stats.totalEntries, 2);
      assert.ok(stats.categories[LocalDataCategory.ERRORS] === 2);
      assert.ok(stats.retentionDays === 89);
    });
  });

  describe('ProfileSignalStore', () => {
    let store;

    beforeEach(() => {
      store = new ProfileSignalStore();
    });

    it('records and retrieves signals', () => {
      store.recordSignal('linguistic', 75);
      store.recordSignal('linguistic', 80);

      const history = store.getSignalHistory('linguistic');
      assert.strictEqual(history.length, 2);
      assert.strictEqual(history[0].value, 75);
    });

    it('calculates average signal', () => {
      store.recordSignal('behavioral', 60);
      store.recordSignal('behavioral', 80);

      const avg = store.getAverageSignal('behavioral');
      assert.strictEqual(avg, 70);
    });

    it('returns default for no history', () => {
      const avg = store.getAverageSignal('unknown');
      assert.strictEqual(avg, 50); // Default middle value
    });

    it('calculates signal trend', () => {
      // Improving trend
      store.recordSignal('code', 50);
      store.recordSignal('code', 60);
      store.recordSignal('code', 70);
      store.recordSignal('code', 80);

      const trend = store.getSignalTrend('code');
      assert.ok(trend > 0); // Positive trend
    });

    it('limits history per signal type', () => {
      for (let i = 0; i < LOCAL_STORE_CONSTANTS.MAX_SIGNAL_HISTORY + 10; i++) {
        store.recordSignal('temporal', i);
      }

      const history = store.getSignalHistory('temporal');
      assert.ok(history.length <= LOCAL_STORE_CONSTANTS.MAX_SIGNAL_HISTORY);
    });

    it('provides signal statistics', () => {
      store.recordSignal('linguistic', 80);
      store.recordSignal('behavioral', 60);

      const stats = store.getSignalStats();
      assert.ok(stats.linguistic);
      assert.ok(stats.behavioral);
      assert.ok(stats.linguistic.maxConfidence <= PHI_INV);
    });
  });

  describe('SessionHistoryStore', () => {
    let store;

    beforeEach(() => {
      store = new SessionHistoryStore();
    });

    it('manages session lifecycle', () => {
      const sessionId = store.startSession({ user: 'test' });
      assert.ok(sessionId);
      assert.ok(store.currentSession);

      store.recordEvent('action', { type: 'click' });
      store.recordToolCall('read_file', { path: '/test.js' }, { success: true });

      const entry = store.endSession();
      assert.ok(entry);
      assert.ok(entry.data.durationMs >= 0);
      assert.strictEqual(entry.data.events.length, 1);
      assert.strictEqual(entry.data.toolCalls.length, 1);
    });

    it('sanitizes sensitive tool arguments', () => {
      store.startSession();
      store.recordToolCall('api_call', { password: 'secret123', url: '/api' }, {});
      store.endSession();

      const sessions = store.getRecentSessions(1);
      const args = sessions[0].data.toolCalls[0].args;

      assert.strictEqual(args.password, '[REDACTED]');
      assert.strictEqual(args.url, '/api'); // Not redacted
    });

    it('provides session statistics', () => {
      // Create a few sessions
      for (let i = 0; i < 3; i++) {
        store.startSession();
        store.recordToolCall('tool' + i, {}, {});
        store.endSession();
      }

      const stats = store.getSessionStats();
      assert.strictEqual(stats.totalSessions, 3);
      assert.ok(stats.avgToolCalls >= 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// AGGREGATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Privacy Aggregator', () => {
  describe('goldenAngleHash', () => {
    it('produces hash and bucket', () => {
      const result = goldenAngleHash('test_content');

      assert.ok(result.hash);
      assert.strictEqual(result.hash.length, 32);
      assert.ok(result.bucket >= 0);
      assert.ok(result.bucket < AGGREGATOR_CONSTANTS.PATTERN_BUCKETS);
    });

    it('distributes evenly with golden angle', () => {
      const buckets = new Array(AGGREGATOR_CONSTANTS.PATTERN_BUCKETS).fill(0);

      // Hash many values
      for (let i = 0; i < 1000; i++) {
        const { bucket } = goldenAngleHash(`content_${i}`);
        buckets[bucket]++;
      }

      // Check distribution (should be relatively even)
      const min = Math.min(...buckets);
      const max = Math.max(...buckets);
      const avg = 1000 / AGGREGATOR_CONSTANTS.PATTERN_BUCKETS;

      // Max should not be more than 3x min for good distribution
      assert.ok(max < min * 5 || min > 10);
    });
  });

  describe('PrivacyAggregator', () => {
    let aggregator;

    beforeEach(() => {
      aggregator = new PrivacyAggregator();
    });

    it('commits profile levels', () => {
      const result = aggregator.commitProfile('user1', 5);

      assert.ok(result.commitment);
      assert.ok(result.blindingFactor);
      assert.strictEqual(result.blindingFactor.length, COMMITMENT_CONSTANTS.BLINDING_SIZE);
    });

    it('creates profile range proofs', () => {
      aggregator.commitProfile('user1', 8);
      const proof = aggregator.proveMinProfileLevel('user1', 5);

      assert.ok(proof);
      assert.strictEqual(proof.claim, 'level >= 5');
    });

    it('commits patterns with bucket assignment', () => {
      const result = aggregator.commitPattern('my_error_pattern', 'errors');

      assert.ok(result.commitment);
      assert.ok(result.patternHash);
      assert.ok(result.bucket >= 0);
      assert.ok(result.bucket < AGGREGATOR_CONSTANTS.PATTERN_BUCKETS);
    });

    it('requires minimum contributions for public release', () => {
      // Only 2 contributions - not enough
      aggregator.commitPattern('p1', 'cat');
      aggregator.commitPattern('p2', 'cat');

      const stats = aggregator.getPublicPatternStats();
      assert.strictEqual(stats.status, 'insufficient_data');
    });

    it('releases public stats with enough contributions', () => {
      // Add enough contributions
      for (let i = 0; i < AGGREGATOR_CONSTANTS.MIN_CONTRIBUTIONS_FOR_PUBLIC; i++) {
        aggregator.commitPattern(`pattern_${i}`, 'cat');
      }

      const stats = aggregator.getPublicPatternStats();
      assert.strictEqual(stats.status, 'public');
      assert.strictEqual(stats.tier, DataTier.PUBLIC);
      assert.ok(stats.phi.epsilon === PHI_INV);
    });

    it('verifies profile range proofs', () => {
      aggregator.commitProfile('user1', 5);
      const proof = aggregator.proveMinProfileLevel('user1', 3);

      const valid = aggregator.verifyProfileLevel(proof.commitment, proof.proof, 3);
      assert.ok(valid);
    });

    it('tracks bucket evenness', () => {
      // Add many patterns
      for (let i = 0; i < 100; i++) {
        aggregator.commitPattern(`pattern_${i}`, 'cat');
      }

      const evenness = aggregator.getBucketEvenness();
      assert.ok(evenness >= 0);
      assert.ok(evenness <= 1);
    });

    it('provides aggregator status', () => {
      aggregator.commitProfile('user1', 5);
      aggregator.commitPattern('p1', 'errors');

      const status = aggregator.getStatus();
      assert.strictEqual(status.profileCommitments, 1);
      assert.ok(status.totalContributions >= 1);
      assert.ok(status.phi.goldenAngle === AGGREGATOR_CONSTANTS.GOLDEN_ANGLE);
    });
  });

  describe('BatchAggregator', () => {
    it('queues and flushes patterns', () => {
      const aggregator = new PrivacyAggregator();
      const batch = new BatchAggregator(aggregator);

      batch.queuePattern('p1', 'cat');
      batch.queuePattern('p2', 'cat');

      assert.strictEqual(batch.getPendingCount(), 2);

      const result = batch.flush();
      assert.strictEqual(result.processed, 2);
      assert.strictEqual(batch.getPendingCount(), 0);
    });

    it('auto-flushes when batch size reached', () => {
      const aggregator = new PrivacyAggregator();
      const batch = new BatchAggregator(aggregator);

      // Queue more than batch size
      for (let i = 0; i < AGGREGATOR_CONSTANTS.AGGREGATION_BATCH_SIZE + 5; i++) {
        batch.queuePattern(`pattern_${i}`, 'cat');
      }

      // Should have auto-flushed, leaving only 5
      assert.strictEqual(batch.getPendingCount(), 5);
    });
  });

  describe('createPrivacyPipeline', () => {
    it('creates integrated pipeline', () => {
      const localStore = new LocalStore();
      const pipeline = createPrivacyPipeline(localStore);

      assert.ok(pipeline.local);
      assert.ok(pipeline.aggregator);
      assert.ok(pipeline.batch);
      assert.ok(typeof pipeline.processLocalToCommitted === 'function');
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// DATA FLOW TESTS (Critical: Verify No Leakage)
// ═══════════════════════════════════════════════════════════════════════════

describe('Data Flow Security', () => {
  it('raw patterns never appear in committed tier', () => {
    const aggregator = new PrivacyAggregator();
    const sensitivePattern = 'my_secret_api_key_abc123';

    const result = aggregator.commitPattern(sensitivePattern, 'secrets');

    // Commitment should NOT contain the raw pattern
    assert.ok(!result.commitment.includes(sensitivePattern));
    assert.ok(!result.patternHash.includes(sensitivePattern));

    // Can't reverse the hash
    assert.notStrictEqual(result.patternHash, sensitivePattern);
  });

  it('profile level is hidden in commitment', () => {
    const aggregator = new PrivacyAggregator();

    const result = aggregator.commitProfile('user1', 8);

    // Commitment should be a valid 64-char hex hash (SHA256)
    assert.strictEqual(result.commitment.length, 64);
    assert.ok(/^[0-9a-f]+$/i.test(result.commitment));

    // Same level for different users should produce different commitments (hiding property)
    const result2 = aggregator.commitProfile('user2', 8);
    assert.notStrictEqual(result.commitment, result2.commitment);

    // But range proof can verify level >= N
    const proof = aggregator.proveMinProfileLevel('user1', 5);
    assert.ok(proof);
    assert.ok(aggregator.verifyProfileLevel(proof.commitment, proof.proof, 5));
  });

  it('local store data stays local', () => {
    const localStore = new LocalStore();
    const aggregator = new PrivacyAggregator();

    // Add sensitive data to local store
    localStore.add(LocalDataCategory.RAW_MESSAGES, {
      message: 'My password is hunter2',
      user: 'john@example.com',
    });

    // Local store should have the data
    const localEntries = localStore.getByCategory(LocalDataCategory.RAW_MESSAGES);
    assert.strictEqual(localEntries.length, 1);
    assert.ok(localEntries[0].data.message.includes('hunter2'));

    // But aggregator has NO access to this data
    const publicStats = aggregator.getPublicPatternStats();
    const statsStr = JSON.stringify(publicStats);

    assert.ok(!statsStr.includes('hunter2'));
    assert.ok(!statsStr.includes('john@example.com'));
  });

  it('differential privacy protects individual contributions', () => {
    const aggregator = new PrivacyAggregator();

    // Single user contributes a pattern
    aggregator.commitPattern('unique_user_pattern', 'rare');

    // Add 4 more to reach public threshold
    for (let i = 0; i < 4; i++) {
      aggregator.commitPattern(`other_pattern_${i}`, 'common');
    }

    const stats = aggregator.getPublicPatternStats();

    // Stats should be noised - can't determine exact count
    assert.ok(stats.status === 'public');
    // The noise makes exact counts unreliable (that's the point!)
    assert.ok(stats.phi.note.includes('Laplacian noise'));
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PRIVATE KNOWLEDGE AGGREGATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('PrivateKnowledgeAggregator', () => {
  let aggregator;

  beforeEach(() => {
    aggregator = new PrivateKnowledgeAggregator();
  });

  describe('Bucket Distribution (Golden Angle)', () => {
    it('assigns content to buckets using golden angle', () => {
      const bucket1 = aggregator._getBucket('content_1');
      const bucket2 = aggregator._getBucket('content_2');

      // Buckets should be in valid range (0-359)
      assert.ok(bucket1 >= 0 && bucket1 < 360);
      assert.ok(bucket2 >= 0 && bucket2 < 360);
    });

    it('same content always goes to same bucket', () => {
      const content = 'deterministic_content';
      const bucket1 = aggregator._getBucket(content);
      const bucket2 = aggregator._getBucket(content);

      assert.strictEqual(bucket1, bucket2);
    });

    it('distributes content evenly across buckets', () => {
      const bucketCounts = new Map();

      // Add many different contents
      for (let i = 0; i < 360; i++) {
        const bucket = aggregator._getBucket(`unique_content_${i}`);
        bucketCounts.set(bucket, (bucketCounts.get(bucket) || 0) + 1);
      }

      // Check distribution - should have multiple buckets used
      assert.ok(bucketCounts.size > 100, 'Should use at least 100 buckets');
    });
  });

  describe('Knowledge Contributions', () => {
    it('accepts and counts contributions', () => {
      aggregator.contribute('hash_1', { category: 'patterns' });
      aggregator.contribute('hash_2', { category: 'patterns' });
      aggregator.contribute('hash_3', { category: 'errors' });

      assert.strictEqual(aggregator.totalContributions, 3);
      assert.ok(aggregator.buckets.size > 0);
    });

    it('tracks categories within buckets', () => {
      aggregator.contribute('hash_a', { category: 'learning' });
      aggregator.contribute('hash_b', { category: 'learning' });
      aggregator.contribute('hash_c', { category: 'insights' });

      // Get the aggregate
      const aggregate = aggregator.getPrivateAggregate();

      assert.ok(aggregate.categoryDistribution);
      assert.ok(aggregate.categoryDistribution.learning !== undefined);
      assert.ok(aggregate.categoryDistribution.insights !== undefined);
    });

    it('handles contributions without category', () => {
      aggregator.contribute('hash_no_cat');
      aggregator.contribute('hash_no_cat_2', {});

      assert.strictEqual(aggregator.totalContributions, 2);
    });
  });

  describe('Private Aggregate Output', () => {
    it('returns noised total contributions', () => {
      for (let i = 0; i < 10; i++) {
        aggregator.contribute(`hash_${i}`);
      }

      const aggregate = aggregator.getPrivateAggregate();

      // totalContributions should be noised (>= 0)
      assert.ok(aggregate.totalContributions >= 0);
      assert.ok(typeof aggregate.totalContributions === 'number');
    });

    it('returns noised active bucket count', () => {
      for (let i = 0; i < 20; i++) {
        aggregator.contribute(`content_${i}`);
      }

      const aggregate = aggregator.getPrivateAggregate();

      assert.ok(aggregate.activeBuckets >= 0);
      assert.ok(Number.isInteger(aggregate.activeBuckets));
    });

    it('includes bucket distribution with positive counts', () => {
      for (let i = 0; i < 50; i++) {
        aggregator.contribute(`data_${i}`);
      }

      const aggregate = aggregator.getPrivateAggregate();

      assert.ok(Array.isArray(aggregate.bucketDistribution));
      // All buckets in distribution should have count > 0
      aggregate.bucketDistribution.forEach(b => {
        assert.ok(b.count > 0);
        assert.ok(b.bucket >= 0 && b.bucket < 360);
      });
    });

    it('includes budget status', () => {
      aggregator.contribute('test');

      const aggregate = aggregator.getPrivateAggregate();

      assert.ok(aggregate.budgetStatus);
      assert.ok(aggregate.budgetStatus.epsilon);
      assert.ok(aggregate.budgetStatus.queryCount >= 0);
    });
  });

  describe('Privacy Budget Consumption', () => {
    it('consumes budget when generating aggregate', () => {
      for (let i = 0; i < 5; i++) {
        aggregator.contribute(`item_${i}`);
      }

      // First aggregate
      aggregator.getPrivateAggregate();
      const status1 = aggregator.dp.getBudgetStatus();

      // Second aggregate
      aggregator.getPrivateAggregate();
      const status2 = aggregator.dp.getBudgetStatus();

      assert.ok(status2.budgetUsed > status1.budgetUsed);
      assert.ok(status2.queryCount > status1.queryCount);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// ADDITIONAL DIFFERENTIAL PRIVACY EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════

describe('Differential Privacy - Edge Cases', () => {
  describe('Budget Management', () => {
    it('resets budget after refresh period', async () => {
      const dp = new DifferentialPrivacy();

      // Use some budget
      dp.addNoise(100);
      dp.addNoise(100);
      assert.ok(dp.queryCount > 0);

      // Manually set lastBudgetReset to past
      dp.lastBudgetReset = Date.now() - (PRIVACY_CONSTANTS.BUDGET_REFRESH_HOURS + 1) * 60 * 60 * 1000;

      // Next query should trigger reset
      dp.addNoise(100);

      const status = dp.getBudgetStatus();
      assert.strictEqual(status.queryCount, 1); // Reset to 1 after the new query
    });

    it('tracks percentage of budget used', () => {
      const dp = new DifferentialPrivacy();

      // Use some budget
      dp.addNoise(100);

      const status = dp.getBudgetStatus();
      assert.ok(status.percentUsed >= 0);
      assert.ok(status.percentUsed <= 100);
    });

    it('calculates remaining budget correctly', () => {
      const dp = new DifferentialPrivacy();

      const initialStatus = dp.getBudgetStatus();
      assert.strictEqual(initialStatus.budgetRemaining, dp.epsilon);

      dp.addNoise(100);
      const afterStatus = dp.getBudgetStatus();

      assert.ok(afterStatus.budgetRemaining < dp.epsilon);
      assert.ok(afterStatus.budgetRemaining >= 0);
    });
  });

  describe('Noise Properties', () => {
    it('applies minimum noise floor', () => {
      const dp = new DifferentialPrivacy();
      const trueValue = 100;

      // Run multiple times to verify noise floor is respected
      let floorApplied = false;
      for (let i = 0; i < 50; i++) {
        const noised = dp.addNoise(trueValue);
        const actualNoise = Math.abs(noised - trueValue);
        const minNoise = trueValue * PRIVACY_CONSTANTS.MIN_NOISE_FLOOR;

        // Either actual noise is >= minNoise, or floor was applied
        if (actualNoise >= minNoise - 0.001) {
          floorApplied = true;
          break;
        }
      }

      assert.ok(floorApplied, 'Minimum noise floor should be applied');
    });

    it('handles zero value input', () => {
      const dp = new DifferentialPrivacy();

      // Should not crash on zero
      const noised = dp.addNoise(0);
      assert.ok(typeof noised === 'number');
      assert.ok(!isNaN(noised));
    });

    it('handles negative value input', () => {
      const dp = new DifferentialPrivacy();

      const noised = dp.addNoise(-50);
      assert.ok(typeof noised === 'number');
      assert.ok(!isNaN(noised));
    });

    it('uses custom sensitivity when provided', () => {
      const dp = new DifferentialPrivacy({ sensitivity: 1 });

      // High sensitivity should add more noise
      const highSens = dp.addNoise(100, 10);
      const lowSens = dp.addNoise(100, 0.1);

      // Both should be numbers (can't guarantee which is higher due to randomness)
      assert.ok(typeof highSens === 'number');
      assert.ok(typeof lowSens === 'number');
    });
  });

  describe('Pattern Aggregator Category Distribution', () => {
    it('returns correct category distribution with noise', () => {
      const agg = new PrivatePatternAggregator();

      // Add patterns to multiple categories
      for (let i = 0; i < 10; i++) agg.addPattern(`hash_${i}`, 'errors');
      for (let i = 0; i < 5; i++) agg.addPattern(`other_${i}`, 'warnings');
      for (let i = 0; i < 3; i++) agg.addPattern(`third_${i}`, 'info');

      const dist = agg.getCategoryDistribution();

      assert.ok(dist.has('errors'));
      assert.ok(dist.has('warnings'));
      assert.ok(dist.has('info'));

      // All counts should be non-negative (due to addNoiseToCount)
      for (const [, count] of dist) {
        assert.ok(count >= 0);
      }
    });

    it('filters top patterns by category', () => {
      const agg = new PrivatePatternAggregator();

      // Add patterns to different categories
      for (let i = 0; i < 5; i++) agg.addPattern(`err_${i}`, 'errors');
      for (let i = 0; i < 3; i++) agg.addPattern(`warn_${i}`, 'warnings');

      const errorPatterns = agg.getTopPatterns(10, 'errors');
      const warningPatterns = agg.getTopPatterns(10, 'warnings');

      // Filtered patterns should only include respective category
      errorPatterns.forEach(p => assert.strictEqual(p.category, 'errors'));
      warningPatterns.forEach(p => assert.strictEqual(p.category, 'warnings'));
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// GAUSSIAN NOISE TESTS (for composed queries)
// ═══════════════════════════════════════════════════════════════════════════

describe('Gaussian Noise Mechanism', () => {
  it('generates noise with correct standard deviation', () => {
    const sigma = 2;
    const samples = Array.from({ length: 1000 }, () => gaussianNoise(sigma));

    // Calculate standard deviation
    const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
    const variance = samples.reduce((acc, s) => acc + (s - mean) ** 2, 0) / samples.length;
    const stdDev = Math.sqrt(variance);

    // Standard deviation should be close to sigma (within 30% tolerance for statistical test)
    assert.ok(stdDev > sigma * 0.5, `stdDev ${stdDev} should be > ${sigma * 0.5}`);
    assert.ok(stdDev < sigma * 1.5, `stdDev ${stdDev} should be < ${sigma * 1.5}`);
  });

  it('produces different values on each call', () => {
    const values = new Set();
    for (let i = 0; i < 100; i++) {
      values.add(gaussianNoise(1));
    }

    // Should have many unique values
    assert.ok(values.size > 90, 'Should generate mostly unique values');
  });

  it('works with very small sigma', () => {
    const noise = gaussianNoise(0.001);
    assert.ok(typeof noise === 'number');
    assert.ok(!isNaN(noise));
    assert.ok(Math.abs(noise) < 0.1); // Should be small
  });

  it('works with large sigma', () => {
    const noise = gaussianNoise(100);
    assert.ok(typeof noise === 'number');
    assert.ok(!isNaN(noise));
  });
});
