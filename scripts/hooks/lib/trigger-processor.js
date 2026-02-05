/**
 * Trigger Processor
 *
 * "Le chien déclenche les réflexes" - Process tool events through the Trigger system
 *
 * Maps tool events to trigger types and fires async requests for pattern recording,
 * learning feedback, and test/commit/build detection.
 *
 * @module scripts/hooks/lib/trigger-processor
 */

'use strict';

import {
  callBrainTool,
  sendTestFeedback,
  sendCommitFeedback,
  sendBuildFeedback,
} from '../../lib/index.js';

import { antiPatternState } from './anti-pattern-detector.js';

/**
 * Map tool names and outcomes to trigger event types
 *
 * @param {string} toolName - Name of the tool
 * @param {boolean} isError - Whether the tool errored
 * @param {Object} toolInput - Tool input parameters
 * @returns {string} The trigger event type
 */
export function mapToTriggerEventType(toolName, isError, toolInput) {
  // Error events
  if (isError) {
    return 'TOOL_ERROR';
  }

  // Git events via Bash
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

/**
 * Parse test output to extract pass/fail counts
 *
 * @param {string} output - Test command output
 * @param {boolean} isError - Whether the command errored
 * @returns {Object} Test result with passed, passCount, failCount, testSuite
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
 * @returns {string|null} Commit hash or null
 */
export function extractCommitHash(output) {
  if (!output) return null;
  // Match: [branch abc1234] or abc1234
  const match = output.match(/\[[\w\-/]+\s+([a-f0-9]{7,})\]|^([a-f0-9]{40})$/im);
  return match ? (match[1] || match[2]) : null;
}

/**
 * Extract commit message from git commit command
 *
 * @param {string} command - Git commit command
 * @returns {string} Commit message or empty string
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
 * Process tool event through the Trigger system
 * Non-blocking - fires async request
 *
 * @param {string} toolName - Name of the tool
 * @param {Object} toolInput - Tool input parameters
 * @param {*} toolOutput - Tool output
 * @param {boolean} isError - Whether the tool errored
 */
export function processTriggerEvent(toolName, toolInput, toolOutput, isError) {
  const eventType = mapToTriggerEventType(toolName, isError, toolInput);

  const event = {
    type: eventType,
    source: toolName,
    data: {
      tool: toolName,
      success: !isError,
      inputSize: JSON.stringify(toolInput).length,
      // Include relevant context based on event type
      ...(eventType === 'COMMIT' && { message: extractCommitMessage(toolInput.command) }),
      ...(eventType === 'CODE_CHANGE' && { file: toolInput.file_path || toolInput.filePath }),
      ...(isError && { error: extractErrorSummary(toolOutput) }),
    },
    timestamp: Date.now(),
  };

  // Fire async request to process triggers (non-blocking)
  callBrainTool('brain_triggers', {
    action: 'process',
    event,
  }).catch(() => {
    // Silently ignore errors - triggers should never block hooks
  });

  // Record pattern to brain memory (non-blocking)
  callBrainTool('brain_patterns', {
    action: 'record',
    pattern: {
      type: eventType.toLowerCase(),
      tool: toolName,
      success: !isError,
      timestamp: Date.now(),
    },
  }).catch(() => {
    // Silently ignore - pattern recording is optional
  });

  // ==========================================================================
  // LEARNING FEEDBACK - External validation (Ralph-inspired)
  // Links feedback to most recent judgment for training pipeline persistence.
  // Without judgmentId, feedback goes to LearningService (memory) but NOT to
  // PostgreSQL feedback table (which requires judgment_id NOT NULL).
  // ==========================================================================

  // Get the most recent judgment ID (if fresh enough)
  const recentJudgmentId = (
    antiPatternState.lastJudgmentId &&
    (Date.now() - antiPatternState.lastJudgmentAt) < antiPatternState.judgmentIdTTL
  ) ? antiPatternState.lastJudgmentId : null;

  // Send learning feedback for test results
  if (toolName === 'Bash' && toolInput.command) {
    const cmd = toolInput.command;
    const output = typeof toolOutput === 'string' ? toolOutput : '';

    // Detect test commands
    if (cmd.match(/npm\s+(run\s+)?test|jest|vitest|mocha|pytest|cargo\s+test|go\s+test/i)) {
      const testResult = parseTestOutput(output, isError);
      testResult.judgmentId = testResult.judgmentId || recentJudgmentId;
      sendTestFeedback(testResult).catch(() => {});
    }

    // Detect successful commits
    if (cmd.startsWith('git commit') && !isError) {
      const commitHash = extractCommitHash(output);
      sendCommitFeedback({
        success: true,
        commitHash,
        hooksPassed: true,
        message: extractCommitMessage(cmd),
        judgmentId: recentJudgmentId,
      }).catch(() => {});
    }

    // Detect build commands
    if (cmd.match(/npm\s+run\s+build|tsc|webpack|vite\s+build|cargo\s+build|go\s+build/i)) {
      sendBuildFeedback({
        success: !isError,
        duration: null,
        judgmentId: recentJudgmentId,
      }).catch(() => {});
    }
  }
}

export default {
  mapToTriggerEventType,
  processTriggerEvent,
  parseTestOutput,
  extractCommitHash,
  extractCommitMessage,
  extractErrorSummary,
};
