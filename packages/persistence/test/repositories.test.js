#!/usr/bin/env node
/**
 * PostgreSQL Repository Tests
 *
 * Tests for JudgmentRepository, PoJBlockRepository, and PatternRepository.
 * Uses mock database for unit tests, real PostgreSQL for integration tests.
 *
 * "φ distrusts φ" - verify all persistence
 *
 * @module @cynic/persistence/test/repositories
 */

import { describe, it, before, after, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import 'dotenv/config';
import { JudgmentRepository } from '../src/postgres/repositories/judgments.js';
import { PoJBlockRepository } from '../src/postgres/repositories/poj-blocks.js';
import { PatternRepository } from '../src/postgres/repositories/patterns.js';

/**
 * Create a mock database for unit testing
 */
function createMockDb() {
  const storage = {
    judgments: [],
    poj_blocks: [],
    patterns: [],
  };

  let idCounter = 1;

  return {
    storage,

    async query(sql, params = []) {
      // Normalize SQL for matching
      const sqlLower = sql.toLowerCase().trim();

      // INSERT INTO judgments
      if (sqlLower.includes('insert into judgments')) {
        const judgment = {
          id: idCounter++,
          judgment_id: params[0],
          user_id: params[1],
          session_id: params[2],
          item_type: params[3],
          item_content: params[4],
          item_hash: params[5],
          q_score: params[6],
          global_score: params[7],
          confidence: params[8],
          verdict: params[9],
          axiom_scores: params[10],
          dimension_scores: params[11],
          weaknesses: params[12],
          context: params[13],
          created_at: new Date(),
          block_hash: null,
          block_number: null,
        };
        storage.judgments.push(judgment);
        return { rows: [judgment] };
      }

      // SELECT * FROM judgments WHERE judgment_id = $1
      if (sqlLower.includes('from judgments') && sqlLower.includes('judgment_id = $1')) {
        const found = storage.judgments.find(j => j.judgment_id === params[0]);
        return { rows: found ? [found] : [] };
      }

      // SELECT * FROM judgments ORDER BY created_at DESC LIMIT
      if (sqlLower.includes('from judgments') && sqlLower.includes('order by created_at desc limit')) {
        const limit = params[params.length - 1] || 10;
        const sorted = [...storage.judgments].sort((a, b) =>
          new Date(b.created_at) - new Date(a.created_at)
        );
        return { rows: sorted.slice(0, limit) };
      }

      // SELECT COUNT(*) FROM judgments
      if (sqlLower.includes('select count(*)') && sqlLower.includes('from judgments')) {
        return { rows: [{ count: storage.judgments.length.toString() }] };
      }

      // Judgment stats
      if (sqlLower.includes('avg(q_score)') && sqlLower.includes('from judgments')) {
        const judgments = storage.judgments;
        const total = judgments.length;
        const avgScore = total > 0
          ? judgments.reduce((sum, j) => sum + (parseFloat(j.q_score) || 0), 0) / total
          : 0;
        const avgConfidence = total > 0
          ? judgments.reduce((sum, j) => sum + (parseFloat(j.confidence) || 0), 0) / total
          : 0;
        const verdictCounts = {
          HOWL: judgments.filter(j => j.verdict === 'HOWL').length,
          WAG: judgments.filter(j => j.verdict === 'WAG').length,
          GROWL: judgments.filter(j => j.verdict === 'GROWL').length,
          BARK: judgments.filter(j => j.verdict === 'BARK').length,
        };
        return {
          rows: [{
            total: total.toString(),
            avg_score: avgScore.toString(),
            avg_confidence: avgConfidence.toString(),
            howl_count: verdictCounts.HOWL.toString(),
            wag_count: verdictCounts.WAG.toString(),
            growl_count: verdictCounts.GROWL.toString(),
            bark_count: verdictCounts.BARK.toString(),
          }],
        };
      }

      // INSERT INTO poj_blocks
      if (sqlLower.includes('insert into poj_blocks')) {
        const block = {
          id: idCounter++,
          block_number: params[0],
          block_hash: params[1],
          prev_hash: params[2],
          merkle_root: params[3],
          judgment_count: params[4],
          judgment_ids: params[5],
          timestamp: params[6],
          created_at: new Date(),
        };
        // Check for conflict
        const existing = storage.poj_blocks.find(b => b.block_number === params[0]);
        if (existing) {
          return { rows: [] };
        }
        storage.poj_blocks.push(block);
        return { rows: [block] };
      }

      // SELECT * FROM poj_blocks ORDER BY block_number DESC LIMIT 1
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('order by block_number desc') && sqlLower.includes('limit 1')) {
        const sorted = [...storage.poj_blocks].sort((a, b) => b.block_number - a.block_number);
        return { rows: sorted.slice(0, 1) };
      }

      // SELECT * FROM poj_blocks WHERE block_number = $1
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('block_number = $1')) {
        const found = storage.poj_blocks.find(b => b.block_number === params[0]);
        return { rows: found ? [found] : [] };
      }

      // SELECT * FROM poj_blocks WHERE block_hash = $1
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('block_hash = $1')) {
        const found = storage.poj_blocks.find(b => b.block_hash === params[0]);
        return { rows: found ? [found] : [] };
      }

      // SELECT * FROM poj_blocks WHERE block_number > $1 ORDER BY block_number ASC
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('block_number > $1') && sqlLower.includes('order by block_number asc')) {
        const filtered = storage.poj_blocks.filter(b => b.block_number > params[0]);
        const sorted = filtered.sort((a, b) => a.block_number - b.block_number);
        const limit = params[1] || 100;
        return { rows: sorted.slice(0, limit) };
      }

      // SELECT * FROM poj_blocks ORDER BY block_number DESC LIMIT (findRecent)
      // Note: Handle both "desc limit" and "desc\n      limit" patterns
      if (sqlLower.includes('from poj_blocks') &&
          sqlLower.includes('order by block_number desc') &&
          sqlLower.includes('limit $1') &&
          !sqlLower.includes('limit 1')) {
        const sorted = [...storage.poj_blocks].sort((a, b) => b.block_number - a.block_number);
        const limit = params[0] || 10;
        return { rows: sorted.slice(0, limit) };
      }

      // poj_blocks stats
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('sum(judgment_count)')) {
        const blocks = storage.poj_blocks;
        const totalBlocks = blocks.length;
        const headSlot = blocks.length > 0 ? Math.max(...blocks.map(b => b.block_number)) : 0;
        const genesisSlot = blocks.length > 0 ? Math.min(...blocks.map(b => b.block_number)) : 0;
        const totalJudgments = blocks.reduce((sum, b) => sum + (b.judgment_count || 0), 0);
        return {
          rows: [{
            total_blocks: totalBlocks.toString(),
            head_slot: headSlot.toString(),
            genesis_slot: genesisSlot.toString(),
            total_judgments: totalJudgments.toString(),
            avg_judgments_per_block: (totalJudgments / (totalBlocks || 1)).toString(),
            chain_start: blocks[0]?.timestamp || null,
            last_block_time: blocks[blocks.length - 1]?.timestamp || null,
          }],
        };
      }

      // poj_blocks integrity check
      if (sqlLower.includes('from poj_blocks') && sqlLower.includes('block_number >= $1') && sqlLower.includes('block_hash, prev_hash')) {
        const filtered = storage.poj_blocks.filter(b => b.block_number >= params[0]);
        const sorted = filtered.sort((a, b) => a.block_number - b.block_number);
        const limit = params[1] || 100;
        return { rows: sorted.slice(0, limit) };
      }

      // SELECT COUNT(*) FROM poj_blocks
      if (sqlLower.includes('select count(*)') && sqlLower.includes('from poj_blocks')) {
        return { rows: [{ count: storage.poj_blocks.length.toString() }] };
      }

      // UPDATE judgments SET block_hash
      if (sqlLower.includes('update judgments') && sqlLower.includes('block_hash')) {
        const judgmentIds = params[3] || [];
        let updated = 0;
        for (const j of storage.judgments) {
          if (judgmentIds.includes(j.judgment_id) && !j.block_hash) {
            j.block_hash = params[0];
            j.block_number = params[1];
            j.prev_hash = params[2];
            updated++;
          }
        }
        return { rowCount: updated };
      }

      // INSERT INTO patterns
      if (sqlLower.includes('insert into patterns')) {
        const existing = storage.patterns.find(p => p.pattern_id === params[0]);
        if (existing) {
          // Update existing
          existing.confidence = params[4];
          existing.frequency = (existing.frequency || 1) + 1;
          existing.source_count = (existing.source_count || 1) + 1;
          existing.updated_at = new Date();
          return { rows: [existing] };
        }
        const pattern = {
          id: idCounter++,
          pattern_id: params[0],
          category: params[1],
          name: params[2],
          description: params[3],
          confidence: params[4],
          frequency: params[5],
          source_judgments: params[6],
          source_count: params[7],
          tags: params[8],
          data: params[9],
          created_at: new Date(),
          updated_at: new Date(),
        };
        storage.patterns.push(pattern);
        return { rows: [pattern] };
      }

      // SELECT * FROM patterns WHERE pattern_id = $1
      if (sqlLower.includes('from patterns') && sqlLower.includes('pattern_id = $1')) {
        const found = storage.patterns.find(p => p.pattern_id === params[0]);
        return { rows: found ? [found] : [] };
      }

      // SELECT * FROM patterns WHERE category = $1
      if (sqlLower.includes('from patterns') && sqlLower.includes('category = $1')) {
        const filtered = storage.patterns.filter(p => p.category === params[0]);
        const sorted = filtered.sort((a, b) => (b.confidence || 0) - (a.confidence || 0));
        const limit = params[1] || 10;
        return { rows: sorted.slice(0, limit) };
      }

      // SELECT * FROM patterns ORDER BY frequency DESC
      if (sqlLower.includes('from patterns') && sqlLower.includes('order by frequency desc')) {
        const sorted = [...storage.patterns].sort((a, b) => (b.frequency || 0) - (a.frequency || 0));
        const limit = params[0] || 10;
        return { rows: sorted.slice(0, limit) };
      }

      // Pattern stats
      if (sqlLower.includes('from patterns') && sqlLower.includes('avg(confidence)')) {
        const patterns = storage.patterns;
        const total = patterns.length;
        const avgConfidence = total > 0
          ? patterns.reduce((sum, p) => sum + (parseFloat(p.confidence) || 0), 0) / total
          : 0;
        const totalFrequency = patterns.reduce((sum, p) => sum + (p.frequency || 0), 0);
        const categories = new Set(patterns.map(p => p.category));
        return {
          rows: [{
            total: total.toString(),
            avg_confidence: avgConfidence.toString(),
            total_frequency: totalFrequency.toString(),
            category_count: categories.size.toString(),
          }],
        };
      }

      // Default fallback
      console.warn('Mock DB: Unhandled query:', sql.slice(0, 100));
      return { rows: [] };
    },
  };
}

