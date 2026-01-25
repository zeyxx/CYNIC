/**
 * Legacy Engine Adapter
 *
 * Wraps existing scripts/lib/*.cjs engines into the new Engine system.
 * Preserves all original functionality while adding registry integration.
 *
 * "Old dogs learn new tricks" - κυνικός
 *
 * @module @cynic/core/engines/philosophy/adapter
 */

'use strict';

import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';
import { Engine, EngineStatus } from '../engine.js';
import { PHI_INV } from '../../axioms/constants.js';

const require = createRequire(import.meta.url);

/**
 * Adapt a legacy .cjs engine to the new Engine system
 *
 * @param {Object} config - Adapter configuration
 * @param {string} config.id - Engine ID
 * @param {string} config.legacyPath - Path to .cjs file (relative to scripts/lib/)
 * @param {string} config.domain - Engine domain
 * @param {string[]} config.capabilities - Engine capabilities
 * @param {string} [config.tradition] - Philosophical tradition
 * @param {string} [config.name] - Human-readable name
 * @param {string} [config.description] - Description
 * @param {string[]} [config.subdomains] - Additional domains
 * @returns {Engine} Adapted engine instance
 */
export function adaptLegacyEngine(config) {
  const {
    id,
    legacyPath,
    domain,
    capabilities,
    tradition,
    name,
    description,
    subdomains = [],
  } = config;

  // Resolve path to scripts/lib/
  const scriptsLibPath = path.resolve(
    fileURLToPath(import.meta.url),
    '../../../../../../scripts/lib'
  );
  const fullPath = path.join(scriptsLibPath, legacyPath);

  // Load the legacy module
  let legacyModule;
  try {
    legacyModule = require(fullPath);
  } catch (error) {
    console.warn(`Failed to load legacy engine ${id}: ${error.message}`);
    legacyModule = null;
  }

  // Create engine wrapper
  const engine = Object.create(Engine.prototype);

  // Initialize engine properties
  engine.id = id;
  engine.name = name || id.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  engine.domain = domain;
  engine.subdomains = subdomains;
  engine.capabilities = capabilities;
  engine.dependencies = [];
  engine.description = description || `Adapted from ${legacyPath}`;
  engine.tradition = tradition || null;
  engine.status = EngineStatus.IDLE;
  engine.lastEvaluation = null;
  engine.evaluationCount = 0;
  engine.totalConfidence = 0;

  // Store reference to legacy module
  engine._legacy = legacyModule;
  engine._legacyPath = fullPath;
  engine._initialized = false;

  // Initialize legacy module if it has init()
  engine.initLegacy = function() {
    if (this._initialized || !this._legacy) return;

    if (typeof this._legacy.init === 'function') {
      try {
        this._legacy.init();
      } catch (error) {
        console.warn(`Failed to init legacy engine ${this.id}: ${error.message}`);
      }
    }
    this._initialized = true;
  };

  // Override evaluate to use legacy module
  engine.evaluate = async function(input, context = {}) {
    this.status = EngineStatus.EVALUATING;

    // Ensure initialized
    this.initLegacy();

    if (!this._legacy) {
      this.status = EngineStatus.ERROR;
      return this.createInsight(
        'Legacy module not available',
        0,
        ['Module failed to load'],
        { error: true }
      );
    }

    try {
      let result;

      // Try different evaluation methods that legacy engines might have
      if (typeof this._legacy.evaluate === 'function') {
        result = await this._legacy.evaluate(input, context);
      } else if (typeof this._legacy.analyze === 'function') {
        result = await this._legacy.analyze(input, context);
      } else if (typeof this._legacy.assess === 'function') {
        result = await this._legacy.assess(input, context);
      } else if (typeof this._legacy.judge === 'function') {
        result = await this._legacy.judge(input, context);
      } else if (typeof this._legacy.consult === 'function') {
        result = await this._legacy.consult(input, context);
      } else if (typeof this._legacy.process === 'function') {
        result = await this._legacy.process(input, context);
      } else {
        // No evaluation method - return engine's knowledge summary
        result = this._getKnowledgeSummary(input);
      }

      this.status = EngineStatus.IDLE;

      // Normalize result to insight format
      return this.createInsight(
        result?.insight || result?.analysis || result?.conclusion || result?.summary || String(result),
        result?.confidence || PHI_INV * 0.8, // Default to 80% of max
        result?.reasoning || result?.steps || [],
        result?.metadata || {}
      );
    } catch (error) {
      this.status = EngineStatus.ERROR;
      return this.createInsight(
        `Evaluation error: ${error.message}`,
        0,
        [error.message],
        { error: true }
      );
    }
  };

  // Get knowledge summary from legacy module
  engine._getKnowledgeSummary = function(input) {
    const module = this._legacy;
    const summaryParts = [];

    // Try to extract knowledge from module exports
    for (const key of Object.keys(module)) {
      const value = module[key];
      if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        if (value.name || value.description) {
          summaryParts.push(value.name || key);
        }
      }
    }

    if (summaryParts.length > 0) {
      return {
        insight: `${this.name} domain knowledge includes: ${summaryParts.slice(0, 5).join(', ')}`,
        confidence: PHI_INV * 0.6,
        reasoning: [`Domain: ${this.domain}`, `Tradition: ${this.tradition || 'general'}`],
      };
    }

    return {
      insight: `${this.name} provides perspective on: ${input}`,
      confidence: PHI_INV * 0.5,
      reasoning: ['General domain knowledge available'],
    };
  };

  // Get legacy module exports (for introspection)
  engine.getLegacyExports = function() {
    if (!this._legacy) return null;
    return Object.keys(this._legacy);
  };

  // Call a specific legacy function
  engine.callLegacy = function(methodName, ...args) {
    if (!this._legacy || typeof this._legacy[methodName] !== 'function') {
      throw new Error(`Legacy method ${methodName} not found in ${this.id}`);
    }
    return this._legacy[methodName](...args);
  };

  return engine;
}

/**
 * Batch adapt multiple legacy engines
 *
 * @param {Object[]} configs - Array of adapter configurations
 * @returns {Engine[]} Array of adapted engines
 */
export function adaptLegacyEngines(configs) {
  return configs.map(config => adaptLegacyEngine(config));
}

export default adaptLegacyEngine;
