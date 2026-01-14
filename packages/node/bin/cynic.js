#!/usr/bin/env node
/**
 * CYNIC Node CLI
 *
 * Command-line interface for running CYNIC nodes
 *
 * Usage:
 *   cynic start [options]     Start a CYNIC node
 *   cynic keygen              Generate a new keypair
 *   cynic status              Show node status
 *
 * @module @cynic/node/cli
 */

import { Command } from 'commander';
import { createCLI } from '../src/cli/index.js';

const program = new Command();
createCLI(program);
program.parse();