// ============================================================================
// JUDGMENT REPOSITORY TESTS
// ============================================================================

describe('JudgmentRepository', () => {
  let repo;
  let mockDb;

  beforeEach(() => {
    mockDb = createMockDb();
    repo = new JudgmentRepository(mockDb);
  });

  describe('create', () => {
    it('creates a new judgment', async () => {
      const judgment = await repo.create({
        item: { type: 'code', content: 'function test() {}' },
        qScore: 75,
        confidence: 0.6,
        verdict: 'WAG',
        axiomScores: { PHI: 80, VERIFY: 70 },
      });

      assert.ok(judgment.judgment_id);
      assert.ok(judgment.judgment_id.startsWith('jdg_'));
      assert.equal(judgment.q_score, 75);
      assert.equal(judgment.verdict, 'WAG');
    });

    it('generates unique IDs', async () => {
      const j1 = await repo.create({ itemContent: 'content1', qScore: 50, verdict: 'GROWL', confidence: 0.5 });
      const j2 = await repo.create({ itemContent: 'content2', qScore: 60, verdict: 'WAG', confidence: 0.6 });

      assert.notEqual(j1.judgment_id, j2.judgment_id);
    });

    it('stores userId and sessionId', async () => {
      const judgment = await repo.create({
        userId: 'user123',
        sessionId: 'session456',
        itemContent: 'test content',
        qScore: 70,
        verdict: 'WAG',
        confidence: 0.55,
      });

      assert.equal(judgment.user_id, 'user123');
      assert.equal(judgment.session_id, 'session456');
    });

    it('stores item content for search', async () => {
      const judgment = await repo.create({
        item: {
          type: 'document',
          content: 'Important document content',
          description: 'A test document',
        },
        qScore: 80,
        verdict: 'WAG',
        confidence: 0.6,
      });

      assert.ok(judgment.item_content.includes('Important document content'));
      assert.ok(judgment.item_content.includes('A test document'));
    });
  });

  describe('findById', () => {
    it('finds existing judgment', async () => {
      const created = await repo.create({
        itemContent: 'test judgment',
        qScore: 65,
        verdict: 'GROWL',
        confidence: 0.5,
      });

      const found = await repo.findById(created.judgment_id);

      assert.ok(found);
      assert.equal(found.judgment_id, created.judgment_id);
    });

    it('returns null for non-existent judgment', async () => {
      const found = await repo.findById('jdg_nonexistent');
      assert.equal(found, null);
    });
  });

  describe('findRecent', () => {
    it('returns recent judgments', async () => {
      await repo.create({ itemContent: 'content1', qScore: 50, verdict: 'BARK', confidence: 0.4 });
      await repo.create({ itemContent: 'content2', qScore: 60, verdict: 'GROWL', confidence: 0.5 });
      await repo.create({ itemContent: 'content3', qScore: 70, verdict: 'WAG', confidence: 0.6 });

      const recent = await repo.findRecent(2);

      assert.equal(recent.length, 2);
    });

    it('respects limit parameter', async () => {
      for (let i = 0; i < 10; i++) {
        await repo.create({ itemContent: `content${i}`, qScore: i * 10, verdict: 'WAG', confidence: 0.5 });
      }

      const recent = await repo.findRecent(5);

      assert.equal(recent.length, 5);
    });
  });

  describe('getStats', () => {
    it('calculates statistics', async () => {
      await repo.create({ itemContent: 'c1', qScore: 80, verdict: 'WAG', confidence: 0.6 });
      await repo.create({ itemContent: 'c2', qScore: 60, verdict: 'GROWL', confidence: 0.5 });
      await repo.create({ itemContent: 'c3', qScore: 40, verdict: 'BARK', confidence: 0.4 });
      await repo.create({ itemContent: 'c4', qScore: 20, verdict: 'HOWL', confidence: 0.3 });

      const stats = await repo.getStats();

      assert.equal(stats.total, 4);
      assert.equal(stats.avgScore, 50);
      assert.equal(stats.verdicts.WAG, 1);
      assert.equal(stats.verdicts.GROWL, 1);
      assert.equal(stats.verdicts.BARK, 1);
      assert.equal(stats.verdicts.HOWL, 1);
    });

    it('handles empty repository', async () => {
      const stats = await repo.getStats();

      assert.equal(stats.total, 0);
      assert.equal(stats.avgScore, 0);
    });
  });

  describe('count', () => {
    it('counts judgments', async () => {
      assert.equal(await repo.count(), 0);

      await repo.create({ itemContent: 'c1', qScore: 50, verdict: 'WAG', confidence: 0.5 });
      await repo.create({ itemContent: 'c2', qScore: 60, verdict: 'WAG', confidence: 0.5 });

      assert.equal(await repo.count(), 2);
    });
  });
});

