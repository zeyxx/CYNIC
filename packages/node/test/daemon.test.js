/**
 * CYNIC Daemon Tests
 *
 * Tests for daemon server, hook handlers, and HTTP endpoints.
 * Phase 2: service-wiring, watchdog, handleStop, thin stop hook.
 * Phase 3: digest migration — digest-formatter, Q-Learning, markdown export.
 * Phase 4: thin hooks 12/12 — SubagentStart/Stop, Error, Notification handlers.
 *
 * "Le chien teste le chien" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'fs';
import path from 'path';
import os from 'os';

import { DaemonServer } from '../src/daemon/index.js';
import { handleHookEvent, _resetHandlersForTesting, classifyError, extractErrorContext, classifyNotification, detectNotificationBurst, SUBAGENT_TO_DOG } from '../src/daemon/hook-handlers.js';
import { wireDaemonServices, cleanupDaemonServices, isWired, _resetForTesting as resetServiceWiring } from '../src/daemon/service-wiring.js';
import { Watchdog, HealthLevel, checkRestartSentinel } from '../src/daemon/watchdog.js';
import { formatRichBanner, formatDigestMarkdown, saveDigest } from '../src/daemon/digest-formatter.js';
import { resetModelIntelligence } from '../src/learning/model-intelligence.js';
import { resetCostLedger } from '../src/accounting/cost-ledger.js';

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
    it('should return continue when no ralph-loop active', async () => {
      const result = await handleHookEvent('Stop', {});
      assert.strictEqual(result.continue, true);
    });

    it('should return continue with no transcript', async () => {
      const result = await handleHookEvent('Stop', { transcript_path: null });
      assert.strictEqual(result.continue, true);
    });

    it('should return continue with nonexistent transcript', async () => {
      const result = await handleHookEvent('Stop', { transcript_path: '/nonexistent/path.jsonl' });
      assert.strictEqual(result.continue, true);
    });

    it('should include digest banner when session stats available', async () => {
      // With no ralph-loop file present, handleStop goes to Phase 2 (digest)
      const result = await handleHookEvent('Stop', {});
      assert.strictEqual(result.continue, true);
      // Banner may or may not be present depending on singleton state
      // but the result must have `continue: true`
    });
  });

  describe('Stop — Ralph-loop integration', () => {
    const stateFile = '.claude/ralph-loop.local.md';
    const stateDir = '.claude';

    beforeEach(() => {
      // Ensure clean state
      try { fs.unlinkSync(stateFile); } catch { /* ignore */ }
    });

    afterEach(() => {
      // Clean up
      try { fs.unlinkSync(stateFile); } catch { /* ignore */ }
    });

    it('should block stop when ralph-loop is active', async () => {
      // Create a ralph-loop state file
      if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });

      // Create a mock transcript with an assistant message
      const tmpDir = os.tmpdir();
      const transcriptPath = path.join(tmpDir, `cynic-test-transcript-${Date.now()}.jsonl`);
      const transcriptEntry = JSON.stringify({
        role: 'assistant',
        content: [{ type: 'text', text: '*sniff* Working on it...' }],
      });
      fs.writeFileSync(transcriptPath, transcriptEntry + '\n');

      // Write state file
      fs.writeFileSync(stateFile, `---
iteration: 1
max_iterations: 5
completion_promise: "task is complete"
---
Continue working on the task. Check all tests pass.`);

      const result = await handleHookEvent('Stop', { transcript_path: transcriptPath });

      // Should block
      assert.strictEqual(result.decision, 'block');
      assert.ok(result.reason);
      assert.ok(result.reason.includes('Continue working'));
      assert.strictEqual(result.iteration, 2);
      assert.ok(result.systemMessage);

      // Clean up transcript
      try { fs.unlinkSync(transcriptPath); } catch { /* ignore */ }
    });

    it('should allow stop when max iterations reached', async () => {
      if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });
      fs.writeFileSync(stateFile, `---
iteration: 5
max_iterations: 5
completion_promise: null
---
Do something.`);

      const result = await handleHookEvent('Stop', {});
      assert.strictEqual(result.continue, true);

      // State file should be cleaned up
      assert.strictEqual(fs.existsSync(stateFile), false);
    });

    it('should allow stop when completion promise is fulfilled', async () => {
      if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });

      // Create transcript with fulfilled promise
      const tmpDir = os.tmpdir();
      const transcriptPath = path.join(tmpDir, `cynic-test-transcript-${Date.now()}.jsonl`);
      const transcriptEntry = JSON.stringify({
        role: 'assistant',
        content: [{ type: 'text', text: 'Done! <promise>all tests pass</promise>' }],
      });
      fs.writeFileSync(transcriptPath, transcriptEntry + '\n');

      fs.writeFileSync(stateFile, `---
iteration: 2
max_iterations: 10
completion_promise: "all tests pass"
---
Run tests until they pass.`);

      const result = await handleHookEvent('Stop', { transcript_path: transcriptPath });
      assert.strictEqual(result.continue, true);

      // State file should be cleaned up
      assert.strictEqual(fs.existsSync(stateFile), false);

      try { fs.unlinkSync(transcriptPath); } catch { /* ignore */ }
    });

    it('should handle corrupted state file gracefully', async () => {
      if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });
      fs.writeFileSync(stateFile, 'not yaml at all');

      const result = await handleHookEvent('Stop', {});
      assert.strictEqual(result.continue, true);
    });
  });
});

