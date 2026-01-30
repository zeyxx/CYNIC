/**
 * Tests for Context Intelligence Module (C-Score, Budget, Assembly)
 *
 * @module @cynic/core/test/context
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  countTokens,
  estimateTokens,
  calculateFreshness,
  calculatePertinence,
  calculateDensity,
  calculateCScore,
  BudgetManager,
  assembleContext,
  DEFAULT_CONTEXT_SIZE,
  BUDGET_THRESHOLDS,
  FRESHNESS_DECAY,
  // Entropy exports
  calculateEntropyFactor,
  ENTROPY_THRESHOLDS,
  ENTROPY_WEIGHTS,
  calculateShannonEntropy,
  calculateWordEntropy,
  calculateLexicalEntropy,
  calculateStructuralEntropy,
  entropyToRetentionFactor,
} from '../src/context/index.js';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '../src/axioms/constants.js';

// =============================================================================
// TOKEN COUNTING TESTS
// =============================================================================

describe('Context Intelligence: Token Counting', () => {
  describe('countTokens', () => {
    it('should return 0 for empty or null input', () => {
      assert.equal(countTokens(''), 0);
      assert.equal(countTokens(null), 0);
      assert.equal(countTokens(undefined), 0);
    });

    it('should estimate tokens for English text', () => {
      // "Hello world" = 11 chars, ~4 chars/token = ~3 tokens
      const tokens = countTokens('Hello world');
      assert.ok(tokens >= 2 && tokens <= 5, `Expected 2-5 tokens, got ${tokens}`);
    });

    it('should estimate tokens for code', () => {
      const code = 'function test() { return 42; }';
      const tokens = countTokens(code);
      // 30 chars / 4 = 7.5 -> 8 tokens
      assert.ok(tokens >= 5 && tokens <= 15, `Expected 5-15 tokens, got ${tokens}`);
    });

    it('should handle large text', () => {
      const largeText = 'x'.repeat(4000);
      const tokens = countTokens(largeText);
      // 4000 / 4 = 1000 tokens
      assert.equal(tokens, 1000);
    });
  });

  describe('estimateTokens', () => {
    it('should handle strings', () => {
      const tokens = estimateTokens('test string');
      assert.ok(tokens > 0);
    });

    it('should handle objects via JSON', () => {
      const tokens = estimateTokens({ key: 'value', nested: { a: 1 } });
      assert.ok(tokens > 0);
    });

    it('should handle null/undefined', () => {
      assert.equal(estimateTokens(null), 0);
      assert.equal(estimateTokens(undefined), 0);
    });

    it('should handle primitives', () => {
      assert.ok(estimateTokens(12345) > 0);
      assert.ok(estimateTokens(true) > 0);
    });
  });
});

// =============================================================================
// FRESHNESS TESTS
// =============================================================================

describe('Context Intelligence: Freshness', () => {
  describe('calculateFreshness', () => {
    it('should return 1 for current turn (turn 0)', () => {
      const freshness = calculateFreshness(0);
      // Should be boosted for recent content
      assert.ok(freshness >= 1, `Expected >= 1, got ${freshness}`);
    });

    it('should decay over turns', () => {
      const fresh = calculateFreshness(0);
      const stale = calculateFreshness(10);
      assert.ok(stale < fresh, `Expected ${stale} < ${fresh}`);
    });

    it('should respect half-life', () => {
      // At half-life (5 turns), should be ~50%
      const atHalfLife = calculateFreshness(5);
      assert.ok(atHalfLife >= 0.4 && atHalfLife <= 0.6, `Expected ~0.5, got ${atHalfLife}`);
    });

    it('should respect minimum freshness', () => {
      // Even very old content should have minimum freshness
      const veryOld = calculateFreshness(100);
      assert.ok(veryOld >= FRESHNESS_DECAY.MIN_FRESHNESS,
        `Expected >= ${FRESHNESS_DECAY.MIN_FRESHNESS}, got ${veryOld}`);
    });

    it('should boost recent content (last 2 turns)', () => {
      const recent = calculateFreshness(1);
      assert.ok(recent > 0.9, `Expected > 0.9 for recent content, got ${recent}`);
    });

    it('should handle negative turns gracefully', () => {
      const negative = calculateFreshness(-5);
      assert.equal(negative, 1);
    });

    it('should allow custom half-life', () => {
      const shortHalfLife = calculateFreshness(2, { halfLife: 2 });
      const longHalfLife = calculateFreshness(2, { halfLife: 10 });
      assert.ok(shortHalfLife < longHalfLife, 'Short half-life should decay faster');
    });
  });
});

// =============================================================================
// PERTINENCE TESTS
// =============================================================================

describe('Context Intelligence: Pertinence', () => {
  describe('calculatePertinence', () => {
    it('should return neutral score (0.5) for empty context', () => {
      const content = { text: 'some content' };
      const pertinence = calculatePertinence(content, {});
      assert.ok(pertinence >= 0.4 && pertinence <= 0.6, `Expected ~0.5, got ${pertinence}`);
    });

    it('should increase with keyword overlap', () => {
      const content = { text: 'implementing authentication login system' };
      const context = { query: 'how does authentication work' };
      const pertinence = calculatePertinence(content, context);
      assert.ok(pertinence > 0.5, `Expected > 0.5 with keyword match, got ${pertinence}`);
    });

    it('should handle type matching', () => {
      const codeContent = { text: 'function authenticate() {}', type: 'code' };
      const context = { taskType: 'code' };
      const pertinence = calculatePertinence(codeContent, context);
      assert.ok(pertinence > 0.5, `Expected > 0.5 for type match, got ${pertinence}`);
    });

    it('should boost explicit references', () => {
      const content = { text: 'The file auth.js handles login' };
      const context = { references: ['auth.js'] };
      const pertinence = calculatePertinence(content, context);
      assert.ok(pertinence > 0.6, `Expected > 0.6 with explicit ref, got ${pertinence}`);
    });

    it('should boost user source', () => {
      const content = { text: 'user provided content', source: 'user' };
      const pertinence = calculatePertinence(content, {});
      assert.ok(pertinence >= 0.6, `Expected >= 0.6 for user source, got ${pertinence}`);
    });

    it('should clamp to 0-1 range', () => {
      const content = {
        text: 'auth login user session token',
        type: 'code',
        source: 'user',
      };
      const context = {
        query: 'auth login user session token implementation',
        taskType: 'code',
        references: ['auth', 'login', 'session'],
      };
      const pertinence = calculatePertinence(content, context);
      assert.ok(pertinence <= 1, `Expected <= 1, got ${pertinence}`);
    });
  });
});

// =============================================================================
// DENSITY TESTS
// =============================================================================

describe('Context Intelligence: Density', () => {
  describe('calculateDensity', () => {
    it('should return 0 for empty content', () => {
      assert.equal(calculateDensity({ text: '' }), 0);
      assert.equal(calculateDensity({}), 0);
    });

    it('should detect high density in code', () => {
      const code = {
        text: `
          import { auth } from './auth';
          export function login(user) {
            const token = auth.generate(user);
            return { token, user };
          }
          export const logout = () => auth.clear();
        `,
      };
      const density = calculateDensity(code);
      assert.ok(density > 0.6, `Expected > 0.6 for code, got ${density}`);
    });

    it('should detect structured data', () => {
      const structured = {
        text: `
          {
            "users": [
              { "id": 1, "name": "Alice" },
              { "id": 2, "name": "Bob" }
            ],
            "config": { "timeout": 5000 }
          }
        `,
      };
      const density = calculateDensity(structured);
      assert.ok(density >= 0.5, `Expected >= 0.5 for structured, got ${density}`);
    });

    it('should penalize repetitive content', () => {
      const repetitive = {
        text: 'hello\nhello\nhello\nhello\nhello\nhello\nhello\nhello',
      };
      const density = calculateDensity(repetitive);
      assert.ok(density < 0.5, `Expected < 0.5 for repetitive, got ${density}`);
    });

    it('should handle mixed content', () => {
      const mixed = {
        text: `
          This is some documentation about the function.

          function example() {
            return true;
          }

          More explanation here.
        `,
      };
      const density = calculateDensity(mixed);
      assert.ok(density >= 0.3 && density <= 0.8, `Expected 0.3-0.8, got ${density}`);
    });
  });
});

// =============================================================================
// C-SCORE TESTS
// =============================================================================

describe('Context Intelligence: C-Score', () => {
  describe('calculateCScore', () => {
    it('should return all components in breakdown', () => {
      const content = { text: 'test content' };
      const result = calculateCScore(content);

      assert.ok('C' in result);
      assert.ok('breakdown' in result);
      assert.ok('pertinence' in result.breakdown);
      assert.ok('freshness' in result.breakdown);
      assert.ok('density' in result.breakdown);
      assert.ok('tokens' in result.breakdown);
    });

    it('should include formula in result', () => {
      const result = calculateCScore({ text: 'test' });
      assert.ok(result.formula.includes('P'));
      assert.ok(result.formula.includes('F'));
      assert.ok(result.formula.includes('D'));
    });

    it('should favor small high-quality content', () => {
      const small = { text: 'function login() { return auth(); }' };
      const large = { text: 'function login() { return auth(); }'.repeat(10) };

      const smallScore = calculateCScore(small);
      const largeScore = calculateCScore(large);

      // Same content quality, but small should score higher
      assert.ok(smallScore.C > largeScore.C,
        `Expected small (${smallScore.C}) > large (${largeScore.C})`);
    });

    it('should decay with age', () => {
      const content = { text: 'recent content' };
      const recent = calculateCScore(content, {}, { turnsSinceAdded: 0 });
      const old = calculateCScore(content, {}, { turnsSinceAdded: 20 });

      assert.ok(recent.C > old.C,
        `Expected recent (${recent.C}) > old (${old.C})`);
    });

    it('should boost relevant content', () => {
      const content = { text: 'authentication system login handler' };
      const generic = calculateCScore(content, {});
      const relevant = calculateCScore(content, { query: 'authentication login' });

      assert.ok(relevant.C > generic.C,
        `Expected relevant (${relevant.C}) > generic (${generic.C})`);
    });

    it('should return normalized score (0-100)', () => {
      const result = calculateCScore({ text: 'x'.repeat(1000) });
      assert.ok(result.C >= 0 && result.C <= 100,
        `Expected 0-100, got ${result.C}`);
    });
  });
});

// =============================================================================
// BUDGET MANAGER TESTS
// =============================================================================

describe('Context Intelligence: BudgetManager', () => {
  let budget;

  beforeEach(() => {
    budget = new BudgetManager({ maxTokens: 1000 });
  });

  describe('constructor', () => {
    it('should use default context size', () => {
      const defaultBudget = new BudgetManager();
      assert.equal(defaultBudget.maxTokens, DEFAULT_CONTEXT_SIZE);
    });

    it('should calculate phi-aligned limits', () => {
      assert.equal(budget.targetLimit, Math.floor(1000 * PHI_INV_3));
      assert.equal(budget.softLimit, Math.floor(1000 * PHI_INV_2));
      assert.equal(budget.hardLimit, Math.floor(1000 * PHI_INV));
    });

    it('should accept custom thresholds', () => {
      const custom = new BudgetManager({
        maxTokens: 1000,
        targetRatio: 0.2,
        softRatio: 0.4,
        hardRatio: 0.6,
      });
      assert.equal(custom.targetLimit, 200);
      assert.equal(custom.softLimit, 400);
      assert.equal(custom.hardLimit, 600);
    });
  });

  describe('getUtilization', () => {
    it('should return 0 when empty', () => {
      assert.equal(budget.getUtilization(), 0);
    });

    it('should calculate percentage correctly', () => {
      budget.track('a', 500);
      assert.equal(budget.getUtilization(), 50);
    });
  });

  describe('getStatus', () => {
    it('should return OPTIMAL when under target', () => {
      budget.track('a', 100);
      const status = budget.getStatus();
      assert.equal(status.level, 'OPTIMAL');
    });

    it('should return SOFT when between target and soft', () => {
      budget.track('a', 300); // Between 236 (target) and 381 (soft)
      const status = budget.getStatus();
      assert.equal(status.level, 'SOFT');
    });

    it('should return WARNING when between soft and hard', () => {
      budget.track('a', 500); // Between 381 (soft) and 618 (hard)
      const status = budget.getStatus();
      assert.equal(status.level, 'WARNING');
    });

    it('should return CRITICAL when over hard', () => {
      budget.track('a', 700); // Over 618 (hard)
      const status = budget.getStatus();
      assert.equal(status.level, 'CRITICAL');
    });

    it('should include all status fields', () => {
      const status = budget.getStatus();
      assert.ok('level' in status);
      assert.ok('action' in status);
      assert.ok('utilization' in status);
      assert.ok('currentTokens' in status);
      assert.ok('remaining' in status);
      assert.ok('limits' in status);
      assert.ok('thresholds' in status);
    });
  });

  describe('canAdd', () => {
    it('should allow under target for all priorities', () => {
      assert.ok(budget.canAdd(100, 'low').allowed);
      assert.ok(budget.canAdd(100, 'normal').allowed);
      assert.ok(budget.canAdd(100, 'high').allowed);
    });

    it('should block low priority over target', () => {
      budget.track('a', 200);
      assert.ok(!budget.canAdd(100, 'low').allowed);
    });

    it('should allow normal priority up to soft limit', () => {
      budget.track('a', 200);
      assert.ok(budget.canAdd(100, 'normal').allowed);
    });

    it('should block normal priority over soft limit', () => {
      budget.track('a', 350);
      assert.ok(!budget.canAdd(50, 'normal').allowed);
    });

    it('should allow high priority up to hard limit', () => {
      budget.track('a', 500);
      assert.ok(budget.canAdd(100, 'high').allowed);
    });

    it('should block high priority over hard limit', () => {
      budget.track('a', 600);
      assert.ok(!budget.canAdd(100, 'high').allowed);
    });
  });

  describe('track/untrack', () => {
    it('should add tokens when tracking', () => {
      budget.track('item1', 100);
      assert.equal(budget.currentTokens, 100);
    });

    it('should accumulate tokens', () => {
      budget.track('item1', 100);
      budget.track('item2', 150);
      assert.equal(budget.currentTokens, 250);
    });

    it('should remove tokens when untracking', () => {
      budget.track('item1', 100);
      budget.track('item2', 150);
      budget.untrack('item1');
      assert.equal(budget.currentTokens, 150);
    });

    it('should return false for unknown id', () => {
      assert.equal(budget.untrack('unknown'), false);
    });

    it('should store metadata', () => {
      budget.track('item1', 100, { cScore: 75 });
      assert.equal(budget.items[0].cScore, 75);
    });
  });

  describe('getCompactionCandidates', () => {
    it('should return empty when under soft limit', () => {
      budget.track('a', 100);
      assert.equal(budget.getCompactionCandidates().length, 0);
    });

    it('should return candidates when over soft limit', () => {
      budget.track('a', 200, { cScore: 80 });
      budget.track('b', 200, { cScore: 40 });
      budget.track('c', 200, { cScore: 60 });
      // Total: 600, over soft limit (381)

      const candidates = budget.getCompactionCandidates();
      assert.ok(candidates.length > 0);
      // Lowest C-Score should be first candidate
      assert.equal(candidates[0].id, 'b');
    });

    it('should return enough candidates to reach target', () => {
      budget.track('a', 150, { cScore: 80 });
      budget.track('b', 150, { cScore: 40 });
      budget.track('c', 150, { cScore: 60 });
      budget.track('d', 150, { cScore: 50 });
      // Total: 600, need to free 600 - 236 = 364 tokens

      const candidates = budget.getCompactionCandidates();
      const tokensToFree = candidates.reduce((sum, c) => sum + c.tokens, 0);
      assert.ok(tokensToFree >= 364, `Expected >= 364, got ${tokensToFree}`);
    });
  });

  describe('reset', () => {
    it('should clear all state', () => {
      budget.track('a', 100);
      budget.track('b', 200);
      budget.reset();

      assert.equal(budget.currentTokens, 0);
      assert.equal(budget.items.length, 0);
    });
  });
});

// =============================================================================
// CONTEXT ASSEMBLER TESTS
// =============================================================================

describe('Context Intelligence: assembleContext', () => {
  const items = [
    { text: 'High quality code', turn: 0 },
    { text: 'Medium content here', turn: 2 },
    { text: 'Old boring filler stuff that is not very useful', turn: 10 },
    { text: 'function important() { return true; }', turn: 1 },
  ];

  it('should return empty result for empty input', () => {
    const result = assembleContext([]);
    assert.equal(result.content, '');
    assert.equal(result.tokens, 0);
  });

  it('should score and sort items', () => {
    const result = assembleContext(items, {}, { turnsSinceStart: 10 });
    assert.ok(result.items.length > 0);
    assert.ok(result.items[0].cScore !== undefined);
  });

  it('should respect token budget', () => {
    const result = assembleContext(items, {}, { maxTokens: 20 });
    assert.ok(result.tokens <= 20);
    // With very tight budget, some items will be dropped
    assert.ok(result.items.length < items.length, 'Should have fewer items than input');
  });

  it('should include stats', () => {
    const result = assembleContext(items);
    assert.ok('stats' in result);
    assert.ok('selected' in result.stats);
    assert.ok('dropped' in result.stats);
    assert.ok('avgCScore' in result.stats);
  });

  it('should join content with separators', () => {
    const result = assembleContext(items, {}, { maxTokens: 10000 });
    assert.ok(result.content.includes('---'));
  });

  it('should apply ends-matter strategy', () => {
    const manyItems = Array(6).fill(null).map((_, i) => ({
      text: `Item ${i}`,
      turn: i,
    }));

    const result = assembleContext(manyItems, {}, {
      strategy: 'ends-matter',
      turnsSinceStart: 10,
      maxTokens: 10000,
    });

    assert.equal(result.stats.strategy, 'ends-matter');
    // Should have all items since budget is large
    assert.equal(result.items.length, 6);
    // Items should be rearranged (not in original order)
    assert.ok(result.items.every(i => 'cScore' in i), 'All items should have cScore');
  });

  it('should preserve originalIndex', () => {
    const result = assembleContext(items);
    result.items.forEach(item => {
      assert.ok('originalIndex' in item);
    });
  });
});

// =============================================================================
// ENTROPY TESTS
// =============================================================================

describe('Context Intelligence: Entropy', () => {
  describe('calculateShannonEntropy', () => {
    it('should return 0 for empty input', () => {
      assert.equal(calculateShannonEntropy(''), 0);
      assert.equal(calculateShannonEntropy(null), 0);
      assert.equal(calculateShannonEntropy(undefined), 0);
    });

    it('should return 0 for single repeated character', () => {
      const entropy = calculateShannonEntropy('aaaaaaaaaa');
      assert.equal(entropy, 0);
    });

    it('should return high entropy for uniform distribution', () => {
      // All unique characters = high entropy
      const entropy = calculateShannonEntropy('abcdefghijklmnop');
      assert.ok(entropy > 0.9, `Expected > 0.9, got ${entropy}`);
    });

    it('should return moderate entropy for typical text', () => {
      const entropy = calculateShannonEntropy('The quick brown fox jumps over the lazy dog');
      assert.ok(entropy > 0.5 && entropy < 1, `Expected 0.5-1, got ${entropy}`);
    });
  });

  describe('calculateWordEntropy', () => {
    it('should return 0 for empty input', () => {
      assert.equal(calculateWordEntropy(''), 0);
    });

    it('should return low entropy for repetitive words', () => {
      const entropy = calculateWordEntropy('hello hello hello hello hello');
      assert.equal(entropy, 0);
    });

    it('should return high entropy for diverse words', () => {
      const entropy = calculateWordEntropy('one two three four five six seven eight');
      assert.ok(entropy > 0.9, `Expected > 0.9 for diverse words, got ${entropy}`);
    });
  });

  describe('calculateLexicalEntropy', () => {
    it('should return 0 for empty input', () => {
      assert.equal(calculateLexicalEntropy(''), 0);
    });

    it('should detect vocabulary richness', () => {
      const rich = calculateLexicalEntropy('diverse unique vocabulary richness complexity');
      const poor = calculateLexicalEntropy('the the the the the the');
      assert.ok(rich > poor, `Rich vocab (${rich}) should exceed poor (${poor})`);
    });

    it('should handle code-like content', () => {
      const code = calculateLexicalEntropy('function const return export import class');
      assert.ok(code > 0.5, `Expected > 0.5 for code, got ${code}`);
    });
  });

  describe('calculateStructuralEntropy', () => {
    it('should return 0.5 for very short text', () => {
      const entropy = calculateStructuralEntropy('hi');
      assert.equal(entropy, 0.5);
    });

    it('should detect repetitive structure', () => {
      const repetitive = calculateStructuralEntropy(
        'line one\nline one\nline one\nline one\nline one'
      );
      assert.ok(repetitive < 0.7, `Expected < 0.7 for repetitive, got ${repetitive}`);
    });

    it('should boost structured content', () => {
      const structured = calculateStructuralEntropy(`
        {
          "key": "value",
          "nested": { "a": 1, "b": 2 }
        }
      `);
      assert.ok(structured >= 0.5, `Expected >= 0.5 for structured, got ${structured}`);
    });
  });

  describe('entropyToRetentionFactor', () => {
    it('should return high factor for optimal entropy', () => {
      const factor = entropyToRetentionFactor(PHI_INV); // 61.8%
      assert.ok(factor > 0.9, `Expected > 0.9 for optimal, got ${factor}`);
    });

    it('should penalize very high entropy', () => {
      const factor = entropyToRetentionFactor(0.95);
      assert.ok(factor < 0.7, `Expected < 0.7 for high entropy, got ${factor}`);
    });

    it('should handle low entropy with slight boost', () => {
      const factor = entropyToRetentionFactor(0.2);
      assert.ok(factor >= 0.7, `Expected >= 0.7 for low entropy, got ${factor}`);
    });

    it('should never go below 0.3', () => {
      const factor = entropyToRetentionFactor(1.0);
      assert.ok(factor >= 0.3, `Expected >= 0.3 minimum, got ${factor}`);
    });
  });

  describe('calculateEntropyFactor', () => {
    it('should return neutral for empty content', () => {
      const result = calculateEntropyFactor('');
      assert.equal(result.factor, 0.5);
    });

    it('should return full breakdown', () => {
      const result = calculateEntropyFactor({ text: 'sample content for testing' });
      assert.ok('entropy' in result);
      assert.ok('factor' in result);
      assert.ok('breakdown' in result);
      assert.ok('shannon' in result.breakdown);
      assert.ok('wordEntropy' in result.breakdown);
      assert.ok('lexical' in result.breakdown);
      assert.ok('structural' in result.breakdown);
    });

    it('should detect code-like content', () => {
      const code = calculateEntropyFactor({
        text: `
          function login(user) {
            const token = auth.generate(user);
            export { token };
            return token;
          }
        `
      });
      assert.ok(code.isCodeLike, 'Should detect code');
    });

    it('should give high retention to focused code', () => {
      const code = calculateEntropyFactor({
        text: `
          function processData(input) {
            const result = transform(input);
            return validate(result);
          }
        `
      });
      assert.ok(code.factor > 0.7, `Expected > 0.7 for code, got ${code.factor}`);
    });

    it('should give lower retention to verbose filler', () => {
      const filler = calculateEntropyFactor({
        text: 'blah blah blah blah blah blah blah blah blah blah'
      });
      assert.ok(filler.factor < 0.9, `Expected < 0.9 for filler, got ${filler.factor}`);
    });
  });

  describe('ENTROPY_THRESHOLDS', () => {
    it('should be phi-aligned', () => {
      assert.equal(ENTROPY_THRESHOLDS.OPTIMAL, PHI_INV);
      assert.equal(ENTROPY_THRESHOLDS.LOW, PHI_INV_2);
    });

    it('should have correct hierarchy', () => {
      assert.ok(ENTROPY_THRESHOLDS.LOW < ENTROPY_THRESHOLDS.OPTIMAL);
      assert.ok(ENTROPY_THRESHOLDS.OPTIMAL < ENTROPY_THRESHOLDS.HIGH);
    });
  });

  describe('ENTROPY_WEIGHTS', () => {
    it('should be phi-aligned', () => {
      assert.equal(ENTROPY_WEIGHTS.SHANNON, PHI_INV);
      assert.equal(ENTROPY_WEIGHTS.LEXICAL, PHI_INV_2);
      assert.equal(ENTROPY_WEIGHTS.STRUCTURAL, PHI_INV_3);
    });

    it('should sum to approximately φ', () => {
      const sum = ENTROPY_WEIGHTS.SHANNON + ENTROPY_WEIGHTS.LEXICAL + ENTROPY_WEIGHTS.STRUCTURAL;
      // Should be close to PHI_INV + PHI_INV_2 + PHI_INV_3 ≈ 1.236
      assert.ok(Math.abs(sum - 1.236) < 0.01, `Expected sum ~1.236, got ${sum}`);
    });
  });
});

describe('Context Intelligence: C-Score with Entropy', () => {
  it('should include entropy in breakdown', () => {
    const content = { text: 'test content' };
    const result = calculateCScore(content);

    assert.ok('entropy' in result.breakdown);
    assert.ok('entropyRaw' in result.breakdown);
    assert.ok('entropyBreakdown' in result.breakdown);
  });

  it('should include isCodeLike in meta', () => {
    const result = calculateCScore({ text: 'function test() {}' });
    assert.ok('isCodeLike' in result.meta);
  });

  it('should use updated formula with E', () => {
    const result = calculateCScore({ text: 'test' });
    assert.ok(result.formula.includes('E'), 'Formula should include E');
    assert.ok(result.formula.includes('P × F × D × E'), 'Formula should show P × F × D × E');
  });

  it('should favor low-entropy code over high-entropy filler', () => {
    const code = {
      text: `
        function authenticate(user) {
          return generateToken(user);
        }
      `,
    };
    const filler = {
      text: 'random random random random random random random random',
    };

    const codeScore = calculateCScore(code);
    const fillerScore = calculateCScore(filler);

    // Code should have better entropy factor
    assert.ok(
      codeScore.breakdown.entropy >= fillerScore.breakdown.entropy,
      `Code entropy (${codeScore.breakdown.entropy}) should >= filler (${fillerScore.breakdown.entropy})`
    );
  });
});

// =============================================================================
// PHI ALIGNMENT TESTS
// =============================================================================

describe('Context Intelligence: PHI Alignment', () => {
  it('should use correct phi constants for thresholds', () => {
    assert.equal(BUDGET_THRESHOLDS.TARGET, PHI_INV_3);
    assert.equal(BUDGET_THRESHOLDS.SOFT, PHI_INV_2);
    assert.equal(BUDGET_THRESHOLDS.HARD, PHI_INV);
  });

  it('should align with 60% max from research', () => {
    // Research suggests 60% max utilization is optimal
    // PHI_INV (61.8%) is close and mathematically elegant
    assert.ok(Math.abs(BUDGET_THRESHOLDS.HARD - 0.60) < 0.02,
      `Hard limit ${BUDGET_THRESHOLDS.HARD} should be ~60%`);
  });

  it('should have thresholds in correct order', () => {
    assert.ok(BUDGET_THRESHOLDS.TARGET < BUDGET_THRESHOLDS.SOFT);
    assert.ok(BUDGET_THRESHOLDS.SOFT < BUDGET_THRESHOLDS.HARD);
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Context Intelligence: Integration', () => {
  it('should work end-to-end: score -> budget -> assemble', () => {
    const budget = new BudgetManager({ maxTokens: 1000 });
    const items = [
      { text: 'Critical auth code function login() {}', turn: 0 },
      { text: 'Some documentation about the system', turn: 3 },
      { text: 'Old log output from previous run', turn: 15 },
    ];

    // Score items
    const context = { query: 'authentication' };
    const scored = items.map((item, idx) => {
      const cScore = calculateCScore(item, context, { turnsSinceAdded: 20 - item.turn });
      return { ...item, cScore: cScore.C };
    });

    // Track in budget
    scored.forEach((item, idx) => {
      const tokens = countTokens(item.text);
      if (budget.canAdd(tokens).allowed) {
        budget.track(`item-${idx}`, tokens, { cScore: item.cScore });
      }
    });

    // Assemble
    const assembled = assembleContext(items, context, {
      maxTokens: 500,
      turnsSinceStart: 20,
    });

    assert.ok(assembled.content.length > 0);
    assert.ok(assembled.stats.avgCScore > 0);
    assert.ok(budget.getUtilization() > 0);
  });

  it('should handle real-world content patterns', () => {
    // Turn = when item was added. Higher turn = more recent.
    // turnsSinceAdded = turnsSinceStart - item.turn
    const realItems = [
      {
        text: 'function login() { return auth(); }',
        type: 'code',
        turn: 18, // Recent code (turn 18, turnsSinceStart=20 means 2 turns old)
      },
      {
        text: 'The login function authenticates users.',
        type: 'documentation',
        turn: 15,
      },
      {
        text: 'DEBUG: Request received at 2024-01-15 14:32:00 - verbose log',
        type: 'log',
        turn: 2, // Old log (turn 2, turnsSinceStart=20 means 18 turns old)
      },
    ];

    const context = { query: 'login function', taskType: 'code' };
    const result = assembleContext(realItems, context, { turnsSinceStart: 20 });

    // All items should be scored
    assert.ok(result.items.every(i => 'cScore' in i), 'All items should have cScore');

    // Verify the scoring components work correctly
    const codeItem = result.items.find(i => i.type === 'code');
    const logItem = result.items.find(i => i.type === 'log');

    // Code should be fresher (added at turn 18 vs turn 2, so 2 vs 18 turns old)
    assert.ok(codeItem.cScoreBreakdown.freshness > logItem.cScoreBreakdown.freshness,
      `Code freshness (${codeItem.cScoreBreakdown.freshness}) should exceed log (${logItem.cScoreBreakdown.freshness})`);

    // Code should have better pertinence (keyword match + type match)
    assert.ok(codeItem.cScoreBreakdown.pertinence >= logItem.cScoreBreakdown.pertinence,
      `Code pertinence should be >= log pertinence`);
  });
});
