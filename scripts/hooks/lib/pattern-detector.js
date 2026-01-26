/**
 * Pattern Detector - Tool Usage Pattern Analysis
 *
 * Extracted from observe.js for code reuse and clarity.
 * Detects patterns in tool usage for learning and improvement.
 *
 * "Le chien observe" - κυνικός
 *
 * @module scripts/hooks/lib/pattern-detector
 */

'use strict';

import path from 'path';

// ═══════════════════════════════════════════════════════════════════════════
// PATTERN DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Detect patterns in tool usage
 *
 * @param {string} toolName - Name of the tool
 * @param {Object} toolInput - Tool input parameters
 * @param {*} toolOutput - Tool output
 * @param {boolean} isError - Whether the tool execution errored
 * @returns {Object[]} Detected patterns
 */
export function detectToolPattern(toolName, toolInput, toolOutput, isError) {
  const patterns = [];

  // Error patterns
  if (isError) {
    const errorText = typeof toolOutput === 'string' ? toolOutput :
                      toolOutput?.error || toolOutput?.message || '';

    patterns.push(...detectErrorPatterns(errorText, toolName));
  }

  // Tool usage patterns
  patterns.push({
    type: 'tool_usage',
    signature: toolName,
    description: `${toolName} usage`,
    context: {
      success: !isError,
      inputSize: JSON.stringify(toolInput).length
    }
  });

  // File extension patterns (for Write/Edit)
  if ((toolName === 'Write' || toolName === 'Edit') && toolInput.file_path) {
    const ext = path.extname(toolInput.file_path || toolInput.filePath || '');
    if (ext) {
      patterns.push({
        type: 'language_usage',
        signature: ext,
        description: `Working with ${ext} files`
      });
    }
  }

  // Git patterns
  if (toolName === 'Bash') {
    const command = toolInput.command || '';
    patterns.push(...detectGitPatterns(command));
  }

  return patterns;
}

/**
 * Detect error patterns from error text
 * @private
 */
function detectErrorPatterns(errorText, toolName) {
  const patterns = [];

  if (errorText.includes('ENOENT')) {
    patterns.push({
      type: 'error',
      signature: 'file_not_found',
      description: 'File not found error',
      context: { tool: toolName }
    });
  } else if (errorText.includes('EACCES') || errorText.includes('Permission denied')) {
    patterns.push({
      type: 'error',
      signature: 'permission_denied',
      description: 'Permission denied error',
      context: { tool: toolName }
    });
  } else if (errorText.includes('ECONNREFUSED')) {
    patterns.push({
      type: 'error',
      signature: 'connection_refused',
      description: 'Connection refused - service not running?',
      context: { tool: toolName }
    });
  } else if (errorText.includes('SyntaxError')) {
    patterns.push({
      type: 'error',
      signature: 'syntax_error',
      description: 'Syntax error in code',
      context: { tool: toolName }
    });
  } else if (errorText.includes('TypeError')) {
    patterns.push({
      type: 'error',
      signature: 'type_error',
      description: 'Type error - check variable types',
      context: { tool: toolName }
    });
  }

  return patterns;
}

/**
 * Detect git-related patterns
 * @private
 */
function detectGitPatterns(command) {
  const patterns = [];

  if (command.startsWith('git ')) {
    const gitCmd = command.split(' ')[1];
    patterns.push({
      type: 'git_operation',
      signature: gitCmd,
      description: `Git ${gitCmd} operation`
    });
  }

  return patterns;
}

// ═══════════════════════════════════════════════════════════════════════════
// TRIGGER EVENT MAPPING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Map tool name to trigger event type
 *
 * @param {string} toolName - Tool name
 * @param {boolean} isError - Whether errored
 * @param {Object} toolInput - Tool input
 * @returns {string} Event type
 */
export function mapToTriggerEventType(toolName, isError, toolInput) {
  // Error always maps to ERROR
  if (isError) {
    return 'ERROR';
  }

  // Test events
  if (toolName === 'Bash') {
    const command = toolInput.command || '';
    if (command.match(/npm\s+(run\s+)?test|jest|vitest|mocha|pytest|cargo\s+test|go\s+test/i)) {
      return 'TEST_COMPLETE';
    }
  }

  // Git events
  if (toolName === 'Bash') {
    const command = toolInput.command || '';
    if (command.startsWith('git commit')) return 'COMMIT';
    if (command.startsWith('git push')) return 'PUSH';
    if (command.startsWith('git merge')) return 'MERGE';
  }

  // Code change events
  if (toolName === 'Write' || toolName === 'Edit') {
    return 'CODE_CHANGE';
  }

  // Default: generic tool use
  return 'TOOL_USE';
}