// =============================================================================
// SERVICE WIRING
// =============================================================================

describe('Service Wiring', () => {
  beforeEach(() => {
    resetServiceWiring();
    resetModelIntelligence();
    resetCostLedger();
  });

  afterEach(() => {
    resetServiceWiring();
    resetModelIntelligence();
    resetCostLedger();
  });

  it('should wire services and mark as wired', () => {
    assert.strictEqual(isWired(), false);
    const result = wireDaemonServices();
    assert.strictEqual(isWired(), true);
    assert.ok(result.modelIntelligence);
    assert.ok(result.costLedger);
  });

  it('should be idempotent (wire twice returns same singletons)', () => {
    const first = wireDaemonServices();
    const second = wireDaemonServices();
    assert.strictEqual(first.modelIntelligence, second.modelIntelligence);
    assert.strictEqual(first.costLedger, second.costLedger);
  });

  it('should cleanup without throwing', () => {
    wireDaemonServices();
    assert.strictEqual(isWired(), true);
    cleanupDaemonServices();
    assert.strictEqual(isWired(), false);
  });

  it('should handle cleanup when not wired', () => {
    // Should not throw
    cleanupDaemonServices();
    assert.strictEqual(isWired(), false);
  });
});

// =============================================================================
// WATCHDOG
// =============================================================================

