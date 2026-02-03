/**
 * @cynic/llm Types Tests
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  LLMResponse,
  ConsensusResult,
  ExecutionTier,
  LLMProvider,
  ConfidenceThresholds,
} from '../src/types.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

describe('LLMResponse', () => {
  it('should create with defaults', () => {
    const response = new LLMResponse();
    assert.ok(response.id.startsWith('resp-'));
    assert.equal(response.provider, 'unknown');
    assert.equal(response.model, 'unknown');
    assert.equal(response.content, '');
    assert.ok(response.timestamp > 0);
  });

  it('should accept data in constructor', () => {
    const response = new LLMResponse({
      provider: 'ollama',
      model: 'gemma2:2b',
      content: 'Hello',
      confidence: 0.5,
    });
    assert.equal(response.provider, 'ollama');
    assert.equal(response.model, 'gemma2:2b');
    assert.equal(response.content, 'Hello');
    assert.equal(response.confidence, 0.5);
  });

  it('should cap confidence at φ⁻¹', () => {
    const response = new LLMResponse({ confidence: 0.9 });
    assert.equal(response.confidence, PHI_INV);
  });

  it('should cap confidence at φ⁻¹ exactly', () => {
    const response = new LLMResponse({ confidence: 1.0 });
    assert.ok(Math.abs(response.confidence - PHI_INV) < 0.0001);
  });

  it('should serialize to JSON', () => {
    const response = new LLMResponse({
      provider: 'test',
      model: 'test-model',
    });
    const json = response.toJSON();
    assert.equal(json.provider, 'test');
    assert.equal(json.model, 'test-model');
    assert.ok('tokens' in json);
    assert.ok('duration' in json);
  });
});

describe('ConsensusResult', () => {
  it('should create with defaults', () => {
    const result = new ConsensusResult();
    assert.ok(result.id.startsWith('consensus-'));
    assert.deepEqual(result.responses, []);
    assert.equal(result.agreement, 0);
    assert.equal(result.verdict, null);
  });

  it('should accept data in constructor', () => {
    const result = new ConsensusResult({
      responses: [new LLMResponse()],
      agreement: 0.8,
      verdict: 'YES',
    });
    assert.equal(result.responses.length, 1);
    assert.equal(result.agreement, 0.8);
    assert.equal(result.verdict, 'YES');
  });

  it('should cap confidence at φ⁻¹', () => {
    const result = new ConsensusResult({ confidence: 0.9 });
    assert.equal(result.confidence, PHI_INV);
  });

  it('should report hasConsensus at φ⁻¹ threshold', () => {
    const weak = new ConsensusResult({ agreement: 0.5 });
    assert.equal(weak.hasConsensus, false);

    const strong = new ConsensusResult({ agreement: PHI_INV });
    assert.equal(strong.hasConsensus, true);

    const veryStrong = new ConsensusResult({ agreement: 0.9 });
    assert.equal(veryStrong.hasConsensus, true);
  });

  it('should report isStrong at 2×φ⁻² threshold (~76.4%)', () => {
    const notStrong = new ConsensusResult({ agreement: 0.7 });
    assert.equal(notStrong.isStrong, false);

    const strong = new ConsensusResult({ agreement: 0.82 });
    assert.equal(strong.isStrong, true);
  });

  it('should serialize to JSON', () => {
    const result = new ConsensusResult({
      responses: [new LLMResponse(), new LLMResponse()],
      agreement: 0.75,
      verdict: 'YES',
    });
    const json = result.toJSON();
    assert.equal(json.responseCount, 2);
    assert.equal(json.agreement, 0.75);
    assert.equal(json.verdict, 'YES');
    assert.equal(json.hasConsensus, true);
  });
});

describe('ExecutionTier', () => {
  it('should have all expected tiers', () => {
    assert.equal(ExecutionTier.LOCAL, 'LOCAL');
    assert.equal(ExecutionTier.LIGHT, 'LIGHT');
    assert.equal(ExecutionTier.FULL, 'FULL');
    assert.equal(ExecutionTier.DEEP, 'DEEP');
  });

  it('should be frozen', () => {
    assert.throws(() => {
      ExecutionTier.NEW_TIER = 'test';
    });
  });
});

describe('LLMProvider', () => {
  it('should have all expected providers', () => {
    assert.equal(LLMProvider.OLLAMA, 'ollama');
    assert.equal(LLMProvider.CLAUDE_CODE, 'claude-code');
    assert.equal(LLMProvider.OPENAI, 'openai');
    assert.equal(LLMProvider.LM_STUDIO, 'lm-studio');
    assert.equal(LLMProvider.AIRLLM, 'airllm');
  });

  it('should be frozen', () => {
    assert.throws(() => {
      LLMProvider.NEW = 'test';
    });
  });
});

describe('ConfidenceThresholds', () => {
  it('should have φ-aligned thresholds', () => {
    assert.ok(Math.abs(ConfidenceThresholds.MAX - PHI_INV) < 0.0001);
    assert.ok(Math.abs(ConfidenceThresholds.OSS_MAX - PHI_INV_2) < 0.0001);
    assert.ok(Math.abs(ConfidenceThresholds.QUORUM - PHI_INV) < 0.0001);
  });

  it('should have MIN_ACTIONABLE', () => {
    assert.equal(ConfidenceThresholds.MIN_ACTIONABLE, 0.3);
  });

  it('should be frozen', () => {
    assert.throws(() => {
      ConfidenceThresholds.NEW = 0.5;
    });
  });
});
