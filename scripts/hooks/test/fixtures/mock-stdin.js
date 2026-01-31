/**
 * Mock stdin for hook testing
 *
 * Provides utilities to simulate hook input via stdin.
 *
 * @module scripts/hooks/test/fixtures/mock-stdin
 */

'use strict';

import { spawnSync, spawn } from 'child_process';
import path from 'path';

const HOOKS_DIR = path.resolve(import.meta.dirname, '../..');

/**
 * Run a hook with mock stdin input
 *
 * @param {string} hookName - Hook name (without extension)
 * @param {Object} input - Input object to pass via stdin
 * @param {Object} options - Additional options
 * @returns {Object} { stdout, stderr, status, output }
 */
export function runHook(hookName, input = {}, options = {}) {
  const hookPath = path.join(HOOKS_DIR, `${hookName}.js`);
  const inputStr = JSON.stringify(input);

  const result = spawnSync('node', [hookPath], {
    input: inputStr,
    encoding: 'utf-8',
    cwd: options.cwd || process.cwd(),
    timeout: options.timeout || 10000,
    env: {
      ...process.env,
      ...options.env,
      CYNIC_TEST: '1',
    },
  });

  let output = null;
  try {
    // Extract JSON from output (may have log lines before/after)
    const stdout = result.stdout || '';
    const jsonStart = stdout.indexOf('{');
    const jsonEnd = stdout.lastIndexOf('}');
    if (jsonStart !== -1 && jsonEnd !== -1 && jsonEnd > jsonStart) {
      output = JSON.parse(stdout.slice(jsonStart, jsonEnd + 1));
    }
  } catch (e) {
    // Non-JSON output
  }

  return {
    stdout: result.stdout,
    stderr: result.stderr,
    status: result.status,
    output,
  };
}

/**
 * Run a hook asynchronously
 *
 * @param {string} hookName - Hook name
 * @param {Object} input - Input object
 * @param {Object} options - Additional options
 * @returns {Promise<Object>} { stdout, stderr, exitCode, output }
 */
export async function runHookAsync(hookName, input = {}, options = {}) {
  const hookPath = path.join(HOOKS_DIR, `${hookName}.js`);
  const inputStr = JSON.stringify(input);

  return new Promise((resolve, reject) => {
    const proc = spawn('node', [hookPath], {
      cwd: options.cwd || process.cwd(),
      env: {
        ...process.env,
        ...options.env,
        CYNIC_TEST: '1',
      },
    });

    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', data => { stdout += data; });
    proc.stderr.on('data', data => { stderr += data; });

    proc.stdin.write(inputStr);
    proc.stdin.end();

    const timeout = setTimeout(() => {
      proc.kill();
      reject(new Error('Hook timeout'));
    }, options.timeout || 10000);

    proc.on('close', exitCode => {
      clearTimeout(timeout);
      let output = null;
      try {
        // Extract JSON from output (may have log lines before/after)
        const jsonStart = stdout.indexOf('{');
        const jsonEnd = stdout.lastIndexOf('}');
        if (jsonStart !== -1 && jsonEnd !== -1 && jsonEnd > jsonStart) {
          output = JSON.parse(stdout.slice(jsonStart, jsonEnd + 1));
        }
      } catch (e) {
        // Non-JSON output
      }
      resolve({ stdout, stderr, exitCode, output });
    });
  });
}

/**
 * Create mock tool input for guard hook
 */
export function createToolInput(toolName, toolInput) {
  return {
    tool_name: toolName,
    tool_input: toolInput,
  };
}

/**
 * Create mock session context for awaken/sleep hooks
 */
export function createSessionContext(overrides = {}) {
  return {
    sessionId: `test-session-${Date.now()}`,
    sessionStartTime: Date.now() - 60000,
    ...overrides,
  };
}

/**
 * Create mock subagent start/stop context for spawn hook
 */
export function createSubagentContext(eventType, overrides = {}) {
  const base = {
    event_type: eventType,
    agent_id: `agent-${Date.now()}`,
    subagent_type: 'Explore',
    prompt: 'Test prompt for agent',
    model: 'default',
  };

  if (eventType === 'SubagentStop' || eventType === 'subagent_stop') {
    return {
      ...base,
      success: true,
      duration_ms: 1500,
      result: { output: 'test result' },
      ...overrides,
    };
  }

  return { ...base, ...overrides };
}

/**
 * Create mock error context for error hook
 */
export function createErrorContext(toolName, errorMessage, toolInput = {}) {
  return {
    tool_name: toolName,
    tool_input: toolInput,
    error: errorMessage,
    error_message: errorMessage,
  };
}

/**
 * Create mock prompt context for perceive hook (UserPromptSubmit)
 */
export function createPromptContext(prompt, overrides = {}) {
  return {
    prompt,
    timestamp: Date.now(),
    session_id: `test-session-${Date.now()}`,
    ...overrides,
  };
}

export default {
  runHook,
  runHookAsync,
  createToolInput,
  createSessionContext,
  createSubagentContext,
  createErrorContext,
  createPromptContext,
};
