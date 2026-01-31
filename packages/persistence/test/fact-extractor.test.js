/**
 * @cynic/persistence - Fact Extractor Tests (M2)
 *
 * Tests for auto fact extraction from tool outputs.
 *
 * @module @cynic/persistence/test/fact-extractor
 */

import { describe, it, mock, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  FactExtractor,
  createFactExtractor,
} from '../src/services/fact-extractor.js';
import { FactType } from '../src/postgres/repositories/facts.js';

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockPool() {
  return {
    query: mock.fn(async (sql) => {
      // Table creation
      if (sql.includes('CREATE TABLE')) {
        return { rows: [] };
      }
      // Trigger creation
      if (sql.includes('CREATE OR REPLACE FUNCTION')) {
        return { rows: [] };
      }
      if (sql.includes('DROP TRIGGER')) {
        return { rows: [] };
      }
      if (sql.includes('CREATE TRIGGER')) {
        return { rows: [] };
      }
      // Insert
      if (sql.includes('INSERT INTO facts')) {
        return {
          rows: [{
            fact_id: 'fact_mock123',
            subject: 'Test Subject',
            content: 'Test Content',
            confidence: 0.5,
            relevance: 0.5,
            tags: [],
            created_at: new Date(),
          }],
        };
      }
      // Stats
      if (sql.includes('SELECT') && sql.includes('COUNT')) {
        return {
          rows: [{
            total: '10',
            types: '3',
            tools: '5',
            avg_confidence: '0.5',
            avg_relevance: '0.5',
            total_accesses: '20',
          }],
        };
      }
      // Decay/Prune
      if (sql.includes('UPDATE') || sql.includes('DELETE')) {
        return { rowCount: 5 };
      }
      // Default
      return { rows: [] };
    }),
  };
}

// =============================================================================
// FACT TYPE TESTS
// =============================================================================

describe('FactType', () => {
  it('should have all expected types', () => {
    assert.strictEqual(FactType.CODE_PATTERN, 'code_pattern');
    assert.strictEqual(FactType.API_DISCOVERY, 'api_discovery');
    assert.strictEqual(FactType.ERROR_RESOLUTION, 'error_resolution');
    assert.strictEqual(FactType.FILE_STRUCTURE, 'file_structure');
    assert.strictEqual(FactType.USER_PREFERENCE, 'user_preference');
    assert.strictEqual(FactType.TOOL_RESULT, 'tool_result');
    assert.strictEqual(FactType.DECISION, 'decision');
    assert.strictEqual(FactType.LEARNING, 'learning');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(FactType));
  });
});

// =============================================================================
// FACT EXTRACTOR CONSTRUCTION
// =============================================================================