// ============================================================================
// POJ BLOCK REPOSITORY TESTS
// ============================================================================

describe('PoJBlockRepository', () => {
  let repo;
  let mockDb;

  beforeEach(() => {
    mockDb = createMockDb();
    repo = new PoJBlockRepository(mockDb);
  });

  describe('create', () => {
    it('creates a new block', async () => {
      const block = await repo.create({
        slot: 1,
        hash: 'abc123',
        prev_hash: '000000',
        judgments_root: 'merkle123',
        judgments: [{ judgment_id: 'jdg_1' }, { judgment_id: 'jdg_2' }],
        timestamp: Date.now(),
      });

      assert.ok(block);
      assert.equal(block.block_number, 1);
      assert.equal(block.block_hash, 'abc123');
      assert.equal(block.judgment_count, 2);
    });

    it('extracts judgment IDs', async () => {
      const block = await repo.create({
        slot: 1,
        hash: 'abc',
        prev_hash: '000',
        judgments: [
          { judgment_id: 'jdg_a' },
          { judgment_id: 'jdg_b' },
          { judgment_id: 'jdg_c' },
        ],
        timestamp: Date.now(),
      });

      assert.deepEqual(block.judgment_ids, ['jdg_a', 'jdg_b', 'jdg_c']);
    });

    it('handles conflict gracefully', async () => {
      await repo.create({
        slot: 1,
        hash: 'first',
        prev_hash: '000',
        judgments: [],
        timestamp: Date.now(),
      });

      const duplicate = await repo.create({
        slot: 1,
        hash: 'second',
        prev_hash: '000',
        judgments: [],
        timestamp: Date.now(),
      });

      // Should return null due to ON CONFLICT DO NOTHING
      assert.equal(duplicate, null);
    });
  });

  describe('getHead', () => {
    it('returns null when empty', async () => {
      const head = await repo.getHead();
      assert.equal(head, null);
    });

    it('returns latest block', async () => {
      await repo.create({ slot: 1, hash: 'h1', prev_hash: '000', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 2, hash: 'h2', prev_hash: 'h1', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 3, hash: 'h3', prev_hash: 'h2', judgments: [], timestamp: Date.now() });

      const head = await repo.getHead();

      assert.equal(head.slot, 3);
      assert.equal(head.hash, 'h3');
    });
  });

  describe('findByNumber', () => {
    it('finds block by slot number', async () => {
      await repo.create({ slot: 5, hash: 'block5', prev_hash: 'block4', judgments: [], timestamp: Date.now() });

      const found = await repo.findByNumber(5);

      assert.ok(found);
      assert.equal(found.slot, 5);
      assert.equal(found.hash, 'block5');
    });

    it('returns null for non-existent block', async () => {
      const found = await repo.findByNumber(999);
      assert.equal(found, null);
    });
  });

  describe('findByHash', () => {
    it('finds block by hash', async () => {
      await repo.create({ slot: 1, hash: 'unique_hash', prev_hash: '000', judgments: [], timestamp: Date.now() });

      const found = await repo.findByHash('unique_hash');

      assert.ok(found);
      assert.equal(found.hash, 'unique_hash');
    });
  });

  describe('findSince', () => {
    it('returns blocks after specified number', async () => {
      for (let i = 1; i <= 5; i++) {
        await repo.create({ slot: i, hash: `h${i}`, prev_hash: `h${i - 1}`, judgments: [], timestamp: Date.now() });
      }

      const blocks = await repo.findSince(2, 10);

      assert.equal(blocks.length, 3); // blocks 3, 4, 5
      assert.equal(blocks[0].slot, 3);
    });
  });

  describe('findRecent', () => {
    it('returns recent blocks', async () => {
      for (let i = 1; i <= 10; i++) {
        await repo.create({ slot: i, hash: `h${i}`, prev_hash: `h${i - 1}`, judgments: [], timestamp: Date.now() });
      }

      const recent = await repo.findRecent(5);

      assert.equal(recent.length, 5);
      assert.equal(recent[0].slot, 10); // Most recent first
    });
  });

  describe('getStats', () => {
    it('returns chain statistics', async () => {
      await repo.create({ slot: 0, hash: 'genesis', prev_hash: '000', judgments: [{ judgment_id: 'j1' }], timestamp: Date.now() });
      await repo.create({ slot: 1, hash: 'h1', prev_hash: 'genesis', judgments: [{ judgment_id: 'j2' }, { judgment_id: 'j3' }], timestamp: Date.now() });

      const stats = await repo.getStats();

      assert.equal(stats.totalBlocks, 2);
      assert.equal(stats.headSlot, 1);
      assert.equal(stats.genesisSlot, 0);
      assert.equal(stats.totalJudgments, 3);
    });
  });

  describe('verifyIntegrity', () => {
    it('validates chain links', async () => {
      await repo.create({ slot: 0, hash: 'h0', prev_hash: '000', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 1, hash: 'h1', prev_hash: 'h0', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 2, hash: 'h2', prev_hash: 'h1', judgments: [], timestamp: Date.now() });

      const result = await repo.verifyIntegrity();

      assert.equal(result.valid, true);
      assert.equal(result.blocksChecked, 3);
      assert.equal(result.errors.length, 0);
    });

    it('detects broken chain links', async () => {
      await repo.create({ slot: 0, hash: 'h0', prev_hash: '000', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 1, hash: 'h1', prev_hash: 'WRONG', judgments: [], timestamp: Date.now() });

      const result = await repo.verifyIntegrity();

      assert.equal(result.valid, false);
      assert.equal(result.errors.length, 1);
      assert.equal(result.errors[0].blockNumber, 1);
    });
  });

  describe('count', () => {
    it('counts blocks', async () => {
      assert.equal(await repo.count(), 0);

      await repo.create({ slot: 0, hash: 'h0', prev_hash: '000', judgments: [], timestamp: Date.now() });
      await repo.create({ slot: 1, hash: 'h1', prev_hash: 'h0', judgments: [], timestamp: Date.now() });

      assert.equal(await repo.count(), 2);
    });
  });

  describe('_toBlock', () => {
    it('converts database row to block format', async () => {
      const created = await repo.create({
        slot: 42,
        hash: 'blockhash',
        prev_hash: 'prevhash',
        judgments_root: 'merkleroot',
        judgments: [{ judgment_id: 'j1' }],
        timestamp: Date.now(),
      });

      const found = await repo.findByNumber(42);

      assert.equal(found.slot, 42);
      assert.equal(found.block_number, 42);
      assert.equal(found.hash, 'blockhash');
      assert.equal(found.block_hash, 'blockhash');
      assert.equal(found.prev_hash, 'prevhash');
    });
  });
});

