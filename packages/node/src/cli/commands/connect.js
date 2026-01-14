/**
 * Connect Command
 *
 * Connect to a peer from command line
 * Note: This is mainly used within the interactive REPL
 *
 * @module @cynic/node/cli/commands/connect
 */

'use strict';

import chalk from 'chalk';

/**
 * Connect command handler
 */
export async function connectCommand(address, options) {
  console.log(chalk.yellow('\n  Note: Use this command within a running node'));
  console.log(chalk.gray('  Start a node first:'));
  console.log(chalk.white(`    cynic start --connect ${address}\n`));
}

export default { connectCommand };
