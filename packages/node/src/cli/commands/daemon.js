/**
 * Daemon Command
 *
 * Manages the CYNIC daemon lifecycle: start, stop, status, restart.
 * The daemon is an independent Node.js process that boots all singletons once
 * and serves hook requests via HTTP on :6180.
 *
 * "Le chien ne dort jamais vraiment" - CYNIC
 *
 * @module @cynic/node/cli/commands/daemon
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import { spawn } from 'child_process';
import chalk from 'chalk';
import { processRegistry } from '@cynic/core';

const DAEMON_DIR = path.join(os.homedir(), '.cynic', 'daemon');
const PID_FILE = path.join(DAEMON_DIR, 'daemon.pid');
const LOG_FILE = path.join(DAEMON_DIR, 'daemon.log');
const DEFAULT_PORT = 6180;

/**
 * Ensure daemon directory exists
 */
function ensureDaemonDir() {
  if (!fs.existsSync(DAEMON_DIR)) {
    fs.mkdirSync(DAEMON_DIR, { recursive: true });
  }
}

/**
 * Check if a process with given PID is alive
 * @param {number} pid
 * @returns {boolean}
 */
function isProcessAlive(pid) {
  try {
    process.kill(pid, 0); // Signal 0 = check existence
    return true;
  } catch {
    return false;
  }
}

/**
 * Read PID from file, return null if stale or missing
 * @returns {number|null}
 */
function readPid() {
  try {
    const pid = parseInt(fs.readFileSync(PID_FILE, 'utf8').trim(), 10);
    if (isNaN(pid)) return null;
    if (!isProcessAlive(pid)) {
      // Stale PID file — clean up
      try { fs.unlinkSync(PID_FILE); } catch { /* ignore */ }
      return null;
    }
    return pid;
  } catch {
    return null;
  }
}

/**
 * Ping the daemon health endpoint
 * @param {number} port
 * @param {number} timeout - ms
 * @returns {Promise<Object|null>}
 */
async function pingDaemon(port = DEFAULT_PORT, timeout = 2000) {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    const res = await fetch(`http://127.0.0.1:${port}/health`, {
      signal: controller.signal,
    });
    clearTimeout(timer);

    if (res.ok) return await res.json();
    return null;
  } catch {
    return null;
  }
}

/**
 * Daemon command handler
 * @param {string} action - start, stop, status, restart
 * @param {Object} options - CLI options
 */
export async function daemonCommand(action, options = {}) {
  ensureDaemonDir();
  const port = parseInt(options.port || DEFAULT_PORT, 10);

  switch (action) {
    case 'start':
      return startDaemon(port, options);
    case 'stop':
      return stopDaemon(port);
    case 'status':
      return showStatus(port);
    case 'restart':
      await stopDaemon(port);
      // Brief wait for port release
      await new Promise(r => setTimeout(r, 1000));
      return startDaemon(port, options);
    default:
      console.log(chalk.yellow('Usage: cynic daemon <start|stop|status|restart>'));
      console.log('  start   Start the CYNIC daemon');
      console.log('  stop    Stop the running daemon');
      console.log('  status  Show daemon status');
      console.log('  restart Restart the daemon');
  }
}

/**
 * Start the daemon
 */