// ============================================================================
// PATTERN REPOSITORY TESTS
// ============================================================================

describe('PatternRepository', () => {
  let repo;
  let mockDb;

  beforeEach(() => {
    mockDb = createMockDb();
    repo = new PatternRepository(mockDb);
  });

  describe('upsert', () => {
    it('creates new pattern', async () => {
      const pattern = await repo.upsert({
        category: 'code',
        name: 'Test Pattern',
        description: 'A test pattern',
        confidence: 0.8,
        tags: ['test', 'example'],
      });

      assert.ok(pattern.pattern_id);
      assert.ok(pattern.pattern_id.startsWith('pat_'));
      assert.equal(pattern.category, 'code');
      assert.equal(pattern.name, 'Test Pattern');
    });

    it('updates existing pattern', async () => {
      const created = await repo.upsert({
        category: 'security',
        name: 'Auth Pattern',
        confidence: 0.6,
      });

      const updated = await repo.upsert({
        patternId: created.pattern_id,
        category: 'security',
        name: 'Auth Pattern',
        confidence: 0.8,
      });

      assert.equal(updated.pattern_id, created.pattern_id);
      assert.equal(updated.confidence, 0.8);
      assert.ok(updated.frequency >= 2);
    });
  });

  describe('findById', () => {
    it('finds pattern by ID', async () => {
      const created = await repo.upsert({
        category: 'test',
        name: 'Find Me',
        confidence: 0.5,
      });

      const found = await repo.findById(created.pattern_id);

      assert.ok(found);
      assert.equal(found.name, 'Find Me');
    });

    it('returns null for non-existent pattern', async () => {
      const found = await repo.findById('pat_nonexistent');
      assert.equal(found, null);
    });
  });

  describe('findByCategory', () => {
    it('finds patterns in category', async () => {
      await repo.upsert({ category: 'code', name: 'Code 1', confidence: 0.8 });
      await repo.upsert({ category: 'code', name: 'Code 2', confidence: 0.7 });
      await repo.upsert({ category: 'docs', name: 'Doc 1', confidence: 0.6 });

      const codePatterns = await repo.findByCategory('code');

      assert.equal(codePatterns.length, 2);
      assert.ok(codePatterns.every(p => p.category === 'code'));
    });

    it('orders by confidence', async () => {
      await repo.upsert({ category: 'test', name: 'Low', confidence: 0.3 });
      await repo.upsert({ category: 'test', name: 'High', confidence: 0.9 });
      await repo.upsert({ category: 'test', name: 'Mid', confidence: 0.6 });

      const patterns = await repo.findByCategory('test');

      assert.ok(patterns[0].confidence >= patterns[1].confidence);
    });
  });

  describe('getTopPatterns', () => {
    it('returns patterns ordered by frequency', async () => {
      await repo.upsert({ category: 'a', name: 'Low Freq', confidence: 0.5, frequency: 2 });
      await repo.upsert({ category: 'b', name: 'High Freq', confidence: 0.5, frequency: 10 });

      const top = await repo.getTopPatterns(5);

      assert.ok(top.length >= 1);
    });
  });

  describe('getStats', () => {
    it('returns pattern statistics', async () => {
      await repo.upsert({ category: 'a', name: 'P1', confidence: 0.8, frequency: 5 });
      await repo.upsert({ category: 'b', name: 'P2', confidence: 0.6, frequency: 3 });

      const stats = await repo.getStats();

      assert.equal(stats.total, 2);
      assert.equal(stats.avgConfidence, 0.7);
      assert.equal(stats.totalFrequency, 8);
      assert.equal(stats.categoryCount, 2);
    });

    it('handles empty repository', async () => {
      const stats = await repo.getStats();

      assert.equal(stats.total, 0);
      assert.equal(stats.avgConfidence, 0);
    });
  });
});

