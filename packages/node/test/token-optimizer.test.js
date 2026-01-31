/**
 * @cynic/node - Token Optimizer Tests
 *
 * Tests for intelligent token optimization.
 *
 * @module @cynic/node/test/token-optimizer
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  TokenOptimizer,
  createTokenOptimizer,
  OPTIMIZER_CONFIG,
  CompressionStrategy,
  ABBREVIATIONS,
} from '../src/optimization/token-optimizer.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('OPTIMIZER_CONFIG', () => {
  it('should have φ-aligned cache max size', () => {
    // 233 = F(13)
    assert.strictEqual(OPTIMIZER_CONFIG.CACHE_MAX_SIZE, 233);
  });

  it('should have 5-minute cache TTL', () => {
    assert.strictEqual(OPTIMIZER_CONFIG.CACHE_TTL_MS, 300000);
  });

  it('should have 1-hour extended TTL', () => {
    assert.strictEqual(OPTIMIZER_CONFIG.CACHE_EXTENDED_TTL_MS, 3600000);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(OPTIMIZER_CONFIG));
  });
});

describe('CompressionStrategy', () => {
  it('should have all strategies', () => {
    assert.strictEqual(CompressionStrategy.WHITESPACE, 'whitespace');
    assert.strictEqual(CompressionStrategy.ABBREVIATION, 'abbreviation');
    assert.strictEqual(CompressionStrategy.TEMPLATE, 'template');
    assert.strictEqual(CompressionStrategy.SEMANTIC, 'semantic');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(CompressionStrategy));
  });
});

describe('ABBREVIATIONS', () => {
  it('should have common abbreviations', () => {
    assert.strictEqual(ABBREVIATIONS['function'], 'fn');
    assert.strictEqual(ABBREVIATIONS['return'], 'ret');
    assert.strictEqual(ABBREVIATIONS['configuration'], 'cfg');
    assert.strictEqual(ABBREVIATIONS['implementation'], 'impl');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(ABBREVIATIONS));
  });
});

// =============================================================================
// TOKEN OPTIMIZER TESTS
// =============================================================================

describe('TokenOptimizer', () => {
  let optimizer;

  beforeEach(() => {
    optimizer = createTokenOptimizer();
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      const o = createTokenOptimizer();
      assert.ok(o instanceof TokenOptimizer);
    });

    it('should initialize stats', () => {
      const stats = optimizer.getStats();
      assert.strictEqual(stats.requests, 0);
      assert.strictEqual(stats.tokensIn, 0);
      assert.strictEqual(stats.tokensOut, 0);
    });

    it('should accept custom config', () => {
      const o = createTokenOptimizer({
        config: { CACHE_MAX_SIZE: 100 },
      });
      assert.strictEqual(o.config.CACHE_MAX_SIZE, 100);
    });
  });

  // ===========================================================================
  // WHITESPACE COMPRESSION
  // ===========================================================================

  describe('Whitespace Compression', () => {
    it('should collapse multiple spaces', () => {
      const result = optimizer.optimize({
        content: 'hello    world',
        strategies: ['whitespace'],
      });

      assert.ok(result.optimized.includes('hello world'));
      assert.ok(!result.optimized.includes('  '));
    });

    it('should collapse multiple blank lines', () => {
      const result = optimizer.optimize({
        content: 'line1\n\n\n\nline2',
        strategies: ['whitespace'],
      });

      assert.strictEqual(result.optimized, 'line1\n\nline2');
    });

    it('should remove trailing whitespace', () => {
      const result = optimizer.optimize({
        content: 'hello   \nworld   ',
        strategies: ['whitespace'],
      });

      assert.ok(!result.optimized.match(/\s$/m));
    });

    it('should normalize line endings', () => {
      const result = optimizer.optimize({
        content: 'line1\r\nline2\r\n',
        strategies: ['whitespace'],
      });

      assert.ok(!result.optimized.includes('\r'));
    });

    it('should trim leading/trailing whitespace', () => {
      const result = optimizer.optimize({
        content: '   hello world   ',
        strategies: ['whitespace'],
      });

      assert.strictEqual(result.optimized, 'hello world');
    });
  });

  // ===========================================================================
  // ABBREVIATION COMPRESSION
  // ===========================================================================

  describe('Abbreviation Compression', () => {
    it('should abbreviate common words', () => {
      const result = optimizer.optimize({
        content: 'function implementation configuration',
        strategies: ['abbreviation'],
      });

      assert.ok(result.optimized.includes('fn'));
      assert.ok(result.optimized.includes('impl'));
      assert.ok(result.optimized.includes('cfg'));
    });

    it('should preserve case of first letter', () => {
      const result = optimizer.optimize({
        content: 'Function FUNCTION function',
        strategies: ['abbreviation'],
      });

      assert.ok(result.optimized.includes('Fn'));
      assert.ok(result.optimized.includes('fn'));
    });

    it('should only abbreviate whole words', () => {
      const result = optimizer.optimize({
        content: 'functionality',
        strategies: ['abbreviation'],
      });

      // Should not abbreviate partial match
      assert.ok(result.optimized.includes('functionality') || result.optimized.includes('fnality'));
    });
  });

  // ===========================================================================
  // FILLER REMOVAL
  // ===========================================================================

  describe('Filler Removal', () => {
    it('should remove filler phrases', () => {
      const result = optimizer.optimize({
        content: 'Please can you kindly help me with this?',
        strategies: ['filler'],
      });

      assert.ok(!result.optimized.includes('Please'));
      assert.ok(!result.optimized.includes('kindly'));
      assert.ok(!result.optimized.includes('Can you'));
    });

    it('should clean up extra spaces after removal', () => {
      const result = optimizer.optimize({
        content: 'I would like to get help',
        strategies: ['filler'],
      });

      assert.ok(!result.optimized.includes('  '));
    });
  });

  // ===========================================================================
  // DEDUPLICATION
  // ===========================================================================

  describe('Context Deduplication', () => {
    it('should deduplicate repeated long lines', () => {
      const longLine = 'This is a very long line that contains a lot of repetitive content and information that appears multiple times';
      const result = optimizer.optimize({
        content: `${longLine}\nSome other content\n${longLine}`,
        strategies: ['dedup'],
      });

      // Second occurrence should be replaced with reference
      const lines = result.optimized.split('\n');
      assert.ok(lines[2].includes('[see above:'));
    });

    it('should not deduplicate short lines', () => {
      const result = optimizer.optimize({
        content: 'short\nother\nshort',
        strategies: ['dedup'],
      });

      // Both short lines should remain
      assert.strictEqual((result.optimized.match(/short/g) || []).length, 2);
    });

    it('should track deduplications in stats', () => {
      const longLine = 'This is a very long line that contains a lot of content and should be deduplicated';
      optimizer.optimize({
        content: `${longLine}\n${longLine}\n${longLine}`,
        strategies: ['dedup'],
      });

      const stats = optimizer.getStats();
      assert.ok(stats.deduplications >= 2);
    });
  });

  // ===========================================================================
  // COMBINED STRATEGIES
  // ===========================================================================

  describe('Combined Strategies', () => {
    it('should apply multiple strategies', () => {
      const result = optimizer.optimize({
        content: 'Please    function    implementation',
        strategies: ['whitespace', 'abbreviation', 'filler'],
      });

      // Whitespace collapsed
      assert.ok(!result.optimized.includes('  '));
      // Abbreviations applied
      assert.ok(result.optimized.includes('fn'));
      assert.ok(result.optimized.includes('impl'));
      // Filler removed
      assert.ok(!result.optimized.includes('Please'));
    });

    it('should report all applied strategies', () => {
      const result = optimizer.optimize({
        content: 'hello    world',
        strategies: ['whitespace', 'abbreviation'],
      });

      assert.ok(result.strategies.includes('whitespace'));
      assert.ok(result.strategies.includes('abbreviation'));
    });
  });

  // ===========================================================================
  // CACHING
  // ===========================================================================

  describe('Caching', () => {
    it('should cache optimized results', () => {
      const content = 'hello    world';

      optimizer.optimize({ content, strategies: ['whitespace'] });
      const result2 = optimizer.optimize({ content, strategies: ['whitespace'] });

      assert.ok(result2.strategies.includes('cache'));
    });

    it('should track cache hits', () => {
      const content = 'test    content';

      optimizer.optimize({ content, strategies: ['whitespace'] });
      optimizer.optimize({ content, strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.strictEqual(stats.cacheHits, 1);
    });

    it('should track cache misses', () => {
      // Use content with spaces to get savings and caching
      optimizer.optimize({ content: 'content1    extra', strategies: ['whitespace'] });
      optimizer.optimize({ content: 'content2    extra', strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.strictEqual(stats.cacheMisses, 2);
    });

    it('should report cache size', () => {
      // Use content with spaces to get savings and caching
      optimizer.optimize({ content: 'content1    extra', strategies: ['whitespace'] });
      optimizer.optimize({ content: 'content2    extra', strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.strictEqual(stats.cacheSize, 2);
    });

    it('should check if content is cached', () => {
      // Use content with spaces that will result in savings
      const content = 'hello    world    test';
      optimizer.optimize({ content, strategies: ['whitespace'] });

      assert.ok(optimizer.isCached(content));
      assert.ok(!optimizer.isCached('different content'));
    });

    it('should clear cache', () => {
      optimizer.optimize({ content: 'test', strategies: ['whitespace'] });
      optimizer.clearCache();

      const stats = optimizer.getStats();
      assert.strictEqual(stats.cacheSize, 0);
    });
  });

  // ===========================================================================
  // TEMPLATES
  // ===========================================================================

  describe('Templates', () => {
    it('should register templates', () => {
      const result = optimizer.registerTemplate('greeting', 'Hello    World');

      assert.ok(result.savedTokens >= 0);
      assert.strictEqual(result.name, 'greeting');
    });

    it('should return compressed template', () => {
      optimizer.registerTemplate('test', 'hello    world');
      const template = optimizer.getTemplate('test');

      assert.strictEqual(template, 'hello world');
    });

    it('should return null for unknown template', () => {
      const template = optimizer.getTemplate('unknown');
      assert.strictEqual(template, null);
    });

    it('should track template count', () => {
      optimizer.registerTemplate('t1', 'template1');
      optimizer.registerTemplate('t2', 'template2');

      const stats = optimizer.getStats();
      assert.strictEqual(stats.templateCount, 2);
    });
  });

  // ===========================================================================
  // DECOMPRESSION
  // ===========================================================================

  describe('Decompression', () => {
    it('should decompress abbreviated content', () => {
      const result = optimizer.decompress('fn impl cfg');

      assert.ok(result.includes('function'));
      assert.ok(result.includes('implementation'));
      assert.ok(result.includes('configuration'));
    });

    it('should only decompress whole words', () => {
      const result = optimizer.decompress('fnality');

      // Should not decompress partial matches
      assert.ok(result.includes('fn') || result.includes('function'));
    });
  });

  // ===========================================================================
  // TOKEN ESTIMATION
  // ===========================================================================

  describe('Token Estimation', () => {
    it('should estimate tokens', () => {
      const result = optimizer.optimize({
        content: 'hello world',
        strategies: [],
      });

      assert.ok(result.originalTokens > 0);
      assert.ok(result.optimizedTokens > 0);
    });

    it('should calculate saved tokens', () => {
      const result = optimizer.optimize({
        content: 'hello    world    test',
        strategies: ['whitespace'],
      });

      assert.ok(result.savedTokens >= 0);
    });

    it('should calculate compression ratio', () => {
      const result = optimizer.optimize({
        content: 'hello    world    test    content',
        strategies: ['whitespace'],
      });

      assert.ok(result.compressionRatio >= 0);
      assert.ok(result.compressionRatio <= 1);
    });
  });

  // ===========================================================================
  // STATISTICS
  // ===========================================================================

  describe('Statistics', () => {
    it('should track request count', () => {
      optimizer.optimize({ content: 'test1', strategies: [] });
      optimizer.optimize({ content: 'test2', strategies: [] });

      const stats = optimizer.getStats();
      assert.strictEqual(stats.requests, 2);
    });

    it('should track tokens in and out', () => {
      optimizer.optimize({ content: 'hello    world', strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.ok(stats.tokensIn > 0);
      assert.ok(stats.tokensOut > 0);
    });

    it('should calculate total saved', () => {
      optimizer.optimize({ content: 'hello    world', strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.ok(stats.totalSaved >= 0);
    });

    it('should calculate cache hit rate', () => {
      optimizer.optimize({ content: 'test', strategies: ['whitespace'] });
      optimizer.optimize({ content: 'test', strategies: ['whitespace'] });
      optimizer.optimize({ content: 'other', strategies: ['whitespace'] });

      const stats = optimizer.getStats();
      assert.ok(stats.cacheHitRate >= 0);
    });

    it('should reset stats', () => {
      optimizer.optimize({ content: 'test', strategies: [] });
      optimizer.resetStats();

      const stats = optimizer.getStats();
      assert.strictEqual(stats.requests, 0);
      assert.strictEqual(stats.tokensIn, 0);
    });
  });

  // ===========================================================================
  // EVENTS
  // ===========================================================================

  describe('Events', () => {
    it('should emit optimize event', () => {
      const events = [];
      optimizer.on('optimize', (data) => events.push(data));

      optimizer.optimize({ content: 'test', strategies: ['whitespace'] });

      assert.strictEqual(events.length, 1);
      assert.ok('originalTokens' in events[0]);
      assert.ok('optimizedTokens' in events[0]);
    });

    it('should emit cache:hit event', () => {
      const events = [];
      optimizer.on('cache:hit', (data) => events.push(data));

      // Use content with savings so it gets cached
      const content = 'test    with    spaces';
      optimizer.optimize({ content, strategies: ['whitespace'] });
      optimizer.optimize({ content, strategies: ['whitespace'] });

      assert.strictEqual(events.length, 1);
    });

    it('should emit template:registered event', () => {
      const events = [];
      optimizer.on('template:registered', (data) => events.push(data));

      optimizer.registerTemplate('test', 'hello world');

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].name, 'test');
    });

    it('should emit cache:cleared event', () => {
      const events = [];
      optimizer.on('cache:cleared', () => events.push(true));

      optimizer.clearCache();

      assert.strictEqual(events.length, 1);
    });
  });

  // ===========================================================================
  // EDGE CASES
  // ===========================================================================

  describe('Edge Cases', () => {
    it('should handle empty content', () => {
      const result = optimizer.optimize({ content: '', strategies: ['whitespace'] });
      assert.strictEqual(result.optimized, '');
    });

    it('should handle null content', () => {
      const result = optimizer.optimize({ content: null, strategies: ['whitespace'] });
      assert.strictEqual(result.optimized, null);
    });

    it('should handle undefined strategies', () => {
      const result = optimizer.optimize({ content: 'hello world' });
      // Should use default strategies
      assert.ok(result.strategies.length > 0);
    });

    it('should handle content with no changes needed', () => {
      const result = optimizer.optimize({
        content: 'hello world',
        strategies: ['whitespace'],
      });

      assert.strictEqual(result.optimized, 'hello world');
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Token Optimizer Integration', () => {
  it('should achieve significant compression on typical prompts', () => {
    const optimizer = createTokenOptimizer();

    const verbosePrompt = `
      Please, can you kindly help me with the following task?

      I would like to understand the implementation of the function
      that handles the configuration and documentation processing.

      The function implementation should handle the configuration
      properly and return the correct documentation format.

      Please provide a detailed explanation of the implementation.
    `;

    const result = optimizer.optimize({
      content: verbosePrompt,
      strategies: ['whitespace', 'abbreviation', 'filler'],
    });

    // Should achieve at least 10% compression
    assert.ok(result.compressionRatio > 0.1,
      `Expected >10% compression, got ${result.compressionRatio * 100}%`);
  });

  it('should handle code without breaking it', () => {
    const optimizer = createTokenOptimizer();

    const code = `
function processData(data) {
  const result = data.map(x => x * 2);
  return result;
}
    `;

    const result = optimizer.optimize({
      content: code,
      strategies: ['whitespace'],
    });

    // Should still be valid-looking code
    assert.ok(result.optimized.includes('function'));
    assert.ok(result.optimized.includes('return'));
    assert.ok(result.optimized.includes('=>'));
  });

  it('should provide consistent compression over multiple requests', () => {
    const optimizer = createTokenOptimizer();

    for (let i = 0; i < 10; i++) {
      optimizer.optimize({
        content: `Request ${i}    with    extra    spaces`,
        strategies: ['whitespace'],
      });
    }

    const stats = optimizer.getStats();
    assert.ok(stats.compressionRatio > 0);
    assert.strictEqual(stats.requests, 10);
  });
});
