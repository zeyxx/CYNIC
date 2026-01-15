#!/usr/bin/env node
/**
 * CYNIC Security Scan - On-Demand Audit
 *
 * "Le chien inspecte" - The dog inspects
 *
 * Standalone security scanner for workspace audit.
 * Can be run manually or integrated with CI/CD.
 *
 * Usage:
 *   node security-scan.cjs              # Scan staged files
 *   node security-scan.cjs --workspace  # Scan entire workspace
 *   node security-scan.cjs --changes    # Scan uncommitted changes
 *
 * @behavior non-blocking (outputs report)
 */

'use strict';

const path = require('path');
const watchdog = require(path.join(__dirname, '..', 'lib', 'watchdog.cjs'));

// =============================================================================
// MAIN
// =============================================================================

async function main() {
  const args = process.argv.slice(2);
  const mode = args.includes('--workspace') ? 'workspace' :
               args.includes('--changes') ? 'changes' : 'staged';

  let results;

  switch (mode) {
    case 'workspace':
      console.log('🔍 Scanning entire workspace...\n');
      results = watchdog.scanWorkspace(process.cwd());
      break;

    case 'changes':
      console.log('🔍 Scanning uncommitted changes...\n');
      results = watchdog.scanUncommittedChanges();
      break;

    case 'staged':
    default:
      console.log('🔍 Scanning staged files...\n');
      results = watchdog.scanStagedFiles();
      break;
  }

  const verdict = watchdog.calculateVerdict(results);
  console.log(watchdog.formatReport(verdict));

  // Exit with error code if critical findings
  if (verdict.shouldBlock) {
    process.exit(1);
  }
}

main().catch(error => {
  console.error('Security scan error:', error.message);
  process.exit(1);
});
