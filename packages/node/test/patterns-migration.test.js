/**
 * @cynic/node - Patterns Migration E2E Tests
 *
 * Verifies that patterns are correctly loaded from PostgreSQL
 * and transformed into SharedMemory format.
 *
 * Task #19: PHASE A1 - Migrate patterns table → SharedMemory
 *
 * @module @cynic/node/test/patterns-migration
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';
import { globalEventBus } from '@cynic/core';
import { _resetForTesting, getSharedMemory, getCollectivePackAsync } from '../src/collective-singleton.js';

// =============================================================================
// CLEANUP HELPER
// =============================================================================

function cleanupSingletons() {
  _resetForTesting();
  globalEventBus.removeAllListeners();
}

// =============================================================================
// MOCK HELPERS
// =============================================================================

/**
 * Create mock persistence with patterns repository
 */
function createMockPersistence(patterns = []) {
  const mockPatternRepo = {
    list: mock.fn(async (options = {}) => {
      const { limit = 1597, offset = 0 } = options;
      return patterns.slice(offset, offset + limit);
    }),
    upsert: mock.fn(async (pattern) => ({ ...pattern, id: pattern.patternId })),
  };

  return {
    query: mock.fn(async (sql) => {
      // Handle cynic_kv table creation
      if (sql.includes('CREATE TABLE IF NOT EXISTS cynic_kv')) {
        return { rows: [] };
      }
      // Handle SharedMemory storage get (return empty to trigger migration)
      if (sql.includes('SELECT data FROM cynic_kv')) {
        return { rows: [] };
      }
      return { rows: [] };
    }),
    getRepository: mock.fn((name) => {
      if (name === 'patterns') return mockPatternRepo;
      return null;
    }),
  };
}

/**
 * Create sample PostgreSQL patterns
 */
function createSamplePgPatterns(count = 10) {
  return Array.from({ length: count }, (_, i) => ({
    pattern_id: `pat_test_${i.toString().padStart(4, '0')}`,
    category: i % 3 === 0 ? 'code' : i % 3 === 1 ? 'security' : 'analysis',
    name: `Test Pattern ${i}`,
    description: `Description for pattern ${i}`,
    confidence: 0.5 + (i % 50) * 0.01, // 0.5 to 0.99
    frequency: Math.pow(2, i % 10), // 1 to 512
    tags: ['test', `tag_${i % 5}`],
    data: { originalIndex: i },
    source_judgments: [`jdg_${i}`],
    source_count: i + 1,
    created_at: new Date(Date.now() - i * 3600000).toISOString(),
    updated_at: new Date(Date.now() - i * 1800000).toISOString(),
  }));
}

// =============================================================================
// TESTS
// =============================================================================

