/**
 * Dashboard Command
 *
 * Launch the CYNIC TUI Dashboard (Cockpit)
 *
 * @module @cynic/node/cli/commands/dashboard
 */

'use strict';

import chalk from 'chalk';

/**
 * Dashboard command handler
 */
export async function dashboardCommand(options) {
  const port = parseInt(options.port) || 3618;
  const url = options.url || `http://localhost:${port}`;

  console.log(chalk.bold.cyan('\n  🐕 CYNIC COCKPIT\n'));
  console.log(chalk.gray('  Connecting to MCP server at ') + chalk.cyan(url));
  console.log(chalk.gray('  Press ') + chalk.bold('q') + chalk.gray(' to quit\n'));

  try {
    // Check if blessed is available
    try {
      await import('blessed');
    } catch {
      console.error(chalk.red('  ✗ blessed module not found'));
      console.error(chalk.gray('    The TUI dashboard requires the blessed library.'));
      console.error(chalk.gray('    Install it with: ') + chalk.white('npm install blessed'));
      console.error();
      console.error(chalk.gray('    Alternatively, use the web dashboard at: ') + chalk.cyan(`${url}/`));
      process.exit(1);
    }

    // Check if MCP server is reachable before launching
    const healthCheck = await fetch(`${url}/health`, {
      signal: AbortSignal.timeout(3000),
    }).catch(() => null);

    if (!healthCheck?.ok) {
      console.log(chalk.yellow('  ⚠ MCP server not responding at ') + chalk.cyan(url));
      console.log(chalk.gray('    Dashboard will show fallback data until connection is established.'));
      console.log(chalk.gray('    Start the MCP server with: ') + chalk.white('npx @cynic/mcp'));
      console.log();
    }

    // Dynamic import to avoid loading blessed when checking --help
    const { createDashboard } = await import('../dashboard/index.js');

    // Launch dashboard
    await createDashboard({ url, port });

  } catch (err) {
    if (err.code === 'ERR_MODULE_NOT_FOUND') {
      console.error(chalk.red('  ✗ Module not found: ') + err.message);
      console.error(chalk.gray('    Run ') + chalk.white('npm install') + chalk.gray(' in the project root.'));
      process.exit(1);
    }

    console.error(chalk.red('  ✗ Dashboard error: ') + err.message);
    process.exit(1);
  }
}

export default { dashboardCommand };
