/**
 * CYNIC Hook Logger
 *
 * "Le chien n'oublie pas ses erreurs" - The dog remembers its errors
 *
 * Provides persistent logging for hooks instead of silent failures.
 * Logs are written to a rotating file for debugging and monitoring.
 *
 * @module scripts/lib/hook-logger
 */

'use strict';

import { appendFileSync, existsSync, mkdirSync, statSync, renameSync, readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// =============================================================================
// CONSTANTS
// =============================================================================

const LOG_DIR = join(__dirname, '../../.claude/logs');
const LOG_FILE = join(LOG_DIR, 'hooks.log');
const MAX_LOG_SIZE = 1024 * 1024; // 1MB
const MAX_LOG_FILES = 5;

// Log levels with φ-aligned severity
const LOG_LEVELS = {
  DEBUG: { value: 10, prefix: '🔍', color: '\x1b[36m' },   // Cyan
  INFO: { value: 20, prefix: '📝', color: '\x1b[32m' },    // Green
  WARN: { value: 30, prefix: '⚠️', color: '\x1b[33m' },    // Yellow
  ERROR: { value: 40, prefix: '❌', color: '\x1b[31m' },   // Red
  CRITICAL: { value: 50, prefix: '🔥', color: '\x1b[35m' }, // Magenta
};

const RESET = '\x1b[0m';

// =============================================================================
// LOG ROTATION
// =============================================================================

function ensureLogDir() {
  if (!existsSync(LOG_DIR)) {
    mkdirSync(LOG_DIR, { recursive: true });
  }
}

function rotateIfNeeded() {
  try {
    if (!existsSync(LOG_FILE)) return;

    const stats = statSync(LOG_FILE);
    if (stats.size < MAX_LOG_SIZE) return;

    // Rotate existing log files
    for (let i = MAX_LOG_FILES - 1; i >= 1; i--) {
      const oldFile = `${LOG_FILE}.${i}`;
      const newFile = `${LOG_FILE}.${i + 1}`;
      if (existsSync(oldFile)) {
        if (i === MAX_LOG_FILES - 1) {
          // Delete oldest
          require('fs').unlinkSync(oldFile);
        } else {
          renameSync(oldFile, newFile);
        }
      }
    }

    // Rename current log
    renameSync(LOG_FILE, `${LOG_FILE}.1`);
  } catch {
    // Rotation failed - continue anyway
  }
}

// =============================================================================
// LOGGER CLASS
// =============================================================================

class HookLogger {
  constructor(hookName) {
    this.hookName = hookName;
    this.startTime = Date.now();
    this.entries = [];
    ensureLogDir();
  }

  /**
   * Log a message
   *
   * @param {string} level - Log level (DEBUG, INFO, WARN, ERROR, CRITICAL)
   * @param {string} message - Log message
   * @param {Object} [data] - Additional data
   */
  log(level, message, data = null) {
    const levelConfig = LOG_LEVELS[level] || LOG_LEVELS.INFO;
    const timestamp = new Date().toISOString();
    const elapsed = Date.now() - this.startTime;

    const entry = {
      timestamp,
      level,
      hook: this.hookName,
      elapsed,
      message,
      data,
    };

    this.entries.push(entry);

    // Format for file
    const fileLine = `[${timestamp}] [${level}] [${this.hookName}] (${elapsed}ms) ${message}${data ? ' ' + JSON.stringify(data) : ''}\n`;

    // Write to file
    try {
      rotateIfNeeded();
      appendFileSync(LOG_FILE, fileLine);
    } catch {
      // Write failed - can't do much about it
    }

    // Also write to stderr if CYNIC_DEBUG is set
    if (process.env.CYNIC_DEBUG) {
      const colorLine = `${levelConfig.color}[${this.hookName}]${RESET} ${levelConfig.prefix} ${message}`;
      console.error(colorLine);
    }
  }

  debug(message, data) { this.log('DEBUG', message, data); }
  info(message, data) { this.log('INFO', message, data); }
  warn(message, data) { this.log('WARN', message, data); }
  error(message, data) { this.log('ERROR', message, data); }
  critical(message, data) { this.log('CRITICAL', message, data); }

  /**
   * Log hook start
   */
  start() {
    this.startTime = Date.now();
    this.info('Hook started');
  }

  /**
   * Log hook completion
   *
   * @param {Object} [result] - Result to log
   */
  complete(result = null) {
    const duration = Date.now() - this.startTime;
    this.info(`Hook completed in ${duration}ms`, result ? { resultType: typeof result } : null);
  }

  /**
   * Log hook failure
   *
   * @param {Error|string} error - Error that occurred
   */
  fail(error) {
    const duration = Date.now() - this.startTime;
    const message = error instanceof Error ? error.message : String(error);
    const stack = error instanceof Error ? error.stack : null;
    this.error(`Hook failed after ${duration}ms: ${message}`, stack ? { stack: stack.split('\n').slice(0, 3) } : null);
  }

  /**
   * Wrap an async function with logging
   *
   * @param {Function} fn - Async function to wrap
   * @returns {Promise} Result of function
   */
  async wrap(fn) {
    this.start();
    try {
      const result = await fn();
      this.complete(result);
      return result;
    } catch (error) {
      this.fail(error);
      throw error;
    }
  }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/**
 * Create a logger for a hook
 *
 * @param {string} hookName - Name of the hook
 * @returns {HookLogger} Logger instance
 */
export function createLogger(hookName) {
  return new HookLogger(hookName);
}

/**
 * Read recent log entries
 *
 * @param {number} [lines=50] - Number of lines to read
 * @returns {string[]} Log lines
 */
export function readRecentLogs(lines = 50) {
  try {
    if (!existsSync(LOG_FILE)) return [];

    const content = readFileSync(LOG_FILE, 'utf8');
    const allLines = content.split('\n').filter(l => l.trim());
    return allLines.slice(-lines);
  } catch {
    return [];
  }
}

/**
 * Parse log entries into structured data
 *
 * @param {number} [count=20] - Number of entries to parse
 * @returns {Object[]} Parsed log entries
 */
export function parseRecentLogs(count = 20) {
  const lines = readRecentLogs(count * 2); // Read extra for filtering
  const entries = [];

  const regex = /^\[([^\]]+)\] \[([^\]]+)\] \[([^\]]+)\] \((\d+)ms\) (.+)$/;

  for (const line of lines) {
    const match = line.match(regex);
    if (match) {
      const [, timestamp, level, hook, elapsed, rest] = match;

      // Try to parse data from rest
      let message = rest;
      let data = null;
      const jsonStart = rest.indexOf(' {');
      if (jsonStart > -1) {
        message = rest.slice(0, jsonStart);
        try {
          data = JSON.parse(rest.slice(jsonStart + 1));
        } catch {
          // Not valid JSON
        }
      }

      entries.push({
        timestamp: new Date(timestamp),
        level,
        hook,
        elapsed: parseInt(elapsed, 10),
        message,
        data,
      });
    }
  }

  return entries.slice(-count);
}

