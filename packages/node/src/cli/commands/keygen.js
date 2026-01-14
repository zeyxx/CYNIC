/**
 * Keygen Command
 *
 * Generate a new CYNIC keypair
 *
 * @module @cynic/node/cli/commands/keygen
 */

'use strict';

import fs from 'fs';
import path from 'path';
import chalk from 'chalk';
import { generateKeypair } from '@cynic/protocol';

/**
 * Keygen command handler
 */
export async function keygenCommand(options) {
  const outputPath = path.resolve(options.output);

  // Check if file exists
  if (fs.existsSync(outputPath) && !options.force) {
    console.error(chalk.red(`\n  Error: File already exists: ${outputPath}`));
    console.error(chalk.gray('  Use --force to overwrite\n'));
    process.exit(1);
  }

  console.log(chalk.bold.cyan('\n  Generating CYNIC Keypair...\n'));

  // Generate keypair
  const keypair = generateKeypair();

  // Prepare output
  const output = {
    publicKey: keypair.publicKey,
    privateKey: keypair.privateKey,
    createdAt: new Date().toISOString(),
    version: 1,
  };

  // Write to file
  fs.writeFileSync(outputPath, JSON.stringify(output, null, 2));

  console.log(chalk.green('  [OK] ') + `Keypair generated successfully\n`);
  console.log(chalk.gray('  Public Key:  ') + chalk.cyan(keypair.publicKey.slice(0, 32) + '...'));
  console.log(chalk.gray('  Private Key: ') + chalk.yellow('[REDACTED]'));
  console.log(chalk.gray('  Output:      ') + chalk.white(outputPath));
  console.log();
  console.log(chalk.yellow('  WARNING: Keep your private key secure!'));
  console.log(chalk.gray('  Never share your keypair file.\n'));

  console.log(chalk.bold('  Usage:'));
  console.log(chalk.gray(`    cynic start --keyfile ${options.output}\n`));
}

export default { keygenCommand };
