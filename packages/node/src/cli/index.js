/**
 * CYNIC CLI
 *
 * Command-line interface for CYNIC node operations
 *
 * @module @cynic/node/cli
 */

'use strict';

import chalk from 'chalk';
import { PHI, PHI_INV } from '@cynic/core';
import { startCommand } from './commands/start.js';
import { keygenCommand } from './commands/keygen.js';
import { connectCommand } from './commands/connect.js';
import { initCommand } from './commands/init.js';
import { doctorCommand } from './commands/doctor.js';
import { migrateCommand } from './commands/migrate.js';
import { benchmarkCommand } from './commands/benchmark.js';
import { traceCommand } from './commands/trace.js';
import { memoryCommand } from './commands/memory.js';
import { qtableCommand } from './commands/qtable.js';
import { handleBudgetCommand } from './commands/budget.js';
import { hooksCommand } from './commands/hooks.js';
import { judgeCommand } from './commands/judge.js';
import { daemonCommand } from './commands/daemon.js';
import { metricsCommand } from './commands/metrics.js';
// Dashboard is loaded lazily to avoid requiring blessed when not needed

const VERSION = '0.2.0';

const BANNER = `
${chalk.yellow('╔═══════════════════════════════════════════════════════════╗')}
${chalk.yellow('║')}  ${chalk.bold.cyan('CYNIC')} - Collective Your Node Into Consciousness        ${chalk.yellow('║')}
${chalk.yellow('║')}  ${chalk.gray('Decentralized judgment with φ-aligned consensus')}         ${chalk.yellow('║')}
${chalk.yellow('╚═══════════════════════════════════════════════════════════╝')}
`;

/**
 * Create CLI program
 * @param {Command} program - Commander program
 */