async function startDaemon(port, options) {
  // Check if already running
  const existingPid = readPid();
  if (existingPid) {
    const health = await pingDaemon(port);
    if (health) {
      console.log(chalk.yellow(`Daemon already running (PID: ${existingPid}, uptime: ${health.uptimeHuman})`));
      return;
    }
    // PID alive but not responding — stale
    console.log(chalk.gray(`Stale PID ${existingPid} found, cleaning up...`));
    try { fs.unlinkSync(PID_FILE); } catch { /* ignore */ }
  }

  console.log(chalk.cyan('Starting CYNIC daemon...'));

  // Find the daemon entry point
  const entryPoint = path.resolve(
    import.meta.dirname || path.dirname(new URL(import.meta.url).pathname),
    '../../daemon/entry.js'
  );

  // Spawn detached daemon with 2GB heap + inspector for memory profiling
  const child = spawn(process.execPath, [
    '--max-old-space-size=2048',
    '--inspect=9229',
    entryPoint,
    '--port',
    String(port)
  ], {
    detached: true,
    stdio: ['ignore', 'ignore', 'ignore'],
    env: {
      ...process.env,
      CYNIC_DAEMON: '1',
      CYNIC_DAEMON_PORT: String(port),
    },
  });

  child.unref();

  // Write PID immediately (child.pid is available synchronously)
  fs.writeFileSync(PID_FILE, child.pid.toString());

  // Poll for health
  console.log(chalk.gray(`Waiting for daemon (PID: ${child.pid})...`));
  const maxWait = 5000;
  const pollInterval = 250;
  let elapsed = 0;

  while (elapsed < maxWait) {
    await new Promise(r => setTimeout(r, pollInterval));
    elapsed += pollInterval;

    const health = await pingDaemon(port, 1000);
    if (health) {
      console.log(chalk.green(`CYNIC daemon started`));
      console.log(chalk.gray(`  PID:    ${child.pid}`));
      console.log(chalk.gray(`  Port:   ${port}`));
      console.log(chalk.gray(`  Memory: ${health.memoryMB}MB`));
      console.log(chalk.gray(`  Log:    ${LOG_FILE}`));
      return;
    }
  }

  console.log(chalk.yellow('Daemon started but health check timed out.'));
  console.log(chalk.gray(`Check log: ${LOG_FILE}`));
}

/**
 * Stop the daemon
 */
async function stopDaemon(port) {
  const pid = readPid();

  if (!pid) {
    // Try health check in case PID file is missing
    const health = await pingDaemon(port);
    if (health) {
      console.log(chalk.yellow(`Daemon responding on :${port} but no PID file. Kill PID ${health.pid} manually.`));
      return;
    }
    console.log(chalk.gray('Daemon is not running'));
    return;
  }

  console.log(chalk.cyan(`Stopping daemon (PID: ${pid})...`));

  try {
    // Cross-platform kill
    if (process.platform === 'win32') {
      spawn('taskkill', ['/PID', String(pid), '/T'], { stdio: 'ignore' });
    } else {
      process.kill(pid, 'SIGTERM');
    }

    // Wait for process to exit
    let attempts = 0;
    while (attempts < 20 && isProcessAlive(pid)) {
      await new Promise(r => setTimeout(r, 250));
      attempts++;
    }

    // Force kill if still alive
    if (isProcessAlive(pid)) {
      try {
        if (process.platform === 'win32') {
          spawn('taskkill', ['/PID', String(pid), '/F', '/T'], { stdio: 'ignore' });
        } else {
          process.kill(pid, 'SIGKILL');
        }
      } catch { /* ignore */ }
    }

    // Clean up PID file
    try { fs.unlinkSync(PID_FILE); } catch { /* ignore */ }

    console.log(chalk.green('Daemon stopped'));
  } catch (err) {
    console.log(chalk.red(`Failed to stop daemon: ${err.message}`));
    try { fs.unlinkSync(PID_FILE); } catch { /* ignore */ }
  }
}

/**
 * Show daemon status
 */
async function showStatus(port) {
  const pid = readPid();
  const health = await pingDaemon(port);

  if (!pid && !health) {
    console.log(chalk.gray('Daemon is not running'));
    return;
  }

  if (health) {
    console.log(chalk.green('CYNIC Daemon: RUNNING'));
    console.log(chalk.gray(`  PID:    ${health.pid}`));
    console.log(chalk.gray(`  Port:   ${health.port}`));
    console.log(chalk.gray(`  Uptime: ${health.uptimeHuman}`));
    console.log(chalk.gray(`  Memory: ${health.memoryMB}MB`));

    // Show ProcessRegistry info
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 2000);
      const statusRes = await fetch(`http://127.0.0.1:${port}/status`, {
        signal: controller.signal,
      });
      clearTimeout(timer);

      if (statusRes.ok) {
        const status = await statusRes.json();
        if (status.processes && Object.keys(status.processes).length > 0) {
          console.log(chalk.gray(`  Processes: ${Object.keys(status.processes).length} registered`));
        }
      }
    } catch { /* ignore */ }
  } else if (pid) {
    console.log(chalk.yellow(`Daemon PID ${pid} exists but not responding on :${port}`));
  }
}

export default daemonCommand;
