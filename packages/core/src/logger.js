/**
 * CYNIC Structured Logger
 *
 * Provides structured logging with phi-aligned severity levels.
 * Replaces console.log/warn/error with contextual, parseable output.
 *
 * @module @cynic/core/logger
 */

'use strict';

/**
 * Log levels (phi-weighted)
 */
export const LogLevel = {
  TRACE: 0,    // Detailed debugging (phi^-3)
  DEBUG: 1,    // Development info (phi^-2)
  INFO: 2,     // Normal operations (phi^-1)
  WARN: 3,     // Warnings (1)
  ERROR: 4,    // Errors (phi)
  FATAL: 5,    // Critical failures (phi^2)
};

/**
 * Log level names
 */
const LEVEL_NAMES = ['TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR', 'FATAL'];

/**
 * ANSI colors for terminal output
 */
const COLORS = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  cyan: '\x1b[36m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  magenta: '\x1b[35m',
  green: '\x1b[32m',
};

/**
 * Logger configuration
 */
const config = {
  level: LogLevel.INFO,
  json: process.env.CYNIC_LOG_JSON === 'true',
  timestamps: process.env.CYNIC_LOG_TIMESTAMPS !== 'false',
  colors: process.stdout.isTTY !== false && process.env.NO_COLOR === undefined,
};

/**
 * Set minimum log level
 * @param {number|string} level - Level number or name
 */
export function setLogLevel(level) {
  if (typeof level === 'string') {
    const idx = LEVEL_NAMES.indexOf(level.toUpperCase());
    config.level = idx >= 0 ? idx : LogLevel.INFO;
  } else {
    config.level = level;
  }
}

/**
 * Enable/disable JSON output
 * @param {boolean} enabled
 */
export function setJsonOutput(enabled) {
  config.json = enabled;
}

/**
 * Format timestamp
 * @returns {string}
 */
function formatTimestamp() {
  const now = new Date();
  return now.toISOString();
}

/**
 * Color text for terminal
 * @param {string} text
 * @param {string} color
 * @returns {string}
 */
function colorize(text, color) {
  if (!config.colors) return text;
  return `${COLORS[color] || ''}${text}${COLORS.reset}`;
}

/**
 * Get level color
 * @param {number} level
 * @returns {string}
 */
function getLevelColor(level) {
  switch (level) {
    case LogLevel.TRACE: return 'dim';
    case LogLevel.DEBUG: return 'cyan';
    case LogLevel.INFO: return 'green';
    case LogLevel.WARN: return 'yellow';
    case LogLevel.ERROR: return 'red';
    case LogLevel.FATAL: return 'magenta';
    default: return 'reset';
  }
}

/**
 * Format log entry as JSON
 * @param {Object} entry
 * @returns {string}
 */
function formatJson(entry) {
  return JSON.stringify(entry);
}

/**
 * Format log entry as text
 * @param {Object} entry
 * @returns {string}
 */
function formatText(entry) {
  const { timestamp, level, context, message, ...extra } = entry;
  const parts = [];

  if (config.timestamps && timestamp) {
    parts.push(colorize(timestamp, 'dim'));
  }

  const levelName = LEVEL_NAMES[level] || 'INFO';
  parts.push(colorize(`[${levelName}]`, getLevelColor(level)));

  if (context) {
    parts.push(colorize(`[${context}]`, 'cyan'));
  }

  parts.push(message);

  // Add extra fields
  const extraKeys = Object.keys(extra);
  if (extraKeys.length > 0) {
    const extraStr = extraKeys.map(k => `${k}=${JSON.stringify(extra[k])}`).join(' ');
    parts.push(colorize(extraStr, 'dim'));
  }

  return parts.join(' ');
}

/**
 * Write log entry
 * @param {Object} entry
 */
function writeLog(entry) {
  if (entry.level < config.level) return;

  const output = config.json ? formatJson(entry) : formatText(entry);

  // ALL log output goes to stderr to keep stdout clean for structured data
  // (hooks, MCP tools, and CLI commands emit JSON on stdout)
  console.error(output);
}

/**
 * Create a log entry
 * @param {number} level
 * @param {string} context
 * @param {string} message
 * @param {Object} [extra]
 * @returns {Object}
 */
function createEntry(level, context, message, extra = {}) {
  const entry = {
    level,
    context,
    message,
  };

  if (config.timestamps) {
    entry.timestamp = formatTimestamp();
  }

  // Add extra fields
  Object.assign(entry, extra);

  return entry;
}

/**
 * Logger instance for a specific context
 */
export class Logger {
  /**
   * @param {string} context - Logger context (e.g., module name)
   */
  constructor(context) {
    this.context = context;
  }

  /**
   * Log at TRACE level
   * @param {string} message
   * @param {Object} [extra]
   */
  trace(message, extra) {
    writeLog(createEntry(LogLevel.TRACE, this.context, message, extra));
  }

  /**
   * Log at DEBUG level
   * @param {string} message
   * @param {Object} [extra]
   */
  debug(message, extra) {
    writeLog(createEntry(LogLevel.DEBUG, this.context, message, extra));
  }

  /**
   * Log at INFO level
   * @param {string} message
   * @param {Object} [extra]
   */
  info(message, extra) {
    writeLog(createEntry(LogLevel.INFO, this.context, message, extra));
  }

  /**
   * Log at WARN level
   * @param {string} message
   * @param {Object} [extra]
   */
  warn(message, extra) {
    writeLog(createEntry(LogLevel.WARN, this.context, message, extra));
  }

  /**
   * Log at ERROR level
   * @param {string} message
   * @param {Object} [extra]
   */
  error(message, extra) {
    writeLog(createEntry(LogLevel.ERROR, this.context, message, extra));
  }

  /**
   * Log at FATAL level
   * @param {string} message
   * @param {Object} [extra]
   */
  fatal(message, extra) {
    writeLog(createEntry(LogLevel.FATAL, this.context, message, extra));
  }

  /**
   * Create a child logger with extended context
   * @param {string} subContext
   * @returns {Logger}
   */
  child(subContext) {
    return new Logger(`${this.context}:${subContext}`);
  }
}

/**
 * Create a logger for a context
 * @param {string} context
 * @returns {Logger}
 */
export function createLogger(context) {
  return new Logger(context);
}

/**
 * Default logger instance
 */
export const logger = createLogger('CYNIC');

// Parse log level from environment
if (process.env.CYNIC_LOG_LEVEL) {
  setLogLevel(process.env.CYNIC_LOG_LEVEL);
}

export default {
  Logger,
  LogLevel,
  createLogger,
  setLogLevel,
  setJsonOutput,
  logger,
};