export function createCLI(program) {
  program
    .name('cynic')
    .description('CYNIC Node - Decentralized Collective Consciousness')
    .version(VERSION)
    .addHelpText('beforeAll', BANNER);

  // Start command
  program
    .command('start')
    .description('Start a CYNIC node')
    .option('-p, --port <port>', 'Server port', '8618')
    .option('-h, --host <host>', 'Server host', '0.0.0.0')
    .option('-k, --keyfile <path>', 'Path to keypair file')
    .option('-c, --connect <addresses...>', 'Peer addresses to connect to')
    .option('--no-server', 'Client-only mode (no server)')
    .option('-v, --verbose', 'Verbose output')
    .option('-d, --daemon', 'Daemon mode (no interactive REPL)')
    .option('--anchor', 'Enable Solana block anchoring (default: true)')
    .option('--no-anchor', 'Disable Solana block anchoring')
    .option('--anchor-interval <n>', 'Anchor every N blocks (default: 500)')
    .action(startCommand);

  // Keygen command
  program
    .command('keygen')
    .description('Generate a new keypair')
    .option('-o, --output <path>', 'Output file path', './cynic-key.json')
    .option('-f, --force', 'Overwrite existing file')
    .action(keygenCommand);

  // Connect command (for running nodes)
  program
    .command('connect <address>')
    .description('Connect to a peer (use with running node)')
    .action(connectCommand);

  // Dashboard command (TUI cockpit) - loaded lazily
  program
    .command('dashboard')
    .alias('cockpit')
    .description('Launch the TUI dashboard (cockpit) for real-time monitoring')
    .option('-p, --port <port>', 'MCP server port', '3618')
    .option('-u, --url <url>', 'MCP server URL (overrides port)')
    .action(async (options) => {
      // Dynamic import to avoid loading blessed when not needed
      const { dashboardCommand } = await import('./commands/dashboard.js');
      return dashboardCommand(options);
    });

  // Info command
  program
    .command('info')
    .description('Show CYNIC protocol information')
    .action(() => {
      console.log(BANNER);
      console.log(chalk.bold('\nProtocol Parameters:'));
      console.log(`  ${chalk.cyan('φ (phi)')}:        ${PHI.toFixed(6)}`);
      console.log(`  ${chalk.cyan('φ⁻¹ (phi_inv)')}: ${PHI_INV.toFixed(6)}`);
      console.log(`  ${chalk.cyan('Supermajority')}: ${(PHI_INV * 100).toFixed(1)}%`);
      console.log(`  ${chalk.cyan('Gossip Fanout')}: 13 (Fibonacci)`);
      console.log(`  ${chalk.cyan('Default Port')}:  8618`);
      console.log(`\n${chalk.bold('4-Layer Protocol Stack:')}`);
      console.log(`  ${chalk.green('L4')}: φ-BFT Consensus`);
      console.log(`  ${chalk.green('L3')}: Gossip Propagation`);
      console.log(`  ${chalk.green('L2')}: Merkle Knowledge Tree`);
      console.log(`  ${chalk.green('L1')}: Proof of Judgment (24 dimensions)`);
      console.log();
    });

  // Judge command ("CYNIC it")
  program
    .command('judge [input]')
    .alias('j')
    .description('Judge anything - "CYNIC it" (file, text, URL, stdin)')
    .option('--url <url>', 'Judge a URL or address')
    .option('-t, --type <type>', 'Force item type (code, text, config, decision)')
    .option('--title <title>', 'Custom title for the card')
    .option('--markdown', 'Output as markdown')
    .option('--json', 'Output as JSON')
    .option('--compact', 'Output as one-liner')
    .option('-s, --save', 'Save card to .cynic/cards/')
    .action(judgeCommand);

  // Init command (project scaffolding)
  program
    .command('init [project-name]')
    .description('Initialize a new CYNIC project')
    .option('-t, --template <template>', 'Project template (minimal, standard, full)', 'standard')
    .option('-f, --force', 'Overwrite existing configuration')
    .action(initCommand);

  // Doctor command (health check)
  program
    .command('doctor')
    .description('Check CYNIC system health')
    .option('-v, --verbose', 'Show detailed diagnostics')
    .option('--fix', 'Attempt to fix issues (not yet implemented)')
    .action(doctorCommand);

  // Migrate command (database migrations)
  program
    .command('migrate')
    .description('Run database migrations')
    .option('-s, --status', 'Show migration status only')
    .option('--reset', 'Reset all migrations (dangerous!)')
    .option('-y, --yes', 'Confirm dangerous operations')
    .option('-v, --verbose', 'Verbose output')
    .action(migrateCommand);

  // Benchmark command (performance testing)
  program
    .command('benchmark')
    .alias('bench')
    .description('Run performance benchmarks')
    .option('-s, --suite <suite>', 'Run specific suite (core, judge, patterns, router, vector)')
    .option('-q, --quick', 'Quick smoke test (fewer iterations)')
    .option('--json', 'Output results as JSON')
    .option('-v, --verbose', 'Show detailed timing')
    .action(benchmarkCommand);

  // ═══════════════════════════════════════════════════════════════════════════
  // DEBUGGING TOOLS
  // ═══════════════════════════════════════════════════════════════════════════

  // Trace command (decision trace viewer)
  program
    .command('trace')
    .description('View decision traces')
    .option('-i, --id <id>', 'Show specific trace by ID')
    .option('-b, --blocked', 'Show only blocked decisions')
    .option('-u, --user <userId>', 'Filter by user ID')
    .option('-d, --domain <domain>', 'Filter by domain')
    .option('-l, --limit <n>', 'Limit results', '20')
    .option('-e, --export <file>', 'Export traces to JSON')
    .option('--import <file>', 'Import traces from JSON')
    .option('--json', 'Output as JSON')
    .option('-v, --verbose', 'Show detailed reasoning')
    .action(traceCommand);

  // Memory command (memory inspector)
  program
    .command('memory')
    .alias('mem')
    .description('Inspect memory tiers')
    .option('-t, --tier <tier>', 'Inspect tier (working, episodic, semantic, vector)')
    .option('-s, --search <query>', 'Search across memories')
    .option('--stats', 'Show statistics only')
    .option('--gc', 'Run garbage collection')
    .option('--json', 'Output as JSON')
    .option('-v, --verbose', 'Show verbose output')
    .action(memoryCommand);

  // Q-table command (Q-table visualizer)
  program
    .command('qtable')
    .alias('q')
    .description('Visualize Q-Learning table')
    .option('--heatmap', 'Display as heatmap')
    .option('-t, --top <n>', 'Show top N state-action pairs')
    .option('-s, --state <state>', 'Show values for specific state')
    .option('-e, --export <file>', 'Export Q-table to JSON')
    .option('--import <file>', 'Import Q-table from JSON')
    .option('--demo', 'Show demo visualization')
    .option('--json', 'Output as JSON')
    .option('-v, --verbose', 'Show verbose output')
    .action(qtableCommand);

  // Hooks command (hook execution timeline)
  program
    .command('hooks')
    .description('View hook execution timeline')
    .option('-e, --event <event>', 'Filter by event type')
    .option('--slow', 'Show only slow hooks (>100ms)')
    .option('--failed', 'Show only failed hooks')
    .option('-c, --config', 'Show hook configuration')
    .option('-l, --limit <n>', 'Limit results', '50')
    .option('--json', 'Output as JSON')
    .option('-v, --verbose', 'Show verbose output')
    .action(hooksCommand);

  // ═══════════════════════════════════════════════════════════════════════════
  // METRICS
  // ═══════════════════════════════════════════════════════════════════════════

  // Metrics command (data-driven progress)
  program
    .command('metrics [subcommand]')
    .alias('m')
    .description('View organism health metrics (week1, autonomy, velocity, snapshot)')
    .action(metricsCommand);

  // Budget command (cost tracking & enforcement)
  program
    .command('budget [subcommand]')
    .alias('b')
    .description('Manage budget (status, reset, set, schedule)')
    .action(handleBudgetCommand);

  // ═══════════════════════════════════════════════════════════════════════════
  // DAEMON
  // ═══════════════════════════════════════════════════════════════════════════

  // Daemon command (independent runtime)
  program
    .command('daemon <action>')
    .alias('d')
    .description('Manage the CYNIC daemon (start, stop, status, restart)')
    .option('-p, --port <port>', 'Daemon HTTP port', '6180')
    .action(daemonCommand);

  return program;
}

export default { createCLI };