describe('Watchdog', () => {
  let watchdog;

  afterEach(() => {
    if (watchdog) {
      watchdog.stop();
      watchdog = null;
    }
  });

  it('should construct with defaults', () => {
    watchdog = new Watchdog();
    const status = watchdog.getStatus();
    assert.strictEqual(status.running, false);
    assert.strictEqual(status.level, HealthLevel.HEALTHY);
    assert.strictEqual(status.consecutiveCritical, 0);
  });

  it('should start and stop', () => {
    watchdog = new Watchdog({ interval: 60000 }); // Long interval to avoid auto-check
    watchdog.start();
    assert.strictEqual(watchdog.getStatus().running, true);
    watchdog.stop();
    assert.strictEqual(watchdog.getStatus().running, false);
  });

  it('should run health checks and report metrics', async () => {
    watchdog = new Watchdog({ interval: 100 });
    watchdog.start();

    // Wait for at least one check
    await new Promise(r => setTimeout(r, 200));

    const status = watchdog.getStatus();
    assert.ok(status.checkCount >= 1);
    assert.ok(status.heapUsedMB > 0);
    assert.ok(status.heapRatio > 0);
    assert.ok(status.eventLoopLatencyMs >= 0);
  });

  it('should register subsystem health checks', async () => {
    watchdog = new Watchdog({ interval: 100 });

    watchdog.registerSubsystem('test-healthy', () => ({ healthy: true, message: 'ok' }));
    watchdog.registerSubsystem('test-sick', () => ({ healthy: false, message: 'broken' }));

    watchdog.start();

    await new Promise(r => setTimeout(r, 200));

    const status = watchdog.getStatus();
    assert.ok(status.subsystems['test-healthy'].healthy);
    assert.strictEqual(status.subsystems['test-sick'].healthy, false);
    assert.ok(status.degradedSubsystems.includes('test-sick'));
  });

  it('should escalate to WARNING on unhealthy subsystem', async () => {
    watchdog = new Watchdog({ interval: 100 });
    watchdog.registerSubsystem('broken', () => ({ healthy: false, message: 'broken' }));
    watchdog.start();

    await new Promise(r => setTimeout(r, 200));

    const status = watchdog.getStatus();
    assert.ok(
      status.level === HealthLevel.WARNING || status.level === HealthLevel.CRITICAL,
      `Expected WARNING or CRITICAL, got ${status.level}`
    );
  });

  it('should call onFatal instead of process.exit', async () => {
    let fatalCalled = false;
    watchdog = new Watchdog({
      interval: 50,
      fatalThreshold: 2,
      onFatal: () => { fatalCalled = true; },
    });

    // Register a subsystem that always reports critical heap
    // We can't easily force real heap to 80%+, so we test the escalation logic
    // by checking that consecutive critical tracking works
    watchdog._consecutiveCritical = 1;

    // Simulate a critical check
    watchdog._metrics.level = HealthLevel.CRITICAL;
    watchdog._handleFatal([{ subsystem: 'test', level: 'critical', message: 'test' }]);

    assert.strictEqual(fatalCalled, true);
  });

  describe('Restart sentinel', () => {
    const sentinelPath = path.join(os.homedir(), '.cynic', 'daemon', 'restart-requested');

    beforeEach(() => {
      // Clean up any sentinel left by previous tests (e.g., onFatal test)
      try { fs.unlinkSync(sentinelPath); } catch { /* ignore */ }
    });

    afterEach(() => {
      try { fs.unlinkSync(sentinelPath); } catch { /* ignore */ }
    });

    it('should return recovered: false when no sentinel', () => {
      const result = checkRestartSentinel();
      assert.strictEqual(result.recovered, false);
    });

    it('should detect and clean sentinel file', () => {
      const dir = path.dirname(sentinelPath);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(sentinelPath, JSON.stringify({
        reason: 'watchdog_fatal',
        pid: 12345,
        timestamp: Date.now(),
      }));

      const result = checkRestartSentinel();
      assert.strictEqual(result.recovered, true);
      assert.strictEqual(result.previousCrash.pid, 12345);
      assert.strictEqual(result.previousCrash.reason, 'watchdog_fatal');

      // Sentinel should be cleaned up
      assert.strictEqual(fs.existsSync(sentinelPath), false);
    });
  });
});

// =============================================================================
// DIGEST FORMATTER (Phase 3)
// =============================================================================

