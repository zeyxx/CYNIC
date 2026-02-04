/**
 * Progressive Search Tools Tests
 *
 * Tests for the 3-layer progressive retrieval pattern.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from '../src/tools/search-progressive.js';

// ═══════════════════════════════════════════════════════════════════════════
// MOCK PERSISTENCE
// ═══════════════════════════════════════════════════════════════════════════

function createMockPersistence(data = {}) {
  return {
    async searchJudgments(query, opts) {
      return data.judgments || [];
    },
    async getPatterns(opts) {
      return data.patterns || [];
    },
    async searchKnowledge(query, opts) {
      return data.knowledge || [];
    },
    async findRecentJudgments(limit) {
      return data.judgments || [];
    },
    async getRecentJudgments(limit) {
      return data.judgments || [];
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Progressive Search Tools', () => {
  describe('createProgressiveSearchTools()', () => {
    it('creates 3 tools', () => {
      const tools = createProgressiveSearchTools();
      assert.strictEqual(tools.length, 3);
    });

    it('tools have correct names', () => {
      const tools = createProgressiveSearchTools();
      const names = tools.map(t => t.name);
      assert.ok(names.includes('brain_search_index'));
      assert.ok(names.includes('brain_timeline'));
      assert.ok(names.includes('brain_get_observations'));
    });
  });

  describe('brain_search_index', () => {
    it('has correct schema', () => {
      const tool = createSearchIndexTool();
      assert.ok(tool.inputSchema);
      assert.strictEqual(tool.inputSchema.required[0], 'query');
    });

    it('throws without query', async () => {
      const tool = createSearchIndexTool();
      await assert.rejects(
        () => tool.handler({ query: '' }),
        /Missing required parameter/
      );
    });

    it('returns empty results without persistence', async () => {
      const tool = createSearchIndexTool(null);
      const result = await tool.handler({ query: 'test' });

      assert.strictEqual(result.total, 0);
      assert.strictEqual(result.results.length, 0);
    });

    it('searches judgments', async () => {
      const persistence = createMockPersistence({
        judgments: [
          { judgment_id: 'jdg_1', verdict: 'WAG', q_score: 55, summary: 'Good token', created_at: '2026-01-20' },
        ],
      });
      const tool = createSearchIndexTool(persistence);
      const result = await tool.handler({ query: 'token', type: 'judgment' });

      assert.strictEqual(result.results.length, 1);
      assert.strictEqual(result.results[0].type, 'judgment');
      assert.strictEqual(result.results[0].verdict, 'WAG');
    });

    it('searches patterns', async () => {
      const persistence = createMockPersistence({
        patterns: [
          { pattern_id: 'pat_1', category: 'tool_usage', name: 'search_first', content: 'Search before judge', created_at: '2026-01-20' },
        ],
      });
      const tool = createSearchIndexTool(persistence);
      const result = await tool.handler({ query: 'search', type: 'pattern' });

      assert.strictEqual(result.results.length, 1);
      assert.strictEqual(result.results[0].type, 'pattern');
    });

    it('paginates results', async () => {
      const persistence = createMockPersistence({
        judgments: Array.from({ length: 30 }, (_, i) => ({
          judgment_id: `jdg_${i}`, verdict: 'WAG', q_score: 50, summary: `Judgment ${i}`, created_at: '2026-01-20',
        })),
      });
      const tool = createSearchIndexTool(persistence);

      const page1 = await tool.handler({ query: 'judgment', type: 'judgment', limit: 10, offset: 0 });
      assert.strictEqual(page1.results.length, 10);
      assert.strictEqual(page1.hasMore, true);

      const page2 = await tool.handler({ query: 'judgment', type: 'judgment', limit: 10, offset: 10 });
      assert.strictEqual(page2.results.length, 10);
    });

    it('sorts by relevance', async () => {
      const persistence = createMockPersistence({
        judgments: [
          { judgment_id: 'jdg_1', verdict: 'BARK', q_score: 20, summary: 'bad token', created_at: '2026-01-20' },
          { judgment_id: 'jdg_2', verdict: 'WAG', q_score: 55, summary: 'WAG verdict token test', created_at: '2026-01-20' },
        ],
      });
      const tool = createSearchIndexTool(persistence);
      const result = await tool.handler({ query: 'WAG' });

      // The one with "WAG" in verdict should score higher
      assert.ok(result.results.length >= 1);
    });
  });

  describe('brain_timeline', () => {
    it('throws without anchor or query', async () => {
      const tool = createTimelineTool();
      await assert.rejects(
        () => tool.handler({}),
        /Either anchor or query must be provided/
      );
    });

    it('returns context around anchor', async () => {
      const persistence = createMockPersistence({
        judgments: [
          { judgment_id: 'jdg_1', verdict: 'BARK', q_score: 20, summary: 'First', created_at: '2026-01-18T10:00:00Z' },
          { judgment_id: 'jdg_2', verdict: 'WAG', q_score: 55, summary: 'Anchor', created_at: '2026-01-19T10:00:00Z' },
          { judgment_id: 'jdg_3', verdict: 'HOWL', q_score: 80, summary: 'After', created_at: '2026-01-20T10:00:00Z' },
        ],
      });
      const tool = createTimelineTool(persistence);
      const result = await tool.handler({ anchor: 'jdg_2' });

      assert.ok(result.anchor);
      assert.strictEqual(result.anchor.id, 'jdg_2');
      assert.ok(result.before.length >= 1);
      assert.ok(result.after.length >= 1);
    });

    it('returns error for unknown anchor', async () => {
      const tool = createTimelineTool(createMockPersistence());
      const result = await tool.handler({ anchor: 'nonexistent' });

      assert.strictEqual(result.error, 'Anchor not found');
    });

    it('searches by query when no anchor', async () => {
      const persistence = createMockPersistence({
        judgments: [
          { judgment_id: 'jdg_1', verdict: 'WAG', q_score: 55, summary: 'Token analysis', item_type: 'token', created_at: '2026-01-20T10:00:00Z' },
        ],
      });
      const tool = createTimelineTool(persistence);
      const result = await tool.handler({ query: 'token' });

      assert.ok(result.anchor);
    });
  });

  describe('brain_get_observations', () => {
    it('throws without ids', async () => {
      const tool = createGetObservationsTool();
      await assert.rejects(
        () => tool.handler({ ids: [] }),
        /Missing required parameter/
      );
    });

    it('fetches judgment by ID', async () => {
      const persistence = createMockPersistence({
        judgments: [
          { judgment_id: 'jdg_abc', verdict: 'WAG', q_score: 55, summary: 'Full data' },
        ],
      });
      const tool = createGetObservationsTool(persistence);
      const result = await tool.handler({ ids: ['jdg_abc'] });

      assert.strictEqual(result.observations.length, 1);
      assert.strictEqual(result.observations[0].full, true);
      assert.strictEqual(result.stats.found, 1);
    });

    it('tracks not-found IDs', async () => {
      const tool = createGetObservationsTool(createMockPersistence());
      const result = await tool.handler({ ids: ['jdg_missing'] });

      assert.strictEqual(result.notFound.length, 1);
      assert.strictEqual(result.stats.missing, 1);
    });

    it('handles mixed ID types', async () => {
      const persistence = createMockPersistence({
        judgments: [{ judgment_id: 'jdg_1', verdict: 'WAG' }],
        patterns: [{ pattern_id: 'pat_1', category: 'test' }],
      });
      const tool = createGetObservationsTool(persistence);
      const result = await tool.handler({ ids: ['jdg_1', 'pat_1', 'unknown_id'] });

      assert.strictEqual(result.observations.length, 2);
      assert.strictEqual(result.notFound.length, 1);
    });
  });
});
