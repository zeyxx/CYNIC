/**
 * DimensionRegistry - Runtime Extension System for Judgment Dimensions
 *
 * Enables plugins to register custom dimensions with scorers at runtime.
 * Part of Architecture Transformation Phase 3.
 *
 * "The 25 dimensions are a floor, not a ceiling" - κυνικός
 *
 * @module @cynic/node/judge/dimension-registry
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, AXIOMS } from '@cynic/core';
import { Dimensions as CoreDimensions, getAllDimensions } from './dimensions.js';

/**
 * Dimension validation schema
 */
const DimensionSchema = {
  required: ['weight', 'threshold', 'description'],
  optional: ['scorer', 'meta', 'formula', 'tags', 'version'],
  weightRange: { min: 0.1, max: 2.0 },
  thresholdRange: { min: 0, max: 100 },
};

/**
 * DimensionRegistry - Central registry for judgment dimensions
 *
 * Features:
 * - Register custom dimensions with scorers
 * - Event-driven architecture
 * - Validation and type safety
 * - Plugin API for extensions
 * - Backward compatible with static Dimensions
 *
 * @extends EventEmitter
 */
export class DimensionRegistry extends EventEmitter {
  /**
   * Create dimension registry
   *
   * @param {Object} [options] - Registry options
   * @param {boolean} [options.loadCore=true] - Load core dimensions on init
   * @param {boolean} [options.strict=true] - Strict validation mode
   */
  constructor(options = {}) {
    super();

    // Dimension storage: axiom -> name -> config
    this._dimensions = new Map();

    // Scorer storage: dimensionKey -> scorer function
    this._scorers = new Map();

    // Plugin metadata
    this._plugins = new Map();

    // Options
    this._options = {
      loadCore: options.loadCore !== false,
      strict: options.strict !== false,
    };

    // Stats
    this._stats = {
      coreLoaded: 0,
      customRegistered: 0,
      scorersRegistered: 0,
      pluginsLoaded: 0,
    };

    // Load core dimensions
    if (this._options.loadCore) {
      this._loadCoreDimensions();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Core Dimension Loading
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Load core dimensions from static definitions
   * @private
   */
  _loadCoreDimensions() {
    for (const [axiom, dims] of Object.entries(CoreDimensions)) {
      if (!this._dimensions.has(axiom)) {
        this._dimensions.set(axiom, new Map());
      }

      const axiomMap = this._dimensions.get(axiom);

      for (const [name, config] of Object.entries(dims)) {
        axiomMap.set(name, {
          ...config,
          axiom,
          core: true,
          registeredAt: Date.now(),
        });
        this._stats.coreLoaded++;
      }
    }

    this.emit('core:loaded', { count: this._stats.coreLoaded });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Dimension Registration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register a custom dimension
   *
   * @param {string} axiom - Axiom (PHI, VERIFY, CULTURE, BURN, META)
   * @param {string} name - Dimension name (UPPER_SNAKE_CASE)
   * @param {Object} config - Dimension configuration
   * @param {number} config.weight - Score weight (0.1-2.0)
   * @param {number} config.threshold - Minimum threshold (0-100)
   * @param {string} config.description - Human-readable description
   * @param {Function} [config.scorer] - Custom scorer function(item, context) => number
   * @param {string[]} [config.tags] - Tags for filtering/grouping
   * @param {string} [config.version] - Dimension version
   * @returns {boolean} Success
   *
   * @example
   * registry.register('VERIFY', 'BLOCKCHAIN_PROOF', {
   *   weight: 1.5,
   *   threshold: 70,
   *   description: 'On-chain verification available',
   *   scorer: (item) => item.onchainProof ? 100 : 0,
   *   tags: ['crypto', 'verification'],
   * });
   */
  register(axiom, name, config) {
    // Validate axiom
    if (!AXIOMS[axiom] && axiom !== 'META') {
      if (this._options.strict) {
        throw new Error(`Invalid axiom: ${axiom}. Must be one of: ${Object.keys(AXIOMS).join(', ')}, META`);
      }
      return false;
    }

    // Validate name format
    if (!/^[A-Z][A-Z0-9_]*$/.test(name)) {
      if (this._options.strict) {
        throw new Error(`Invalid dimension name: ${name}. Must be UPPER_SNAKE_CASE`);
      }
      return false;
    }

    // Validate config
    const validation = this._validateConfig(config);
    if (!validation.valid) {
      if (this._options.strict) {
        throw new Error(`Invalid dimension config: ${validation.errors.join(', ')}`);
      }
      return false;
    }

    // Ensure axiom map exists
    if (!this._dimensions.has(axiom)) {
      this._dimensions.set(axiom, new Map());
    }

    const axiomMap = this._dimensions.get(axiom);
    const dimensionKey = `${axiom}.${name}`;

    // Check for duplicate
    if (axiomMap.has(name)) {
      this.emit('dimension:overwritten', { axiom, name, dimensionKey });
    }

    // Store dimension
    axiomMap.set(name, {
      ...config,
      axiom,
      core: false,
      registeredAt: Date.now(),
    });

    // Store scorer if provided
    if (config.scorer && typeof config.scorer === 'function') {
      this._scorers.set(dimensionKey, config.scorer);
      this._stats.scorersRegistered++;
    }

    this._stats.customRegistered++;

    this.emit('dimension:registered', {
      axiom,
      name,
      dimensionKey,
      hasScorer: !!config.scorer,
    });

    return true;
  }

  /**
   * Validate dimension config
   * @private
   */
  _validateConfig(config) {
    const errors = [];

    // Required fields
    for (const field of DimensionSchema.required) {
      if (config[field] === undefined) {
        errors.push(`Missing required field: ${field}`);
      }
    }

    // Weight range
    if (config.weight !== undefined) {
      if (config.weight < DimensionSchema.weightRange.min ||
          config.weight > DimensionSchema.weightRange.max) {
        errors.push(`Weight ${config.weight} out of range [${DimensionSchema.weightRange.min}-${DimensionSchema.weightRange.max}]`);
      }
    }

    // Threshold range
    if (config.threshold !== undefined) {
      if (config.threshold < DimensionSchema.thresholdRange.min ||
          config.threshold > DimensionSchema.thresholdRange.max) {
        errors.push(`Threshold ${config.threshold} out of range [${DimensionSchema.thresholdRange.min}-${DimensionSchema.thresholdRange.max}]`);
      }
    }

    // Scorer type
    if (config.scorer !== undefined && typeof config.scorer !== 'function') {
      errors.push('Scorer must be a function');
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Unregister a custom dimension
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @returns {boolean} Success
   */
  unregister(axiom, name) {
    const axiomMap = this._dimensions.get(axiom);
    if (!axiomMap) return false;

    const dim = axiomMap.get(name);
    if (!dim) return false;

    // Cannot unregister core dimensions
    if (dim.core) {
      if (this._options.strict) {
        throw new Error(`Cannot unregister core dimension: ${axiom}.${name}`);
      }
      return false;
    }

    const dimensionKey = `${axiom}.${name}`;

    axiomMap.delete(name);
    this._scorers.delete(dimensionKey);
    this._stats.customRegistered--;

    this.emit('dimension:unregistered', { axiom, name, dimensionKey });

    return true;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Dimension Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get a dimension by axiom and name
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @returns {Object|null} Dimension config or null
   */
  get(axiom, name) {
    const axiomMap = this._dimensions.get(axiom);
    if (!axiomMap) return null;
    return axiomMap.get(name) || null;
  }

  /**
   * Get dimension by full key
   *
   * @param {string} key - Full key (AXIOM.NAME)
   * @returns {Object|null} Dimension config or null
   */
  getByKey(key) {
    const [axiom, name] = key.split('.');
    return this.get(axiom, name);
  }

  /**
   * Get all dimensions for an axiom
   *
   * @param {string} axiom - Axiom
   * @returns {Object} Dimensions object { name: config }
   */
  getForAxiom(axiom) {
    const axiomMap = this._dimensions.get(axiom);
    if (!axiomMap) return {};

    const result = {};
    for (const [name, config] of axiomMap) {
      result[name] = config;
    }
    return result;
  }

  /**
   * Get all dimensions flat
   *
   * @returns {Object} All dimensions { name: { ...config, axiom } }
   */
  getAll() {
    const result = {};
    for (const [axiom, axiomMap] of this._dimensions) {
      for (const [name, config] of axiomMap) {
        result[name] = { ...config, axiom };
      }
    }
    return result;
  }

  /**
   * Get dimensions by tag
   *
   * @param {string} tag - Tag to filter by
   * @returns {Array} Dimensions with tag
   */
  getByTag(tag) {
    const result = [];
    for (const [axiom, axiomMap] of this._dimensions) {
      for (const [name, config] of axiomMap) {
        if (config.tags?.includes(tag)) {
          result.push({ axiom, name, ...config });
        }
      }
    }
    return result;
  }

  /**
   * Check if dimension exists
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @returns {boolean} Whether dimension exists
   */
  has(axiom, name) {
    const axiomMap = this._dimensions.get(axiom);
    return axiomMap?.has(name) || false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Scorer Access
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get scorer for a dimension
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @returns {Function|null} Scorer function or null
   */
  getScorer(axiom, name) {
    const dimensionKey = `${axiom}.${name}`;
    return this._scorers.get(dimensionKey) || null;
  }

  /**
   * Register a scorer for an existing dimension
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @param {Function} scorer - Scorer function(item, context) => number
   * @returns {boolean} Success
   */
  registerScorer(axiom, name, scorer) {
    if (!this.has(axiom, name)) {
      if (this._options.strict) {
        throw new Error(`Dimension not found: ${axiom}.${name}`);
      }
      return false;
    }

    if (typeof scorer !== 'function') {
      if (this._options.strict) {
        throw new Error('Scorer must be a function');
      }
      return false;
    }

    const dimensionKey = `${axiom}.${name}`;
    this._scorers.set(dimensionKey, scorer);
    this._stats.scorersRegistered++;

    this.emit('scorer:registered', { axiom, name, dimensionKey });

    return true;
  }

  /**
   * Score an item with a specific dimension
   *
   * @param {string} axiom - Axiom
   * @param {string} name - Dimension name
   * @param {Object} item - Item to score
   * @param {Object} [context={}] - Scoring context
   * @returns {number|null} Score (0-100) or null if no scorer
   */
  score(axiom, name, item, context = {}) {
    const scorer = this.getScorer(axiom, name);
    if (!scorer) return null;

    try {
      const score = scorer(item, context);

      this.emit('score:calculated', {
        axiom,
        name,
        score,
        itemType: item?.type,
      });

      return Math.max(0, Math.min(100, score));
    } catch (err) {
      this.emit('score:error', {
        axiom,
        name,
        error: err.message,
      });
      return null;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Plugin API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register a dimension plugin
   *
   * @param {Object} plugin - Plugin definition
   * @param {string} plugin.name - Plugin name
   * @param {string} plugin.version - Plugin version
   * @param {Array} plugin.dimensions - Dimensions to register
   * @returns {boolean} Success
   *
   * @example
   * registry.registerPlugin({
   *   name: 'crypto-dimensions',
   *   version: '1.0.0',
   *   dimensions: [
   *     { axiom: 'VERIFY', name: 'ONCHAIN_PROOF', config: { ... } },
   *     { axiom: 'BURN', name: 'TOKEN_BURNED', config: { ... } },
   *   ],
   * });
   */
  registerPlugin(plugin) {
    if (!plugin.name || !plugin.version || !Array.isArray(plugin.dimensions)) {
      if (this._options.strict) {
        throw new Error('Plugin must have name, version, and dimensions array');
      }
      return false;
    }

    const registeredDimensions = [];

    for (const dim of plugin.dimensions) {
      const success = this.register(dim.axiom, dim.name, dim.config);
      if (success) {
        registeredDimensions.push(`${dim.axiom}.${dim.name}`);
      }
    }

    this._plugins.set(plugin.name, {
      version: plugin.version,
      dimensions: registeredDimensions,
      loadedAt: Date.now(),
    });

    this._stats.pluginsLoaded++;

    this.emit('plugin:loaded', {
      name: plugin.name,
      version: plugin.version,
      dimensionsCount: registeredDimensions.length,
    });

    return true;
  }

  /**
   * Unload a plugin and its dimensions
   *
   * @param {string} pluginName - Plugin name
   * @returns {boolean} Success
   */
  unloadPlugin(pluginName) {
    const plugin = this._plugins.get(pluginName);
    if (!plugin) return false;

    for (const key of plugin.dimensions) {
      const [axiom, name] = key.split('.');
      this.unregister(axiom, name);
    }

    this._plugins.delete(pluginName);
    this._stats.pluginsLoaded--;

    this.emit('plugin:unloaded', { name: pluginName });

    return true;
  }

  /**
   * Get loaded plugins
   * @returns {Object} Plugin metadata
   */
  getPlugins() {
    const result = {};
    for (const [name, meta] of this._plugins) {
      result[name] = { ...meta };
    }
    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Weights & Calculation Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Calculate total weight for an axiom
   *
   * @param {string} axiom - Axiom
   * @returns {number} Total weight
   */
  getAxiomTotalWeight(axiom) {
    const axiomMap = this._dimensions.get(axiom);
    if (!axiomMap) return 0;

    let total = 0;
    for (const config of axiomMap.values()) {
      total += config.weight || 0;
    }
    return total;
  }

  /**
   * Calculate global total weight
   * @returns {number} Total weight
   */
  getTotalWeight() {
    let total = 0;
    for (const axiom of this._dimensions.keys()) {
      total += this.getAxiomTotalWeight(axiom);
    }
    return total;
  }

  /**
   * Get dimension count
   * @returns {number} Total dimensions
   */
  get count() {
    let count = 0;
    for (const axiomMap of this._dimensions.values()) {
      count += axiomMap.size;
    }
    return count;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Export
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get registry stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      totalDimensions: this.count,
      axioms: [...this._dimensions.keys()],
    };
  }

  /**
   * Export registry state
   * @returns {Object} Exportable state
   */
  export() {
    const dimensions = {};
    for (const [axiom, axiomMap] of this._dimensions) {
      dimensions[axiom] = {};
      for (const [name, config] of axiomMap) {
        // Don't export core dimensions or scorers (functions)
        if (!config.core) {
          const { scorer, ...rest } = config;
          dimensions[axiom][name] = rest;
        }
      }
    }

    return {
      dimensions,
      plugins: this.getPlugins(),
      stats: this._stats,
    };
  }

  /**
   * Import registry state
   *
   * @param {Object} state - Saved state
   */
  import(state) {
    if (!state.dimensions) return;

    for (const [axiom, dims] of Object.entries(state.dimensions)) {
      for (const [name, config] of Object.entries(dims)) {
        this.register(axiom, name, config);
      }
    }
  }
}

// Global singleton instance
export const globalDimensionRegistry = new DimensionRegistry();

export default DimensionRegistry;
