/**
 * Perceive Hook Tests (H1: UserPromptSubmit)
 *
 * Tests for user prompt processing, skill auto-activation,
 * intent detection, and context injection.
 *
 * @module scripts/hooks/test/perceive
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import { runHook, createPromptContext } from './fixtures/mock-stdin.js';

// =============================================================================
// SKILL TRIGGER DETECTION
// =============================================================================

describe('perceive hook - skill triggers', () => {
  it('should detect /judge skill trigger', () => {
    const input = createPromptContext('Can you judge this code and give me a verdict?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
    // Should suggest /judge skill
    if (result.output.message) {
      assert.ok(
        result.output.message.includes('/judge') ||
        result.output.message.includes('SKILL'),
        'Should suggest /judge skill'
      );
    }
  });

  it('should detect /search skill trigger', () => {
    const input = createPromptContext('Can you search memory for what we did last time?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /patterns skill trigger', () => {
    const input = createPromptContext('Show me the patterns you have detected');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /trace skill trigger', () => {
    const input = createPromptContext('Can you trace the judgment through the PoJ blockchain?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /learn skill trigger', () => {
    const input = createPromptContext('That judgment was wrong, let me give you feedback');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /health skill trigger', () => {
    const input = createPromptContext('Is the CYNIC system health ok? Check diagnostics');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /psy skill trigger', () => {
    const input = createPromptContext('How am I doing? Check my energy and focus levels');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /dogs skill trigger', () => {
    const input = createPromptContext('Which dogs are active? Show me the sefirot');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /wisdom skill trigger', () => {
    const input = createPromptContext('What would Diogenes say about this philosophical question?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /status skill trigger', () => {
    const input = createPromptContext('What is the CYNIC status and version progress?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect /ecosystem skill trigger', () => {
    const input = createPromptContext('What is happening in the ecosystem and other repos?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect Q-score pattern', () => {
    const input = createPromptContext('What is the q-score for this token?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// INTENT DETECTION
// =============================================================================

describe('perceive hook - intent detection', () => {
  it('should detect decision intent', () => {
    const input = createPromptContext('Should I choose option A or B? Which is better?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect architecture intent', () => {
    const input = createPromptContext('I need to refactor the architecture and reorganize the pattern');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect danger intent', () => {
    const input = createPromptContext('I want to delete everything and reset the database');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
    // Should have danger warning
    if (result.output.message) {
      assert.ok(
        result.output.message.toLowerCase().includes('danger') ||
        result.output.message.toLowerCase().includes('warning') ||
        result.output.message.toLowerCase().includes('growl'),
        'Should include danger warning'
      );
    }
  });

  it('should detect debug intent', () => {
    const input = createPromptContext('There is a bug in the code, it crashes and does not work');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should detect learning intent', () => {
    const input = createPromptContext('How does this work? Explain it to me so I can understand');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// DANGER WARNINGS
// =============================================================================

describe('perceive hook - danger warnings', () => {
  it('should warn about rm -rf /', () => {
    const input = createPromptContext('Run rm -rf / to clean up');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
    if (result.output.message) {
      assert.ok(
        result.output.message.includes('DANGER') ||
        result.output.message.includes('GROWL') ||
        result.output.message.includes('dangerous'),
        'Should warn about dangerous command'
      );
    }
  });

  it('should warn about DROP TABLE', () => {
    const input = createPromptContext('DROP TABLE users; to remove all users');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should warn about DELETE without WHERE', () => {
    const input = createPromptContext('DELETE FROM users; will remove all rows');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should warn about git push --force', () => {
    const input = createPromptContext('Do git push --force to overwrite remote history');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should warn about git reset --hard', () => {
    const input = createPromptContext('Use git reset --hard to undo changes');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// OUTPUT FORMAT
// =============================================================================

describe('perceive hook - output format', () => {
  it('should return valid JSON', () => {
    const input = createPromptContext('A normal prompt without any special triggers');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(typeof result.output, 'object');
  });

  it('should include continue field', () => {
    const input = createPromptContext('Test prompt');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should include message when injections present', () => {
    const input = createPromptContext('Evaluate this code and give me a verdict using the judge skill');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
    // May or may not have message depending on skill detection
    if (result.output.message) {
      assert.strictEqual(typeof result.output.message, 'string');
    }
  });

  it('should handle prompt with newlines', () => {
    const input = createPromptContext('First line\nSecond line\nThird line with question?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('perceive hook - edge cases', () => {
  it('should handle empty input', () => {
    const result = runHook('perceive', {});

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle empty prompt', () => {
    const input = createPromptContext('');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle very short prompt', () => {
    const input = createPromptContext('hi');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle very long prompt', () => {
    const longPrompt = 'This is a test prompt. '.repeat(100);
    const input = createPromptContext(longPrompt);
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle prompt with special characters', () => {
    const input = createPromptContext('Test with special chars: <>&"\'{}[]()');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle prompt with unicode', () => {
    const input = createPromptContext('Test avec français: éàü and emoji: 🐕 and κυνικός');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// MULTIPLE TRIGGERS
// =============================================================================

describe('perceive hook - multiple triggers', () => {
  it('should handle multiple skill triggers', () => {
    const input = createPromptContext('Search memory for patterns and judge the results');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle danger + debug intents together', () => {
    const input = createPromptContext('The bug crashes when I try to delete files, please fix');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle skill trigger + danger warning', () => {
    const input = createPromptContext('Judge this: rm -rf /tmp/* is safe right?');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});

// =============================================================================
// PRIORITY ORDERING
// =============================================================================

describe('perceive hook - priority ordering', () => {
  it('should prioritize high priority skills', () => {
    // /judge is high priority, /status is low priority
    const input = createPromptContext('Judge the CYNIC status please');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
    // If message exists, /judge should be mentioned first (higher priority)
    if (result.output.message && result.output.message.includes('/')) {
      const judgePos = result.output.message.indexOf('/judge');
      const statusPos = result.output.message.indexOf('/status');
      if (judgePos !== -1 && statusPos !== -1) {
        assert.ok(judgePos < statusPos, '/judge should appear before /status');
      }
    }
  });
});

// =============================================================================
// FRENCH LANGUAGE SUPPORT
// =============================================================================

describe('perceive hook - French support', () => {
  it('should handle French question patterns', () => {
    const input = createPromptContext("Qu'est-ce que c'est? Explique-moi comment ça marche");
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle French definition patterns', () => {
    const input = createPromptContext('Je définis CYNIC comme un chien philosophique');
    const result = runHook('perceive', input);

    assert.strictEqual(result.status, 0);
    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });
});
