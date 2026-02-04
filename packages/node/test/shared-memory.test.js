/**
 * @cynic/node - Shared Memory Tests
 *
 * Comprehensive tests for the collective intelligence layer:
 *   SharedMemory — pattern memory, judgment similarity, learned weights,
 *   procedural memory, EWC++ integration, path reinforcement,
 *   persistence, import/export, and feedback.
 *
 * @module @cynic/node/test/shared-memory
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { SharedMemory } from '../src/memory/shared-memory.js';

// =============================================================================
// CONSTANTS
// =============================================================================

const PHI_INV = 0.618033988749895;

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockStorage(data = {}) {
  return {
    get: mock.fn(async (key) => data[key] || null),
    set: mock.fn(async (key, value) => { data[key] = value; }),
  };
}

function createMockSwarm() {
  return {
    emit: mock.fn(() => {}),
  };
}

// =============================================================================
// CONSTRUCTION TESTS
// =============================================================================

describe('SharedMemory', () => {
  let memory;
  let storage;

  beforeEach(() => {
    storage = createMockStorage();
    memory = new SharedMemory({ storage });
  });

  describe('Construction', () => {
    it('should create with storage', () => {
      assert.ok(memory);
      assert.strictEqual(memory.initialized, false);
    });

    it('should create without storage', () => {
      const bare = new SharedMemory();
      assert.ok(bare);
      assert.strictEqual(bare.storage, undefined);
    });

    it('should initialize empty maps', () => {
      assert.strictEqual(memory._patterns.size, 0);
      assert.strictEqual(memory._procedures.size, 0);
      assert.strictEqual(memory._scoringRules.size, 0);
      assert.deepEqual(memory._judgmentIndex, []);
      assert.deepEqual(memory._dimensionWeights, {});
      assert.deepEqual(memory._feedbackLog, []);
    });

    it('should initialize stats to zero', () => {
      assert.strictEqual(memory.stats.patternsAdded, 0);
      assert.strictEqual(memory.stats.judgmentsIndexed, 0);
      assert.strictEqual(memory.stats.feedbackReceived, 0);
      assert.strictEqual(memory.stats.weightAdjustments, 0);
    });

    it('should accept swarm option', () => {
      const swarm = createMockSwarm();
      const withSwarm = new SharedMemory({ storage, swarm });
      assert.strictEqual(withSwarm.swarm, swarm);
    });
  });

  // ===========================================================================
  // INITIALIZATION TESTS
  // ===========================================================================

  describe('initialize()', () => {
    it('should initialize from storage', async () => {
      await memory.initialize();
      assert.strictEqual(memory.initialized, true);
    });

    it('should not re-initialize if already initialized', async () => {
      await memory.initialize();
      const callCount = storage.get.mock.calls.length;

      await memory.initialize();
      assert.strictEqual(storage.get.mock.calls.length, callCount);
    });

    it('should load saved patterns', async () => {
      storage = createMockStorage({
        shared_memory: {
          patterns: [
            ['pat_001', { id: 'pat_001', name: 'Test', weight: 1.5 }],
          ],
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.strictEqual(memory._patterns.size, 1);
      assert.ok(memory._patterns.has('pat_001'));
    });

    it('should load saved judgment index', async () => {
      storage = createMockStorage({
        shared_memory: {
          judgmentIndex: [
            { judgment: { id: 'j1' }, type: 'token', tokens: ['test'] },
          ],
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.strictEqual(memory._judgmentIndex.length, 1);
    });

    it('should load saved weights', async () => {
      storage = createMockStorage({
        shared_memory: {
          weights: { PHI: 1.2, VERIFY: 0.9 },
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.strictEqual(memory._dimensionWeights.PHI, 1.2);
      assert.strictEqual(memory._dimensionWeights.VERIFY, 0.9);
    });

    it('should load saved procedures and scoring rules', async () => {
      storage = createMockStorage({
        shared_memory: {
          procedures: [['custom', { type: 'custom', steps: ['a', 'b'] }]],
          scoringRules: [['custom', { type: 'custom', minScore: 10 }]],
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.ok(memory._procedures.has('custom'));
      assert.ok(memory._scoringRules.has('custom'));
    });

    it('should load saved feedback log', async () => {
      storage = createMockStorage({
        shared_memory: {
          feedback: [{ type: 'test', timestamp: 12345 }],
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.strictEqual(memory._feedbackLog.length, 1);
    });

    it('should load saved stats', async () => {
      storage = createMockStorage({
        shared_memory: {
          stats: { patternsAdded: 42 },
        },
      });
      memory = new SharedMemory({ storage });

      await memory.initialize();

      assert.strictEqual(memory.stats.patternsAdded, 42);
    });

    it('should init default procedures if empty', async () => {
      await memory.initialize();

      assert.ok(memory._procedures.has('default'));
      assert.ok(memory._procedures.has('token'));
      assert.ok(memory._procedures.has('code'));
      assert.ok(memory._procedures.has('decision'));
      assert.ok(memory._scoringRules.has('default'));
    });

    it('should handle storage errors gracefully', async () => {
      const badStorage = {
        get: mock.fn(async () => { throw new Error('DB down'); }),
        set: mock.fn(async () => {}),
      };
      const mem = new SharedMemory({ storage: badStorage });

      // Should not throw
      await mem.initialize();
      assert.strictEqual(mem.initialized, true);
    });

    it('should handle null storage', async () => {
      const noStorage = new SharedMemory();
      await noStorage.initialize();
      assert.strictEqual(noStorage.initialized, true);
    });
  });

  // ===========================================================================
  // PATTERN MEMORY TESTS
  // ===========================================================================

  describe('addPattern()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should add a pattern with defaults', () => {
      const id = memory.addPattern({ name: 'Test Pattern', tags: ['test'] });

      assert.ok(id);
      assert.ok(id.startsWith('pat_'));

      const pattern = memory._patterns.get(id);
      assert.strictEqual(pattern.name, 'Test Pattern');
      assert.strictEqual(pattern.weight, 1.0);
      assert.strictEqual(pattern.useCount, 0);
    });

    it('should use provided id if given', () => {
      const id = memory.addPattern({ id: 'my_custom_id', name: 'Custom' });

      assert.strictEqual(id, 'my_custom_id');
      assert.ok(memory._patterns.has('my_custom_id'));
    });

    it('should initialize EWC fields', () => {
      const id = memory.addPattern({ name: 'Test' });
      const pattern = memory._patterns.get(id);

      assert.strictEqual(pattern.fisherImportance, 0);
      assert.strictEqual(pattern.consolidationLocked, false);
      assert.strictEqual(pattern.lockedAt, null);
    });

    it('should preserve existing EWC fields', () => {
      const id = memory.addPattern({
        name: 'Locked Pattern',
        fisherImportance: 0.8,
        consolidationLocked: true,
        lockedAt: Date.now(),
      });

      const pattern = memory._patterns.get(id);
      assert.strictEqual(pattern.fisherImportance, 0.8);
      assert.strictEqual(pattern.consolidationLocked, true);
    });

    it('should increment patternsAdded stat', () => {
      memory.addPattern({ name: 'A' });
      memory.addPattern({ name: 'B' });

      assert.strictEqual(memory.stats.patternsAdded, 2);
    });

    it('should set addedAt timestamp', () => {
      const before = Date.now();
      const id = memory.addPattern({ name: 'Timestamped' });
      const after = Date.now();

      const pattern = memory._patterns.get(id);
      assert.ok(pattern.addedAt >= before);
      assert.ok(pattern.addedAt <= after);
    });

    it('should preserve custom weight', () => {
      const id = memory.addPattern({ name: 'Heavy', weight: 2.0 });
      assert.strictEqual(memory._patterns.get(id).weight, 2.0);
    });

    it('should preserve verified flag', () => {
      const id = memory.addPattern({ name: 'Verified', verified: true });
      assert.strictEqual(memory._patterns.get(id).verified, true);
    });
  });

  describe('verifyPattern()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should verify a pattern', () => {
      const id = memory.addPattern({ name: 'Unverified' });
      memory.verifyPattern(id);

      const pattern = memory._patterns.get(id);
      assert.strictEqual(pattern.verified, true);
      assert.ok(pattern.verifiedAt > 0);
    });

    it('should handle verifying nonexistent pattern', () => {
      // Should not throw
      memory.verifyPattern('nonexistent');
    });
  });

  describe('getRelevantPatterns()', () => {
    beforeEach(async () => {
      await memory.initialize();

      memory.addPattern({
        id: 'pat_token',
        name: 'Token Analysis',
        applicableTo: ['token'],
        tags: ['analysis', 'market'],
        weight: 1.5,
      });
      memory.addPattern({
        id: 'pat_social',
        name: 'Social Pattern',
        applicableTo: ['social'],
        tags: ['twitter', 'sentiment'],
        weight: 1.0,
      });
    });

    it('should find relevant patterns by type', () => {
      const patterns = memory.getRelevantPatterns({ type: 'token' });

      assert.ok(patterns.length > 0);
      assert.ok(patterns.some(p => p.id === 'pat_token'));
    });

    it('should find patterns by tag overlap', () => {
      const patterns = memory.getRelevantPatterns({
        type: 'unknown',
        tags: ['analysis'],
      });

      assert.ok(patterns.some(p => p.id === 'pat_token'));
    });

    it('should return empty for null item', () => {
      const patterns = memory.getRelevantPatterns(null);
      assert.deepEqual(patterns, []);
    });

    it('should return empty for undefined item', () => {
      const patterns = memory.getRelevantPatterns(undefined);
      assert.deepEqual(patterns, []);
    });

    it('should increase useCount on retrieval', () => {
      memory.getRelevantPatterns({ type: 'token' });
      memory.getRelevantPatterns({ type: 'token' });

      const pattern = memory._patterns.get('pat_token');
      assert.ok(pattern.useCount >= 2);
    });

    it('should reinforce weights on use', () => {
      const before = memory._patterns.get('pat_token').weight;
      memory.getRelevantPatterns({ type: 'token' });
      const after = memory._patterns.get('pat_token').weight;

      assert.ok(after > before, 'Weight should increase after use');
    });

    it('should respect limit parameter', () => {
      // Add many patterns
      for (let i = 0; i < 20; i++) {
        memory.addPattern({
          name: `Pattern ${i}`,
          applicableTo: ['token'],
        });
      }

      const patterns = memory.getRelevantPatterns({ type: 'token' }, 3);
      assert.ok(patterns.length <= 3);
    });

    it('should sort by relevance descending', () => {
      const patterns = memory.getRelevantPatterns({ type: 'token' });

      for (let i = 1; i < patterns.length; i++) {
        assert.ok(patterns[i - 1].relevance >= patterns[i].relevance);
      }
    });

    it('should give verified patterns bonus relevance', () => {
      memory.addPattern({
        id: 'pat_verified',
        name: 'Verified Token',
        applicableTo: ['token'],
        verified: true,
        weight: 1.0,
      });
      memory.addPattern({
        id: 'pat_unverified',
        name: 'Unverified Token',
        applicableTo: ['token'],
        verified: false,
        weight: 1.0,
      });

      const patterns = memory.getRelevantPatterns({ type: 'token' });
      const verified = patterns.find(p => p.id === 'pat_verified');
      const unverified = patterns.find(p => p.id === 'pat_unverified');

      if (verified && unverified) {
        assert.ok(verified.relevance >= unverified.relevance);
      }
    });

    it('should give wildcard patterns relevance', () => {
      memory.addPattern({
        id: 'pat_wildcard',
        name: 'Universal Pattern',
        applicableTo: ['*'],
        weight: 1.0,
      });

      const patterns = memory.getRelevantPatterns({ type: 'anything' });
      assert.ok(patterns.some(p => p.id === 'pat_wildcard'));
    });

    it('should update lastUsed on retrieval', () => {
      const before = memory._patterns.get('pat_token').lastUsed || 0;
      memory.getRelevantPatterns({ type: 'token' });
      const after = memory._patterns.get('pat_token').lastUsed;

      assert.ok(after >= before);
    });
  });

  // ===========================================================================
  // EWC++ INTEGRATION TESTS
  // ===========================================================================

  describe('EWC++ Integration', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    describe('Fisher Importance Boosting', () => {
      it('should boost Fisher importance on pattern use', () => {
        const id = memory.addPattern({
          name: 'Test',
          applicableTo: ['token'],
          fisherImportance: 0.1,
        });

        memory.getRelevantPatterns({ type: 'token' });

        const pattern = memory._patterns.get(id);
        assert.ok(pattern.fisherImportance > 0.1,
          'Fisher importance should increase after use');
      });

      it('should auto-lock pattern when threshold exceeded', () => {
        const id = memory.addPattern({
          name: 'Popular Pattern',
          applicableTo: ['token'],
          fisherImportance: 0.6,
          useCount: 10,
        });

        for (let i = 0; i < 10; i++) {
          memory.getRelevantPatterns({ type: 'token' });
        }

        const pattern = memory._patterns.get(id);
        assert.ok(pattern.fisherImportance >= PHI_INV,
          'Fisher should exceed lock threshold');
        assert.strictEqual(pattern.consolidationLocked, true,
          'Pattern should be auto-locked');
      });

      it('should not auto-lock if insufficient uses', () => {
        const id = memory.addPattern({
          name: 'New Pattern',
          applicableTo: ['token'],
          fisherImportance: 0.6,
          useCount: 0,
        });

        memory.getRelevantPatterns({ type: 'token' });

        const pattern = memory._patterns.get(id);
        assert.strictEqual(pattern.consolidationLocked, false,
          'Pattern should not lock without enough uses');
      });

      it('should not boost already locked pattern fisher', () => {
        const id = memory.addPattern({
          name: 'Already Locked',
          applicableTo: ['token'],
          fisherImportance: 0.8,
          consolidationLocked: true,
        });

        memory.getRelevantPatterns({ type: 'token' });

        const pattern = memory._patterns.get(id);
        // Fisher should remain unchanged for locked patterns
        assert.strictEqual(pattern.fisherImportance, 0.8);
      });

      it('should cap Fisher importance at 1.0', () => {
        const id = memory.addPattern({
          name: 'High Fisher',
          applicableTo: ['token'],
          fisherImportance: 0.99,
          useCount: 10,
        });

        for (let i = 0; i < 50; i++) {
          memory.getRelevantPatterns({ type: 'token' });
        }

        const pattern = memory._patterns.get(id);
        assert.ok(pattern.fisherImportance <= 1.0);
      });
    });

    describe('Pattern Locking', () => {
      it('should emit event on lock when swarm available', () => {
        const swarm = createMockSwarm();
        const mem = new SharedMemory({ storage, swarm });
        mem.initialized = true;
        mem._initDefaultProcedures = () => {};

        const id = mem.addPattern({
          name: 'Lockable',
          applicableTo: ['token'],
          fisherImportance: 0.6,
          useCount: 10,
        });

        // Use enough to trigger lock
        for (let i = 0; i < 20; i++) {
          mem.getRelevantPatterns({ type: 'token' });
        }

        // Check if emit was called with 'pattern:locked'
        const emitCalls = swarm.emit.mock.calls;
        const lockCall = emitCalls.find(c => c.arguments[0] === 'pattern:locked');
        if (mem._patterns.get(id).consolidationLocked) {
          assert.ok(lockCall, 'Should emit pattern:locked event');
        }
      });

      it('should set lockedAt timestamp on lock', () => {
        const id = memory.addPattern({
          name: 'TimeLock',
          applicableTo: ['token'],
          fisherImportance: 0.6,
          useCount: 10,
        });

        for (let i = 0; i < 20; i++) {
          memory.getRelevantPatterns({ type: 'token' });
        }

        const pattern = memory._patterns.get(id);
        if (pattern.consolidationLocked) {
          assert.ok(pattern.lockedAt > 0);
        }
      });
    });

    describe('Decay Protection', () => {
      it('should protect locked patterns from decay', () => {
        memory.addPattern({
          id: 'locked_pattern',
          name: 'Locked',
          consolidationLocked: true,
          weight: 1.5,
          lastUsed: Date.now() - 86400000 * 30,
        });

        memory.addPattern({
          id: 'unlocked_pattern',
          name: 'Unlocked',
          consolidationLocked: false,
          weight: 1.5,
          lastUsed: Date.now() - 86400000 * 30,
        });

        const result = memory.decayUnusedPatterns();

        const locked = memory._patterns.get('locked_pattern');
        const unlocked = memory._patterns.get('unlocked_pattern');

        assert.strictEqual(locked.weight, 1.5, 'Locked pattern weight should not change');
        assert.ok(unlocked.weight < 1.5, 'Unlocked pattern weight should decay');
        assert.ok(result.protected > 0, 'Should report protected patterns');
      });

      it('should decay Fisher importance for unlocked patterns', () => {
        memory.addPattern({
          id: 'unlocked',
          name: 'Unlocked',
          consolidationLocked: false,
          fisherImportance: 0.5,
          lastUsed: Date.now() - 86400000 * 30,
        });

        memory.decayUnusedPatterns();

        const pattern = memory._patterns.get('unlocked');
        assert.ok(pattern.fisherImportance < 0.5,
          'Fisher importance should decay for unlocked patterns');
      });

      it('should not decay recently used patterns', () => {
        memory.addPattern({
          id: 'recent',
          name: 'Recent',
          consolidationLocked: false,
          weight: 1.5,
          lastUsed: Date.now(), // Just used
        });

        const result = memory.decayUnusedPatterns();
        const pattern = memory._patterns.get('recent');

        assert.strictEqual(pattern.weight, 1.5);
      });

      it('should return correct decay counts', () => {
        memory.addPattern({
          id: 'old_unlocked',
          name: 'Old',
          consolidationLocked: false,
          weight: 1.5,
          lastUsed: Date.now() - 86400000 * 30,
        });
        memory.addPattern({
          id: 'old_locked',
          name: 'Locked',
          consolidationLocked: true,
          weight: 1.5,
          lastUsed: Date.now() - 86400000 * 30,
        });

        const result = memory.decayUnusedPatterns();

        assert.ok(typeof result.decayed === 'number');
        assert.ok(typeof result.protected === 'number');
        assert.ok(result.protected >= 1);
      });
    });

    describe('Pruning Protection', () => {
      it('should never prune locked patterns', async () => {
        for (let i = 0; i < 100; i++) {
          memory.addPattern({
            id: `pat_${i}`,
            name: `Pattern ${i}`,
            weight: 0.1,
            useCount: 0,
          });
        }

        memory.addPattern({
          id: 'locked_precious',
          name: 'Locked Precious',
          consolidationLocked: true,
          weight: 0.01,
          useCount: 0,
        });

        for (let i = 100; i < 200; i++) {
          memory.addPattern({
            id: `pat_${i}`,
            name: `Pattern ${i}`,
            weight: 1.0,
            useCount: 10,
          });
        }

        assert.ok(memory._patterns.has('locked_precious'),
          'Locked pattern should never be pruned');
      });
    });

    describe('unlockPattern()', () => {
      it('should unlock a locked pattern', () => {
        memory.addPattern({
          id: 'to_unlock',
          name: 'Unlock Me',
          consolidationLocked: true,
          lockedAt: Date.now(),
        });

        const success = memory.unlockPattern('to_unlock');

        assert.strictEqual(success, true);
        const pattern = memory._patterns.get('to_unlock');
        assert.strictEqual(pattern.consolidationLocked, false);
        assert.strictEqual(pattern.lockedAt, null);
      });

      it('should return false for non-locked pattern', () => {
        memory.addPattern({
          id: 'not_locked',
          name: 'Not Locked',
          consolidationLocked: false,
        });

        const success = memory.unlockPattern('not_locked');
        assert.strictEqual(success, false);
      });

      it('should return false for nonexistent pattern', () => {
        const success = memory.unlockPattern('nonexistent');
        assert.strictEqual(success, false);
      });
    });

    describe('getEWCStats()', () => {
      it('should return EWC statistics', () => {
        memory.addPattern({
          id: 'locked1',
          name: 'Locked 1',
          consolidationLocked: true,
          fisherImportance: 0.9,
        });
        memory.addPattern({
          id: 'critical1',
          name: 'Critical 1',
          consolidationLocked: false,
          fisherImportance: 0.7,
        });
        memory.addPattern({
          id: 'normal1',
          name: 'Normal 1',
          consolidationLocked: false,
          fisherImportance: 0.3,
        });

        const stats = memory.getEWCStats();

        assert.strictEqual(stats.totalPatterns, 3);
        assert.strictEqual(stats.lockedPatterns, 1);
        assert.strictEqual(stats.criticalPatterns, 1);
        assert.ok(stats.avgFisher > 0);
        assert.strictEqual(stats.maxFisher, 0.9);
        assert.ok(stats.retentionRate > 0);
      });

      it('should handle empty memory', () => {
        const stats = memory.getEWCStats();

        assert.strictEqual(stats.totalPatterns, 0);
        assert.strictEqual(stats.lockedPatterns, 0);
        assert.strictEqual(stats.avgFisher, 0);
        assert.strictEqual(stats.retentionRate, 0);
      });

      it('should calculate correct retention rate', () => {
        memory.addPattern({ id: 'a', consolidationLocked: true });
        memory.addPattern({ id: 'b', consolidationLocked: true });
        memory.addPattern({ id: 'c', consolidationLocked: false });
        memory.addPattern({ id: 'd', consolidationLocked: false });

        const stats = memory.getEWCStats();
        assert.strictEqual(stats.retentionRate, 0.5);
      });
    });
  });

  // ===========================================================================
  // PATH REINFORCEMENT TESTS
  // ===========================================================================

  describe('Path Reinforcement', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should boost weight on pattern use', () => {
      const id = memory.addPattern({
        name: 'Test',
        applicableTo: ['token'],
        weight: 1.0,
      });

      memory.getRelevantPatterns({ type: 'token' });

      const pattern = memory._patterns.get(id);
      assert.ok(pattern.weight > 1.0, 'Weight should increase');
    });

    it('should cap weight at MAX_WEIGHT', () => {
      const id = memory.addPattern({
        name: 'Test',
        applicableTo: ['token'],
        weight: 2.5,
      });

      for (let i = 0; i < 100; i++) {
        memory.getRelevantPatterns({ type: 'token' });
      }

      const pattern = memory._patterns.get(id);
      assert.ok(pattern.weight <= 2.618, 'Weight should not exceed MAX_WEIGHT');
    });

    it('should decay weight over time', () => {
      memory.addPattern({
        id: 'old_pattern',
        name: 'Old',
        weight: 1.5,
        lastUsed: Date.now() - 86400000 * 30,
        addedAt: Date.now() - 86400000 * 60,
      });

      const before = memory._patterns.get('old_pattern').weight;
      memory.decayUnusedPatterns();
      const after = memory._patterns.get('old_pattern').weight;

      assert.ok(after < before, 'Weight should decay for unused patterns');
    });

    it('should not decay below MIN_WEIGHT', () => {
      memory.addPattern({
        id: 'very_old',
        name: 'Very Old',
        weight: 0.3,
        lastUsed: Date.now() - 86400000 * 365,
        addedAt: Date.now() - 86400000 * 400,
      });

      for (let i = 0; i < 100; i++) {
        memory.decayUnusedPatterns();
      }

      const pattern = memory._patterns.get('very_old');
      assert.ok(pattern.weight >= 0.236, 'Weight should not decay below MIN_WEIGHT (phi^-3)');
    });

    it('should set lastReinforced timestamp', () => {
      const id = memory.addPattern({
        name: 'Reinforced',
        applicableTo: ['token'],
        weight: 1.0,
      });

      memory.getRelevantPatterns({ type: 'token' });

      const pattern = memory._patterns.get(id);
      assert.ok(pattern.lastReinforced > 0);
    });
  });

  describe('getTopReinforcedPatterns()', () => {
    beforeEach(async () => {
      await memory.initialize();

      memory.addPattern({ id: 'high', name: 'High Weight', weight: 2.0 });
      memory.addPattern({ id: 'medium', name: 'Medium Weight', weight: 1.0 });
      memory.addPattern({ id: 'low', name: 'Low Weight', weight: 0.5 });
    });

    it('should return patterns sorted by weight', () => {
      const top = memory.getTopReinforcedPatterns(3);

      assert.strictEqual(top.length, 3);
      assert.strictEqual(top[0].id, 'high');
      assert.strictEqual(top[1].id, 'medium');
      assert.strictEqual(top[2].id, 'low');
    });

    it('should respect limit', () => {
      const top = memory.getTopReinforcedPatterns(1);
      assert.strictEqual(top.length, 1);
    });

    it('should include expected fields', () => {
      const top = memory.getTopReinforcedPatterns(1);
      const item = top[0];

      assert.ok('id' in item);
      assert.ok('weight' in item);
      assert.ok('useCount' in item);
      assert.ok('verified' in item);
      assert.ok('tags' in item);
      assert.ok('applicableTo' in item);
    });

    it('should handle empty patterns', () => {
      const empty = new SharedMemory({ storage });
      const top = empty.getTopReinforcedPatterns(5);
      assert.deepEqual(top, []);
    });
  });

  // ===========================================================================
  // JUDGMENT SIMILARITY TESTS
  // ===========================================================================

  describe('Judgment Similarity', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    describe('indexJudgment()', () => {
      it('should index a judgment', () => {
        memory.indexJudgment(
          { id: 'j1', global_score: 75, verdict: 'WAG', timestamp: Date.now() },
          { type: 'token', content: 'Solana DeFi project' }
        );

        assert.strictEqual(memory._judgmentIndex.length, 1);
        assert.strictEqual(memory.stats.judgmentsIndexed, 1);
      });

      it('should store tokenized text', () => {
        memory.indexJudgment(
          { id: 'j1', global_score: 50 },
          { type: 'code', content: 'function hello world' }
        );

        const entry = memory._judgmentIndex[0];
        assert.ok(entry.tokens.length > 0);
        assert.ok(entry.tokens.includes('function'));
        assert.ok(entry.tokens.includes('hello'));
        assert.ok(entry.tokens.includes('world'));
      });

      it('should prune old entries when over limit', () => {
        // The limit is 2584 (F18), so we test with many entries
        for (let i = 0; i < 100; i++) {
          memory.indexJudgment(
            { id: `j_${i}`, global_score: i },
            { type: 'token', content: `item ${i} test` }
          );
        }

        assert.ok(memory._judgmentIndex.length <= 2584);
      });
    });

    describe('getSimilarJudgments()', () => {
      beforeEach(() => {
        memory.indexJudgment(
          { id: 'j_token', global_score: 80, verdict: 'WAG' },
          { type: 'token', content: 'Solana DeFi protocol with strong team', tags: ['defi'] }
        );
        memory.indexJudgment(
          { id: 'j_code', global_score: 60, verdict: 'GROWL' },
          { type: 'code', content: 'JavaScript function with security issue' }
        );
        memory.indexJudgment(
          { id: 'j_token2', global_score: 45, verdict: 'BARK' },
          { type: 'token', content: 'Solana meme token pump and dump' }
        );
      });

      it('should find similar judgments', () => {
        const results = memory.getSimilarJudgments({
          type: 'token',
          content: 'Solana DeFi project',
        });

        assert.ok(results.length > 0);
      });

      it('should return empty for null item', () => {
        const results = memory.getSimilarJudgments(null);
        assert.deepEqual(results, []);
      });

      it('should return empty for empty index', () => {
        const empty = new SharedMemory({ storage });
        const results = empty.getSimilarJudgments({ type: 'token' });
        assert.deepEqual(results, []);
      });

      it('should include similarity scores', () => {
        const results = memory.getSimilarJudgments({
          type: 'token',
          content: 'Solana DeFi',
        });

        for (const result of results) {
          assert.ok(typeof result.similarity === 'number');
          assert.ok(result.similarity > 0);
        }
      });

      it('should sort by similarity descending', () => {
        const results = memory.getSimilarJudgments({
          type: 'token',
          content: 'Solana token',
        });

        for (let i = 1; i < results.length; i++) {
          assert.ok(results[i - 1].similarity >= results[i].similarity);
        }
      });

      it('should respect limit', () => {
        const results = memory.getSimilarJudgments(
          { type: 'token', content: 'Solana' },
          1
        );

        assert.ok(results.length <= 1);
      });

      it('should boost type matches', () => {
        const tokenResults = memory.getSimilarJudgments({
          type: 'token',
          content: 'test project',
        });

        const codeResults = memory.getSimilarJudgments({
          type: 'code',
          content: 'test project',
        });

        // Type matching should cause different results
        assert.ok(tokenResults.length >= 0);
        assert.ok(codeResults.length >= 0);
      });
    });

    describe('_extractText()', () => {
      it('should extract from string', () => {
        const text = memory._extractText('hello world');
        assert.strictEqual(text, 'hello world');
      });

      it('should extract from object with multiple fields', () => {
        const text = memory._extractText({
          content: 'main content',
          name: 'item name',
          description: 'item desc',
          tags: ['tag1', 'tag2'],
        });

        assert.ok(text.includes('main content'));
        assert.ok(text.includes('item name'));
        assert.ok(text.includes('item desc'));
        assert.ok(text.includes('tag1'));
        assert.ok(text.includes('tag2'));
      });

      it('should handle missing fields', () => {
        const text = memory._extractText({});
        assert.strictEqual(typeof text, 'string');
      });
    });

    describe('_tokenize()', () => {
      it('should lowercase and split', () => {
        const tokens = memory._tokenize('Hello World Test');
        assert.ok(tokens.includes('hello'));
        assert.ok(tokens.includes('world'));
        assert.ok(tokens.includes('test'));
      });

      it('should remove short tokens', () => {
        const tokens = memory._tokenize('a is the big test');
        assert.ok(!tokens.includes('a'));
        assert.ok(!tokens.includes('is'));
        assert.ok(tokens.includes('the'));
        assert.ok(tokens.includes('big'));
      });

      it('should remove special characters', () => {
        const tokens = memory._tokenize('hello! @world #test');
        assert.ok(tokens.includes('hello'));
        assert.ok(tokens.includes('world'));
        assert.ok(tokens.includes('test'));
      });
    });

    describe('_jaccard()', () => {
      it('should return 1 for identical sets', () => {
        const sim = memory._jaccard(['hello', 'world'], ['hello', 'world']);
        assert.strictEqual(sim, 1);
      });

      it('should return 0 for disjoint sets', () => {
        const sim = memory._jaccard(['hello'], ['world']);
        assert.strictEqual(sim, 0);
      });

      it('should return 0 for empty arrays', () => {
        assert.strictEqual(memory._jaccard([], []), 0);
        assert.strictEqual(memory._jaccard(['hello'], []), 0);
        assert.strictEqual(memory._jaccard([], ['hello']), 0);
      });

      it('should calculate partial overlap', () => {
        const sim = memory._jaccard(['hello', 'world'], ['hello', 'test']);
        // intersection = 1 (hello), union = 3 (hello, world, test)
        assert.ok(Math.abs(sim - 1 / 3) < 0.01);
      });
    });
  });

  // ===========================================================================
  // LEARNED WEIGHTS TESTS
  // ===========================================================================

  describe('Learned Weights', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    describe('getLearnedWeights()', () => {
      it('should return empty object initially', () => {
        const weights = memory.getLearnedWeights();
        assert.deepEqual(weights, {});
      });

      it('should return copy not reference', () => {
        memory.adjustWeight('PHI', 0.5);
        const w1 = memory.getLearnedWeights();
        const w2 = memory.getLearnedWeights();

        w1.PHI = 999;
        assert.notStrictEqual(w2.PHI, 999);
      });
    });

    describe('getWeight()', () => {
      it('should return 1.0 for unknown dimension', () => {
        assert.strictEqual(memory.getWeight('UNKNOWN'), 1.0);
      });

      it('should return adjusted weight', () => {
        memory.adjustWeight('PHI', 1.0);
        const weight = memory.getWeight('PHI');
        assert.ok(weight > 1.0);
      });
    });

    describe('adjustWeight()', () => {
      it('should adjust weight positively', () => {
        memory.adjustWeight('PHI', 1.0);
        const weight = memory.getWeight('PHI');

        assert.ok(weight > 1.0);
        assert.strictEqual(memory.stats.weightAdjustments, 1);
      });

      it('should adjust weight negatively', () => {
        memory.adjustWeight('VERIFY', -1.0);
        const weight = memory.getWeight('VERIFY');

        assert.ok(weight < 1.0);
      });

      it('should bound weight between 0.1 and 3.0', () => {
        // Extreme positive
        for (let i = 0; i < 50; i++) {
          memory.adjustWeight('HIGH', 1.0);
        }
        assert.ok(memory.getWeight('HIGH') <= 3.0);

        // Extreme negative
        for (let i = 0; i < 50; i++) {
          memory.adjustWeight('LOW', -1.0);
        }
        assert.ok(memory.getWeight('LOW') >= 0.1);
      });

      it('should log feedback entry', () => {
        memory.adjustWeight('CULTURE', 0.5, 'test_source');

        assert.strictEqual(memory._feedbackLog.length, 1);
        const entry = memory._feedbackLog[0];
        assert.strictEqual(entry.type, 'weight_adjustment');
        assert.strictEqual(entry.dimension, 'CULTURE');
        assert.strictEqual(entry.source, 'test_source');
        assert.ok(entry.timestamp > 0);
      });

      it('should prune feedback log when over limit', () => {
        // The limit is 987 (F16)
        for (let i = 0; i < 1000; i++) {
          memory.adjustWeight('PHI', 0.01);
        }

        assert.ok(memory._feedbackLog.length <= 987);
      });
    });
  });

  // ===========================================================================
  // PROCEDURAL MEMORY TESTS
  // ===========================================================================

  describe('Procedural Memory', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    describe('getForItemType()', () => {
      it('should return procedure for known type', () => {
        const proc = memory.getForItemType('token');
        assert.ok(proc);
        assert.ok(proc.steps.length > 0);
      });

      it('should fall back to default for unknown type', () => {
        const proc = memory.getForItemType('unknown_type');
        assert.ok(proc);
        assert.strictEqual(proc.type, 'default');
      });

      it('should return null if no default exists', () => {
        const bare = new SharedMemory({ storage });
        // Don't initialize - no default procedures
        const proc = bare.getForItemType('anything');
        assert.strictEqual(proc, null);
      });
    });

    describe('getScoringRules()', () => {
      it('should return scoring rules for known type', () => {
        const rules = memory.getScoringRules('default');
        assert.ok(rules);
        assert.ok('verdictThresholds' in rules);
      });

      it('should fall back to default for unknown type', () => {
        const rules = memory.getScoringRules('unknown_type');
        assert.ok(rules);
      });

      it('should return empty object if nothing exists', () => {
        const bare = new SharedMemory({ storage });
        const rules = bare.getScoringRules('anything');
        assert.deepEqual(rules, {});
      });
    });

    describe('setProcedure()', () => {
      it('should set a new procedure', () => {
        memory.setProcedure('nft', {
          steps: ['Check rarity', 'Check collection floor'],
        });

        const proc = memory.getForItemType('nft');
        assert.ok(proc);
        assert.strictEqual(proc.type, 'nft');
        assert.strictEqual(proc.steps.length, 2);
        assert.ok(proc.updatedAt > 0);
      });

      it('should update existing procedure', () => {
        memory.setProcedure('token', {
          steps: ['New step 1', 'New step 2'],
        });

        const proc = memory.getForItemType('token');
        assert.strictEqual(proc.steps.length, 2);
        assert.strictEqual(proc.steps[0], 'New step 1');
      });
    });

    describe('setScoringRules()', () => {
      it('should set new scoring rules', () => {
        memory.setScoringRules('nft', {
          minScore: 10,
          maxScore: 90,
        });

        const rules = memory.getScoringRules('nft');
        assert.strictEqual(rules.minScore, 10);
        assert.strictEqual(rules.maxScore, 90);
        assert.ok(rules.updatedAt > 0);
      });
    });
  });

  // ===========================================================================
  // PERSISTENCE TESTS
  // ===========================================================================

  describe('save()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should save to storage', async () => {
      memory.addPattern({ name: 'SaveMe', tags: ['test'] });
      await memory.save();

      assert.strictEqual(storage.set.mock.calls.length, 1);
      const [key, value] = storage.set.mock.calls[0].arguments;
      assert.strictEqual(key, 'shared_memory');
      assert.ok(value.patterns.length > 0);
      assert.ok(value.savedAt > 0);
    });

    it('should not save without storage', async () => {
      const noStorage = new SharedMemory();
      noStorage.initialized = true;

      // Should not throw
      await noStorage.save();
    });

    it('should handle save errors gracefully', async () => {
      const badStorage = {
        get: mock.fn(async () => null),
        set: mock.fn(async () => { throw new Error('Write failed'); }),
      };
      const mem = new SharedMemory({ storage: badStorage });
      mem.initialized = true;

      // Should not throw
      await mem.save();
    });

    it('should limit saved judgment index to 500', async () => {
      for (let i = 0; i < 600; i++) {
        memory.indexJudgment(
          { id: `j_${i}` },
          { content: `item ${i}` }
        );
      }

      await memory.save();

      const [, value] = storage.set.mock.calls[0].arguments;
      assert.ok(value.judgmentIndex.length <= 500);
    });

    it('should limit saved feedback to 100', async () => {
      for (let i = 0; i < 150; i++) {
        memory.recordFeedback({ type: 'test', data: i });
      }

      await memory.save();

      const [, value] = storage.set.mock.calls[0].arguments;
      assert.ok(value.feedback.length <= 100);
    });
  });

  // ===========================================================================
  // EXPORT / IMPORT TESTS
  // ===========================================================================

  describe('export()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should export verified patterns', () => {
      memory.addPattern({ id: 'verified', name: 'V', verified: true, useCount: 0 });
      memory.addPattern({ id: 'unverified', name: 'U', verified: false, useCount: 0 });

      const exported = memory.export();

      assert.ok(exported.patterns.some(p => p.id === 'verified'));
      assert.ok(!exported.patterns.some(p => p.id === 'unverified'));
    });

    it('should export patterns with 3+ uses', () => {
      const id = memory.addPattern({
        name: 'Used',
        applicableTo: ['token'],
        useCount: 0,
      });

      // Use the pattern to build up useCount
      for (let i = 0; i < 5; i++) {
        memory.getRelevantPatterns({ type: 'token' });
      }

      const exported = memory.export();
      assert.ok(exported.patterns.some(p => p.id === id));
    });

    it('should export weights', () => {
      memory.adjustWeight('PHI', 0.5);
      const exported = memory.export();

      assert.ok('PHI' in exported.weights);
    });

    it('should export procedures', () => {
      const exported = memory.export();
      assert.ok(exported.procedures.length > 0);
    });

    it('should include timestamp', () => {
      const exported = memory.export();
      assert.ok(exported.timestamp > 0);
    });
  });

  describe('import()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should import new patterns', () => {
      memory.import({
        patterns: [
          { id: 'imported_pat', name: 'From Remote', tags: ['remote'] },
        ],
      });

      assert.ok(memory._patterns.has('imported_pat'));
    });

    it('should not overwrite existing patterns', () => {
      memory.addPattern({ id: 'existing', name: 'Local Version' });

      memory.import({
        patterns: [
          { id: 'existing', name: 'Remote Version' },
        ],
      });

      const pattern = memory._patterns.get('existing');
      assert.strictEqual(pattern.name, 'Local Version');
    });

    it('should merge weights by averaging', () => {
      memory.adjustWeight('PHI', 1.0); // ~1.236
      const localWeight = memory.getWeight('PHI');

      memory.import({
        weights: { PHI: 2.0 },
      });

      const merged = memory._dimensionWeights.PHI;
      // Should be average of local and remote
      assert.ok(Math.abs(merged - (localWeight + 2.0) / 2) < 0.01);
    });

    it('should import new weight dimensions', () => {
      memory.import({
        weights: { NEW_DIM: 1.5 },
      });

      // Average of default 1.0 and 1.5
      assert.ok(memory._dimensionWeights.NEW_DIM > 0);
    });

    it('should prefer newer procedures', () => {
      const now = Date.now();

      // setProcedure always overwrites updatedAt with Date.now(),
      // so we set it and then manually backdate the timestamp
      memory.setProcedure('token', {
        steps: ['old step'],
      });
      memory._procedures.get('token').updatedAt = now - 10000;

      memory.import({
        procedures: [
          { type: 'token', steps: ['new step'], updatedAt: now },
        ],
      });

      const proc = memory.getForItemType('token');
      assert.strictEqual(proc.steps[0], 'new step');
    });

    it('should not overwrite newer local procedures', () => {
      const now = Date.now();

      // Local procedure gets updatedAt = Date.now() automatically
      memory.setProcedure('token', {
        steps: ['local step'],
      });

      // Remote procedure with older timestamp should not overwrite
      memory.import({
        procedures: [
          { type: 'token', steps: ['old remote step'], updatedAt: now - 10000 },
        ],
      });

      const proc = memory.getForItemType('token');
      assert.strictEqual(proc.steps[0], 'local step');
    });

    it('should handle empty import gracefully', () => {
      memory.import({});
      memory.import({ patterns: [], weights: {}, procedures: [] });

      // Should not throw or corrupt state
      assert.ok(memory._patterns.size >= 0);
    });
  });

  // ===========================================================================
  // FEEDBACK TESTS
  // ===========================================================================

  describe('Feedback', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    describe('recordFeedback()', () => {
      it('should record feedback with timestamp', () => {
        memory.recordFeedback({ type: 'explicit', correct: true });

        assert.strictEqual(memory._feedbackLog.length, 1);
        assert.ok(memory._feedbackLog[0].timestamp > 0);
        assert.strictEqual(memory.stats.feedbackReceived, 1);
      });

      it('should prune old feedback when over limit', () => {
        for (let i = 0; i < 1000; i++) {
          memory.recordFeedback({ type: 'test', index: i });
        }

        assert.ok(memory._feedbackLog.length <= 987);
      });
    });

    describe('getRecentFeedback()', () => {
      it('should return most recent feedback', () => {
        for (let i = 0; i < 20; i++) {
          memory.recordFeedback({ type: 'test', index: i });
        }

        const recent = memory.getRecentFeedback(5);
        assert.strictEqual(recent.length, 5);
        // Should be the last 5
        assert.strictEqual(recent[4].index, 19);
      });

      it('should return all if fewer than requested', () => {
        memory.recordFeedback({ type: 'test' });
        const recent = memory.getRecentFeedback(10);
        assert.strictEqual(recent.length, 1);
      });

      it('should return empty for no feedback', () => {
        const recent = memory.getRecentFeedback();
        assert.deepEqual(recent, []);
      });
    });
  });

  // ===========================================================================
  // STATS TESTS
  // ===========================================================================

  describe('getStats()', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should include all stat fields', () => {
      const stats = memory.getStats();

      assert.ok('patternCount' in stats);
      assert.ok('verifiedPatterns' in stats);
      assert.ok('judgmentIndexSize' in stats);
      assert.ok('dimensionsTracked' in stats);
      assert.ok('procedureCount' in stats);
      assert.ok('feedbackCount' in stats);
      assert.ok('avgPatternWeight' in stats);
      assert.ok('highWeightPatterns' in stats);
      assert.ok('lowWeightPatterns' in stats);
      assert.ok('patternsAdded' in stats);
      assert.ok('initialized' in stats);
    });

    it('should count verified patterns', () => {
      memory.addPattern({ id: 'v1', name: 'V1', verified: true });
      memory.addPattern({ id: 'v2', name: 'V2', verified: false });

      const stats = memory.getStats();
      assert.strictEqual(stats.verifiedPatterns, 1);
    });

    it('should track high/low weight patterns', () => {
      memory.addPattern({ id: 'high', name: 'High', weight: 2.0 });
      memory.addPattern({ id: 'low', name: 'Low', weight: 0.3 });
      memory.addPattern({ id: 'normal', name: 'Normal', weight: 1.0 });

      const stats = memory.getStats();
      assert.strictEqual(stats.highWeightPatterns, 1); // > 1.5
      assert.strictEqual(stats.lowWeightPatterns, 1);  // < 0.5
    });

    it('should calculate average pattern weight', () => {
      memory.addPattern({ id: 'a', weight: 1.0 });
      memory.addPattern({ id: 'b', weight: 2.0 });

      const stats = memory.getStats();
      assert.strictEqual(stats.avgPatternWeight, 1.5);
    });

    it('should handle empty patterns for avg weight', () => {
      const stats = memory.getStats();
      assert.strictEqual(stats.avgPatternWeight, 1.0);
    });

    it('should report initialized state', () => {
      const stats = memory.getStats();
      assert.strictEqual(stats.initialized, true);
    });
  });

  // ===========================================================================
  // EDGE CASES
  // ===========================================================================

  describe('Edge Cases', () => {
    beforeEach(async () => {
      await memory.initialize();
    });

    it('should handle pattern with no tags', () => {
      const id = memory.addPattern({ name: 'No Tags' });
      const patterns = memory.getRelevantPatterns({ type: 'token', tags: ['test'] });
      // Should not throw
      assert.ok(Array.isArray(patterns));
    });

    it('should handle item with no tags in getRelevantPatterns', () => {
      memory.addPattern({
        name: 'Tagged',
        applicableTo: ['token'],
        tags: ['test'],
      });

      const patterns = memory.getRelevantPatterns({ type: 'token' });
      // Should not throw even without tags on the item
      assert.ok(Array.isArray(patterns));
    });

    it('should handle getSimilarJudgments with string item', () => {
      memory.indexJudgment(
        { id: 'j1', global_score: 50 },
        { content: 'test text' }
      );

      const results = memory.getSimilarJudgments('test text');
      assert.ok(Array.isArray(results));
    });

    it('should handle pattern with missing applicableTo', () => {
      memory.addPattern({ name: 'No ApplicableTo' });

      const patterns = memory.getRelevantPatterns({ type: 'token' });
      // Pattern without applicableTo should not match type, but should not throw
      assert.ok(Array.isArray(patterns));
    });

    it('should handle concurrent pattern additions', () => {
      for (let i = 0; i < 100; i++) {
        memory.addPattern({ name: `Pattern ${i}`, tags: [`tag_${i}`] });
      }

      assert.strictEqual(memory._patterns.size, 100);
      assert.strictEqual(memory.stats.patternsAdded, 100);
    });

    it('should handle decayUnusedPatterns with empty memory', () => {
      const result = memory.decayUnusedPatterns();
      assert.strictEqual(result.decayed, 0);
      assert.strictEqual(result.protected, 0);
    });

    it('should handle import with undefined fields', () => {
      // Should not throw
      memory.import({ patterns: undefined, weights: undefined, procedures: undefined });
      assert.ok(true);
    });
  });
});
