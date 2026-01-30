/**
 * BaseHook - Abstract Base Class for CYNIC Hooks
 *
 * Provides common functionality for all hooks:
 * - stdin parsing
 * - stdout response formatting
 * - Error handling
 * - Logging
 *
 * "The foundation all dogs stand on" - κυνικός
 *
 * @module scripts/hooks/lib/base-hook
 */

'use strict';

/**
 * Hook types (Claude Code hook events)
 */
export const HookType = Object.freeze({
  PRE_TOOL_USE: 'PreToolUse',
  POST_TOOL_USE: 'PostToolUse',
  STOP: 'Stop',
  SUBAGENT_STOP: 'SubagentStop',
  SESSION_START: 'SessionStart',
  SESSION_END: 'SessionEnd',
  USER_PROMPT_SUBMIT: 'UserPromptSubmit',
  PRE_COMPACT: 'PreCompact',
  NOTIFICATION: 'Notification',
});

/**
 * Hook decision types
 */
export const Decision = Object.freeze({
  ALLOW: 'allow',
  BLOCK: 'block',
  MODIFY: 'modify',
});

/**
 * BaseHook - Abstract base class for CYNIC hooks
 *
 * @example
 * class MyHook extends BaseHook {
 *   constructor() {
 *     super('MyHook', HookType.POST_TOOL_USE);
 *   }
 *
 *   async handle(context) {
 *     // Process hook
 *     return this.allow();
 *   }
 * }
 */
export class BaseHook {
  /**
   * Default timeout for hook handlers (5 seconds)
   */
  static DEFAULT_TIMEOUT_MS = 5000;

