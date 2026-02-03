/**
 * @cynic/llm Router Tests
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  LLMRouter,
  createLLMRouter,
  getLLMRouter,
  _resetLLMRouterForTesting,
  ComplexityClassifier,
  TIER_COSTS,
  TIER_LATENCIES,
} from '../src/router.js';

import { ExecutionTier } from '../src/types.js';
import { ClaudeCodeAdapter } from '../src/adapters/claude-code.js';
import { OSSLLMAdapter } from '../src/adapters/oss-llm.js';
import { PHI_INV } from '@cynic/core';

describe('ComplexityClassifier', () => {
  let classifier;

  beforeEach(() => {
    classifier = new ComplexityClassifier();
  });

  it('should classify simple patterns as LOCAL', () => {
    const result = classifier.classify({ content: 'list files' });
    assert.equal(result.tier, ExecutionTier.LOCAL);
  });

  it('should classify "show status" as LOCAL', () => {
    const result = classifier.classify({ content: 'show status' });
    assert.equal(result.tier, ExecutionTier.LOCAL);
  });

  it('should classify "get version" as LOCAL', () => {
    const result = classifier.classify({ content: 'get version' });
    assert.equal(result.tier, ExecutionTier.LOCAL);
  });

  it('should classify simple questions as LIGHT', () => {
    const result = classifier.classify({ content: 'What is 2+2?' });
    assert.equal(result.tier, ExecutionTier.LIGHT);
  });

  it('should classify "analyze" keyword as FULL', () => {
    const result = classifier.classify({ content: 'analyze this code' });
    assert.equal(result.tier, ExecutionTier.FULL);
  });

  it('should classify "security" keyword as FULL', () => {
    const result = classifier.classify({ content: 'check security issues' });
    assert.equal(result.tier, ExecutionTier.FULL);
  });

  it('should classify long content as FULL', () => {
    const longContent = 'a'.repeat(600);
    const result = classifier.classify({ content: longContent });
    assert.equal(result.tier, ExecutionTier.FULL);
  });

  it('should classify very long content as DEEP', () => {
    const veryLong = 'a'.repeat(2100);
    const result = classifier.classify({ content: veryLong });
    assert.equal(result.tier, ExecutionTier.DEEP);
  });

  it('should respect forced tier in context', () => {
    const result = classifier.classify({
      content: 'simple question',
      context: { tier: ExecutionTier.DEEP },
    });
    assert.equal(result.tier, ExecutionTier.DEEP);
  });

  it('should track stats', () => {
    classifier.classify({ content: 'list files' });
    classifier.classify({ content: 'analyze code' });
    classifier.classify({ content: 'simple' });
    assert.equal(classifier.stats.classifications, 3);
  });
});

describe('TIER_COSTS', () => {
  it('should have costs for all tiers', () => {
    assert.equal(TIER_COSTS[ExecutionTier.LOCAL], 0);
    assert.equal(TIER_COSTS[ExecutionTier.LIGHT], 1);
    assert.equal(TIER_COSTS[ExecutionTier.FULL], 15);
    assert.equal(TIER_COSTS[ExecutionTier.DEEP], 50);
  });

  it('should be frozen', () => {
    assert.throws(() => {
      TIER_COSTS.NEW = 100;
    });
  });
});

describe('TIER_LATENCIES', () => {
  it('should have latencies for all tiers', () => {
    assert.equal(TIER_LATENCIES[ExecutionTier.LOCAL], 1);
    assert.ok(TIER_LATENCIES[ExecutionTier.LIGHT] > 0);
    assert.ok(TIER_LATENCIES[ExecutionTier.FULL] > TIER_LATENCIES[ExecutionTier.LIGHT]);
    assert.ok(TIER_LATENCIES[ExecutionTier.DEEP] > TIER_LATENCIES[ExecutionTier.FULL]);
  });
});

describe('LLMRouter', () => {
  let router;

  beforeEach(() => {
    router = new LLMRouter();
  });

  afterEach(() => {
    _resetLLMRouterForTesting();
  });

  it('should create with ClaudeCode primary by default', () => {
    assert.ok(router.primary instanceof ClaudeCodeAdapter);
  });

  it('should create with empty validators by default', () => {
    assert.deepEqual(router.validators, []);
  });

  it('should use φ⁻¹ quorum by default', () => {
    assert.ok(Math.abs(router.consensusConfig.quorum - PHI_INV) < 0.0001);
  });

  it('should track stats', () => {
    assert.equal(router.stats.singleRequests, 0);
    assert.equal(router.stats.consensusRequests, 0);
    assert.equal(router.stats.routedRequests, 0);
  });

  describe('complete()', () => {
    it('should delegate to primary adapter', async () => {
      const response = await router.complete('Test prompt');
      assert.equal(response.provider, 'claude-code');
      assert.equal(response.content, 'Test prompt');
    });

    it('should increment singleRequests stat', async () => {
      await router.complete('Test');
      assert.equal(router.stats.singleRequests, 1);
    });
  });

  describe('consensus()', () => {
    it('should fall back to single when no validators', async () => {
      const result = await router.consensus('Test');
      assert.equal(result.responses.length, 1);
      assert.equal(result.agreement, 1.0);
    });

    it('should increment consensusRequests stat', async () => {
      await router.consensus('Test');
      assert.equal(router.stats.consensusRequests, 1);
    });
  });

  describe('route()', () => {
    it('should classify and route LOCAL tier', async () => {
      const result = await router.route({ content: 'list files' });
      assert.equal(result.tier, ExecutionTier.LOCAL);
      assert.equal(result.cost, 0);
    });

    it('should respect forceTier', async () => {
      const result = await router.route({
        content: 'simple',
        forceTier: ExecutionTier.FULL,
      });
      assert.equal(result.tier, ExecutionTier.FULL);
    });

    it('should track byTier stats', async () => {
      await router.route({ content: 'list files' });
      assert.equal(router.stats.byTier[ExecutionTier.LOCAL], 1);
    });
  });

  describe('addValidator()', () => {
    it('should add validator to list', () => {
      const validator = new OSSLLMAdapter({ provider: 'test' });
      router.addValidator(validator);
      assert.equal(router.validators.length, 1);
      assert.equal(router.validators[0].provider, 'test');
    });
  });

  describe('removeValidator()', () => {
    it('should remove validator by provider', () => {
      const v1 = new OSSLLMAdapter({ provider: 'test1' });
      const v2 = new OSSLLMAdapter({ provider: 'test2' });
      router.addValidator(v1);
      router.addValidator(v2);
      router.removeValidator('test1');
      assert.equal(router.validators.length, 1);
      assert.equal(router.validators[0].provider, 'test2');
    });
  });

  describe('getStatus()', () => {
    it('should return status object', () => {
      const status = router.getStatus();
      assert.ok('primary' in status);
      assert.ok('validators' in status);
      assert.ok('config' in status);
      assert.ok('stats' in status);
    });
  });
});

describe('createLLMRouter', () => {
  it('should create new router', () => {
    const router = createLLMRouter();
    assert.ok(router instanceof LLMRouter);
  });

  it('should accept options', () => {
    const router = createLLMRouter({ timeout: 5000 });
    assert.equal(router.consensusConfig.timeout, 5000);
  });
});

describe('getLLMRouter singleton', () => {
  afterEach(() => {
    _resetLLMRouterForTesting();
  });

  it('should return same instance', () => {
    const r1 = getLLMRouter();
    const r2 = getLLMRouter();
    assert.strictEqual(r1, r2);
  });

  it('should reset with _resetLLMRouterForTesting', () => {
    const r1 = getLLMRouter();
    _resetLLMRouterForTesting();
    const r2 = getLLMRouter();
    assert.notStrictEqual(r1, r2);
  });
});
