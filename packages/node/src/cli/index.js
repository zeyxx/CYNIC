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
// Dashboard is loaded lazily to avoid requiring blessed when not needed

const VERSION = '0.1.0';

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

  return program;
}

export default { createCLI };
