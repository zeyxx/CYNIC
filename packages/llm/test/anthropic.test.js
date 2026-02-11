/**
 * CYNIC Anthropic Adapter Tests
 *
 * Tests for AnthropicAdapter — no real API calls, SDK fully mocked.
 *
 * "Le chien teste son propre cerveau" — κυνικός
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { PHI_INV } from '@cynic/core';
import { AnthropicAdapter, createAnthropicValidator, MODEL_MAP } from '../src/adapters/anthropic.js';

// =============================================================================
// MOCK SDK
// =============================================================================

function createMockClient(responseOverrides = {}) {
  return {
    messages: {
      create: mock.fn(async () => ({
        id: 'msg_test_123',
        content: [{ type: 'text', text: responseOverrides.text || 'Mock response from Anthropic' }],
        usage: {
          input_tokens: responseOverrides.inputTokens || 42,
          output_tokens: responseOverrides.outputTokens || 17,
        },
        stop_reason: responseOverrides.stopReason || 'end_turn',
        model: responseOverrides.model || 'claude-sonnet-4-5-20250929',
      })),
    },
  };
}

// =============================================================================
// MODEL_MAP
// =============================================================================

describe('MODEL_MAP', () => {
  it('should map opus to claude-opus-4-6', () => {
    assert.strictEqual(MODEL_MAP.opus, 'claude-opus-4-6');
  });

  it('should map sonnet to claude-sonnet-4-5-20250929', () => {
    assert.strictEqual(MODEL_MAP.sonnet, 'claude-sonnet-4-5-20250929');
  });

  it('should map haiku to claude-haiku-4-5-20251001', () => {
    assert.strictEqual(MODEL_MAP.haiku, 'claude-haiku-4-5-20251001');
  });

  it('should be frozen', () => {
    assert.throws(() => { MODEL_MAP.opus = 'nope'; }, TypeError);
  });
});

// =============================================================================
// CONSTRUCTOR
// =============================================================================

describe('AnthropicAdapter', () => {
  describe('constructor', () => {
    it('should create with provider anthropic', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      assert.strictEqual(adapter.provider, 'anthropic');
    });

    it('should default to sonnet model', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      assert.strictEqual(adapter.model, MODEL_MAP.sonnet);
    });

    it('should accept custom model', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test', model: MODEL_MAP.opus });
      assert.strictEqual(adapter.model, MODEL_MAP.opus);
    });

    it('should be enabled when apiKey provided', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      assert.strictEqual(adapter.enabled, true);
    });

    it('should be disabled without apiKey', () => {
      const adapter = new AnthropicAdapter();
      assert.strictEqual(adapter.enabled, false);
    });

    it('should default timeout to 60000', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      assert.strictEqual(adapter.timeout, 60000);
    });

    it('should accept custom timeout', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test', timeout: 30000 });
      assert.strictEqual(adapter.timeout, 30000);
    });
  });

  // ===========================================================================
  // CONFIGURE
  // ===========================================================================

  describe('configure', () => {
    it('should update apiKey and enable', () => {
      const adapter = new AnthropicAdapter();
      assert.strictEqual(adapter.enabled, false);

      adapter.configure({ apiKey: 'sk-new' });
      assert.strictEqual(adapter.apiKey, 'sk-new');
      assert.strictEqual(adapter.enabled, true);
    });

    it('should update model with tier key', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      adapter.configure({ model: 'opus' });
      assert.strictEqual(adapter.model, MODEL_MAP.opus);
    });

    it('should update model with full ID', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      adapter.configure({ model: 'claude-haiku-4-5-20251001' });
      assert.strictEqual(adapter.model, 'claude-haiku-4-5-20251001');
    });

    it('should clear client on reconfigure', () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      adapter._client = { fake: true };
      adapter.configure({ apiKey: 'sk-new' });
      assert.strictEqual(adapter._client, null);
    });
  });

  // ===========================================================================
  // COMPLETE
  // ===========================================================================

  describe('complete', () => {
    let adapter;
    let mockClient;

    beforeEach(() => {
      adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      mockClient = createMockClient();
      adapter._client = mockClient; // Inject mock — skip SDK load
    });

    it('should return LLMResponse with content', async () => {
      const response = await adapter.complete('What is 2+2?');
      assert.strictEqual(response.content, 'Mock response from Anthropic');
      assert.strictEqual(response.provider, 'anthropic');
    });

    it('should pass prompt as user message', async () => {
      await adapter.complete('test prompt');
      const call = mockClient.messages.create.mock.calls[0];
      const params = call.arguments[0];
      assert.deepStrictEqual(params.messages, [{ role: 'user', content: 'test prompt' }]);
    });

    it('should pass system prompt when provided', async () => {
      await adapter.complete('test', { system: 'You are CYNIC' });
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.system, 'You are CYNIC');
    });

    it('should not include system when not provided', async () => {
      await adapter.complete('test');
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.system, undefined);
    });

    it('should use default temperature and maxTokens', async () => {
      await adapter.complete('test');
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.temperature, 0.7);
      assert.strictEqual(params.max_tokens, 1024);
    });

    it('should accept custom temperature and maxTokens', async () => {
      await adapter.complete('test', { temperature: 0.3, maxTokens: 2048 });
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.temperature, 0.3);
      assert.strictEqual(params.max_tokens, 2048);
    });

    it('should resolve model override from tier key', async () => {
      await adapter.complete('test', { model: 'opus' });
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.model, MODEL_MAP.opus);
    });

    it('should pass full model ID directly', async () => {
      await adapter.complete('test', { model: 'claude-haiku-4-5-20251001' });
      const params = mockClient.messages.create.mock.calls[0].arguments[0];
      assert.strictEqual(params.model, 'claude-haiku-4-5-20251001');
    });

    it('should cap confidence at PHI_INV', async () => {
      const response = await adapter.complete('test');
      assert.ok(response.confidence <= PHI_INV);
      assert.strictEqual(response.confidence, PHI_INV);
    });

    it('should map token usage from API response', async () => {
      const response = await adapter.complete('test');
      assert.strictEqual(response.tokens.input, 42);
      assert.strictEqual(response.tokens.output, 17);
    });

    it('should include metadata with stop reason and id', async () => {
      const response = await adapter.complete('test');
      assert.strictEqual(response.metadata.type, 'anthropic');
      assert.strictEqual(response.metadata.stopReason, 'end_turn');
      assert.strictEqual(response.metadata.id, 'msg_test_123');
    });

    it('should track stats on success', async () => {
      await adapter.complete('test');
      assert.strictEqual(adapter.stats.requests, 1);
      assert.strictEqual(adapter.stats.successes, 1);
      assert.strictEqual(adapter.stats.failures, 0);
      assert.strictEqual(adapter.stats.totalTokens, 59); // 42 + 17
    });

    it('should emit complete event', async () => {
      let emitted = null;
      adapter.on('complete', (resp) => { emitted = resp; });
      await adapter.complete('test');
      assert.ok(emitted);
      assert.strictEqual(emitted.content, 'Mock response from Anthropic');
    });

    it('should throw when not configured', async () => {
      const disabled = new AnthropicAdapter();
      await assert.rejects(
        () => disabled.complete('test'),
        /ANTHROPIC_API_KEY missing/
      );
    });

    it('should track failures on error', async () => {
      mockClient.messages.create = mock.fn(async () => {
        throw new Error('API error');
      });
      await assert.rejects(() => adapter.complete('test'), /API error/);
      assert.strictEqual(adapter.stats.requests, 1);
      assert.strictEqual(adapter.stats.failures, 1);
      assert.strictEqual(adapter.stats.successes, 0);
    });

    it('should handle empty content array', async () => {
      mockClient.messages.create = mock.fn(async () => ({
        id: 'msg_empty',
        content: [],
        usage: { input_tokens: 10, output_tokens: 0 },
        stop_reason: 'end_turn',
      }));
      const response = await adapter.complete('test');
      assert.strictEqual(response.content, '');
    });

    it('should measure duration', async () => {
      const response = await adapter.complete('test');
      assert.ok(response.duration >= 0);
    });
  });

  // ===========================================================================
  // IS AVAILABLE
  // ===========================================================================

  describe('isAvailable', () => {
    it('should return false when disabled', async () => {
      const adapter = new AnthropicAdapter();
      assert.strictEqual(await adapter.isAvailable(), false);
    });

    it('should return true when client loads successfully', async () => {
      const adapter = new AnthropicAdapter({ apiKey: 'sk-test' });
      adapter._client = createMockClient(); // Pre-inject mock
      assert.strictEqual(await adapter.isAvailable(), true);
    });
  });
});

// =============================================================================
// FACTORY
// =============================================================================

describe('createAnthropicValidator', () => {
  it('should create adapter with provided apiKey', () => {
    const adapter = createAnthropicValidator({ apiKey: 'sk-factory' });
    assert.strictEqual(adapter.provider, 'anthropic');
    assert.strictEqual(adapter.apiKey, 'sk-factory');
    assert.strictEqual(adapter.enabled, true);
  });

  it('should default to sonnet model', () => {
    const adapter = createAnthropicValidator({ apiKey: 'sk-test' });
    assert.strictEqual(adapter.model, MODEL_MAP.sonnet);
  });

  it('should accept tier key for model', () => {
    const adapter = createAnthropicValidator({ apiKey: 'sk-test', model: 'opus' });
    assert.strictEqual(adapter.model, MODEL_MAP.opus);
  });

  it('should accept custom timeout', () => {
    const adapter = createAnthropicValidator({ apiKey: 'sk-test', timeout: 30000 });
    assert.strictEqual(adapter.timeout, 30000);
  });

  it('should be disabled without apiKey', () => {
    // No env var set in test
    const adapter = createAnthropicValidator({ apiKey: '' });
    assert.strictEqual(adapter.enabled, false);
  });
});
