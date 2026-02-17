/**
 * CYNIC Thin Hook — Daemon Client
 *
 * Shared module used by ALL thin hooks. Auto-starts daemon if needed.
 *
 * Flow:
 *   1. Check ProcessRegistry (~5ms file read)
 *   2. If daemon alive → HTTP POST → done
 *   3. If daemon absent → spawn detached → wait for health → POST
 *   4. If spawn fails → degrade silently (output { continue: true })
 *
 * "Le chien se réveille tout seul" - CYNIC
 *
 * @module scripts/hooks/thin/daemon-client
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import { spawn } from 'child_process';

const DAEMON_DIR = path.join(os.homedir(), '.cynic', 'daemon');
const PID_FILE = path.join(DAEMON_DIR, 'daemon.pid');
const LOCK_FILE = path.join(DAEMON_DIR, 'daemon.lock');
const DEFAULT_PORT = parseInt(process.env.CYNIC_DAEMON_PORT || '6180', 10);
const DAEMON_URL = `http://127.0.0.1:${DEFAULT_PORT}`;

/**
 * Call the daemon with a hook event
 *
 * @param {string} event - Hook event name (UserPromptSubmit, PreToolUse, etc.)
 * @param {Object} hookInput - Parsed hook stdin JSON
 * @param {Object} [options]
 * @param {number} [options.timeout=8000] - Request timeout in ms
 * @param {boolean} [options.canBlock=false] - If true, daemon can block execution
 * @returns {Promise<Object>} Hook output JSON
 */
export async function callDaemon(event, hookInput, options = {}) {
  const { timeout = 8000, canBlock = false } = options;

  try {
    // 1. Check if daemon is alive
    let alive = await isDaemonAlive();

    // 2. If not alive, try to auto-start
    let wakeMessage = null;
    if (!alive) {
      alive = await autoStartDaemon();
      if (alive) {
        wakeMessage = '*sniff* CYNIC daemon waking up...';
      }
    }

    // 3. If still not alive, degrade silently
    if (!alive) {
      return { continue: true };
    }

    // 4. POST to daemon
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    const res = await fetch(`${DAEMON_URL}/hook/${event}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(hookInput),
      signal: controller.signal,
    });
    clearTimeout(timer);

    if (!res.ok) {
      return { continue: true };
    }

    const result = await res.json();

    // Inject wake message into result (avoids double-safeOutput anti-pattern)
    if (wakeMessage && !result.message) {
      result.message = wakeMessage;
    }

    // Guard hooks can block — check the result
    if (canBlock && result.blocked) {
      return result;
    }

    return result;
  } catch (err) {
    // Timeout, connection refused, etc. — degrade silently
    return { continue: true };
  }
}

/**
 * Check if daemon is responding
 * @returns {Promise<boolean>}
 */
async function isDaemonAlive() {
  // Fast path: check PID file first
  try {
    if (!fs.existsSync(PID_FILE)) return false;
    const pid = parseInt(fs.readFileSync(PID_FILE, 'utf8').trim(), 10);
    if (isNaN(pid)) return false;

    // Check if process exists
    try { process.kill(pid, 0); } catch { return false; }
  } catch {
    return false;
  }

  // Health check
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 1000);
    const res = await fetch(`${DAEMON_URL}/health`, { signal: controller.signal });
    clearTimeout(timer);
    return res.ok;
  } catch {
    return false;
  }
}

/**
 * Auto-start daemon with file-lock coordination
 * @returns {Promise<boolean>} true if daemon is now alive
 */
async function autoStartDaemon() {
  // Acquire lock to prevent multiple hooks from spawning simultaneously
  try {
    if (!fs.existsSync(DAEMON_DIR)) {
      fs.mkdirSync(DAEMON_DIR, { recursive: true });
    }

    // Try to create lock file (exclusive)
    try {
      fs.writeFileSync(LOCK_FILE, process.pid.toString(), { flag: 'wx' });
    } catch {
      // Lock exists — another hook is spawning. Wait and check.
      return waitForDaemon(3000);
    }

    // We hold the lock — spawn daemon
    try {
      const entryPoint = path.resolve(
        path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Z]:)/, '$1')),
        '../../../packages/node/src/daemon/entry.js'
      );

      const child = spawn(process.execPath, [entryPoint, '--port', String(DEFAULT_PORT)], {
        detached: true,
        stdio: ['ignore', 'ignore', 'ignore'],
        env: {
          ...process.env,
          CYNIC_DAEMON: '1',
          CYNIC_DAEMON_PORT: String(DEFAULT_PORT),
        },
      });

      child.unref();

      // Write PID file
      fs.writeFileSync(PID_FILE, child.pid.toString());

      // Wait for health
      const alive = await waitForDaemon(3000);
      return alive;
    } finally {
      // Release lock
      try { fs.unlinkSync(LOCK_FILE); } catch { /* ignore */ }
    }
  } catch {
    // Lock or spawn failed — clean up and degrade
    try { fs.unlinkSync(LOCK_FILE); } catch { /* ignore */ }
    return false;
  }
}

/**
 * Wait for daemon to become responsive
 * @param {number} maxWait - Maximum wait in ms
 * @returns {Promise<boolean>}
 */
async function waitForDaemon(maxWait) {
  const pollInterval = 200;
  let elapsed = 0;

  while (elapsed < maxWait) {
    await new Promise(r => setTimeout(r, pollInterval));
    elapsed += pollInterval;

    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 500);
      const res = await fetch(`${DAEMON_URL}/health`, { signal: controller.signal });
      clearTimeout(timer);
      if (res.ok) return true;
    } catch { /* retry */ }
  }

  return false;
}

/**
 * Read stdin (hook input from Claude Code)
 * @returns {Promise<Object>}
 */
export async function readHookInput() {
  try {
    const input = fs.readFileSync(0, 'utf8');
    if (input && input.trim()) return JSON.parse(input);
  } catch { /* sync failed */ }

  // Async fallback (Windows pipe race condition)
  try {
    const data = await new Promise((resolve) => {
      let buf = '';
      process.stdin.setEncoding('utf8');
      process.stdin.on('data', chunk => { buf += chunk; });
      process.stdin.on('end', () => resolve(buf));
      process.stdin.on('error', () => resolve(''));
      process.stdin.resume();
      setTimeout(() => resolve(buf), 300);  // 300ms max — hook has 15s total
    });
    if (data && data.trim()) return JSON.parse(data);
  } catch { /* ignore */ }

  return {};
}

/**
 * Safe stdout output (handles EPIPE)
 * @param {Object} data
 */
export function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}
