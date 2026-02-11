/**
 * CYNIC Daemon Tests
 *
 * Tests for daemon server, hook handlers, and HTTP endpoints.
 *
 * "Le chien teste le chien" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { DaemonServer } from '../src/daemon/index.js';
import { handleHookEvent } from '../src/daemon/hook-handlers.js';

// =============================================================================
// HOOK HANDLERS (pure functions, no HTTP needed)
// =============================================================================

describe('Hook Handlers', () => {
  describe('handleHookEvent', () => {
    it('should handle UserPromptSubmit with empty prompt', async () => {
      const result = await handleHookEvent('UserPromptSubmit', { prompt: '' });
      assert.strictEqual(result.continue, true);
    });

    it('should handle UserPromptSubmit with normal prompt', async () => {
      const result = await handleHookEvent('UserPromptSubmit', { prompt: 'add a button' });
      assert.strictEqual(result.continue, true);
    });

    it('should detect danger in UserPromptSubmit', async () => {
      const result = await handleHookEvent('UserPromptSubmit', { prompt: 'rm -rf /' });
      assert.strictEqual(result.continue, true);
      // Should include a danger warning message
      assert.ok(result.message);
      assert.ok(result.message.includes('GROWL') || result.message.includes('DANGER'));
    });

    it('should handle unknown events gracefully', async () => {
      const result = await handleHookEvent('UnknownEvent', {});
      assert.strictEqual(result.continue, true);
    });

    it('should handle null/undefined input', async () => {
      const result = await handleHookEvent('UserPromptSubmit', null);
      assert.strictEqual(result.continue, true);
    });
  });

  describe('PreToolUse guard', () => {
    it('should allow safe Bash commands', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'ls -la' },
      });
      assert.strictEqual(result.continue, true);
      assert.strictEqual(result.blocked, false);
    });

    it('should block rm -rf /', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'rm -rf /' },
      });
      assert.strictEqual(result.blocked, true);
      assert.strictEqual(result.continue, false);
      assert.ok(result.blockReason);
    });

    it('should block rm -rf *', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'rm -rf *' },
      });
      assert.strictEqual(result.blocked, true);
    });

    it('should block fork bombs', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: ':(){ :|:& };:' },
      });
      assert.strictEqual(result.blocked, true);
    });

    it('should block DROP TABLE', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'psql -c "DROP TABLE users"' },
      });
      assert.strictEqual(result.blocked, true);
    });

    it('should warn on force push', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'git push --force origin main' },
      });
      assert.strictEqual(result.continue, true);
      assert.strictEqual(result.blocked, false);
      assert.ok(result.issues.length > 0);
      assert.strictEqual(result.issues[0].action, 'warn');
    });

    it('should warn on .env file writes', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Write',
        tool_input: { file_path: '/home/user/.env' },
      });
      assert.strictEqual(result.continue, true);
      assert.ok(result.issues.length > 0);
    });

    it('should allow safe Write operations', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Write',
        tool_input: { file_path: '/src/index.js' },
      });
      assert.strictEqual(result.continue, true);
      assert.strictEqual(result.issues.length, 0);
    });

    it('should handle missing tool input', async () => {
      const result = await handleHookEvent('PreToolUse', {
        tool_name: 'Bash',
        tool_input: {},
      });
      assert.strictEqual(result.continue, true);
      assert.strictEqual(result.blocked, false);
    });
  });

  describe('PostToolUse observe', () => {
    it('should always return continue', async () => {
      const result = await handleHookEvent('PostToolUse', {
        tool_name: 'Bash',
        tool_input: { command: 'npm test' },
        tool_output: 'All tests passed',
      });
      assert.strictEqual(result.continue, true);
    });
  });

  describe('SessionStart awaken', () => {
    it('should return welcome message', async () => {
      const result = await handleHookEvent('SessionStart', {});
      assert.strictEqual(result.continue, true);
      assert.ok(result.message);
      assert.ok(result.message.includes('daemon'));
    });
  });

  describe('SessionEnd sleep', () => {
    it('should return continue', async () => {
      const result = await handleHookEvent('SessionEnd', {});
      assert.strictEqual(result.continue, true);
    });
  });

  describe('Stop', () => {
    it('should return continue', async () => {
      const result = await handleHookEvent('Stop', {});
      assert.strictEqual(result.continue, true);
    });
  });
});

// =============================================================================
// DAEMON SERVER (HTTP integration)
// =============================================================================

describe('DaemonServer', () => {
  let server;
  const TEST_PORT = 16180; // Test port to avoid conflicts

  beforeEach(async () => {
    server = new DaemonServer({ port: TEST_PORT, host: '127.0.0.1' });
  });

  afterEach(async () => {
    if (server) {
      await server.stop();
    }
  });

  it('should construct with defaults', () => {
    const s = new DaemonServer();
    assert.strictEqual(s.port, 6180);
    assert.strictEqual(s.host, '127.0.0.1');
  });

  it('should construct with custom port', () => {
    assert.strictEqual(server.port, TEST_PORT);
  });

  it('should start and stop', async () => {
    await server.start();
    assert.ok(server.startTime);
    assert.ok(server.server);

    await server.stop();
    assert.strictEqual(server.server, null);
    assert.strictEqual(server.startTime, null);
  });

  it('should respond to /health', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/health`);
    assert.strictEqual(res.status, 200);

    const health = await res.json();
    assert.strictEqual(health.status, 'healthy');
    assert.strictEqual(health.port, TEST_PORT);
    assert.ok(health.pid > 0);
    assert.ok(health.uptime >= 0);
    assert.ok(health.uptimeHuman);
    assert.ok(health.memoryMB > 0);
  });

  it('should respond to /status', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/status`);
    assert.strictEqual(res.status, 200);

    const status = await res.json();
    assert.ok(status.daemon);
    assert.strictEqual(status.daemon.port, TEST_PORT);
  });

  it('should handle POST /hook/UserPromptSubmit', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/hook/UserPromptSubmit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt: 'hello world' }),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.continue, true);
  });

  it('should handle POST /hook/PreToolUse — block dangerous command', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/hook/PreToolUse`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tool_name: 'Bash', tool_input: { command: 'rm -rf /' } }),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.blocked, true);
    assert.strictEqual(result.continue, false);
  });

  it('should handle POST /hook/PreToolUse — allow safe command', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/hook/PreToolUse`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tool_name: 'Bash', tool_input: { command: 'git status' } }),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.blocked, false);
    assert.strictEqual(result.continue, true);
  });

  it('should return 400 for /llm/ask without prompt', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/ask`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 400);
    const result = await res.json();
    assert.ok(result.error.includes('prompt'));
  });

  it('should handle /llm/ask with prompt (adapter-dependent)', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/ask`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt: 'test question' }),
    });
    // 200 = adapter found and responded, 503 = no adapter, 500 = adapter error
    const status = res.status;
    assert.ok(
      status === 200 || status === 503 || status === 500,
      `Expected 200, 503, or 500, got ${status}`
    );
    const body = await res.json();
    if (status === 200) {
      assert.ok(body.content !== undefined, 'Should have content on success');
      assert.ok(body.tier, 'Should have tier on success');
    } else if (status === 503) {
      assert.ok(body.error.includes('No adapter'), 'Should indicate no adapter');
    }
  });

  it('should respond to GET /llm/models', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/models`);
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.ok(Array.isArray(result.models));
    assert.ok(result.thompson);
    assert.ok(result.stats);
  });

  it('should return 400 for /llm/feedback without required fields', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/feedback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 400);
  });

  it('should accept /llm/feedback with valid fields', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/feedback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ taskType: 'code', model: 'sonnet', success: true }),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.recorded, true);
    assert.ok(result.stats);
  });

  it('should return 400 for /llm/consensus without prompt', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/llm/consensus`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 400);
  });

  it('should handle unknown hook events', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${TEST_PORT}/hook/UnknownEvent`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.continue, true);
  });

  it('should reject double start', async () => {
    await server.start();
    await assert.rejects(() => server.start(), /already running/);
  });

  it('should handle stop when not running', async () => {
    // Should not throw
    await server.stop();
  });
});