describe('Patterns Migration (PostgreSQL → SharedMemory)', () => {
  beforeEach(() => {
    process.env.NODE_ENV = 'test';
    _resetForTesting();
  });

  afterEach(() => cleanupSingletons());

  it('should load patterns from PostgreSQL when SharedMemory is empty', async () => {
    // Create 100 sample patterns
    const pgPatterns = createSamplePgPatterns(100);
    const persistence = createMockPersistence(pgPatterns);

    // Initialize CollectivePack with persistence
    await getCollectivePackAsync({ persistence });

    // Verify patterns repository was called (may be called 1-2 times due to concurrent init)
    const patternsRepo = persistence.getRepository('patterns');
    assert.ok(patternsRepo.list.mock.callCount() >= 1, 'list() should be called at least once');

    // Verify list was called with correct limit (500 via loadFromPostgres)
    const listCall = patternsRepo.list.mock.calls[0];
    assert.strictEqual(listCall.arguments[0].limit, 500, 'Should request up to 500 patterns (fast startup)');

    // Verify SharedMemory has patterns
    const sharedMemory = getSharedMemory();
    assert.strictEqual(sharedMemory._patterns.size, 100, 'SharedMemory should have 100 patterns');
  });

  it('should transform PostgreSQL pattern format correctly', async () => {
    const pgPatterns = [{
      pattern_id: 'pat_transform_test',
      category: 'security',
      name: 'Security Pattern',
      description: 'Test transformation',
      confidence: 0.75,
      frequency: 10,
      tags: ['security', 'critical'],
      data: { extra: 'data' },
      source_judgments: ['jdg_1', 'jdg_2'],
      source_count: 2,
      created_at: '2026-01-15T00:00:00Z',
      updated_at: '2026-01-16T00:00:00Z',
    }];

    const persistence = createMockPersistence(pgPatterns);
    await getCollectivePackAsync({ persistence });

    const sharedMemory = getSharedMemory();
    const pattern = sharedMemory._patterns.get('pat_transform_test');

    assert.ok(pattern, 'Pattern should exist in SharedMemory');

    // Verify transformation
    assert.strictEqual(pattern.id, 'pat_transform_test');
    assert.strictEqual(pattern.name, 'Security Pattern');
    assert.strictEqual(pattern.category, 'security');
    assert.deepStrictEqual(pattern.tags, ['security', 'critical']);
    assert.deepStrictEqual(pattern.applicableTo, ['security', '*']);

    // Verify φ-aligned weights
    assert.ok(pattern.weight >= 0.5, 'Weight should be at least 0.5');
    assert.ok(pattern.weight <= 2.618, 'Weight should not exceed φ+1');

    // Verify Fisher importance derived from confidence
    assert.ok(pattern.fisherImportance >= 0.75 * 1.2 || pattern.fisherImportance === 1.0,
      'Fisher importance should be derived from confidence');

    // Verify useCount from frequency
    assert.strictEqual(pattern.useCount, 10, 'useCount should equal frequency');

    // Verify timestamps
    assert.ok(pattern.addedAt, 'Should have addedAt');
    assert.ok(pattern.lastUsed, 'Should have lastUsed');
  });

  it('should skip PostgreSQL load if SharedMemory already has patterns', async () => {
    // Pre-populate SharedMemory
    const persistence = {
      query: mock.fn(async (sql) => {
        if (sql.includes('CREATE TABLE IF NOT EXISTS cynic_kv')) {
          return { rows: [] };
        }
        // Return existing patterns from cynic_kv
        if (sql.includes('SELECT data FROM cynic_kv')) {
          return {
            rows: [{
              data: {
                patterns: [['existing_pat', { id: 'existing_pat', name: 'Existing' }]],
                weights: {},
                procedures: [],
                scoringRules: [],
                feedback: [],
                stats: {},
              },
            }],
          };
        }
        return { rows: [] };
      }),
      getRepository: mock.fn((name) => {
        if (name === 'patterns') {
          return {
            list: mock.fn(async () => [{ pattern_id: 'should_not_load', name: 'Should Not Load' }]),
          };
        }
        return null;
      }),
    };

    await getCollectivePackAsync({ persistence });

    // Verify patterns repo was NOT called (since SharedMemory had patterns)
    const patternsRepo = persistence.getRepository('patterns');
    assert.strictEqual(patternsRepo.list.mock.callCount(), 0, 'list() should NOT be called when patterns exist');

    // Verify SharedMemory has the existing pattern (not the PostgreSQL one)
    const sharedMemory = getSharedMemory();
    assert.ok(sharedMemory._patterns.has('existing_pat'), 'Should have existing pattern');
    assert.ok(!sharedMemory._patterns.has('should_not_load'), 'Should NOT load from PostgreSQL');
  });

  it('should handle high-confidence patterns as verified and locked', async () => {
    const pgPatterns = [{
      pattern_id: 'pat_high_conf',
      category: 'proven',
      name: 'High Confidence Pattern',
      confidence: 0.85, // Above φ⁻¹ (0.618)
      frequency: 5, // Meets minimum threshold
      tags: [],
      data: {},
    }];

    const persistence = createMockPersistence(pgPatterns);
    await getCollectivePackAsync({ persistence });

    const sharedMemory = getSharedMemory();
    const pattern = sharedMemory._patterns.get('pat_high_conf');

    assert.ok(pattern.verified, 'High-confidence + high-frequency pattern should be verified');
    assert.ok(pattern.consolidationLocked, 'High Fisher importance should trigger EWC lock');
  });

  it('should respect MAX_PATTERNS limit (1597 = F17)', async () => {
    // Create 2000 patterns (more than limit)
    const pgPatterns = createSamplePgPatterns(2000);
    const persistence = createMockPersistence(pgPatterns);

    await getCollectivePackAsync({ persistence });

    const sharedMemory = getSharedMemory();

    // Should be limited to 1597 by the list() call
    // (SharedMemory also has its own limit via addPattern pruning)
    assert.ok(sharedMemory._patterns.size <= 1597,
      `Patterns should not exceed 1597, got ${sharedMemory._patterns.size}`);
  });
});

describe('getLockedPatterns() for PostgreSQL sync', () => {
  beforeEach(() => {
    process.env.NODE_ENV = 'test';
    _resetForTesting();
  });

  afterEach(() => cleanupSingletons());

  it('should return locked patterns in PostgreSQL format', async () => {
    const persistence = createMockPersistence([]);
    await getCollectivePackAsync({ persistence });

    const sharedMemory = getSharedMemory();

    // Add a pattern and lock it
    sharedMemory.addPattern({
      id: 'pat_to_lock',
      name: 'Lockable Pattern',
      category: 'test',
      tags: ['important'],
      fisherImportance: 0.7, // Above lock threshold
      consolidationLocked: true,
    });

    const lockedPatterns = sharedMemory.getLockedPatterns();

    assert.strictEqual(lockedPatterns.length, 1, 'Should have 1 locked pattern');

    // Verify PostgreSQL format
    const pgFormat = lockedPatterns[0];
    assert.strictEqual(pgFormat.patternId, 'pat_to_lock');
    assert.strictEqual(pgFormat.name, 'Lockable Pattern');
    assert.strictEqual(pgFormat.category, 'test');
    assert.deepStrictEqual(pgFormat.tags, ['important']);
    assert.ok(pgFormat.confidence > 0, 'Should have confidence');
    assert.ok(pgFormat.frequency >= 1, 'Should have frequency');
    assert.ok(pgFormat.data.consolidationLocked, 'Should preserve lock state in data');
  });
});
