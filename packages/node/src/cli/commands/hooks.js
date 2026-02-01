/**
 * cynic hooks - Hook Execution Timeline
 *
 * View hook executions and timing for debugging.
 *
 * Usage:
 *   cynic hooks                    Show recent hook executions
 *   cynic hooks --event <event>    Filter by event type
 *   cynic hooks --slow             Show only slow hooks (>100ms)
 *   cynic hooks --failed           Show only failed hooks
 *   cynic hooks --config           Show hook configuration
 *
 * @module @cynic/node/cli/commands/hooks
 */

'use strict';

import { readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Hook event colors
 */
const EVENT_COLORS = {
  SessionStart: chalk.green,
  SessionEnd: chalk.blue,
  PreToolUse: chalk.yellow,
  PostToolUse: chalk.cyan,
  Stop: chalk.red,
  SubagentStop: chalk.magenta,
  UserPromptSubmit: chalk.white,
  PreCompact: chalk.gray,
  Notification: chalk.green,
  ToolError: chalk.red,
  PermissionRequest: chalk.yellow,
  ContextMemory: chalk.cyan,
};

/**
 * Format duration with color
 */
function formatDuration(ms) {
  if (ms === undefined || ms === null) return chalk.gray('---');

  if (ms < 10) {
    return chalk.green(`${ms.toFixed(0)}ms`);
  } else if (ms < 100) {
    return chalk.white(`${ms.toFixed(0)}ms`);
  } else if (ms < 500) {
    return chalk.yellow(`${ms.toFixed(0)}ms`);
  } else {
    return chalk.red(`${ms.toFixed(0)}ms`);
  }
}

/**
 * Format timestamp
 */
function formatTime(timestamp) {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', { hour12: false }) +
    '.' + date.getMilliseconds().toString().padStart(3, '0');
}

/**
 * Format status
 */
function formatStatus(status) {
  if (status === 'success' || status === true) {
    return chalk.green('✓');
  } else if (status === 'failed' || status === false) {
    return chalk.red('✗');
  } else if (status === 'skipped') {
    return chalk.gray('○');
  } else {
    return chalk.yellow('?');
  }
}

/**
 * Display hook execution in timeline
 */
function formatHookExecution(exec, verbose = false) {
  const time = formatTime(exec.timestamp);
  const eventColor = EVENT_COLORS[exec.event] || chalk.gray;
  const event = eventColor(exec.event.padEnd(18));
  const status = formatStatus(exec.status);
  const duration = formatDuration(exec.durationMs);
  const hook = chalk.gray((exec.hookName || exec.command || 'unknown').slice(0, 25));

  let line = `  ${chalk.gray(time)} ${status} ${event} ${duration.padStart(8)} ${hook}`;

  if (verbose && exec.output) {
    const output = exec.output.slice(0, 60).replace(/\n/g, ' ');
    line += chalk.gray(`\n${''.padStart(50)}└─ ${output}`);
  }

  return line;
}

/**
 * Display hook configuration
 */
async function displayConfig() {
  console.log(chalk.bold('\n  ── Hook Configuration ──\n'));

  // Try to load settings
  const settingsPath = join(process.cwd(), '.claude', 'settings.local.json');

  try {
    const content = await readFile(settingsPath, 'utf8');
    const settings = JSON.parse(content);
    const hooks = settings.hooks || {};

    const eventCount = Object.keys(hooks).length;
    console.log(`  ${chalk.gray('Config file:')} ${settingsPath}`);
    console.log(`  ${chalk.gray('Events configured:')} ${eventCount}\n`);

    for (const [event, hookList] of Object.entries(hooks)) {
      const eventColor = EVENT_COLORS[event] || chalk.gray;
      const count = Array.isArray(hookList) ? hookList.length : 1;

      console.log(`  ${eventColor(event.padEnd(20))} ${count} hook(s)`);

      if (Array.isArray(hookList)) {
        for (const hook of hookList) {
          const type = hook.type || 'command';
          const cmd = hook.command || hook.prompt || '(unknown)';
          const async = hook.async ? chalk.cyan(' [async]') : '';
          console.log(chalk.gray(`    └─ ${type}: ${cmd.slice(0, 50)}${async}`));
        }
      }
    }
  } catch (error) {
    console.log(chalk.yellow('  ⚠ Cannot read hook configuration'));
    console.log(chalk.gray(`  ${error.message}`));
    console.log(chalk.gray('\n  Create .claude/settings.local.json to configure hooks'));
  }
}

/**
 * Try to fetch hook executions from MCP server
 */
async function fetchHookExecutions(options = {}) {
  const port = process.env.CYNIC_MCP_PORT || 3000;
  const baseUrl = `http://localhost:${port}`;

  try {
    const params = new URLSearchParams();
    if (options.event) params.set('event', options.event);
    if (options.limit) params.set('limit', options.limit);
    if (options.slow) params.set('minDuration', '100');
    if (options.failed) params.set('status', 'failed');

    const url = `${baseUrl}/api/hooks?${params}`;
    const response = await fetch(url, { signal: AbortSignal.timeout(5000) });

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    return await response.json();
  } catch (error) {
    return null;
  }
}

/**
 * Get simulated hook executions (for demo/offline mode)
 */
function getSimulatedExecutions() {
  const now = Date.now();
  return [
    { timestamp: now - 120000, event: 'SessionStart', status: 'success', durationMs: 245, hookName: 'awaken.js' },
    { timestamp: now - 90000, event: 'UserPromptSubmit', status: 'success', durationMs: 12, hookName: 'validate-prompt' },
    { timestamp: now - 60000, event: 'PreToolUse', status: 'success', durationMs: 8, hookName: 'guard.js', toolName: 'Bash' },
    { timestamp: now - 55000, event: 'PostToolUse', status: 'success', durationMs: 156, hookName: 'observe.js' },
    { timestamp: now - 30000, event: 'PreToolUse', status: 'success', durationMs: 5, hookName: 'guard.js', toolName: 'Edit' },
    { timestamp: now - 25000, event: 'PostToolUse', status: 'success', durationMs: 89, hookName: 'observe.js' },
    { timestamp: now - 10000, event: 'Notification', status: 'success', durationMs: 34, hookName: 'notify.js' },
  ];
}

/**
 * Display timeline of hook executions
 */
function displayTimeline(executions, verbose = false) {
  if (executions.length === 0) {
    console.log(chalk.gray('  No hook executions found.'));
    return;
  }

  console.log(chalk.bold(`  ── Hook Execution Timeline (${executions.length} events) ──\n`));

  for (const exec of executions) {
    console.log(formatHookExecution(exec, verbose));
  }
}

/**
 * Display summary statistics
 */
function displayStats(executions) {
  const byEvent = {};
  let totalDuration = 0;
  let slowCount = 0;
  let failCount = 0;

  for (const exec of executions) {
    byEvent[exec.event] = (byEvent[exec.event] || 0) + 1;
    if (exec.durationMs) {
      totalDuration += exec.durationMs;
      if (exec.durationMs > 100) slowCount++;
    }
    if (exec.status === 'failed' || exec.status === false) {
      failCount++;
    }
  }

  console.log(chalk.bold('\n  ── Statistics ──'));
  console.log(`    ${chalk.gray('Total executions:')} ${executions.length}`);
  console.log(`    ${chalk.gray('Total duration:')}   ${totalDuration.toFixed(0)}ms`);
  console.log(`    ${chalk.gray('Avg duration:')}     ${(totalDuration / executions.length).toFixed(1)}ms`);
  console.log(`    ${chalk.gray('Slow (>100ms):')}    ${slowCount}`);
  console.log(`    ${chalk.gray('Failed:')}           ${failCount}`);

  console.log(chalk.bold('\n  ── By Event Type ──'));
  const sorted = Object.entries(byEvent).sort((a, b) => b[1] - a[1]);
  for (const [event, count] of sorted) {
    const eventColor = EVENT_COLORS[event] || chalk.gray;
    console.log(`    ${eventColor(event.padEnd(20))} ${count}`);
  }
}

/**
 * Hooks command handler
 */
export async function hooksCommand(options) {
  const { event, slow = false, failed = false, config = false, limit = 50, verbose = false, json = false } = options;

  if (!json) {
    console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
    console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Hook Execution Timeline         ') + chalk.yellow('║'));
    console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));
  }

  // Config mode
  if (config) {
    await displayConfig();
    return;
  }

  // Try to fetch from server
  console.log(chalk.gray('  *sniff* Fetching hook executions...\n'));

  let executions = await fetchHookExecutions({ event, limit, slow, failed });

  // Fallback to simulated data
  if (!executions) {
    console.log(chalk.yellow('  ⚠ MCP server not available, showing simulated data\n'));
    executions = getSimulatedExecutions();
  }

  // Ensure array
  const execList = Array.isArray(executions) ? executions : (executions.executions || []);

  // Apply filters
  let filtered = execList;
  if (event) {
    filtered = filtered.filter(e => e.event === event);
  }
  if (slow) {
    filtered = filtered.filter(e => e.durationMs && e.durationMs > 100);
  }
  if (failed) {
    filtered = filtered.filter(e => e.status === 'failed' || e.status === false);
  }

  // JSON output
  if (json) {
    console.log(JSON.stringify(filtered, null, 2));
    return;
  }

  // Display
  displayTimeline(filtered.slice(0, limit), verbose);
  displayStats(filtered);

  // Summary
  console.log(chalk.yellow('\n═════════════════════════════════════════'));
  console.log(chalk.gray('\n  12 Official Hook Events:'));
  console.log(chalk.gray('    SessionStart, SessionEnd, PreToolUse, PostToolUse'));
  console.log(chalk.gray('    Stop, SubagentStop, UserPromptSubmit, PreCompact'));
  console.log(chalk.gray('    Notification, ToolError, PermissionRequest, ContextMemory'));
  console.log(chalk.cyan(`\n  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));
}

export default { hooksCommand };