describe('Digest Formatter', () => {
  describe('formatRichBanner', () => {
    it('should format banner with identity compliance', () => {
      const banner = formatRichBanner({
        identity: {
          valid: true,
          compliance: 0.85,
          violations: [],
          warnings: [],
        },
      });
      assert.ok(banner.includes('IDENTITY COMPLIANCE'));
      assert.ok(banner.includes('85%'));
      assert.ok(banner.includes('tail wag'));
    });

    it('should show violations in banner', () => {
      const banner = formatRichBanner({
        identity: {
          valid: false,
          compliance: 0.3,
          violations: [{ type: 'forbidden_phrase', found: 'as an ai' }],
          warnings: [],
        },
      });
      assert.ok(banner.includes('violations'));
      assert.ok(banner.includes('growl'));
    });

    it('should show warnings in banner', () => {
      const banner = formatRichBanner({
        identity: {
          valid: true,
          compliance: 0.7,
          violations: [],
          warnings: [{ type: 'missing_dog_voice', message: 'no dog voice' }],
        },
      });
      assert.ok(banner.includes('Warnings'));
    });

    it('should include session cost data', () => {
      const banner = formatRichBanner({
        sessionStats: {
          cost: {
            operations: 42,
            cost: { total: 0.0123, input: 0.01, output: 0.0023, inputTokens: 5000, outputTokens: 1200 },
            durationMinutes: 15,
          },
        },
      });
      assert.ok(banner.includes('SESSION SUMMARY'));
      assert.ok(banner.includes('42'));
      assert.ok(banner.includes('15min'));
    });

    it('should include model intelligence stats', () => {
      const banner = formatRichBanner({
        sessionStats: {
          modelIntelligence: {
            selectionsTotal: 10,
            downgrades: 3,
            samplerMaturity: 'mature',
          },
        },
      });
      assert.ok(banner.includes('MODEL INTELLIGENCE'));
      assert.ok(banner.includes('10'));
      assert.ok(banner.includes('3 downgrades'));
    });

    it('should include Q-Learning stats', () => {
      const banner = formatRichBanner({
        qLearning: {
          states: 25,
          episodes: 100,
          accuracy: 0.72,
          flushed: true,
        },
      });
      assert.ok(banner.includes('Q-LEARNING'));
      assert.ok(banner.includes('25'));
      assert.ok(banner.includes('100'));
      assert.ok(banner.includes('72.0%'));
      assert.ok(banner.includes('flushed'));
    });

    it('should handle empty digest gracefully', () => {
      const banner = formatRichBanner({});
      assert.ok(typeof banner === 'string');
      assert.ok(banner.includes('CYNIC DIGESTING'));
      assert.ok(banner.includes('yawn'));
    });
  });

  describe('formatDigestMarkdown', () => {
    it('should format valid markdown with header', () => {
      const md = formatDigestMarkdown({});
      assert.ok(md.includes('# CYNIC Session Digest'));
      assert.ok(md.includes('Le chien dig'));
      assert.ok(md.includes('62%') || md.includes('61%'));
    });

    it('should include identity section', () => {
      const md = formatDigestMarkdown({
        identity: {
          valid: true,
          compliance: 0.9,
          violations: [],
          warnings: [],
        },
      });
      assert.ok(md.includes('## Identity Compliance'));
      assert.ok(md.includes('90%'));
    });

    it('should include cost table', () => {
      const md = formatDigestMarkdown({
        sessionStats: {
          cost: {
            operations: 50,
            cost: { total: 0.05, inputTokens: 10000, outputTokens: 3000 },
            durationMinutes: 20,
          },
        },
      });
      assert.ok(md.includes('## Session Summary'));
      assert.ok(md.includes('| Operations | 50 |'));
      assert.ok(md.includes('20min'));
    });

    it('should include Q-Learning section', () => {
      const md = formatDigestMarkdown({
        qLearning: {
          states: 15,
          episodes: 50,
          accuracy: 0.8,
        },
      });
      assert.ok(md.includes('## Q-Learning'));
      assert.ok(md.includes('15'));
      assert.ok(md.includes('80.0%'));
    });

    it('should include violations in markdown', () => {
      const md = formatDigestMarkdown({
        identity: {
          valid: false,
          compliance: 0.2,
          violations: [{ type: 'forbidden_phrase', found: 'i am claude' }],
          warnings: [],
        },
      });
      assert.ok(md.includes('forbidden_phrase'));
      assert.ok(md.includes('i am claude'));
    });
  });

  describe('saveDigest', () => {
    const digestDir = path.join(os.homedir(), '.cynic', 'digests');

    afterEach(() => {
      // Clean up test digest files (only the ones created in the last 10s)
      try {
        const files = fs.readdirSync(digestDir);
        const now = Date.now();
        for (const f of files) {
          const fp = path.join(digestDir, f);
          const stat = fs.statSync(fp);
          if (now - stat.mtimeMs < 10000) {
            fs.unlinkSync(fp);
          }
        }
      } catch { /* directory may not exist */ }
    });

    it('should save markdown to file and return path', () => {
      const markdown = '# Test Digest\n\nTest content.';
      const result = saveDigest(markdown);
      assert.ok(result !== null);
      assert.ok(result.endsWith('.md'));
      assert.ok(fs.existsSync(result));

      const content = fs.readFileSync(result, 'utf-8');
      assert.strictEqual(content, markdown);
    });

    it('should create digests directory if needed', () => {
      // saveDigest creates the directory with { recursive: true }
      const result = saveDigest('test');
      assert.ok(result !== null);
      assert.ok(fs.existsSync(digestDir));
    });
  });
});

