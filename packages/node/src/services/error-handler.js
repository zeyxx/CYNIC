/**
 * CYNIC Error Handler
 *
 * Unified error handling with consciousness integration.
 * Catches, categorizes, and routes errors to awareness system.
 *
 * "Le chien sent le danger" - CYNIC knows when something's wrong
 *
 * @module @cynic/node/services/error-handler
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Error severity levels
 */
export const ErrorSeverity = {
  INFO: 'info',
  WARN: 'warn',
  ERROR: 'error',
  CRITICAL: 'critical',
};

/**
 * Error categories for classification
 */
export const ErrorCategory = {
  NETWORK: 'network',
  DATABASE: 'database',
  VALIDATION: 'validation',
  PERMISSION: 'permission',
  TIMEOUT: 'timeout',
  RESOURCE: 'resource',
  LOGIC: 'logic',
  EXTERNAL: 'external',
  UNKNOWN: 'unknown',
};

/**
 * Classify error by its properties
 */
function classifyError(error) {
  const message = error.message?.toLowerCase() || '';
  const name = error.name?.toLowerCase() || '';

  if (message.includes('network') || message.includes('econnrefused') ||
      message.includes('enotfound') || name.includes('network')) {
    return ErrorCategory.NETWORK;
  }
  if (message.includes('database') || message.includes('postgres') ||
      message.includes('sql') || message.includes('connection')) {
    return ErrorCategory.DATABASE;
  }
  if (message.includes('timeout') || name.includes('timeout')) {
    return ErrorCategory.TIMEOUT;
  }
  if (message.includes('permission') || message.includes('unauthorized') ||
      message.includes('forbidden') || error.code === 'EACCES') {
    return ErrorCategory.PERMISSION;
  }
  if (message.includes('validation') || message.includes('invalid') ||
      name.includes('validation') || name.includes('typeerror')) {
    return ErrorCategory.VALIDATION;
  }
  if (message.includes('memory') || message.includes('heap') ||
      error.code === 'ENOMEM') {
    return ErrorCategory.RESOURCE;
  }
  if (message.includes('external') || message.includes('api') ||
      message.includes('service')) {
    return ErrorCategory.EXTERNAL;
  }
  if (name === 'referenceerror' || name === 'syntaxerror') {
    return ErrorCategory.LOGIC;
  }

  return ErrorCategory.UNKNOWN;
}

/**
 * Determine severity from error and context
 */
function determineSeverity(error, context = {}) {
  // Explicit severity takes precedence
  if (context.severity) return context.severity;

  const category = classifyError(error);

  // Critical categories
  if (category === ErrorCategory.DATABASE && !context.recoverable) {
    return ErrorSeverity.CRITICAL;
  }
  if (category === ErrorCategory.RESOURCE) {
    return ErrorSeverity.CRITICAL;
  }

  // High severity categories
  if (category === ErrorCategory.PERMISSION) {
    return ErrorSeverity.ERROR;
  }
  if (category === ErrorCategory.NETWORK && context.retries === 0) {
    return ErrorSeverity.ERROR;
  }

  // Warnings
  if (category === ErrorCategory.VALIDATION) {
    return ErrorSeverity.WARN;
  }
  if (category === ErrorCategory.TIMEOUT && context.retryable) {
    return ErrorSeverity.WARN;
  }

  return ErrorSeverity.ERROR;
}

/**
 * Error Handler - Centralized error management
 */
export class ErrorHandler extends EventEmitter {
  /**
   * @param {Object} options
   * @param {ConsciousnessBridge} [options.consciousness] - Bridge for awareness
   * @param {Object} [options.alertManager] - For critical alerts
   * @param {boolean} [options.captureGlobal=true] - Capture global errors
   * @param {boolean} [options.exitOnUncaught=false] - Exit process on uncaught
   */
  constructor(options = {}) {
    super();
    this.consciousness = options.consciousness || null;
    this.alertManager = options.alertManager || null;
    this.captureGlobal = options.captureGlobal !== false;
    this.exitOnUncaught = options.exitOnUncaught || false;

    // Error statistics
    this.stats = {
      total: 0,
      bySeverity: { info: 0, warn: 0, error: 0, critical: 0 },
      byCategory: {},
      byComponent: {},
      unhandledRejections: 0,
      uncaughtExceptions: 0,
    };

    // Rate limiting for repeated errors
    this._errorThrottle = new Map();
    this._throttleWindowMs = 60000; // 1 minute
    this._maxErrorsPerWindow = 10;

    // Setup global handlers if requested
    if (this.captureGlobal) {
      this._setupGlobalHandlers();
    }
  }

  /**
   * Set dependencies (for late binding)
   */
  setDependencies({ consciousness, alertManager }) {
    if (consciousness) this.consciousness = consciousness;
    if (alertManager) this.alertManager = alertManager;
  }

  /**
   * Setup global error handlers
   * @private
   */
  _setupGlobalHandlers() {
    // Unhandled Promise rejections
    process.on('unhandledRejection', (reason, promise) => {
      const error = reason instanceof Error
        ? reason
        : new Error(String(reason));
      error.name = 'UnhandledRejection';

      this.stats.unhandledRejections++;
      this.handle(error, {
        component: 'process',
        severity: ErrorSeverity.CRITICAL,
        isUnhandled: true,
        promise: String(promise),
      });
    });

    // Uncaught exceptions
    process.on('uncaughtException', (error) => {
      this.stats.uncaughtExceptions++;
      this.handle(error, {
        component: 'process',
        severity: ErrorSeverity.CRITICAL,
        isUnhandled: true,
      });

      if (this.exitOnUncaught) {
        console.error('[ErrorHandler] Uncaught exception - exiting');
        process.exit(1);
      }
    });
  }

