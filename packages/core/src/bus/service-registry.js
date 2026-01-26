/**
 * CYNIC Service Registry
 *
 * Central registry for service discovery following SOLID principles:
 * - D: Dependency Inversion - services register by interface, not implementation
 * - S: Single Responsibility - only handles service registration/discovery
 * - O: Open/Closed - extensible without modification
 *
 * N-tier integration:
 * - Each layer registers its services
 * - Layers discover services through interfaces
 * - Layer boundaries are enforced
 *
 * "Find the right dog for the job" - κυνικός
 *
 * @module @cynic/core/bus/service-registry
 */

'use strict';

import { Layer, isLayerCallAllowed, assertImplements } from './interfaces.js';
import { CYNICError, ErrorCode } from '../errors.js';

/**
 * Service metadata
 */
class ServiceEntry {
  constructor(name, instance, options = {}) {
    this.name = name;
    this.instance = instance;
    this.layer = options.layer || Layer.DOMAIN;
    this.interfaces = options.interfaces || [];
    this.tags = options.tags || [];
    this.singleton = options.singleton !== false;
    this.factory = options.factory || null;
    this.registeredAt = Date.now();
  }
}

/**
 * CYNIC Service Registry
 *
 * Provides service discovery and dependency injection
 * with N-tier layer enforcement.
 */
export class ServiceRegistry {
  #services = new Map();
  #byInterface = new Map();
  #byLayer = new Map();
  #byTag = new Map();
  #strictLayers = false;

  constructor(options = {}) {
    this.#strictLayers = options.strictLayers || false;

    // Initialize layer maps
    for (const layer of Object.values(Layer)) {
      this.#byLayer.set(layer, new Set());
    }
  }

  /**
   * Register a service
   *
   * @param {string} name - Unique service name
   * @param {Object|Function} instanceOrFactory - Service instance or factory
   * @param {Object} [options] - Registration options
   * @param {string} [options.layer] - N-tier layer
   * @param {Object[]} [options.interfaces] - Interfaces this service implements
   * @param {string[]} [options.tags] - Tags for grouping
   * @param {boolean} [options.singleton=true] - Single instance or create new each time
   * @returns {ServiceRegistry} this (for chaining)
   *
   * @example
   * // Register a singleton service
   * registry.register('judge', new CYNICJudge(), {
   *   layer: Layer.APPLICATION,
   *   interfaces: [IJudge],
   *   tags: ['core'],
   * });
   *
   * @example
   * // Register a factory
   * registry.register('session', (deps) => new Session(deps), {
   *   layer: Layer.APPLICATION,
   *   singleton: false,
   * });
   */
  register(name, instanceOrFactory, options = {}) {
    if (this.#services.has(name)) {
      throw new CYNICError(
        ErrorCode.DUPLICATE,
        `Service "${name}" already registered`
      );
    }

    const isFactory = typeof instanceOrFactory === 'function' && !options.interfaces;
    const entry = new ServiceEntry(name, isFactory ? null : instanceOrFactory, {
      ...options,
      factory: isFactory ? instanceOrFactory : null,
    });

    // Validate interface implementation
    if (!isFactory && entry.interfaces.length > 0) {
      for (const iface of entry.interfaces) {
        assertImplements(entry.instance, iface, name);
      }
    }

    // Store by name
    this.#services.set(name, entry);

    // Index by layer
    this.#byLayer.get(entry.layer)?.add(name);

    // Index by interfaces
    for (const iface of entry.interfaces) {
      const ifaceName = iface.name || JSON.stringify(iface);
      if (!this.#byInterface.has(ifaceName)) {
        this.#byInterface.set(ifaceName, new Set());
      }
      this.#byInterface.get(ifaceName).add(name);
    }

    // Index by tags
    for (const tag of entry.tags) {
      if (!this.#byTag.has(tag)) {
        this.#byTag.set(tag, new Set());
      }
      this.#byTag.get(tag).add(name);
    }

    return this;
  }

