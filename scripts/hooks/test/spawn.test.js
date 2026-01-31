/**
 * Spawn Hook Tests (H2)
 *
 * Tests for SubagentStart/SubagentStop hooks that coordinate the collective.
 *
 * @module scripts/hooks/test/spawn
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { runHook, createSubagentContext } from './fixtures/mock-stdin.js';

// =============================================================================
// SUBAGENT START
// =============================================================================

describe('spawn hook - SubagentStart', () => {
  it('should handle SubagentStart event', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'Explore',
      prompt: 'Find all test files',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output, 'Should return JSON output');
    assert.strictEqual(result.output.continue, true, 'Should continue');
    assert.strictEqual(result.output.event, 'SubagentStart');
  });

  it('should map Explore agent to Scout dog', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'Explore',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'SCOUT');
    assert.strictEqual(result.output.agentInfo.sefirah, 'Netzach');
    assert.strictEqual(result.output.agentInfo.icon, '🔍');
  });

  it('should map Plan agent to Architect dog', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'Plan',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'ARCHITECT');
    assert.strictEqual(result.output.agentInfo.sefirah, 'Chesed');
  });

  it('should map Bash agent to Cartographer dog', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'Bash',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'CARTOGRAPHER');
    assert.strictEqual(result.output.agentInfo.sefirah, 'Malkhut');
  });

  it('should map general-purpose to CYNIC', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'general-purpose',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'CYNIC');
    assert.strictEqual(result.output.agentInfo.sefirah, 'Keter');
  });

  it('should handle custom CYNIC agents', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'cynic-guardian',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'GUARDIAN');
    assert.strictEqual(result.output.agentInfo.sefirah, 'Gevurah');
  });

  it('should default unknown agents to CYNIC', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'unknown-agent-type',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.agentInfo);
    assert.strictEqual(result.output.agentInfo.dog, 'CYNIC');
  });

  it('should record agent info with startTime', () => {
    const input = createSubagentContext('SubagentStart');
    const result = runHook('spawn', input);

    assert.ok(result.output.agentInfo);
    assert.ok(result.output.agentInfo.startTime);
    assert.ok(result.output.agentInfo.id);
  });

  it('should include message about dispatch', () => {
    const input = createSubagentContext('SubagentStart', {
      subagent_type: 'Explore',
    });
    const result = runHook('spawn', input);

    assert.ok(result.output.message);
    assert.ok(result.output.message.includes('SCOUT'));
    assert.ok(result.output.message.includes('dispatched'));
  });
});

// =============================================================================
// SUBAGENT STOP
// =============================================================================

describe('spawn hook - SubagentStop', () => {
  it('should handle SubagentStop event', () => {
    const input = createSubagentContext('SubagentStop', {
      subagent_type: 'Explore',
      success: true,
      duration_ms: 2500,
    });
    const result = runHook('spawn', input);

    assert.ok(result.output, 'Should return JSON output');
    assert.strictEqual(result.output.continue, true, 'Should continue');
    assert.strictEqual(result.output.event, 'SubagentStop');
  });

  it('should handle successful completion', () => {
    const input = createSubagentContext('SubagentStop', {
      success: true,
      duration_ms: 1000,
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.message);
    assert.ok(result.output.message.includes('returns'));
  });

  it('should handle failed completion', () => {
    const input = createSubagentContext('SubagentStop', {
      success: false,
      duration_ms: 500,
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.message);
    assert.ok(result.output.message.includes('issues') || result.output.message.includes('failed'));
  });

  it('should include agent info in stop event', () => {
    const input = createSubagentContext('SubagentStop', {
      subagent_type: 'Plan',
      success: true,
    });
    const result = runHook('spawn', input);

    assert.ok(result.output.agentInfo);
    // Note: Since each hook run is a separate process, activeSubagents map is empty
    // The stop handler falls back to UNKNOWN when agent wasn't tracked
    // This is expected behavior for isolated hook testing
    assert.ok(result.output.agentInfo.dog);
  });

  it('should handle lowercase event type', () => {
    const input = createSubagentContext('subagent_stop', {
      success: true,
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// SEFIROT MAPPING
// =============================================================================

describe('spawn hook - Sefirot mappings', () => {
  const sefirotMappings = [
    { agent: 'cynic-analyst', dog: 'ANALYST', sefirah: 'Binah' },
    { agent: 'cynic-sage', dog: 'SAGE', sefirah: 'Chochmah' },
    { agent: 'cynic-scholar', dog: 'SCHOLAR', sefirah: 'Daat' },
    { agent: 'cynic-oracle', dog: 'ORACLE', sefirah: 'Tiferet' },
    { agent: 'cynic-deployer', dog: 'DEPLOYER', sefirah: 'Hod' },
    { agent: 'cynic-janitor', dog: 'JANITOR', sefirah: 'Yesod' },
  ];

  for (const { agent, dog, sefirah } of sefirotMappings) {
    it(`should map ${agent} to ${dog} (${sefirah})`, () => {
      const input = createSubagentContext('SubagentStart', {
        subagent_type: agent,
      });
      const result = runHook('spawn', input);

      assert.ok(result.output.agentInfo);
      assert.strictEqual(result.output.agentInfo.dog, dog);
      assert.strictEqual(result.output.agentInfo.sefirah, sefirah);
    });
  }
});

// =============================================================================
// OUTPUT FORMAT
// =============================================================================

describe('spawn hook - output format', () => {
  it('should return valid JSON', () => {
    const input = createSubagentContext('SubagentStart');
    const result = runHook('spawn', input);

    assert.ok(result.output, 'Should parse as JSON');
    assert.ok(typeof result.output === 'object', 'Should be an object');
  });

  it('should include type field', () => {
    const input = createSubagentContext('SubagentStart');
    const result = runHook('spawn', input);

    assert.strictEqual(result.output.type, 'Subagent');
  });

  it('should include timestamp', () => {
    const input = createSubagentContext('SubagentStart');
    const result = runHook('spawn', input);

    assert.ok(result.output.timestamp);
    assert.ok(!isNaN(Date.parse(result.output.timestamp)));
  });

  it('should include continue field', () => {
    const input = createSubagentContext('SubagentStart');
    const result = runHook('spawn', input);

    assert.ok(typeof result.output.continue === 'boolean');
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('spawn hook - edge cases', () => {
  it('should handle empty input', () => {
    const result = runHook('spawn', {});

    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle missing agent_id', () => {
    const input = createSubagentContext('SubagentStart');
    delete input.agent_id;
    const result = runHook('spawn', input);

    assert.ok(result.output);
    // Should generate agent_id
    assert.ok(result.output.agentInfo.id);
  });

  it('should handle unknown event type', () => {
    const input = { event_type: 'UnknownEvent' };
    const result = runHook('spawn', input);

    assert.ok(result.output);
    assert.ok(result.output.message.includes('Unknown'));
  });

  it('should handle very long prompt', () => {
    const input = createSubagentContext('SubagentStart', {
      prompt: 'a'.repeat(10000),
    });
    const result = runHook('spawn', input);

    assert.ok(result.output);
    // Should not crash
    assert.ok(result.output.agentInfo.promptLength > 0);
  });
});