// ═══════════════════════════════════════════════════════════════════════════
// OUTPUT PARSING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Parse test output to extract pass/fail counts
 *
 * @param {string} output - Test output
 * @param {boolean} isError - Whether errored
 * @returns {Object} Test result
 */
export function parseTestOutput(output, isError) {
  let passed = !isError;
  let passCount = 0;
  let failCount = 0;
  let testSuite = 'unknown';

  // Jest/Vitest format
  const jestMatch = output.match(/Tests?:\s*(\d+)\s*passed(?:,\s*(\d+)\s*failed)?/i);
  if (jestMatch) {
    passCount = parseInt(jestMatch[1], 10) || 0;
    failCount = parseInt(jestMatch[2], 10) || 0;
    testSuite = output.includes('vitest') ? 'vitest' : 'jest';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Mocha format
  const mochaMatch = output.match(/(\d+)\s*passing(?:.*?(\d+)\s*failing)?/i);
  if (mochaMatch) {
    passCount = parseInt(mochaMatch[1], 10) || 0;
    failCount = parseInt(mochaMatch[2], 10) || 0;
    testSuite = 'mocha';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Node.js test runner format
  const nodeMatch = output.match(/(?:tests|ok)\s+(\d+)(?:.*?fail(?:ed)?\s+(\d+))?/i);
  if (nodeMatch) {
    passCount = parseInt(nodeMatch[1], 10) || 0;
    failCount = parseInt(nodeMatch[2], 10) || 0;
    testSuite = 'node';
    passed = failCount === 0;
    return { passed, passCount, failCount, testSuite };
  }

  // Fallback based on error state
  if (isError) {
    failCount = 1;
    passed = false;
  } else if (output.includes('PASS') || output.includes('passed') || output.includes('ok')) {
    passCount = 1;
    passed = true;
  }

  return { passed, passCount, failCount, testSuite };
}

/**
 * Extract commit hash from git output
 *
 * @param {string} output - Git commit output
 * @returns {string|null} Commit hash
 */
export function extractCommitHash(output) {
  if (!output) return null;
  // Match: [branch abc1234] or abc1234
  const match = output.match(/\[[\w\-\/]+\s+([a-f0-9]{7,})\]|^([a-f0-9]{40})$/im);
  return match ? (match[1] || match[2]) : null;
}

/**
 * Extract commit message from git commit command
 *
 * @param {string} command - Git commit command
 * @returns {string} Commit message
 */
export function extractCommitMessage(command) {
  if (!command) return '';
  const match = command.match(/-m\s+["']([^"']+)["']/);
  return match ? match[1] : '';
}

/**
 * Extract error summary from tool output
 *
 * @param {*} output - Tool output
 * @returns {string} Error summary
 */
export function extractErrorSummary(output) {
  if (typeof output === 'string') {
    // First 200 chars of error
    return output.slice(0, 200);
  }
  return output?.error || output?.message || 'Unknown error';
}

/**
 * Detect error type from error text
 *
 * @param {string} errorText - Error text
 * @returns {string} Error type
 */
export function detectErrorType(errorText) {
  if (!errorText) return 'unknown';

  const lower = errorText.toLowerCase();

  if (lower.includes('enoent') || lower.includes('no such file')) return 'file_not_found';
  if (lower.includes('eacces') || lower.includes('permission denied')) return 'permission';
  if (lower.includes('econnrefused')) return 'connection';
  if (lower.includes('timeout')) return 'timeout';
  if (lower.includes('syntaxerror')) return 'syntax';
  if (lower.includes('typeerror')) return 'type';
  if (lower.includes('referenceerror')) return 'reference';
  if (lower.includes('eslint') || lower.includes('lint')) return 'lint';
  if (lower.includes('test') && lower.includes('fail')) return 'test_failure';
  if (lower.includes('build') && lower.includes('fail')) return 'build_failure';

  return 'generic';
}

export default {
  detectToolPattern,
  mapToTriggerEventType,
  parseTestOutput,
  extractCommitHash,
  extractCommitMessage,
  extractErrorSummary,
  detectErrorType,
};