describe('FactExtractor', () => {
  describe('Construction', () => {
    it('should require a pool', () => {
      assert.throws(() => new FactExtractor(), /requires.*pool/i);
    });

    it('should create with pool', () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });
      assert.ok(extractor);
    });

    it('should accept optional vectorStore', () => {
      const pool = createMockPool();
      const vectorStore = { store: mock.fn(), search: mock.fn() };
      const extractor = new FactExtractor({ pool, vectorStore });
      assert.ok(extractor);
    });
  });

  // ===========================================================================
  // EXTRACTION TESTS
  // ===========================================================================

  describe('Extraction', () => {
    let extractor;
    let pool;

    beforeEach(() => {
      pool = createMockPool();
      extractor = new FactExtractor({ pool });
    });

    it('should extract facts from Read tool output', async () => {
      const toolResult = {
        tool: 'Read',
        input: { file_path: 'src/index.js' },
        output: `
          export class MyClass {}
          export function myFunction() {}
          export const MY_CONST = 1;
          import { foo } from '@cynic/core';
          import { bar } from '@cynic/protocol';
          import { baz } from '@cynic/persistence';
          import { qux } from '@cynic/node';
        `,
      };

      const facts = await extractor.extract(toolResult);

      // Extract returns facts (mock pool stores them)
      assert.ok(facts.length > 0);
      // Verify extraction was called (mock returns factId)
      assert.ok(facts[0].factId);
    });

    it('should extract facts from Glob tool output', async () => {
      const toolResult = {
        tool: 'Glob',
        input: { pattern: '**/*.test.js' },
        output: `
          src/test/a.test.js
          src/test/b.test.js
          lib/c.test.js
        `,
      };

      const facts = await extractor.extract(toolResult);

      assert.ok(facts.length > 0);
      assert.ok(facts[0].factId);
    });

    it('should extract facts from Grep tool output', async () => {
      const toolResult = {
        tool: 'Grep',
        input: { pattern: 'TODO' },
        output: `
          src/a.js:10:// TODO: fix this
          src/b.js:20:// TODO: refactor
          src/c.js:30:// TODO: optimize
        `,
      };

      const facts = await extractor.extract(toolResult);

      assert.ok(facts.length > 0);
      assert.ok(facts[0].factId);
    });

    it('should extract facts from Edit tool', async () => {
      const toolResult = {
        tool: 'Edit',
        input: {
          file_path: 'src/config.js',
          old_string: 'const value = 1;',
          new_string: 'const value = 2;',
        },
        output: 'OK',
      };

      const facts = await extractor.extract(toolResult);

      assert.ok(facts.length > 0);
      assert.ok(facts[0].factId);
    });

    it('should extract facts from Bash npm install', async () => {
      const toolResult = {
        tool: 'Bash',
        input: { command: 'npm install lodash' },
        output: 'added 1 package',
      };

      const facts = await extractor.extract(toolResult);

      assert.ok(facts.length > 0);
      assert.ok(facts[0].factId);
    });

    it('should extract facts from Bash test output', async () => {
      const toolResult = {
        tool: 'Bash',
        input: { command: 'npm test' },
        output: `
          ✓ test 1
          ✓ test 2
          10 pass
          2 fail
        `,
      };

      const facts = await extractor.extract(toolResult);

      assert.ok(facts.length > 0);
      assert.ok(facts[0].factId);
    });

    it('should return empty for unsupported tools', async () => {
      const toolResult = {
        tool: 'UnknownTool',
        input: {},
        output: 'something',
      };

      const facts = await extractor.extract(toolResult);
      assert.strictEqual(facts.length, 0);
    });

    it('should return empty for insufficient output', async () => {
      const toolResult = {
        tool: 'Read',
        input: { file_path: 'small.txt' },
        output: 'hi',
      };

      const facts = await extractor.extract(toolResult);
      assert.strictEqual(facts.length, 0);
    });
  });

  // ===========================================================================
  // ERROR RESOLUTION EXTRACTION
  // ===========================================================================

  describe('Error Resolution', () => {
    it('should extract error resolutions', async () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });

      const fact = await extractor.extractErrorResolution({
        error: 'ENOENT: no such file',
        solution: 'Created the directory first',
        tool: 'Write',
        file: 'src/new-file.js',
      });

      assert.ok(fact);
      assert.ok(fact.factId); // Mock returns factId
    });
  });

  // ===========================================================================
  // USER PREFERENCE EXTRACTION
  // ===========================================================================

  describe('User Preference', () => {
    it('should extract preferences seen multiple times', async () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });

      const fact = await extractor.extractUserPreference({
        type: 'test_framework',
        value: 'node:test',
        occurrences: 5,
      });

      assert.ok(fact);
      assert.ok(fact.factId); // Mock returns factId
    });

    it('should not extract rarely seen preferences', async () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });

      const fact = await extractor.extractUserPreference({
        type: 'test_framework',
        value: 'mocha',
        occurrences: 1,
      });

      assert.strictEqual(fact, null);
    });
  });

  // ===========================================================================
  // MAINTENANCE
  // ===========================================================================

  describe('Maintenance', () => {
    it('should run maintenance', async () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });

      const result = await extractor.runMaintenance();

      assert.ok('decayed' in result);
      assert.ok('pruned' in result);
    });
  });

  // ===========================================================================
  // STATISTICS
  // ===========================================================================

  describe('Statistics', () => {
    it('should get statistics', async () => {
      const pool = createMockPool();
      const extractor = new FactExtractor({ pool });

      const stats = await extractor.getStats();

      assert.ok('total' in stats);
      assert.ok('processing' in stats);
    });
  });
});

// =============================================================================
// FACTORY FUNCTION
// =============================================================================

describe('createFactExtractor', () => {
  it('should create instance', () => {
    const pool = createMockPool();
    const extractor = createFactExtractor({ pool });

    assert.ok(extractor instanceof FactExtractor);
  });
});