  /**
   * Get a service by name
   *
   * @param {string} name - Service name
   * @param {Object} [context] - Context for layer enforcement
   * @param {string} [context.callerLayer] - Calling layer
   * @returns {Object} Service instance
   */
  get(name, context = {}) {
    const entry = this.#services.get(name);
    if (!entry) {
      throw new CYNICError(
        ErrorCode.NOT_FOUND,
        `Service "${name}" not found`
      );
    }

    // Enforce layer boundaries
    if (context.callerLayer && this.#strictLayers) {
      if (!isLayerCallAllowed(context.callerLayer, entry.layer)) {
        throw new CYNICError(
          ErrorCode.LAYER_VIOLATION,
          `Layer violation: ${context.callerLayer} cannot access ${entry.layer} service "${name}"`
        );
      }
    }

    // Return instance or create from factory
    if (entry.factory) {
      if (entry.singleton && entry.instance) {
        return entry.instance;
      }
      const instance = entry.factory(this);
      if (entry.singleton) {
        entry.instance = instance;
      }
      return instance;
    }

    return entry.instance;
  }

  /**
   * Get a service implementing a specific interface
   *
   * @param {Object} iface - Interface definition
   * @param {Object} [context] - Context
   * @returns {Object} Service instance
   */
  getByInterface(iface, context = {}) {
    const ifaceName = iface.name || JSON.stringify(iface);
    const names = this.#byInterface.get(ifaceName);

    if (!names || names.size === 0) {
      throw new CYNICError(
        ErrorCode.NOT_FOUND,
        `No service implements interface: ${ifaceName}`
      );
    }

    // Return first matching service
    const name = names.values().next().value;
    return this.get(name, context);
  }

  /**
   * Get all services implementing an interface
   *
   * @param {Object} iface - Interface definition
   * @param {Object} [context] - Context
   * @returns {Object[]} Service instances
   */
  getAllByInterface(iface, context = {}) {
    const ifaceName = iface.name || JSON.stringify(iface);
    const names = this.#byInterface.get(ifaceName);

    if (!names) return [];

    return Array.from(names).map(name => this.get(name, context));
  }

  /**
   * Get all services in a layer
   *
   * @param {string} layer - Layer name
   * @returns {Object[]}
   */
  getByLayer(layer) {
    const names = this.#byLayer.get(layer);
    if (!names) return [];
    return Array.from(names).map(name => this.get(name));
  }

  /**
   * Get all services with a tag
   *
   * @param {string} tag - Tag name
   * @returns {Object[]}
   */
  getByTag(tag) {
    const names = this.#byTag.get(tag);
    if (!names) return [];
    return Array.from(names).map(name => this.get(name));
  }

  /**
   * Check if a service is registered
   */
  has(name) {
    return this.#services.has(name);
  }

  /**
   * Unregister a service
   */
  unregister(name) {
    const entry = this.#services.get(name);
    if (!entry) return false;

    // Remove from all indexes
    this.#services.delete(name);
    this.#byLayer.get(entry.layer)?.delete(name);

    for (const iface of entry.interfaces) {
      const ifaceName = iface.name || JSON.stringify(iface);
      this.#byInterface.get(ifaceName)?.delete(name);
    }

    for (const tag of entry.tags) {
      this.#byTag.get(tag)?.delete(name);
    }

    return true;
  }

  /**
   * Get registry statistics
   */
  getStats() {
    const byLayer = {};
    for (const [layer, names] of this.#byLayer) {
      byLayer[layer] = names.size;
    }

    return {
      totalServices: this.#services.size,
      byLayer,
      interfaces: this.#byInterface.size,
      tags: this.#byTag.size,
    };
  }

  /**
   * List all registered services
   */
  list() {
    return Array.from(this.#services.entries()).map(([name, entry]) => ({
      name,
      layer: entry.layer,
      interfaces: entry.interfaces.map(i => i.name || 'anonymous'),
      tags: entry.tags,
      singleton: entry.singleton,
    }));
  }

  /**
   * Clear all services
   */
  clear() {
    this.#services.clear();
    this.#byInterface.clear();
    this.#byTag.clear();
    for (const set of this.#byLayer.values()) {
      set.clear();
    }
  }
}

/**
 * Global service registry
 */
export const globalServiceRegistry = new ServiceRegistry();

/**
 * Convenience function to register to global registry
 */
export function registerService(name, instance, options = {}) {
  return globalServiceRegistry.register(name, instance, options);
}

/**
 * Convenience function to get from global registry
 */
export function getService(name, context = {}) {
  return globalServiceRegistry.get(name, context);
}
