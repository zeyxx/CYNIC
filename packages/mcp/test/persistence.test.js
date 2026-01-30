/**
 * MCP Persistence Tests
 *
 * Tests for the unified persistence layer with fallback chain.
 *
 * @module @cynic/mcp/test/persistence
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { PersistenceManager } from '../src/persistence.js';
import path from 'path';
import fs from 'fs/promises';
import os from 'os';

describe('PersistenceManager', () => {
  let manager;

  beforeEach(() => {
    // Create manager without any external dependencies (pure in-memory)
    manager = new PersistenceManager({ forceMemory: true });
  });

  afterEach(async () => {
    if (manager._initialized) {
      await manager.close();
    }
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      assert.equal(manager.dataDir, null);
      assert.equal(manager._backend, 'none');
      assert.equal(manager._initialized, false);
    });

    it('accepts dataDir option', () => {
      const m = new PersistenceManager({ dataDir: '/tmp/cynic' });
      assert.equal(m.dataDir, '/tmp/cynic');
    });
  });

  describe('initialize', () => {
    it('initializes in memory mode when no config', async () => {
      await manager.initialize();

      assert.equal(manager._initialized, true);
      assert.equal(manager._backend, 'memory');
      assert.ok(manager._fallback);
    });

    it('is idempotent', async () => {
      await manager.initialize();
      await manager.initialize();

      assert.equal(manager._initialized, true);
    });
  });

  describe('capabilities', () => {
    it('reports available capabilities', async () => {
      await manager.initialize();

      const caps = manager.capabilities;

      // With fallback, most features are available
      assert.equal(caps.judgments, true);
      assert.equal(caps.patterns, true);
      assert.equal(caps.feedback, true);
      assert.equal(caps.knowledge, true);
      assert.equal(caps.pojChain, true);
      assert.equal(caps.libraryCache, false); // No PostgreSQL
      assert.equal(caps.sessions, false); // No Redis
    });
  });

  describe('isAvailable', () => {
    it('returns false before initialization', () => {
      assert.ok(!manager.isAvailable);
    });

    it('returns true after initialization', async () => {
      await manager.initialize();
      assert.ok(manager.isAvailable);
    });
  });

  describe('storeJudgment', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('stores judgment in fallback', async () => {
      const judgment = {
        q_score: 75,
        verdict: 'WAG',
        confidence: 0.6,
        item_type: 'test',
      };

      const stored = await manager.storeJudgment(judgment);

      assert.ok(stored);
      assert.ok(stored.judgment_id);
      assert.equal(stored.q_score, 75);
    });
  });

  describe('searchJudgments', () => {
    beforeEach(async () => {
      await manager.initialize();
      await manager.storeJudgment({ q_score: 80, verdict: 'WAG', content: 'test alpha' });
      await manager.storeJudgment({ q_score: 60, verdict: 'GROWL', content: 'test beta' });
    });

    it('searches judgments by query', async () => {
      const results = await manager.searchJudgments('alpha');

      assert.ok(Array.isArray(results));
    });

    it('returns all when no query', async () => {
      const results = await manager.searchJudgments('', { limit: 10 });

      assert.ok(results.length >= 2);
    });

    it('respects limit option', async () => {
      const results = await manager.searchJudgments('', { limit: 1 });

      assert.equal(results.length, 1);
    });
  });

  describe('getRecentJudgments', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('returns recent judgments', async () => {
      await manager.storeJudgment({ q_score: 70, verdict: 'WAG' });
      await manager.storeJudgment({ q_score: 80, verdict: 'WAG' });

      const recent = await manager.getRecentJudgments(5);

      assert.ok(Array.isArray(recent));
      assert.ok(recent.length >= 2);
    });

    it('returns empty array when no judgments', async () => {
      const recent = await manager.getRecentJudgments(10);

      assert.deepEqual(recent, []);
    });
  });

  describe('getJudgmentStats', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('calculates stats from judgments', async () => {
      await manager.storeJudgment({ q_score: 80, verdict: 'WAG', confidence: 0.6 });
      await manager.storeJudgment({ q_score: 60, verdict: 'GROWL', confidence: 0.5 });

      const stats = await manager.getJudgmentStats();

      assert.equal(stats.total, 2);
      assert.ok(stats.avgScore > 0);
      assert.ok(stats.verdicts);
    });

    it('returns zeros when empty', async () => {
      const stats = await manager.getJudgmentStats();

      assert.equal(stats.total, 0);
      assert.equal(stats.avgScore, 0);
    });
  });

  describe('storeFeedback', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('stores feedback', async () => {
      const feedback = {
        judgment_id: 'jdg_test',
        outcome: 'correct',
        reason: 'good judgment',
      };

      const stored = await manager.storeFeedback(feedback);

      assert.ok(stored);
      assert.ok(stored.feedback_id);
    });
  });

  describe('storeKnowledge', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('stores knowledge', async () => {
      const knowledge = {
        summary: 'Test knowledge entry',
        content: 'Full content here',
        source_type: 'document',
      };

      const stored = await manager.storeKnowledge(knowledge);

      assert.ok(stored);
      assert.ok(stored.knowledge_id);
    });
  });

  describe('searchKnowledge', () => {
    beforeEach(async () => {
      await manager.initialize();
      await manager.storeKnowledge({ summary: 'React hooks guide', content: 'useEffect usage' });
      await manager.storeKnowledge({ summary: 'Python basics', content: 'Variables and types' });
    });

    it('searches knowledge by query', async () => {
      const results = await manager.searchKnowledge('React');

      assert.ok(Array.isArray(results));
    });

    it('searches in content', async () => {
      const results = await manager.searchKnowledge('useEffect');

      assert.ok(results.length >= 0);
    });
  });

  describe('upsertPattern', () => {
    beforeEach(async () => {
      await manager.initialize();
    });

    it('creates new pattern', async () => {
      const pattern = {
        name: 'test-pattern',
        category: 'anomaly',
        frequency: 5,
      };

      const stored = await manager.upsertPattern(pattern);

      assert.ok(stored);
      assert.equal(stored.name, 'test-pattern');
    });

    it('updates existing pattern', async () => {
      await manager.upsertPattern({ name: 'update-pattern', frequency: 1 });
      const updated = await manager.upsertPattern({ name: 'update-pattern', frequency: 10 });

      assert.equal(updated.frequency, 10);
    });
  });

  describe('getPatterns', () => {
    beforeEach(async () => {
      await manager.initialize();
      await manager.upsertPattern({ name: 'pat1', category: 'anomaly' });
      await manager.upsertPattern({ name: 'pat2', category: 'verdict' });
    });

    it('gets all patterns', async () => {
      const patterns = await manager.getPatterns();

      assert.ok(Array.isArray(patterns));
      assert.ok(patterns.length >= 2);
    });

    it('filters by category', async () => {
      const patterns = await manager.getPatterns({ category: 'anomaly' });

      for (const p of patterns) {
        assert.equal(p.category, 'anomaly');
      }
    });
  });
});

describe('PersistenceManager PoJ Chain', () => {
  let manager;

  beforeEach(async () => {
    manager = new PersistenceManager({ forceMemory: true });
    await manager.initialize();
  });

  afterEach(async () => {
    await manager.close();
  });

  describe('storePoJBlock', () => {
    it('stores genesis block', async () => {
      const block = {
        slot: 0,
        prev_hash: 'genesis_prev',
        judgments_root: 'genesis_root',
        judgments: [],
        hash: 'genesis_hash',
      };

      const stored = await manager.storePoJBlock(block);

      assert.ok(stored);
      assert.equal(stored.slot, 0);
    });

    it('stores subsequent blocks', async () => {
      // Store genesis first
      await manager.storePoJBlock({
        slot: 0,
        prev_hash: 'gen',
        judgments_root: 'root0',
        judgments: [],
        hash: 'hash0',
      });

      // Store next block
      const block = {
        slot: 1,
        prev_hash: 'hash0',
        judgments_root: 'root1',
        judgments: [{ judgment_id: 'jdg_1' }],
        hash: 'hash1',
      };

      const stored = await manager.storePoJBlock(block);

      assert.ok(stored);
      assert.equal(stored.slot, 1);
    });

    it('validates chain integrity', async () => {
      await manager.storePoJBlock({
        slot: 0,
        prev_hash: 'gen',
        judgments_root: 'root0',
        judgments: [],
        hash: 'hash0',
      });

      // Try to store block with wrong prev_hash
      await assert.rejects(
        async () => {
          await manager._fallback.storePoJBlock({
            slot: 1,
            prev_hash: 'wrong_hash',
            judgments_root: 'root1',
            judgments: [],
            hash: 'hash1',
          });
        },
        /Chain integrity violation/
      );
    });
  });

  describe('getPoJHead', () => {
    it('returns null when empty', async () => {
      const head = await manager.getPoJHead();
      assert.equal(head, null);
    });

    it('returns latest block', async () => {
      await manager.storePoJBlock({ slot: 0, prev_hash: 'g', judgments_root: 'r', judgments: [], hash: 'h0' });
      await manager.storePoJBlock({ slot: 1, prev_hash: 'h0', judgments_root: 'r', judgments: [], hash: 'h1' });

      const head = await manager.getPoJHead();

      assert.equal(head.slot, 1);
    });
  });

  describe('getPoJStats', () => {
    it('returns stats', async () => {
      await manager.storePoJBlock({
        slot: 0,
        prev_hash: 'g',
        judgments_root: 'r',
        judgments: [{ judgment_id: 'j1' }, { judgment_id: 'j2' }],
        hash: 'h0',
      });

      const stats = await manager.getPoJStats();

      assert.equal(stats.totalBlocks, 1);
      assert.equal(stats.headSlot, 0);
      assert.equal(stats.totalJudgments, 2);
    });
  });

  describe('getRecentPoJBlocks', () => {
    it('returns recent blocks in reverse order', async () => {
      for (let i = 0; i < 5; i++) {
        await manager.storePoJBlock({
          slot: i,
          prev_hash: i === 0 ? 'g' : `h${i - 1}`,
          judgments_root: `r${i}`,
          judgments: [],
          hash: `h${i}`,
        });
      }

      const recent = await manager.getRecentPoJBlocks(3);

      assert.equal(recent.length, 3);
      assert.equal(recent[0].slot, 4); // Most recent first
    });
  });

  describe('getPoJBlockBySlot', () => {
    beforeEach(async () => {
      await manager.storePoJBlock({ slot: 0, prev_hash: 'g', judgments_root: 'r', judgments: [], hash: 'h0' });
      await manager.storePoJBlock({ slot: 1, prev_hash: 'h0', judgments_root: 'r', judgments: [], hash: 'h1' });
    });

    it('finds block by slot', async () => {
      const block = await manager.getPoJBlockBySlot(0);

      assert.ok(block);
      assert.equal(block.slot, 0);
    });

    it('returns null for non-existent slot', async () => {
      const block = await manager.getPoJBlockBySlot(99);

      assert.equal(block, null);
    });
  });

  describe('verifyPoJChain', () => {
    it('verifies valid chain', async () => {
      await manager.storePoJBlock({ slot: 0, prev_hash: 'g', judgments_root: 'r', judgments: [], hash: 'h0' });
      await manager.storePoJBlock({ slot: 1, prev_hash: 'h0', judgments_root: 'r', judgments: [], hash: 'h1' });
      await manager.storePoJBlock({ slot: 2, prev_hash: 'h1', judgments_root: 'r', judgments: [], hash: 'h2' });

      const result = await manager.verifyPoJChain();

      assert.equal(result.valid, true);
      assert.equal(result.blocksChecked, 3);
      assert.deepEqual(result.errors, []);
    });
  });
});

describe('PersistenceManager File Storage', () => {
  let manager;
  let tempDir;

  beforeEach(async () => {
    tempDir = path.join(os.tmpdir(), `cynic-test-${Date.now()}`);
    manager = new PersistenceManager({ dataDir: tempDir, skipDatabase: true });
  });

  afterEach(async () => {
    if (manager._initialized) {
      await manager.close();
    }
    // Clean up temp directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch (e) {
      // Ignore cleanup errors
    }
  });

  describe('file-based fallback', () => {
    it('initializes with file backend', async () => {
      await manager.initialize();

      assert.equal(manager._backend, 'file');
    });

    it('persists data to file', async () => {
      await manager.initialize();
      await manager.storeJudgment({ q_score: 90, verdict: 'HOWL' });

      // Force save
      await manager._fallback.save();

      const filePath = path.join(tempDir, 'cynic-state.json');
      const content = await fs.readFile(filePath, 'utf-8');
      const data = JSON.parse(content);

      assert.ok(data.judgments);
      assert.equal(data.judgments.length, 1);
    });

    it('loads existing data on initialize', async () => {
      // Create initial data
      await fs.mkdir(tempDir, { recursive: true });
      const filePath = path.join(tempDir, 'cynic-state.json');
      await fs.writeFile(
        filePath,
        JSON.stringify({
          judgments: [{ judgment_id: 'existing', q_score: 50 }],
          patterns: [],
          feedback: [],
          knowledge: [],
          pojBlocks: [],
        })
      );

      await manager.initialize();

      const judgments = await manager.getRecentJudgments(10);
      assert.ok(judgments.some((j) => j.judgment_id === 'existing'));
    });
  });
});

describe('PersistenceManager health', () => {
  let manager;

  beforeEach(async () => {
    manager = new PersistenceManager({ forceMemory: true });
    await manager.initialize();
  });

  afterEach(async () => {
    await manager.close();
  });

  it('returns health status', async () => {
    const health = await manager.health();

    assert.ok(health.postgres);
    assert.ok(health.redis);
    assert.equal(health.postgres.status, 'not_configured');
    assert.equal(health.redis.status, 'not_configured');
  });
});

describe('PersistenceManager edge cases', () => {
  it('handles null persistence gracefully', async () => {
    const manager = new PersistenceManager({ forceMemory: true });
    // Don't initialize

    const result = await manager.storeJudgment({ q_score: 50 });
    assert.equal(result, null);

    const judgments = await manager.searchJudgments('test');
    assert.deepEqual(judgments, []);
  });
});

describe('PersistenceManager Triggers State', () => {
  let manager;

  beforeEach(async () => {
    manager = new PersistenceManager({ forceMemory: true });
    await manager.initialize();
  });

  afterEach(async () => {
    await manager.close();
  });

  describe('getTriggersState', () => {
    it('returns null when no state stored', async () => {
      const state = await manager.getTriggersState();
      assert.equal(state, null);
    });
  });

  describe('saveTriggersState', () => {
    it('stores and retrieves trigger state', async () => {
      const triggerState = {
        triggers: [
          {
            id: 'trig_test1',
            name: 'Test Trigger',
            type: 'event',
            action: 'log',
            condition: { eventType: 'PostJudgment' },
            enabled: true,
          },
        ],
        metadata: {
          savedAt: new Date().toISOString(),
          version: '1.0.0',
        },
      };

      const saved = await manager.saveTriggersState(triggerState);

      assert.ok(saved);
      assert.equal(saved.triggers.length, 1);
      assert.equal(saved.triggers[0].name, 'Test Trigger');

      // Verify retrieval
      const retrieved = await manager.getTriggersState();
      assert.deepEqual(retrieved, triggerState);
    });

    it('overwrites previous state', async () => {
      await manager.saveTriggersState({
        triggers: [{ id: 'old', name: 'Old Trigger' }],
      });

      await manager.saveTriggersState({
        triggers: [
          { id: 'new1', name: 'New Trigger 1' },
          { id: 'new2', name: 'New Trigger 2' },
        ],
      });

      const state = await manager.getTriggersState();
      assert.equal(state.triggers.length, 2);
      assert.equal(state.triggers[0].id, 'new1');
    });
  });

  describe('capabilities', () => {
    it('reports triggers capability', async () => {
      const caps = manager.capabilities;
      assert.equal(caps.triggers, true);
    });
  });
});

describe('PersistenceManager Triggers State (File)', () => {
  let manager;
  let tempDir;

  beforeEach(async () => {
    tempDir = path.join(os.tmpdir(), `cynic-triggers-test-${Date.now()}`);
    manager = new PersistenceManager({ dataDir: tempDir, skipDatabase: true });
    await manager.initialize();
  });

  afterEach(async () => {
    if (manager._initialized) {
      await manager.close();
    }
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch (e) {
      // Ignore cleanup errors
    }
  });

  it('persists triggers state to file', async () => {
    const triggerState = {
      triggers: [
        {
          id: 'trig_file1',
          name: 'File Test Trigger',
          type: 'periodic',
          action: 'judge',
        },
      ],
    };

    await manager.saveTriggersState(triggerState);

    // Force save
    await manager._fallback.save();

    const filePath = path.join(tempDir, 'cynic-state.json');
    const content = await fs.readFile(filePath, 'utf-8');
    const data = JSON.parse(content);

    assert.ok(data.triggersState);
    assert.equal(data.triggersState.triggers[0].name, 'File Test Trigger');
  });

  it('loads trigger state on initialize', async () => {
    // Create initial data with trigger state
    await fs.mkdir(tempDir, { recursive: true });
    const filePath = path.join(tempDir, 'cynic-state.json');
    await fs.writeFile(
      filePath,
      JSON.stringify({
        judgments: [],
        patterns: [],
        feedback: [],
        knowledge: [],
        pojBlocks: [],
        triggersState: {
          triggers: [
            { id: 'loaded_trig', name: 'Loaded Trigger', type: 'threshold' },
          ],
        },
      })
    );

    // Re-initialize to load existing data
    await manager.close();
    manager = new PersistenceManager({ dataDir: tempDir, skipDatabase: true });
    await manager.initialize();

    const state = await manager.getTriggersState();
    assert.ok(state);
    assert.equal(state.triggers[0].id, 'loaded_trig');
  });
});