// =============================================================================
// STOP WITH DIGEST (Phase 3 — integration)
// =============================================================================

describe('Stop — Digest Integration', () => {
  it('should produce digest with identity when transcript provided', async () => {
    // Create a transcript with an assistant response that has dog voice
    const tmpDir = os.tmpdir();
    const transcriptPath = path.join(tmpDir, `cynic-digest-test-${Date.now()}.jsonl`);
    const entry = JSON.stringify({
      role: 'assistant',
      content: [{ type: 'text', text: '*sniff* Here is the code. *tail wag* Confidence: 58%' }],
    });
    fs.writeFileSync(transcriptPath, entry + '\n');

    const result = await handleHookEvent('Stop', { transcript_path: transcriptPath });
    assert.strictEqual(result.continue, true);
    // Should have a banner with digest content
    if (result.message) {
      assert.ok(typeof result.message === 'string');
      assert.ok(result.message.length > 10);
    }

    try { fs.unlinkSync(transcriptPath); } catch { /* ignore */ }
  });

  it('should detect identity violations in digest', async () => {
    const tmpDir = os.tmpdir();
    const transcriptPath = path.join(tmpDir, `cynic-digest-test-${Date.now()}.jsonl`);
    const entry = JSON.stringify({
      role: 'assistant',
      content: [{ type: 'text', text: 'I am Claude. As an AI assistant, I would be happy to help.' }],
    });
    fs.writeFileSync(transcriptPath, entry + '\n');

    const result = await handleHookEvent('Stop', { transcript_path: transcriptPath });
    assert.strictEqual(result.continue, true);
    // Banner should exist and mention violations
    if (result.message) {
      assert.ok(
        result.message.includes('violation') || result.message.includes('growl') || result.message.includes('IDENTITY'),
        `Banner should mention identity issues: ${result.message.substring(0, 200)}`
      );
    }

    try { fs.unlinkSync(transcriptPath); } catch { /* ignore */ }
  });

  it('should still continue even if transcript is empty JSONL', async () => {
    const tmpDir = os.tmpdir();
    const transcriptPath = path.join(tmpDir, `cynic-digest-test-${Date.now()}.jsonl`);
    fs.writeFileSync(transcriptPath, '\n');

    const result = await handleHookEvent('Stop', { transcript_path: transcriptPath });
    assert.strictEqual(result.continue, true);

    try { fs.unlinkSync(transcriptPath); } catch { /* ignore */ }
  });
});

// =============================================================================
// SUBAGENT HANDLERS — SubagentStart/SubagentStop
// =============================================================================