  /**
   * Handle an error
   *
   * @param {Error} error - The error to handle
   * @param {Object} context - Additional context
   * @param {string} [context.component] - Component that threw
   * @param {string} [context.severity] - Override severity
   * @param {boolean} [context.isUnhandled] - Was this unhandled?
   * @param {boolean} [context.recoverable] - Is this recoverable?
   * @param {number} [context.retries] - Remaining retries
   * @returns {Object} Processed error info
   */
  handle(error, context = {}) {
    const {
      component = 'unknown',
      isUnhandled = false,
      recoverable = true,
      retries,
    } = context;

    // Classify and determine severity
    const category = classifyError(error);
    const severity = determineSeverity(error, { ...context, category });

    // Check rate limiting
    const throttleKey = `${component}:${error.name}`;
    if (this._isThrottled(throttleKey)) {
      return { throttled: true, error, severity };
    }

    // Update statistics
    this.stats.total++;
    this.stats.bySeverity[severity] = (this.stats.bySeverity[severity] || 0) + 1;
    this.stats.byCategory[category] = (this.stats.byCategory[category] || 0) + 1;
    this.stats.byComponent[component] = (this.stats.byComponent[component] || 0) + 1;

    // Build error info
    const errorInfo = {
      error,
      name: error.name,
      message: error.message,
      stack: error.stack,
      category,
      severity,
      component,
      isUnhandled,
      recoverable,
      timestamp: Date.now(),
    };

    // Log based on severity
    this._log(errorInfo);

    // Notify consciousness
    if (this.consciousness) {
      this.consciousness.observeError(error, {
        component,
        severity,
        isUnhandled,
      });
    }

    // Emit for listeners (using 'handled_error' to avoid Node's special 'error' event)
    this.emit('handled_error', errorInfo);
    this.emit(`handled_error:${severity}`, errorInfo);
    this.emit(`handled_error:${category}`, errorInfo);

    // Critical alerts
    if (severity === ErrorSeverity.CRITICAL) {
      this._handleCritical(errorInfo);
    }

    return errorInfo;
  }

  /**
   * Check if error should be throttled
   * @private
   */
  _isThrottled(key) {
    const now = Date.now();
    const record = this._errorThrottle.get(key);

    if (!record || now - record.windowStart > this._throttleWindowMs) {
      // New window
      this._errorThrottle.set(key, { windowStart: now, count: 1 });
      return false;
    }

    record.count++;
    if (record.count > this._maxErrorsPerWindow) {
      // First time hitting threshold, emit warning
      if (record.count === this._maxErrorsPerWindow + 1) {
        console.warn(`[ErrorHandler] Throttling errors for: ${key}`);
      }
      return true;
    }

    return false;
  }

  /**
   * Log error based on severity
   * @private
   */
  _log(errorInfo) {
    const prefix = `[ErrorHandler] [${errorInfo.severity.toUpperCase()}]`;
    const msg = `${prefix} ${errorInfo.component}: ${errorInfo.message}`;

    switch (errorInfo.severity) {
      case ErrorSeverity.INFO:
        console.log(msg);
        break;
      case ErrorSeverity.WARN:
        console.warn(msg);
        break;
      case ErrorSeverity.CRITICAL:
        console.error(msg);
        console.error(errorInfo.stack);
        break;
      default:
        console.error(msg);
    }
  }

  /**
   * Handle critical errors
   * @private
   */
  _handleCritical(errorInfo) {
    // Alert manager
    if (this.alertManager) {
      this.alertManager.critical?.(
        `Critical error in ${errorInfo.component}`,
        errorInfo
      );
    }

    // Emit special event
    this.emit('critical', errorInfo);
  }

  /**
   * Create a wrapped function that auto-reports errors
   *
   * @param {Function} fn - Function to wrap
   * @param {string} component - Component name
   * @returns {Function} Wrapped function
   */
  wrap(fn, component) {
    const handler = this;
    return async function wrapped(...args) {
      try {
        return await fn.apply(this, args);
      } catch (error) {
        handler.handle(error, { component });
        throw error;
      }
    };
  }

  /**
   * Create error boundary for async operations
   *
   * @param {string} component - Component name
   * @returns {Function} Boundary function
   */
  boundary(component) {
    const handler = this;
    return async (operation) => {
      try {
        return await operation();
      } catch (error) {
        handler.handle(error, { component, recoverable: true });
        return null;
      }
    };
  }

  /**
   * Get error statistics
   */
  getStats() {
    return {
      ...this.stats,
      throttledPatterns: this._errorThrottle.size,
    };
  }

  /**
   * Get recent errors by category
   */
  getRecentByCategory(category, limit = 10) {
    // Would need error history storage - for now return stats
    return {
      category,
      count: this.stats.byCategory[category] || 0,
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      total: 0,
      bySeverity: { info: 0, warn: 0, error: 0, critical: 0 },
      byCategory: {},
      byComponent: {},
      unhandledRejections: 0,
      uncaughtExceptions: 0,
    };
    this._errorThrottle.clear();
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create the ErrorHandler singleton
 */
export function getErrorHandler(options = {}) {
  if (!_instance) {
    _instance = new ErrorHandler(options);
  }
  return _instance;
}

/**
 * Reset the singleton (for testing)
 */
export function resetErrorHandler() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  ErrorHandler,
  ErrorSeverity,
  ErrorCategory,
  getErrorHandler,
  resetErrorHandler,
};
