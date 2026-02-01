/**
 * cynic doctor - System Health Check
 *
 * Checks all CYNIC systems and reports their status.
 *
 * Usage:
 *   cynic doctor
 *   cynic doctor --fix
 *   cynic doctor --verbose
 *
 * @module @cynic/node/cli/commands/doctor
 */

'use strict';

import { access, readFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Check result types
 */
const Status = {
  OK: 'ok',
  WARN: 'warn',
  ERROR: 'error',
  SKIP: 'skip',
};

/**
 * Format status with color
 */
function formatStatus(status) {
  switch (status) {
    case Status.OK:
      return chalk.green('✓ OK');
    case Status.WARN:
      return chalk.yellow('⚠ WARN');
    case Status.ERROR:
      return chalk.red('✗ ERROR');
    case Status.SKIP:
      return chalk.gray('○ SKIP');
    default:
      return chalk.gray('? UNKNOWN');
  }
}

/**
 * Check if a file exists
 */
async function fileExists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * Check PostgreSQL connection
 */
async function checkPostgres(verbose) {
  const checks = [];

  // Check for environment variables
  const hasEnv = process.env.CYNIC_DB_HOST || process.env.DATABASE_URL;
  if (!hasEnv) {
    return {
      name: 'PostgreSQL',
      status: Status.SKIP,
      message: 'No database configuration found',
      details: verbose ? 'Set CYNIC_DB_HOST or DATABASE_URL' : null,
    };
  }

  try {
    // Try to import and connect
    const { PostgresClient } = await import('@cynic/persistence');
    const client = new PostgresClient();
    await client.connect();

    // Check migrations table
    const { rows } = await client.query(
      'SELECT COUNT(*) as count FROM _migrations'
    );
    const migrationCount = parseInt(rows[0]?.count || 0);

    await client.close();

    return {
      name: 'PostgreSQL',
      status: Status.OK,
      message: `Connected (${migrationCount} migrations applied)`,
      details: verbose ? `Host: ${process.env.CYNIC_DB_HOST || 'from URL'}` : null,
    };
  } catch (error) {
    return {
      name: 'PostgreSQL',
      status: Status.ERROR,
      message: error.message.split('\n')[0],
      details: verbose ? error.stack : null,
    };
  }
}

/**
 * Check Redis connection
 */
async function checkRedis(verbose) {
  const redisUrl = process.env.CYNIC_REDIS_URL || process.env.REDIS_URL;

  if (!redisUrl) {
    return {
      name: 'Redis',
      status: Status.SKIP,
      message: 'No Redis configuration found (optional)',
      details: verbose ? 'Set CYNIC_REDIS_URL for caching' : null,
    };
  }

  try {
    const { createClient } = await import('redis');
    const client = createClient({ url: redisUrl });
    await client.connect();
    await client.ping();
    await client.disconnect();

    return {
      name: 'Redis',
      status: Status.OK,
      message: 'Connected',
      details: verbose ? `URL: ${redisUrl.replace(/:[^:]*@/, ':***@')}` : null,
    };
  } catch (error) {
    return {
      name: 'Redis',
      status: Status.WARN,
      message: 'Not available (optional)',
      details: verbose ? error.message : null,
    };
  }
}

/**
 * Check MCP server
 */
async function checkMcpServer(verbose) {
  const port = process.env.CYNIC_MCP_PORT || 3000;
  const url = `http://localhost:${port}/health`;

  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(5000) });

    if (response.ok) {
      const data = await response.json();
      return {
        name: 'MCP Server',
        status: Status.OK,
        message: `Running on port ${port}`,
        details: verbose ? JSON.stringify(data, null, 2) : null,
      };
    } else {
      return {
        name: 'MCP Server',
        status: Status.WARN,
        message: `Responding but unhealthy (${response.status})`,
        details: null,
      };
    }
  } catch (error) {
    return {
      name: 'MCP Server',
      status: Status.SKIP,
      message: 'Not running (start with: npm run mcp)',
      details: verbose ? `Tried: ${url}` : null,
    };
  }
}

/**
 * Check local Claude configuration
 */