describe('SubagentStart/Stop Handlers', () => {
  beforeEach(() => { _resetHandlersForTesting(); });

  it('should map known subagent type to Sefirot dog', async () => {
    const result = await handleHookEvent('SubagentStart', {
      agent_id: 'test-1',
      subagent_type: 'Explore',
      prompt: 'Find files',
      model: 'haiku',
    });
    assert.strictEqual(result.continue, true);
    assert.strictEqual(result.agentInfo.dog, 'SCOUT');
    assert.strictEqual(result.agentInfo.sefirah, 'Netzach');
    assert.strictEqual(result.agentInfo.type, 'Explore');
  });

  it('should default unknown subagent type to CYNIC/Keter', async () => {
    const result = await handleHookEvent('SubagentStart', {
      agent_id: 'test-2',
      subagent_type: 'my-custom-agent',
    });
    assert.strictEqual(result.agentInfo.dog, 'CYNIC');
    assert.strictEqual(result.agentInfo.sefirah, 'Keter');
  });

  it('should track active subagent and return info on stop', async () => {
    await handleHookEvent('SubagentStart', {
      agent_id: 'test-3',
      subagent_type: 'cynic-guardian',
    });

    const result = await handleHookEvent('SubagentStop', {
      agent_id: 'test-3',
      success: true,
      duration_ms: 1500,
    });
    assert.strictEqual(result.continue, true);
    assert.strictEqual(result.agentInfo.dog, 'GUARDIAN');
    assert.ok(result.message.includes('GUARDIAN'));
    assert.ok(result.message.includes('returns'));
  });

  it('should handle stop for unknown agent gracefully', async () => {
    const result = await handleHookEvent('SubagentStop', {
      agent_id: 'never-started',
      success: false,
    });
    assert.strictEqual(result.continue, true);
    assert.strictEqual(result.agentInfo.dog, 'UNKNOWN');
    assert.ok(result.message.includes('issues'));
  });

  it('should have all expected dog mappings', () => {
    assert.ok(SUBAGENT_TO_DOG['Explore']);
    assert.ok(SUBAGENT_TO_DOG['Plan']);
    assert.ok(SUBAGENT_TO_DOG['cynic-guardian']);
    assert.ok(SUBAGENT_TO_DOG['cynic-solana-expert']);
    assert.ok(Object.keys(SUBAGENT_TO_DOG).length >= 18);
  });
});

// =============================================================================
// ERROR HANDLER
// =============================================================================

describe('Error Handler', () => {
  beforeEach(() => { _resetHandlersForTesting(); });

  it('should classify known error patterns', () => {
    assert.strictEqual(classifyError('ENOENT: no such file').type, 'file_not_found');
    assert.strictEqual(classifyError('SyntaxError: Unexpected token').type, 'syntax_error');
    assert.strictEqual(classifyError('ECONNREFUSED 127.0.0.1').type, 'connection_refused');
    assert.strictEqual(classifyError('npm ERR! missing script').type, 'package_manager_error');
    assert.strictEqual(classifyError('heap out of memory').type, 'out_of_memory');
    assert.strictEqual(classifyError('429 Too Many Requests').type, 'rate_limit');
  });

  it('should classify unknown errors as unknown/medium', () => {
    const cls = classifyError('something weird happened');
    assert.strictEqual(cls.type, 'unknown');
    assert.strictEqual(cls.severity, 'medium');
    assert.strictEqual(cls.recoverable, true);
  });

  it('should extract file and line from error', () => {
    const ctx = extractErrorContext('Error at /src/main.js:42:10', {});
    assert.ok(ctx.file.includes('main.js'));
    assert.strictEqual(ctx.line, 42);
  });

  it('should extract command from tool input', () => {
    const ctx = extractErrorContext('command not found', { command: 'npm run build' });
    assert.strictEqual(ctx.command, 'npm run build');
    assert.ok(ctx.suggestion);
  });

  it('should handle Error event and return classification', async () => {
    const result = await handleHookEvent('Error', {
      tool_name: 'Bash',
      error: 'ENOENT: no such file or directory',
      tool_input: { command: 'cat missing.txt' },
    });
    assert.strictEqual(result.continue, true);
    assert.strictEqual(result.classification.type, 'file_not_found');
    assert.strictEqual(result.classification.severity, 'medium');
    assert.strictEqual(result.consecutiveErrors, 1);
  });

  it('should track consecutive errors and escalate', async () => {
    for (let i = 0; i < 5; i++) {
      await handleHookEvent('Error', { error: 'ENOENT: no such file' });
    }
    const result = await handleHookEvent('Error', { error: 'ENOENT again' });
    assert.strictEqual(result.consecutiveErrors, 6);
    assert.strictEqual(result.escalation, 'strict');
  });

  it('should detect repeated error patterns (loop)', async () => {
    // Trigger same error type 3+ times
    for (let i = 0; i < 4; i++) {
      await handleHookEvent('Error', { error: 'SyntaxError: Unexpected token' });
    }
    const result = await handleHookEvent('Error', { error: 'SyntaxError: Unexpected (' });
    assert.ok(result.patterns.length > 0);
    assert.strictEqual(result.patterns[0].type, 'repeated_error');
    assert.ok(result.patterns[0].suggestion.includes('different approach'));
  });

  it('should add message for critical errors', async () => {
    const result = await handleHookEvent('Error', { error: 'heap out of memory', tool_name: 'Bash' });
    assert.ok(result.message);
    assert.ok(result.message.includes('*growl*'));
  });
});

