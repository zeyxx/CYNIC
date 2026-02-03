/**
 * @cynic/llm Adapters Tests
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  LLMAdapter,
  ClaudeCodeAdapter,
  OSSLLMAdapter,
  AirLLMAdapter,
  createOllamaValidator,
  createLMStudioValidator,
  createAirLLMValidator,
} from '../src/adapters/index.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

describe('LLMAdapter (base)', () => {
  it('should create with defaults', () => {
    const adapter = new LLMAdapter();
    assert.equal(adapter.provider, 'abstract');
    assert.equal(adapter.model, 'unknown');
    assert.equal(adapter.enabled, false);
  });

  it('should accept options', () => {
    const adapter = new LLMAdapter({ provider: 'test', model: 'test-model' });
    assert.equal(adapter.provider, 'test');
    assert.equal(adapter.model, 'test-model');
  });

  it('should track stats', () => {
    const adapter = new LLMAdapter();
    assert.equal(adapter.stats.requests, 0);
    assert.equal(adapter.stats.successes, 0);
    assert.equal(adapter.stats.failures, 0);
  });

  it('should throw on complete() (abstract)', async () => {
    const adapter = new LLMAdapter();
    await assert.rejects(
      () => adapter.complete('test'),
      /complete\(\) must be implemented/
    );
  });

  it('should report availability based on enabled', async () => {
    const adapter = new LLMAdapter();
    assert.equal(await adapter.isAvailable(), false);
    adapter.enabled = true;
    assert.equal(await adapter.isAvailable(), true);
  });

  it('should return info', () => {
    const adapter = new LLMAdapter({ provider: 'test', model: 'model' });
    adapter.enabled = true;
    const info = adapter.getInfo();
    assert.equal(info.provider, 'test');
    assert.equal(info.model, 'model');
    assert.equal(info.enabled, true);
  });
});

describe('ClaudeCodeAdapter', () => {
  it('should create with claude-code provider', () => {
    const adapter = new ClaudeCodeAdapter();
    assert.equal(adapter.provider, 'claude-code');
    assert.ok(adapter.model.includes('claude'));
    assert.equal(adapter.enabled, true);
  });

  it('should always be available', async () => {
    const adapter = new ClaudeCodeAdapter();
    assert.equal(await adapter.isAvailable(), true);
  });

  it('should return prompt as content (pass-through)', async () => {
    const adapter = new ClaudeCodeAdapter();
    const response = await adapter.complete('Test prompt');
    assert.equal(response.content, 'Test prompt');
    assert.equal(response.provider, 'claude-code');
  });

  it('should have φ⁻¹ confidence', async () => {
    const adapter = new ClaudeCodeAdapter();
    const response = await adapter.complete('Test');
    assert.ok(Math.abs(response.confidence - PHI_INV) < 0.0001);
  });

  it('should track stats', async () => {
    const adapter = new ClaudeCodeAdapter();
    await adapter.complete('Test 1');
    await adapter.complete('Test 2');
    assert.equal(adapter.stats.requests, 2);
    assert.equal(adapter.stats.successes, 2);
  });

  it('should emit complete event', async () => {
    const adapter = new ClaudeCodeAdapter();
    let emitted = null;
    adapter.on('complete', (resp) => { emitted = resp; });
    await adapter.complete('Test');
    assert.ok(emitted);
    assert.equal(emitted.content, 'Test');
  });
});

describe('OSSLLMAdapter', () => {
  it('should create with ollama defaults', () => {
    const adapter = new OSSLLMAdapter();
    assert.equal(adapter.provider, 'ollama');
    assert.equal(adapter.apiFormat, 'ollama');
    assert.equal(adapter.enabled, false);
  });

  it('should accept configuration', () => {
    const adapter = new OSSLLMAdapter({
      provider: 'custom',
      model: 'custom-model',
      endpoint: 'http://localhost:8080',
      apiFormat: 'openai',
    });
    assert.equal(adapter.provider, 'custom');
    assert.equal(adapter.model, 'custom-model');
    assert.equal(adapter.endpoint, 'http://localhost:8080');
    assert.equal(adapter.apiFormat, 'openai');
  });

  it('should configure via configure()', () => {
    const adapter = new OSSLLMAdapter();
    adapter.configure({
      endpoint: 'http://localhost:11434',
      model: 'gemma2:2b',
    });
    assert.equal(adapter.endpoint, 'http://localhost:11434');
    assert.equal(adapter.model, 'gemma2:2b');
    assert.equal(adapter.enabled, true);
  });

  it('should throw when not configured', async () => {
    const adapter = new OSSLLMAdapter();
    await assert.rejects(
      () => adapter.complete('test'),
      /not configured/
    );
  });
});

describe('AirLLMAdapter', () => {
  it('should create with airllm provider', () => {
    const adapter = new AirLLMAdapter();
    assert.equal(adapter.provider, 'airllm');
    assert.ok(adapter.model.includes('mistral') || adapter.model.includes('7b'));
  });

  it('should have longer timeout', () => {
    const adapter = new AirLLMAdapter();
    assert.equal(adapter.timeout, 120000);
  });

  it('should have deep analysis enabled by default', () => {
    const adapter = new AirLLMAdapter();
    assert.equal(adapter.deepAnalysis, true);
  });

  it('should allow disabling deep analysis', () => {
    const adapter = new AirLLMAdapter({ deepAnalysis: false });
    assert.equal(adapter.deepAnalysis, false);
  });

  it('should extend OSSLLMAdapter', () => {
    const adapter = new AirLLMAdapter();
    assert.ok(adapter instanceof OSSLLMAdapter);
    assert.ok(adapter instanceof LLMAdapter);
  });
});

describe('Validator factories', () => {
  describe('createOllamaValidator', () => {
    it('should create enabled Ollama adapter', () => {
      const validator = createOllamaValidator({ model: 'test' });
      assert.equal(validator.provider, 'ollama');
      assert.equal(validator.model, 'test');
      assert.equal(validator.enabled, true);
      assert.equal(validator.apiFormat, 'ollama');
    });

    it('should use default endpoint', () => {
      const validator = createOllamaValidator();
      assert.equal(validator.endpoint, 'http://localhost:11434');
    });
  });

  describe('createLMStudioValidator', () => {
    it('should create enabled LM Studio adapter', () => {
      const validator = createLMStudioValidator({ model: 'test' });
      assert.equal(validator.provider, 'lm-studio');
      assert.equal(validator.model, 'test');
      assert.equal(validator.enabled, true);
      assert.equal(validator.apiFormat, 'openai');
    });

    it('should use default endpoint', () => {
      const validator = createLMStudioValidator();
      assert.equal(validator.endpoint, 'http://localhost:1234');
    });
  });

  describe('createAirLLMValidator', () => {
    it('should create enabled AirLLM adapter', () => {
      const validator = createAirLLMValidator();
      assert.equal(validator.provider, 'airllm');
      assert.equal(validator.enabled, true);
      assert.equal(validator.deepAnalysis, true);
    });

    it('should use long timeout', () => {
      const validator = createAirLLMValidator();
      assert.equal(validator.timeout, 120000);
    });
  });
});

describe('Adapter inheritance', () => {
  it('ClaudeCodeAdapter extends LLMAdapter', () => {
    const adapter = new ClaudeCodeAdapter();
    assert.ok(adapter instanceof LLMAdapter);
  });

  it('OSSLLMAdapter extends LLMAdapter', () => {
    const adapter = new OSSLLMAdapter();
    assert.ok(adapter instanceof LLMAdapter);
  });

  it('AirLLMAdapter extends OSSLLMAdapter', () => {
    const adapter = new AirLLMAdapter();
    assert.ok(adapter instanceof OSSLLMAdapter);
    assert.ok(adapter instanceof LLMAdapter);
  });
});
