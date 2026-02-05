/**
 * CYNIC Filesystem Watcher - Perception Layer
 *
 * Watches filesystem changes and emits events to the EventBus.
 * Part of the multi-dimensional awareness system.
 *
 * "The nose knows before the eyes see" - cynic
 *
 * @module @cynic/node/perception/filesystem-watcher
 */

'use strict';

import chokidar from 'chokidar';
import { createLogger } from '@cynic/core';
import { getEventBus } from '../services/event-bus.js';

const log = createLogger('FilesystemWatcher');

/**
 * Filesystem event types emitted to EventBus
 * @readonly
 * @enum {string}
 */
export const FilesystemEventType = {
  CHANGE: 'perception:fs:change',
  ADD: 'perception:fs:add',
  UNLINK: 'perception:fs:unlink',
  ADD_DIR: 'perception:fs:addDir',
  UNLINK_DIR: 'perception:fs:unlinkDir',
  ERROR: 'perception:fs:error',
  READY: 'perception:fs:ready',
};

/**
 * Default paths to ignore
 */
const DEFAULT_IGNORED = [
  /(^|[/\\])\../,           // dotfiles
  /node_modules/,
  /\.git/,
  /dist/,
  /build/,
  /coverage/,
  /\.next/,
  /\.nuxt/,
  /\.turbo/,
  /__pycache__/,
  /\.pytest_cache/,
  /target/,                  // Rust
  /\.cargo/,
];

/**
 * FilesystemWatcher - Watches filesystem and emits to EventBus
 *
 * Implements the perception layer for code changes.
 * Uses chokidar for cross-platform file watching.
 */
export class FilesystemWatcher {
  /**
   * Create a new FilesystemWatcher
   *
   * @param {Object} [options] - Configuration options
   * @param {string|string[]} [options.paths] - Paths to watch (default: cwd)
   * @param {EventBus} [options.eventBus] - EventBus instance
   * @param {RegExp[]} [options.ignored] - Patterns to ignore
   * @param {boolean} [options.persistent=true] - Keep process running
   * @param {boolean} [options.ignoreInitial=true] - Ignore initial add events
   * @param {number} [options.debounceMs=100] - Debounce rapid changes
   */
  constructor(options = {}) {
    this.paths = options.paths || [process.cwd()];
    this.eventBus = options.eventBus || getEventBus();
    this.ignored = options.ignored || DEFAULT_IGNORED;
    this.persistent = options.persistent ?? true;
    this.ignoreInitial = options.ignoreInitial ?? true;
    this.debounceMs = options.debounceMs ?? 100;

    this.watcher = null;
    this._isRunning = false;
    this._stats = {
      eventsEmitted: 0,
      filesWatched: 0,
      errors: 0,
      startedAt: null,
    };

    // Debounce map for rapid-fire events
    this._debounceTimers = new Map();
  }

  /**
   * Start watching filesystem
   *
   * @returns {FilesystemWatcher} this (for chaining)
   */
  start() {
    if (this._isRunning) {
      log.warn('FilesystemWatcher already running');
      return this;
    }

    log.info('Starting filesystem watcher', { paths: this.paths });

    this.watcher = chokidar.watch(this.paths, {
      ignored: this.ignored,
      persistent: this.persistent,
      ignoreInitial: this.ignoreInitial,
      awaitWriteFinish: {
        stabilityThreshold: this.debounceMs,
        pollInterval: 50,
      },
    });

    // Wire up events
    this.watcher
      .on('change', (path) => this._emitDebounced('change', path))
      .on('add', (path) => this._emitDebounced('add', path))
      .on('unlink', (path) => this._emit('unlink', path))
      .on('addDir', (path) => this._emit('addDir', path))
      .on('unlinkDir', (path) => this._emit('unlinkDir', path))
      .on('error', (error) => this._emitError(error))
      .on('ready', () => this._emitReady());

    this._isRunning = true;
    this._stats.startedAt = Date.now();

    return this;
  }

