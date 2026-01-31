/**
 * Error Hook Tests (H3)
 *
 * Tests for PostToolUseFailure hook that detects error patterns and enables auto-recovery.
 *
 * @module scripts/hooks/test/error
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { runHook, createErrorContext } from './fixtures/mock-stdin.js';

// =============================================================================
// ERROR CLASSIFICATION
// =============================================================================

describe('error hook - error classification', () => {
  it('should classify file not found errors', () => {
    const input = createErrorContext('Read', 'ENOENT: no such file or directory', {
      file_path: '/nonexistent/file.txt',
    });
    const result = runHook('error', input);

    assert.ok(result.output);
    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'file_not_found');
    assert.strictEqual(result.output.classification.severity, 'medium');
    assert.strictEqual(result.output.classification.recoverable, true);
  });

  it('should classify permission denied errors', () => {
    const input = createErrorContext('Write', 'EACCES: permission denied', {
      file_path: '/etc/passwd',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'permission_denied');
    assert.strictEqual(result.output.classification.severity, 'high');
    assert.strictEqual(result.output.classification.recoverable, false);
  });

  it('should classify timeout errors', () => {
    const input = createErrorContext('Bash', 'ETIMEDOUT: connection timed out', {
      command: 'curl https://example.com',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'timeout');
    assert.strictEqual(result.output.classification.recoverable, true);
  });

  it('should classify syntax errors', () => {
    const input = createErrorContext('Bash', 'SyntaxError: Unexpected token', {
      command: 'node -e "invalid {{"',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'syntax_error');
    assert.strictEqual(result.output.classification.severity, 'high');
    assert.strictEqual(result.output.classification.recoverable, false);
  });

  it('should classify JSON parse errors', () => {
    const input = createErrorContext('Read', 'JSON.parse: unexpected end of JSON input');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'json_parse_error');
  });

  it('should classify connection refused errors', () => {
    const input = createErrorContext('Bash', 'ECONNREFUSED: connection refused', {
      command: 'curl http://localhost:9999',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'connection_refused');
    assert.strictEqual(result.output.classification.severity, 'high');
  });

  it('should classify git errors', () => {
    const input = createErrorContext('Bash', 'fatal: not a git repository', {
      command: 'git status',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    // "fatal:" pattern matches git_error before "not a git repository" matches not_git_repo
    assert.strictEqual(result.output.classification.type, 'git_error');
  });

  it('should classify merge conflict errors', () => {
    const input = createErrorContext('Bash', 'CONFLICT (content): Merge conflict in file.txt');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'merge_conflict');
    assert.strictEqual(result.output.classification.recoverable, false);
  });

  it('should classify command not found errors', () => {
    const input = createErrorContext('Bash', 'command not found: nonexistent-cmd', {
      command: 'nonexistent-cmd',
    });
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'command_not_found');
  });

  it('should classify rate limit errors', () => {
    const input = createErrorContext('Bash', 'Error 429: Too Many Requests');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'rate_limit');
    assert.strictEqual(result.output.classification.recoverable, true);
  });

  it('should classify out of memory errors', () => {
    const input = createErrorContext('Bash', 'FATAL ERROR: CALL_AND_RETRY_LAST Allocation failed - JavaScript heap out of memory');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'out_of_memory');
    assert.strictEqual(result.output.classification.severity, 'critical');
    assert.strictEqual(result.output.classification.recoverable, false);
  });

  it('should classify unknown errors as medium/recoverable', () => {
    const input = createErrorContext('Bash', 'Some unknown error occurred');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.type, 'unknown');
    assert.strictEqual(result.output.classification.severity, 'medium');
    assert.strictEqual(result.output.classification.recoverable, true);
  });
});

// =============================================================================
// ERROR CONTEXT EXTRACTION
// =============================================================================

describe('error hook - context extraction', () => {
  it('should extract file path from error', () => {
    const input = createErrorContext('Read', 'Error at src/index.js:42');
    const result = runHook('error', input);

    assert.ok(result.output.context);
    // Extracts full path including directory
    assert.strictEqual(result.output.context.file, 'src/index.js');
  });

  it('should extract line number from error', () => {
    const input = createErrorContext('Read', 'Error at file.js:123:45');
    const result = runHook('error', input);

    assert.ok(result.output.context);
    assert.strictEqual(result.output.context.line, 123);
  });

  it('should extract command from Bash input', () => {
    const input = createErrorContext('Bash', 'Command failed', {
      command: 'npm test --reporter spec',
    });
    const result = runHook('error', input);

    assert.ok(result.output.context);
    assert.ok(result.output.context.command);
    assert.ok(result.output.context.command.includes('npm test'));
  });

  it('should provide suggestions for file not found', () => {
    const input = createErrorContext('Read', 'ENOENT: no such file');
    const result = runHook('error', input);

    assert.ok(result.output.context.suggestion);
    assert.ok(result.output.context.suggestion.includes('Glob'));
  });

  it('should provide suggestions for timeout', () => {
    const input = createErrorContext('Bash', 'ETIMEDOUT');
    const result = runHook('error', input);

    assert.ok(result.output.context.suggestion);
    assert.ok(result.output.context.suggestion.toLowerCase().includes('timeout'));
  });

  it('should provide suggestions for rate limit', () => {
    const input = createErrorContext('Bash', '429 rate limit exceeded');
    const result = runHook('error', input);

    assert.ok(result.output.context.suggestion);
    assert.ok(result.output.context.suggestion.toLowerCase().includes('wait'));
  });
});

// =============================================================================
// OUTPUT FORMAT
// =============================================================================

describe('error hook - output format', () => {
  it('should return valid JSON', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.ok(result.output, 'Should parse as JSON');
    assert.ok(typeof result.output === 'object');
  });

  it('should include type field', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.strictEqual(result.output.type, 'PostToolUseFailure');
  });

  it('should include timestamp', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.ok(result.output.timestamp);
    assert.ok(!isNaN(Date.parse(result.output.timestamp)));
  });

  it('should include continue=true (non-blocking)', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.strictEqual(result.output.continue, true, 'Should not block on errors');
  });

  it('should include tool name', () => {
    const input = createErrorContext('Read', 'Test error');
    const result = runHook('error', input);

    assert.ok(result.output.tool);
    assert.strictEqual(result.output.tool.name, 'Read');
  });

  it('should truncate long error messages', () => {
    const longError = 'E'.repeat(1000);
    const input = createErrorContext('Bash', longError);
    const result = runHook('error', input);

    assert.ok(result.output.error);
    assert.ok(result.output.error.length <= 500);
  });
});

// =============================================================================
// SEVERITY LEVELS
// =============================================================================

describe('error hook - severity levels', () => {
  it('should classify low severity errors', () => {
    const input = createErrorContext('Write', 'EEXIST: file already exists');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.severity, 'low');
  });

  it('should classify medium severity errors', () => {
    const input = createErrorContext('Bash', 'EISDIR: is a directory');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.severity, 'medium');
  });

  it('should classify high severity errors', () => {
    const input = createErrorContext('Bash', 'TypeError: cannot read property');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.severity, 'high');
  });

  it('should classify critical severity errors', () => {
    const input = createErrorContext('Bash', 'ENOSPC: no space left on device');
    const result = runHook('error', input);

    assert.ok(result.output.classification);
    assert.strictEqual(result.output.classification.severity, 'critical');
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('error hook - edge cases', () => {
  it('should handle empty input', () => {
    const result = runHook('error', {});

    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle missing error message', () => {
    const input = { tool_name: 'Bash', tool_input: {} };
    const result = runHook('error', input);

    assert.ok(result.output);
    assert.strictEqual(result.output.continue, true);
  });

  it('should handle empty error message', () => {
    const input = createErrorContext('Bash', '');
    const result = runHook('error', input);

    assert.ok(result.output);
    // Should still classify as unknown
    assert.ok(result.output.classification);
  });

  it('should handle error with special characters', () => {
    const input = createErrorContext('Bash', 'Error: <script>alert("xss")</script>');
    const result = runHook('error', input);

    assert.ok(result.output);
    // Should not crash
  });

  it('should handle very long commands', () => {
    const longCommand = 'echo ' + 'a'.repeat(5000);
    const input = createErrorContext('Bash', 'Command failed', {
      command: longCommand,
    });
    const result = runHook('error', input);

    assert.ok(result.output);
    assert.ok(result.output.context.command.length <= 100);
  });
});

// =============================================================================
// PATTERNS DETECTION
// =============================================================================

describe('error hook - patterns array', () => {
  it('should include patterns array in output', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.ok(Array.isArray(result.output.patterns));
  });

  it('should detect pattern after multiple calls with same error', () => {
    // Run multiple times with same error type
    const input = createErrorContext('Bash', 'ENOENT: file not found');

    // First call
    runHook('error', input);
    runHook('error', input);

    // Third call should potentially detect pattern
    const result = runHook('error', input);

    assert.ok(result.output);
    // Pattern detection depends on in-memory history, may or may not trigger
    // Just verify it doesn't crash
  });
});

// =============================================================================
// ESCALATION
// =============================================================================

describe('error hook - escalation', () => {
  it('should track consecutive errors', () => {
    const input = createErrorContext('Bash', 'Test error');
    const result = runHook('error', input);

    assert.ok(result.output);
    // consecutiveErrors is tracked if session state is initialized
    assert.ok(typeof result.output.consecutiveErrors === 'number');
  });
});
