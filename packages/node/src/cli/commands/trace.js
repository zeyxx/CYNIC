/**
 * cynic trace - Decision Trace Viewer
 *
 * View and inspect decision traces from the orchestration system.
 *
 * Usage:
 *   cynic trace                    Show recent decisions
 *   cynic trace --id <id>          Show specific trace
 *   cynic trace --blocked          Show blocked decisions
 *   cynic trace --user <userId>    Filter by user
 *   cynic trace --domain <domain>  Filter by domain
 *   cynic trace --export <file>    Export traces to JSON
 *
 * @module @cynic/node/cli/commands/trace
 */

'use strict';

import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Format timestamp
 */
function formatTime(timestamp) {
  const date = new Date(timestamp);
  const time = date.toLocaleTimeString('en-US', { hour12: false });
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${time}.${ms}`;
}

/**
 * Format outcome with color
 */
function formatOutcome(outcome) {
  const colors = {
    ALLOW: chalk.green,
    BLOCK: chalk.red,
    WARN: chalk.yellow,
    DEFER: chalk.blue,
    UNKNOWN: chalk.gray,
  };
  const fn = colors[outcome] || colors.UNKNOWN;
  return fn(outcome.padEnd(6));
}

/**
 * Format dog/sefirah with color
 */
function formatDog(dog) {
  const colors = {
    GUARDIAN: chalk.red,
    ANALYST: chalk.cyan,
    SCHOLAR: chalk.yellow,
    SAGE: chalk.cyan,
    ARCHITECT: chalk.blue,
    ORACLE: chalk.yellow,
    JANITOR: chalk.magenta,
    SCOUT: chalk.green,
    CARTOGRAPHER: chalk.green,
    DEPLOYER: chalk.yellow,
    CYNIC: chalk.white,
  };
  const fn = colors[dog?.toUpperCase()] || chalk.gray;
  return fn((dog || 'unknown').padEnd(12));
}

/**
 * Format a single trace for display
 */
function formatTrace(trace, verbose = false) {
  const lines = [];

  // Header
  const time = formatTime(trace.timestamp);
  const outcome = formatOutcome(trace.outcome);
  const dog = formatDog(trace.routing?.sefirah || trace.routing?.domain);

  lines.push(`${chalk.gray(time)} ${outcome} ${dog} ${chalk.gray(trace.id.slice(0, 8))}`);

  // Event type and domain
  if (trace.eventType) {
    lines.push(`  ${chalk.gray('Event:')} ${trace.eventType}`);
  }
  if (trace.routing?.domain) {
    lines.push(`  ${chalk.gray('Domain:')} ${trace.routing.domain}`);
  }

  // Intervention details
  if (trace.intervention) {
    const level = trace.intervention.level;
    const risk = trace.intervention.actionRisk || 'unknown';
    lines.push(`  ${chalk.gray('Intervention:')} ${level} (risk: ${risk})`);
  }

  // Reasoning
  if (trace.reasoning && verbose) {
    lines.push(`  ${chalk.gray('Reasoning:')}`);
    for (const reason of trace.reasoning.slice(0, 3)) {
      lines.push(`    - ${reason}`);
    }
    if (trace.reasoning.length > 3) {
      lines.push(chalk.gray(`    ... and ${trace.reasoning.length - 3} more`));
    }
  }

  // Duration
  if (trace.durationMs) {
    lines.push(`  ${chalk.gray('Duration:')} ${trace.durationMs}ms`);
  }

  return lines.join('\n');
}

/**
 * Display traces in timeline format
 */
function displayTimeline(traces, verbose = false) {
  if (traces.length === 0) {
    console.log(chalk.gray('  No traces found.'));
    return;
  }

  console.log(chalk.bold(`  ── Decision Timeline (${traces.length} traces) ──\n`));

  for (const trace of traces) {
    console.log(formatTrace(trace, verbose));
    console.log();
  }
}

/**
 * Display single trace in detail
 */
function displayTraceDetail(trace) {
  console.log(chalk.bold(`\n  ── Trace Detail ──\n`));

  // ID and timestamp
  console.log(`  ${chalk.gray('ID:')}        ${trace.id}`);
  console.log(`  ${chalk.gray('Time:')}      ${new Date(trace.timestamp).toISOString()}`);
  console.log(`  ${chalk.gray('Outcome:')}   ${formatOutcome(trace.outcome)}`);

  // Routing
  if (trace.routing) {
    console.log(`\n  ${chalk.bold('Routing:')}`);
    console.log(`    ${chalk.gray('Domain:')}   ${trace.routing.domain || 'unknown'}`);
    console.log(`    ${chalk.gray('Sefirah:')}  ${trace.routing.sefirah || 'unknown'}`);
    console.log(`    ${chalk.gray('Pillar:')}   ${trace.routing.pillar || 'unknown'}`);
  }

  // Intervention
  if (trace.intervention) {
    console.log(`\n  ${chalk.bold('Intervention:')}`);
    console.log(`    ${chalk.gray('Level:')}    ${trace.intervention.level}`);
    console.log(`    ${chalk.gray('Risk:')}     ${trace.intervention.actionRisk}`);
    if (trace.intervention.toolName) {
      console.log(`    ${chalk.gray('Tool:')}     ${trace.intervention.toolName}`);
    }
  }

  // User context
  if (trace.userContext) {
    console.log(`\n  ${chalk.bold('User Context:')}`);
    console.log(`    ${chalk.gray('User ID:')}  ${trace.userContext.userId || 'unknown'}`);
    console.log(`    ${chalk.gray('Session:')}  ${trace.userContext.sessionId || 'unknown'}`);
  }

  // Reasoning
  if (trace.reasoning && trace.reasoning.length > 0) {
    console.log(`\n  ${chalk.bold('Reasoning:')}`);
    for (let i = 0; i < trace.reasoning.length; i++) {
      console.log(`    ${i + 1}. ${trace.reasoning[i]}`);
    }
  }

  // Duration
  if (trace.durationMs) {
    console.log(`\n  ${chalk.gray('Duration:')} ${trace.durationMs}ms`);
  }

  console.log();
}

/**
 * Try to connect to MCP server and get traces
 */
async function fetchTracesFromServer(options) {
  const port = process.env.CYNIC_MCP_PORT || 3000;
  const baseUrl = `http://localhost:${port}`;

  try {
    // Build query params
    const params = new URLSearchParams();
    if (options.limit) params.set('limit', options.limit);
    if (options.user) params.set('userId', options.user);
    if (options.domain) params.set('domain', options.domain);
    if (options.blocked) params.set('outcome', 'BLOCK');

    const url = `${baseUrl}/api/traces?${params}`;
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
 * Load traces from file
 */
async function loadTracesFromFile(filePath) {
  try {
    const content = await readFile(filePath, 'utf8');
    return JSON.parse(content);
  } catch (error) {
    return null;
  }
}

/**
 * Trace command handler
 */
export async function traceCommand(options) {
  const { id, blocked = false, user, domain, limit = 20, verbose = false, json = false } = options;
  const exportPath = options.export;
  const importPath = options.import;

  if (!json) {
    console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
    console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Decision Trace Viewer           ') + chalk.yellow('║'));
    console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));
  }

  // Import mode
  if (importPath) {
    const traces = await loadTracesFromFile(importPath);
    if (!traces) {
      console.log(chalk.red(`  ✗ Cannot read file: ${importPath}`));
      process.exit(1);
    }

    const traceList = Array.isArray(traces) ? traces : (traces.traces || []);
    console.log(chalk.gray(`  Loaded ${traceList.length} trace(s) from ${importPath}\n`));
    displayTimeline(traceList, verbose);
    return;
  }

  // Try to fetch from server
  console.log(chalk.gray('  *sniff* Fetching traces from MCP server...\n'));

  const serverTraces = await fetchTracesFromServer({ limit, user, domain, blocked });

  if (!serverTraces) {
    console.log(chalk.yellow('  ⚠ MCP server not available'));
    console.log(chalk.gray('  Start the server with: npm run mcp'));
    console.log(chalk.gray('\n  Or import traces from file: cynic trace --import <file.json>\n'));
    return;
  }

  const traces = Array.isArray(serverTraces) ? serverTraces : (serverTraces.traces || []);

  // Export mode
  if (exportPath) {
    try {
      await writeFile(exportPath, JSON.stringify(traces, null, 2));
      console.log(chalk.green(`  ✓ Exported ${traces.length} trace(s) to ${exportPath}\n`));
    } catch (error) {
      console.log(chalk.red(`  ✗ Cannot write file: ${error.message}`));
    }
    return;
  }

  // JSON output mode
  if (json) {
    console.log(JSON.stringify(traces, null, 2));
    return;
  }

  // Show specific trace
  if (id) {
    const trace = traces.find(t => t.id === id || t.id.startsWith(id));
    if (trace) {
      displayTraceDetail(trace);
    } else {
      console.log(chalk.red(`  ✗ Trace not found: ${id}`));
    }
    return;
  }

  // Show timeline
  displayTimeline(traces, verbose);

  // Summary
  console.log(chalk.yellow('═════════════════════════════════════════'));
  const outcomes = {};
  for (const trace of traces) {
    outcomes[trace.outcome] = (outcomes[trace.outcome] || 0) + 1;
  }
  console.log(chalk.gray('\n  Summary:'));
  for (const [outcome, count] of Object.entries(outcomes)) {
    console.log(`    ${formatOutcome(outcome)} ${count}`);
  }
  console.log(chalk.cyan(`\n  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));
}

export default { traceCommand };