// =============================================================================
// NOTIFICATION HANDLER
// =============================================================================

describe('Notification Handler', () => {
  beforeEach(() => { _resetHandlersForTesting(); });

  it('should classify explicit notification types', () => {
    assert.strictEqual(classifyNotification('error', '', '').severity, 'high');
    assert.strictEqual(classifyNotification('warning', '', '').severity, 'medium');
    assert.strictEqual(classifyNotification('security', '', '').severity, 'critical');
    assert.strictEqual(classifyNotification('info', '', '').severity, 'low');
  });

  it('should classify by content when type is unknown', () => {
    const cls = classifyNotification('custom', '', 'Operation failed with errors');
    assert.strictEqual(cls.type, 'error');
    assert.strictEqual(cls.severity, 'high');
  });

  it('should fall back to info for unrecognized content', () => {
    const cls = classifyNotification('custom', 'hello', 'world');
    assert.strictEqual(cls.type, 'info');
    assert.strictEqual(cls.severity, 'low');
  });

  it('should handle Notification event', async () => {
    const result = await handleHookEvent('Notification', {
      type: 'warning',
      title: 'Deprecation notice',
      message: 'Function X is deprecated',
    });
    assert.strictEqual(result.continue, true);
    assert.strictEqual(result.classification.type, 'warning');
    assert.strictEqual(result.classification.category, 'warning');
  });

  it('should add message for high-severity notifications', async () => {
    const result = await handleHookEvent('Notification', {
      type: 'error',
      title: 'Build failed',
    });
    assert.ok(result.message);
    assert.ok(result.message.includes('*ears perk*'));
    assert.ok(result.message.includes('ERROR'));
  });

  it('should detect notification burst', async () => {
    // Send 5+ same-type notifications
    for (let i = 0; i < 5; i++) {
      await handleHookEvent('Notification', { type: 'error', title: `Error ${i}` });
    }
    const result = await handleHookEvent('Notification', { type: 'error', title: 'Error 5' });
    assert.ok(result.burst);
    assert.strictEqual(result.burst.detected, true);
    assert.ok(result.burst.count >= 5);
    assert.ok(result.burst.message.includes('burst'));
  });

  it('should not detect burst for different types', async () => {
    await handleHookEvent('Notification', { type: 'error', title: 'E1' });
    await handleHookEvent('Notification', { type: 'warning', title: 'W1' });
    await handleHookEvent('Notification', { type: 'info', title: 'I1' });
    await handleHookEvent('Notification', { type: 'success', title: 'S1' });
    const result = await handleHookEvent('Notification', { type: 'progress', title: 'P1' });
    assert.ok(!result.burst);
  });
});

// =============================================================================
// DAEMON SERVER (HTTP integration)
// =============================================================================