  /**
   * Stop watching filesystem
   *
   * @returns {Promise<void>}
   */
  async stop() {
    if (!this._isRunning || !this.watcher) {
      return;
    }

    log.info('Stopping filesystem watcher');

    // Clear all debounce timers
    for (const timer of this._debounceTimers.values()) {
      clearTimeout(timer);
    }
    this._debounceTimers.clear();

    await this.watcher.close();
    this.watcher = null;
    this._isRunning = false;
  }

  /**
   * Check if watcher is running
   *
   * @returns {boolean}
   */
  isRunning() {
    return this._isRunning;
  }

  /**
   * Get watcher statistics
   *
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      ...this._stats,
      isRunning: this._isRunning,
      uptime: this._stats.startedAt ? Date.now() - this._stats.startedAt : 0,
    };
  }

  /**
   * Emit event with debouncing
   *
   * @private
   * @param {string} action - Event action
   * @param {string} path - File path
   */
  _emitDebounced(action, path) {
    const key = `${action}:${path}`;

    // Clear existing timer
    if (this._debounceTimers.has(key)) {
      clearTimeout(this._debounceTimers.get(key));
    }

    // Set new timer
    const timer = setTimeout(() => {
      this._debounceTimers.delete(key);
      this._emit(action, path);
    }, this.debounceMs);

    this._debounceTimers.set(key, timer);
  }

  /**
   * Emit event to EventBus
   *
   * @private
   * @param {string} action - Event action
   * @param {string} path - File path
   */
  _emit(action, path) {
    const eventType = FilesystemEventType[action.toUpperCase()] || `perception:fs:${action}`;

    const data = {
      path,
      action,
      timestamp: Date.now(),
      extension: this._getExtension(path),
      isCode: this._isCodeFile(path),
    };

    this.eventBus.publish(eventType, data, {
      source: 'FilesystemWatcher',
    });

    this._stats.eventsEmitted++;

    log.debug('Filesystem event', { action, path });
  }

  /**
   * Emit error event
   *
   * @private
   * @param {Error} error - Error object
   */
  _emitError(error) {
    log.error('Filesystem watcher error', { error: error.message });

    this.eventBus.publish(FilesystemEventType.ERROR, {
      error: error.message,
      stack: error.stack,
      timestamp: Date.now(),
    }, {
      source: 'FilesystemWatcher',
    });

    this._stats.errors++;
  }

  /**
   * Emit ready event
   *
   * @private
   */
  _emitReady() {
    const watched = this.watcher.getWatched();
    let fileCount = 0;

    for (const files of Object.values(watched)) {
      fileCount += files.length;
    }

    this._stats.filesWatched = fileCount;

    log.info('Filesystem watcher ready', { filesWatched: fileCount });

    this.eventBus.publish(FilesystemEventType.READY, {
      paths: this.paths,
      filesWatched: fileCount,
      timestamp: Date.now(),
    }, {
      source: 'FilesystemWatcher',
    });
  }

  /**
   * Get file extension from path
   *
   * @private
   * @param {string} path - File path
   * @returns {string} Extension (lowercase, without dot)
   */
  _getExtension(path) {
    const match = path.match(/\.([^./\\]+)$/);
    return match ? match[1].toLowerCase() : '';
  }

  /**
   * Check if path is a code file
   *
   * @private
   * @param {string} path - File path
   * @returns {boolean}
   */
  _isCodeFile(path) {
    const codeExtensions = new Set([
      'js', 'jsx', 'ts', 'tsx', 'mjs', 'cjs',
      'py', 'pyi',
      'rs',
      'go',
      'java', 'kt', 'scala',
      'c', 'cpp', 'cc', 'h', 'hpp',
      'cs',
      'rb',
      'php',
      'swift',
      'sol',          // Solana/Ethereum
      'move',         // Move language
      'anchor',       // Anchor IDL
      'json', 'yaml', 'yml', 'toml',
      'md', 'mdx',
      'html', 'css', 'scss', 'sass', 'less',
      'sql',
      'sh', 'bash', 'zsh',
      'dockerfile',
    ]);

    return codeExtensions.has(this._getExtension(path));
  }
}

/**
 * Create a new FilesystemWatcher
 *
 * @param {Object} [options] - Options
 * @returns {FilesystemWatcher}
 */
export function createFilesystemWatcher(options) {
  return new FilesystemWatcher(options);
}

export default FilesystemWatcher;