// ============================================================================
// INTEGRATION TESTS (with real PostgreSQL)
// ============================================================================

describe('PostgreSQL Integration', () => {
  const hasPostgres = !!process.env.CYNIC_DATABASE_URL;

  describe('JudgmentRepository Integration', {
    skip: !hasPostgres,
  }, () => {
    it('creates and retrieves judgment', async () => {
      const repo = new JudgmentRepository();

      const judgment = await repo.create({
        item: { type: 'test', content: 'integration test' },
        qScore: 75,
        confidence: 0.6,
        verdict: 'WAG',
      });

      assert.ok(judgment.judgment_id);

      const found = await repo.findById(judgment.judgment_id);
      assert.equal(parseFloat(found.q_score), 75);
    });
  });

  describe('PoJBlockRepository Integration', {
    skip: !hasPostgres,
  }, () => {
    it('creates and retrieves block', async () => {
      const repo = new PoJBlockRepository();
      const timestamp = Date.now();
      const uniqueSlot = Math.floor(timestamp / 1000) + Math.floor(Math.random() * 10000);

      const block = await repo.create({
        slot: uniqueSlot,
        hash: `test_${timestamp}`,
        prev_hash: 'integration_test',
        judgments_root: 'merkle_test',
        judgments: [],
        timestamp,
      });

      // Block might be null if slot already exists
      if (block) {
        const found = await repo.findByNumber(uniqueSlot);
        assert.ok(found);
        assert.equal(found.block_hash, `test_${timestamp}`);
      }
    });
  });

  describe('PatternRepository Integration', {
    skip: !hasPostgres,
  }, () => {
    it('upserts and retrieves pattern', async () => {
      const repo = new PatternRepository();
      const timestamp = Date.now();

      const pattern = await repo.upsert({
        category: 'integration',
        name: `Test Pattern ${timestamp}`,
        confidence: 0.75,
      });

      assert.ok(pattern.pattern_id);

      const found = await repo.findById(pattern.pattern_id);
      assert.equal(found.name, `Test Pattern ${timestamp}`);
    });
  });
});