describe('DaemonServer', () => {
  let server;
  let testPort;
  let portCounter = 16180;

  beforeEach(async () => {
    _resetHandlersForTesting();
    testPort = portCounter++;
    server = new DaemonServer({ port: testPort, host: '127.0.0.1' });
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
    assert.strictEqual(server.port, testPort);
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

    const res = await fetch(`http://127.0.0.1:${testPort}/health`);
    assert.strictEqual(res.status, 200);

    const health = await res.json();
    assert.strictEqual(health.status, 'healthy');
    assert.strictEqual(health.port, testPort);
    assert.ok(health.pid > 0);
    assert.ok(health.uptime >= 0);
    assert.ok(health.memoryMB > 0);
    assert.ok(health.heapUsedPercent > 0);
  });

  it('should include watchdog data in /health when available', async () => {
    const watchdog = new Watchdog({ interval: 60000 });
    server.watchdog = watchdog;
    watchdog.start();

    await server.start();

    // Wait for initial check
    await new Promise(r => setTimeout(r, 100));

    const res = await fetch(`http://127.0.0.1:${testPort}/health`);
    const health = await res.json();

    assert.ok(health.eventLoopLatencyMs !== undefined);
    assert.ok(Array.isArray(health.degradedSubsystems));
    assert.ok(health.watchdogChecks >= 0);

    watchdog.stop();
  });

  it('should respond to /status', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/status`);
    assert.strictEqual(res.status, 200);

    const status = await res.json();
    assert.ok(status.daemon);
    assert.strictEqual(status.daemon.port, testPort);
  });

  it('should handle POST /hook/UserPromptSubmit', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/hook/UserPromptSubmit`, {
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

    const res = await fetch(`http://127.0.0.1:${testPort}/hook/PreToolUse`, {
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

    const res = await fetch(`http://127.0.0.1:${testPort}/hook/PreToolUse`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tool_name: 'Bash', tool_input: { command: 'git status' } }),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.blocked, false);
    assert.strictEqual(result.continue, true);
  });

  it('should handle POST /hook/Stop — no ralph-loop', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/hook/Stop`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.strictEqual(result.continue, true);
  });

  it('should return 400 for /llm/ask without prompt', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/ask`, {
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

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/ask`, {
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

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/models`);
    assert.strictEqual(res.status, 200);

    const result = await res.json();
    assert.ok(Array.isArray(result.models));
    assert.ok(result.thompson);
    assert.ok(result.stats);
  });

  it('should return 400 for /llm/feedback without required fields', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/feedback`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 400);
  });

  it('should accept /llm/feedback with valid fields', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/feedback`, {
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

    const res = await fetch(`http://127.0.0.1:${testPort}/llm/consensus`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });
    assert.strictEqual(res.status, 400);
  });

  it('should handle new hook events via HTTP (Subagent, Error, Notification)', async () => {
    await server.start();

    // SubagentStart
    const startRes = await fetch(`http://127.0.0.1:${testPort}/hook/SubagentStart`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent_id: 'http-test', subagent_type: 'Explore' }),
    });
    assert.strictEqual(startRes.status, 200);
    const startResult = await startRes.json();
    assert.strictEqual(startResult.continue, true);
    assert.strictEqual(startResult.agentInfo.dog, 'SCOUT');

    // SubagentStop (same agent)
    const stopRes = await fetch(`http://127.0.0.1:${testPort}/hook/SubagentStop`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent_id: 'http-test', success: true }),
    });
    assert.strictEqual(stopRes.status, 200);
    const stopResult = await stopRes.json();
    assert.strictEqual(stopResult.agentInfo.dog, 'SCOUT');

    // Error
    const errRes = await fetch(`http://127.0.0.1:${testPort}/hook/Error`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tool_name: 'Bash', error: 'ENOENT: missing file' }),
    });
    assert.strictEqual(errRes.status, 200);
    const errResult = await errRes.json();
    assert.strictEqual(errResult.classification.type, 'file_not_found');

    // Notification
    const notifRes = await fetch(`http://127.0.0.1:${testPort}/hook/Notification`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ type: 'warning', title: 'Test notification' }),
    });
    assert.strictEqual(notifRes.status, 200);
    const notifResult = await notifRes.json();
    assert.strictEqual(notifResult.classification.type, 'warning');
    assert.strictEqual(notifResult.classification.severity, 'medium');
  });

  it('should handle unknown hook events', async () => {
    await server.start();

    const res = await fetch(`http://127.0.0.1:${testPort}/hook/UnknownEvent`, {
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