/**
 * Get log statistics
 *
 * @returns {Object} Log statistics
 */
export function getLogStats() {
  const entries = parseRecentLogs(100);

  const stats = {
    totalEntries: entries.length,
    byLevel: {},
    byHook: {},
    errors: [],
    avgElapsed: 0,
  };

  let totalElapsed = 0;

  for (const entry of entries) {
    // Count by level
    stats.byLevel[entry.level] = (stats.byLevel[entry.level] || 0) + 1;

    // Count by hook
    stats.byHook[entry.hook] = (stats.byHook[entry.hook] || 0) + 1;

    // Track errors
    if (entry.level === 'ERROR' || entry.level === 'CRITICAL') {
      stats.errors.push({
        hook: entry.hook,
        message: entry.message,
        timestamp: entry.timestamp,
      });
    }

    totalElapsed += entry.elapsed || 0;
  }

  stats.avgElapsed = entries.length > 0 ? Math.round(totalElapsed / entries.length) : 0;
  stats.errors = stats.errors.slice(-5); // Keep last 5 errors

  return stats;
}

// =============================================================================
// EXPORTS
// =============================================================================

export { HookLogger, LOG_LEVELS, LOG_FILE, LOG_DIR };

export default {
  createLogger,
  readRecentLogs,
  parseRecentLogs,
  getLogStats,
  HookLogger,
  LOG_LEVELS,
  LOG_FILE,
  LOG_DIR,
};