  /**
   * Create a hook
   *
   * @param {string} name - Hook name for logging
   * @param {string} hookType - Hook type (from HookType enum)
   * @param {Object} [options] - Hook options
   * @param {boolean} [options.blocking=false] - Whether hook blocks execution
   * @param {boolean} [options.silent=true] - Suppress logging
   * @param {number} [options.timeoutMs=5000] - Handler timeout in milliseconds
   */
  constructor(name, hookType, options = {}) {
    this._name = name;
    this._hookType = hookType;
    this._blocking = options.blocking ?? false;
    this._silent = options.silent ?? true;
    this._timeoutMs = options.timeoutMs ?? BaseHook.DEFAULT_TIMEOUT_MS;
    this._startTime = Date.now();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Accessors
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {string} Hook name */
  get name() {
    return this._name;
  }

  /** @returns {string} Hook type */
  get hookType() {
    return this._hookType;
  }

  /** @returns {boolean} Whether blocking */
  get isBlocking() {
    return this._blocking;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Input/Output
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Read and parse hook context from stdin
   * Uses robust sync-first approach for ESM compatibility
   *
   * @returns {Promise<Object>} Parsed context
   */
  async readContext() {
    const fs = await import('fs');
    let input = '';

    // Try synchronous read first (works when piped before module load)
    try {
      input = fs.readFileSync(0, 'utf8');
      if (process.env.CYNIC_DEBUG) this.log('debug', 'Sync read', { bytes: input.length });
    } catch (syncErr) {
      if (process.env.CYNIC_DEBUG) this.log('debug', 'Sync failed', { error: syncErr.message });
      // Fall back to async read with timeout
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
      if (process.env.CYNIC_DEBUG) this.log('debug', 'Async read', { bytes: input.length });
    }

    if (!input.trim()) {
      return {};
    }

    try {
      return JSON.parse(input);
    } catch (e) {
      this.log('warn', 'Failed to parse context', { error: e.message });
      return { raw: input };
    }
  }

  /**
   * Write response to stdout (handles EPIPE gracefully)
   *
   * @param {Object} response - Response object
   */
  respond(response) {
    try {
      const str = JSON.stringify(response);
      process.stdout.write(str + '\n');
    } catch (e) {
      if (e.code === 'EPIPE') process.exit(0);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Response Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create an "allow" response
   *
   * @param {Object} [options] - Options
   * @param {string} [options.reason] - Reason for allowing
   * @returns {Object} Allow response
   */
  allow(options = {}) {
    return {
      decision: Decision.ALLOW,
      reason: options.reason,
      hookName: this._name,
      duration: Date.now() - this._startTime,
    };
  }

  /**
   * Create a "block" response
   *
   * @param {string} reason - Reason for blocking
   * @param {Object} [options] - Options
   * @returns {Object} Block response
   */
  block(reason, options = {}) {
    return {
      decision: Decision.BLOCK,
      reason,
      hookName: this._name,
      duration: Date.now() - this._startTime,
      ...options,
    };
  }

  /**
   * Create a "modify" response
   *
   * @param {Object} modifications - Modifications to apply
   * @param {Object} [options] - Options
   * @returns {Object} Modify response
   */
  modify(modifications, options = {}) {
    return {
      decision: Decision.MODIFY,
      modifications,
      hookName: this._name,
      duration: Date.now() - this._startTime,
      ...options,
    };
  }

  /**
   * Create a "continue" response (alias for allow, for PostToolUse)
   *
   * @returns {Object} Continue response
   */
  continue() {
    return this.allow({ reason: 'continue' });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Logging
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Log a message (if not silent)
   *
   * @param {string} level - Log level (debug, info, warn, error)
   * @param {string} message - Message
   * @param {Object} [data] - Additional data
   */
  log(level, message, data) {
    if (this._silent && level !== 'error') return;

    const entry = {
      hook: this._name,
      level,
      message,
      ...data,
      timestamp: new Date().toISOString(),
    };

    // Write to stderr to not interfere with stdout response
    console.error(JSON.stringify(entry));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Abstract Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Handle the hook event
   * Must be implemented by subclasses
   *
   * @param {Object} context - Hook context from stdin
   * @returns {Promise<Object>} Response object
   * @abstract
   */
  async handle(context) {
    throw new Error('handle() must be implemented by subclass');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Timeout Utilities
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Wrap a promise with a timeout
   *
   * @param {Promise} promise - Promise to wrap
   * @param {number} timeoutMs - Timeout in milliseconds
   * @param {string} [operation='operation'] - Operation name for error message
   * @returns {Promise} Promise that rejects on timeout
   */
  async _withTimeout(promise, timeoutMs, operation = 'operation') {
    let timeoutId;
    const timeoutPromise = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        reject(new Error(`Hook ${operation} timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });

    try {
      const result = await Promise.race([promise, timeoutPromise]);
      clearTimeout(timeoutId);
      return result;
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Runner
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Run the hook
   * Reads context, calls handle() with timeout, writes response
   *
   * @returns {Promise<void>}
   */
  async run() {
    try {
      const context = await this.readContext();

      // Run handler with timeout protection
      const response = await this._withTimeout(
        this.handle(context),
        this._timeoutMs,
        'handler'
      );

      if (response) {
        this.respond(response);
      }

      // Exit successfully
      process.exit(0);
    } catch (error) {
      const isTimeout = error.message?.includes('timed out');
      this.log('error', isTimeout ? 'Hook timed out' : 'Hook failed', {
        error: error.message,
        stack: error.stack,
        timeoutMs: this._timeoutMs,
      });

      // For blocking hooks, fail safe by allowing
      if (this._blocking) {
        this.respond(this.allow({ reason: isTimeout ? 'timeout_failsafe' : 'error_failsafe' }));
      }

      process.exit(1);
    }
  }
}

/**
 * Create and run a hook in one call
 *
 * @param {Object} options - Hook options
 * @param {string} options.name - Hook name
 * @param {string} options.type - Hook type
 * @param {Function} options.handler - Handler function (context) => response
 * @param {boolean} [options.blocking=false] - Whether blocking
 * @param {number} [options.timeoutMs=5000] - Handler timeout in milliseconds
 *
 * @example
 * runHook({
 *   name: 'MyHook',
 *   type: HookType.POST_TOOL_USE,
 *   handler: async (context) => {
 *     console.error('Observed:', context.tool_name);
 *     return null; // No response needed
 *   },
 * });
 */
export async function runHook(options) {
  const { name, type, handler, blocking = false, timeoutMs } = options;

  class InlineHook extends BaseHook {
    constructor() {
      super(name, type, { blocking, timeoutMs });
    }

    async handle(context) {
      return handler(context);
    }
  }

  const hook = new InlineHook();
  await hook.run();
}

export default BaseHook;