async function checkClaudeConfig(verbose) {
  const claudeDir = join(process.cwd(), '.claude');
  const settingsPath = join(claudeDir, 'settings.local.json');

  const hasClaude = await fileExists(claudeDir);
  if (!hasClaude) {
    return {
      name: 'Claude Config',
      status: Status.WARN,
      message: 'No .claude directory found',
      details: verbose ? 'Run: cynic init' : null,
    };
  }

  const hasSettings = await fileExists(settingsPath);
  if (!hasSettings) {
    return {
      name: 'Claude Config',
      status: Status.WARN,
      message: '.claude exists but no settings.local.json',
      details: verbose ? 'Run: cynic init --force' : null,
    };
  }

  try {
    const content = await readFile(settingsPath, 'utf8');
    const settings = JSON.parse(content);
    const hookCount = Object.keys(settings.hooks || {}).length;

    return {
      name: 'Claude Config',
      status: Status.OK,
      message: `Found settings with ${hookCount} hook event(s)`,
      details: verbose ? `Path: ${settingsPath}` : null,
    };
  } catch (error) {
    return {
      name: 'Claude Config',
      status: Status.ERROR,
      message: 'Invalid settings.local.json',
      details: verbose ? error.message : null,
    };
  }
}

/**
 * Check Node.js version
 */
async function checkNodeVersion(verbose) {
  const version = process.version;
  const major = parseInt(version.slice(1).split('.')[0]);

  if (major < 18) {
    return {
      name: 'Node.js',
      status: Status.ERROR,
      message: `${version} (requires >= 18)`,
      details: null,
    };
  } else if (major < 20) {
    return {
      name: 'Node.js',
      status: Status.WARN,
      message: `${version} (recommend >= 20)`,
      details: null,
    };
  }

  return {
    name: 'Node.js',
    status: Status.OK,
    message: version,
    details: verbose ? `Platform: ${process.platform} ${process.arch}` : null,
  };
}

/**
 * Check package dependencies
 */
async function checkDependencies(verbose) {
  try {
    const packagePath = join(process.cwd(), 'package.json');
    const content = await readFile(packagePath, 'utf8');
    const pkg = JSON.parse(content);

    const cynicDeps = Object.keys(pkg.dependencies || {})
      .filter(d => d.startsWith('@cynic/'));

    if (cynicDeps.length === 0) {
      return {
        name: 'Dependencies',
        status: Status.SKIP,
        message: 'No @cynic/* packages in dependencies',
        details: verbose ? 'This may be expected for hooks-only setup' : null,
      };
    }

    return {
      name: 'Dependencies',
      status: Status.OK,
      message: `${cynicDeps.length} @cynic/* package(s)`,
      details: verbose ? cynicDeps.join(', ') : null,
    };
  } catch (error) {
    return {
      name: 'Dependencies',
      status: Status.SKIP,
      message: 'No package.json found',
      details: null,
    };
  }
}

/**
 * Doctor command handler
 */
export async function doctorCommand(options) {
  const { verbose = false, fix = false } = options;

  console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
  console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Health Check                    ') + chalk.yellow('║'));
  console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));

  console.log(chalk.gray(`  *sniff sniff* Checking systems...\n`));

  // Run all checks
  const checks = await Promise.all([
    checkNodeVersion(verbose),
    checkClaudeConfig(verbose),
    checkDependencies(verbose),
    checkPostgres(verbose),
    checkRedis(verbose),
    checkMcpServer(verbose),
  ]);

  // Display results
  let hasErrors = false;
  let hasWarnings = false;

  for (const check of checks) {
    const status = formatStatus(check.status);
    const name = check.name.padEnd(15);
    console.log(`  ${status}  ${name} ${chalk.gray(check.message)}`);

    if (verbose && check.details) {
      console.log(chalk.gray(`              ${check.details}`));
    }

    if (check.status === Status.ERROR) hasErrors = true;
    if (check.status === Status.WARN) hasWarnings = true;
  }

  // Summary
  console.log(chalk.yellow('\n═════════════════════════════════════════'));

  const okCount = checks.filter(c => c.status === Status.OK).length;
  const total = checks.filter(c => c.status !== Status.SKIP).length;
  const health = total > 0 ? (okCount / total) * 100 : 100;
  const healthBar = '█'.repeat(Math.round(health / 10)) + '░'.repeat(10 - Math.round(health / 10));

  console.log(`\n  Health: [${healthBar}] ${health.toFixed(0)}%`);

  if (hasErrors) {
    console.log(chalk.red('\n  *GROWL* Some systems need attention!'));
    console.log(chalk.gray('  Run with --verbose for more details\n'));
    process.exit(1);
  } else if (hasWarnings) {
    console.log(chalk.yellow('\n  *head tilt* Minor issues detected'));
    console.log(chalk.gray('  Run with --verbose for more details\n'));
  } else {
    console.log(chalk.green('\n  *tail wag* All systems healthy!'));
    console.log(chalk.cyan(`  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));
  }
}

export default { doctorCommand };
