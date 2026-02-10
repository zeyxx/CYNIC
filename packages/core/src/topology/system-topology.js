/**
 * CYNIC System Topology — Self-Awareness Module
 *
 * CYNIC must know WHERE it runs, WHAT is available, and WHAT it can do.
 * This is the "mirror" — the module that lets CYNIC perceive itself.
 *
 * THREE QUESTIONS this module answers:
 *   1. Where am I? (mode: hook | mcp | daemon | test | unknown)
 *   2. What's here? (services: postgres, redis, mcp, dogs, solana...)
 *   3. What can I do? (capabilities per mode)
 *
 * "γνῶθι σεαυτόν" (know thyself) — but with φ⁻¹ doubt
 *
 * @module @cynic/core/topology/system-topology
 */

'use strict';

import { PHI_INV } from '../axioms/constants.js';
import { createLogger } from '../logger.js';

const log = createLogger('Topology');

// ──────────────────────────────────────────────────────────────
// EXECUTION MODES — where CYNIC can run
// ──────────────────────────────────────────────────────────────

export const ExecutionMode = {
  HOOK:    'hook',     // Claude Code hook (ephemeral, 5-10s, no shared memory)
  MCP:     'mcp',      // MCP server (long-lived, stdio or HTTP)
  DAEMON:  'daemon',   // Node daemon (Render, automation, Fibonacci triggers)
  TEST:    'test',     // Test runner (mocks expected, no external services)
  UNKNOWN: 'unknown',  // Can't determine — investigate
};

// ──────────────────────────────────────────────────────────────
// CAPABILITY MATRIX — what each mode can do
// ──────────────────────────────────────────────────────────────

const CAPABILITY_MATRIX = {
  [ExecutionMode.HOOK]: {
    judge: false,        // No Judge in hooks (separate process)
    consensus: false,    // No Dogs in hooks
    persistence: false,  // No direct DB (hooks are ephemeral)
    mcp_call: true,      // Can call MCP tools (if MCP is reachable)
    file_io: true,       // Can read/write ~/.cynic/ files
    guardian: true,      // Can block commands (pre-tool.js exit(1))
    framing: true,       // Can inject consciousness framing
    solana: false,       // No Solana ops in hooks
    learning: false,     // No direct learning (signals via MCP)
  },
  [ExecutionMode.MCP]: {
    judge: true,
    consensus: true,
    persistence: true,   // If PostgreSQL is wired
    mcp_call: false,     // IS the MCP — doesn't call itself
    file_io: true,
    guardian: false,      // Guardian lives in hooks, not MCP
    framing: false,       // Framing lives in perceive hook
    solana: true,         // Can anchor (if SolanaWatcher initialized)
    learning: true,       // Full learning pipeline
  },
  [ExecutionMode.DAEMON]: {
    judge: true,
    consensus: true,
    persistence: true,
    mcp_call: false,
    file_io: true,
    guardian: false,
    framing: false,
    solana: true,
    learning: true,
  },
  [ExecutionMode.TEST]: {
    judge: true,          // Judge works in tests
    consensus: true,      // With mocks
    persistence: false,   // Usually mocked
    mcp_call: false,
    file_io: false,       // Tests shouldn't touch filesystem
    guardian: false,
    framing: false,
    solana: false,
    learning: true,       // Learning logic testable
  },
  [ExecutionMode.UNKNOWN]: {
    judge: false,
    consensus: false,
    persistence: false,
    mcp_call: false,
    file_io: true,
    guardian: false,
    framing: false,
    solana: false,
    learning: false,
  },
};

// ──────────────────────────────────────────────────────────────
// SERVICE STATUS
// ──────────────────────────────────────────────────────────────

export const ServiceStatus = {
  HEALTHY:  'healthy',
  DEGRADED: 'degraded',
  DEAD:     'dead',
  UNKNOWN:  'unknown',
};

// ──────────────────────────────────────────────────────────────
// 7×7 REALITY DIMENSIONS (what SystemTopology maps)
// ──────────────────────────────────────────────────────────────

export const RealityDimension = {
  CODE:   'CODE',    // R1: Codebase, files, dependencies
  SOLANA: 'SOLANA',  // R2: Blockchain state, transactions
  MARKET: 'MARKET',  // R3: Price, liquidity, sentiment
  SOCIAL: 'SOCIAL',  // R4: Twitter, Discord, community
  HUMAN:  'HUMAN',   // R5: User psychology, energy, focus
  CYNIC:  'CYNIC',   // R6: Self-state, Dogs, memory
  COSMOS: 'COSMOS',  // R7: Ecosystem, collective patterns
};

export const AnalysisDimension = {
  PERCEIVE: 'PERCEIVE',  // A1: Observe current state
  JUDGE:    'JUDGE',      // A2: Evaluate with 36 dimensions
  DECIDE:   'DECIDE',     // A3: Governance (approve/reject)
  ACT:      'ACT',        // A4: Execute transformation
  LEARN:    'LEARN',      // A5: Update from feedback
  ACCOUNT:  'ACCOUNT',    // A6: Economic cost/value
  EMERGE:   'EMERGE',     // A7: Meta-patterns, transcendence
};

// ──────────────────────────────────────────────────────────────
// SYSTEM TOPOLOGY SINGLETON
// ──────────────────────────────────────────────────────────────

class SystemTopology {
  constructor() {
    this._mode = null;       // Detected execution mode
    this._services = new Map();   // Registered services
    this._components = new Map(); // Active components
    this._bootTime = Date.now();
    this._matrixState = null;     // 7×7 cell states (lazy)
    this._listeners = [];
  }

  // ────────────────────────────────────────────────────────────
  // MODE DETECTION — "Where am I?"
  // ────────────────────────────────────────────────────────────

  /**
   * Detect current execution mode.
   * Called once, cached forever (mode doesn't change mid-process).
   * @returns {string} ExecutionMode value
   */
  detectMode() {
    if (this._mode) return this._mode;

    // Test mode — most specific check first
    // node --test sets argv[1] to the test file, but doesn't set NODE_ENV
    // Also detect vitest, jest, and NODE_ENV=test
    if (
      process.env.NODE_ENV === 'test' ||
      process.env.VITEST === 'true' ||
      process.env.JEST_WORKER_ID != null ||
      process.argv.some(a =>
        a.includes('node:test') ||
        a.includes('vitest') ||
        a.includes('jest') ||
        a.endsWith('.test.js') ||
        a.endsWith('.test.mjs') ||
        a.endsWith('.test.cjs') ||
        a.endsWith('.spec.js')
      ) ||
      // node --test passes --test as an arg
      process.execArgv?.some(a => a === '--test' || a === '--test-force-exit')
    ) {
      this._mode = ExecutionMode.TEST;
    }
    // Hook mode — Claude Code injects specific env vars + stdin is piped
    else if (
      process.env.CLAUDE_PLUGIN_ROOT ||
      process.env.MCP_CLAUDE_CODE === 'true' ||
      (process.env.CYNIC_HOOK_NAME && process.env.CYNIC_HOOK_NAME !== '')
    ) {
      this._mode = ExecutionMode.HOOK;
    }
    // MCP mode — server process
    else if (
      process.env.CYNIC_MCP === 'true' ||
      process.env.MCP_SERVER === 'true' ||
      process.title?.includes('cynic-mcp') ||
      process.argv.some(a => a.includes('mcp') && a.includes('server'))
    ) {
      this._mode = ExecutionMode.MCP;
    }
    // Daemon mode — background service
    else if (
      process.env.CYNIC_DAEMON === 'true' ||
      process.argv.some(a => a.includes('daemon'))
    ) {
      this._mode = ExecutionMode.DAEMON;
    }
    else {
      this._mode = ExecutionMode.UNKNOWN;
    }

    log.info(`Mode detected: ${this._mode}`);
    return this._mode;
  }

  /**
   * Get current mode (detects if not yet detected)
   */
  get mode() {
    return this._mode || this.detectMode();
  }

  /**
   * Override mode (for testing or explicit init)
   */
  setMode(mode) {
    if (!Object.values(ExecutionMode).includes(mode)) {
      throw new Error(`Invalid mode: ${mode}. Valid: ${Object.values(ExecutionMode).join(', ')}`);
    }
    this._mode = mode;
    log.info(`Mode set explicitly: ${mode}`);
  }

  // ────────────────────────────────────────────────────────────
  // SERVICE REGISTRY — "What's here?"
  // ────────────────────────────────────────────────────────────

  /**
   * Register a service with its location and status
   * @param {string} name - Service name (e.g., 'postgres', 'mcp', 'dogs')
   * @param {Object} info - Service information
   * @param {string} info.status - ServiceStatus value
   * @param {string} [info.location] - Where it runs (url, path, pid)
   * @param {string} [info.mode] - Which mode it belongs to
   * @param {Object} [info.meta] - Additional metadata
   */
  registerService(name, info) {
    const service = {
      name,
      status: info.status || ServiceStatus.UNKNOWN,
      location: info.location || null,
      mode: info.mode || this.mode,
      meta: info.meta || {},
      registeredAt: Date.now(),
      lastChecked: Date.now(),
    };
    this._services.set(name, service);
    this._notify('service:registered', { name, service });
  }

  /**
   * Update service status
   */
  updateService(name, updates) {
    const service = this._services.get(name);
    if (!service) {
      this.registerService(name, updates);
      return;
    }
    Object.assign(service, updates, { lastChecked: Date.now() });
    this._notify('service:updated', { name, service });
  }

  /**
   * Get a registered service
   */
  getService(name) {
    return this._services.get(name) || null;
  }

  /**
   * Get all registered services
   */
  getServices() {
    return Object.fromEntries(this._services);
  }

  // ────────────────────────────────────────────────────────────
  // COMPONENT REGISTRY — Active CYNIC components
  // ────────────────────────────────────────────────────────────

  /**
   * Register an active component (Judge, Router, Dogs, etc.)
   */
  registerComponent(name, instance) {
    this._components.set(name, {
      name,
      instance,
      registeredAt: Date.now(),
    });
    this._notify('component:registered', { name });
  }

  /**
   * Check if a component is available
   */
  hasComponent(name) {
    return this._components.has(name);
  }

  /**
   * Get component instance
   */
  getComponent(name) {
    return this._components.get(name)?.instance || null;
  }

  /**
   * List all registered components
   */
  getComponentNames() {
    return Array.from(this._components.keys());
  }

  // ────────────────────────────────────────────────────────────
  // CAPABILITIES — "What can I do?"
  // ────────────────────────────────────────────────────────────

  /**
   * Get capability matrix for current mode
   * @returns {Object} capability → boolean
   */
  getCapabilities() {
    const base = CAPABILITY_MATRIX[this.mode] || CAPABILITY_MATRIX[ExecutionMode.UNKNOWN] || {};
    // Override with actual service availability
    const actual = { ...base };

    // If persistence service is registered and healthy, enable persistence
    const pg = this._services.get('postgres');
    if (pg && pg.status === ServiceStatus.HEALTHY) {
      actual.persistence = true;
    } else if (base.persistence && (!pg || pg.status === ServiceStatus.DEAD)) {
      actual.persistence = false;
    }

    // If dogs are loaded, enable consensus
    if (this._components.has('collectivePack')) {
      actual.consensus = true;
    }

    // If solana watcher exists, enable solana
    if (this._components.has('solanaWatcher')) {
      actual.solana = true;
    }

    return actual;
  }

  /**
   * Check a single capability
   */
  can(capability) {
    return this.getCapabilities()[capability] || false;
  }

  // ────────────────────────────────────────────────────────────
  // 7×7 MATRIX INTROSPECTION — "How complete am I?"
  // ────────────────────────────────────────────────────────────

  /**
   * Get the 7×7 matrix state — which cells are active.
   * This is CYNIC knowing its own topology.
   *
   * Returns a map of "R{n}.A{m}" → { active, coverage, components }
   */
  getMatrixState() {
    const matrix = {};
    const realities = Object.values(RealityDimension);
    const analyses = Object.values(AnalysisDimension);

    for (const r of realities) {
      for (const a of analyses) {
        const key = `${r}.${a}`;
        matrix[key] = this._assessCell(r, a);
      }
    }

    return matrix;
  }

  /**
   * Get completion percentage of the 7×7 matrix
   * @returns {number} 0-100 (capped at φ⁻¹ × 100 = 61.8 for confidence)
   */
  getMatrixCompletion() {
    const matrix = this.getMatrixState();
    const cells = Object.values(matrix);
    const total = cells.length; // 49
    const active = cells.filter(c => c.active).length;
    const coverage = cells.reduce((sum, c) => sum + c.coverage, 0) / total;

    return {
      cells: total,
      active,
      coverage: Math.round(coverage * 1000) / 10, // e.g. 18.4%
      confidence: Math.min(coverage, PHI_INV),     // φ⁻¹ cap
    };
  }

  /**
   * Assess a single cell of the 7×7 matrix
   * @private
   */
  _assessCell(reality, analysis) {
    // Map cell to required components/services
    const requirements = CELL_REQUIREMENTS[`${reality}.${analysis}`];
    if (!requirements) {
      return { active: false, coverage: 0, components: [], missing: ['definition'] };
    }

    const present = [];
    const missing = [];

    for (const req of requirements) {
      if (this._components.has(req) || this._services.has(req)) {
        present.push(req);
      } else {
        missing.push(req);
      }
    }

    const coverage = requirements.length > 0
      ? present.length / requirements.length
      : 0;

    return {
      active: coverage > 0,
      coverage,
      components: present,
      missing,
    };
  }

  // ────────────────────────────────────────────────────────────
  // FULL TOPOLOGY SNAPSHOT — "Who am I right now?"
  // ────────────────────────────────────────────────────────────

  /**
   * Get complete topology snapshot.
   * This is the answer to "CYNIC, connais-toi toi-même."
   */
  snapshot() {
    const matrix = this.getMatrixCompletion();

    return {
      mode: this.mode,
      uptime: Date.now() - this._bootTime,
      services: this.getServices(),
      components: this.getComponentNames(),
      capabilities: this.getCapabilities(),
      matrix: {
        completion: matrix.coverage,
        activeCells: matrix.active,
        totalCells: matrix.cells,
        confidence: Math.round(matrix.confidence * 1000) / 10,
      },
      phi: {
        maxConfidence: PHI_INV,
        selfAwareness: Math.min(matrix.coverage / 100, PHI_INV),
      },
      timestamp: Date.now(),
    };
  }

  // ────────────────────────────────────────────────────────────
  // PROCESS BOUNDARIES — "Who else is running?"
  // ────────────────────────────────────────────────────────────

  /**
   * Get process boundary information.
   * Hooks, MCP, and Daemon are separate processes.
   * This documents what each can see.
   */
  getProcessBoundaries() {
    return {
      current: {
        mode: this.mode,
        pid: process.pid,
        ppid: process.ppid,
        title: process.title,
        argv: process.argv.slice(0, 3),
      },
      boundaries: {
        hooks: {
          description: 'Ephemeral (5-10s), no shared memory',
          communicatesVia: ['stdin/stdout JSON', 'file I/O (~/.cynic/)', 'MCP HTTP calls'],
          cannotAccess: ['globalEventBus', 'CollectivePack', 'PostgreSQL directly'],
        },
        mcp: {
          description: 'Long-lived, stdio or HTTP transport',
          communicatesVia: ['MCP protocol (tools/resources)', 'globalEventBus', 'PostgreSQL'],
          cannotAccess: ['Hook context', 'Claude Code conversation'],
        },
        daemon: {
          description: 'Background service on Render',
          communicatesVia: ['globalEventBus', 'PostgreSQL', 'Automation ticks'],
          cannotAccess: ['Hook context', 'MCP tools', 'Claude Code'],
        },
      },
    };
  }

  // ────────────────────────────────────────────────────────────
  // EVENT LISTENERS
  // ────────────────────────────────────────────────────────────

  /**
   * Listen for topology changes
   */
  on(callback) {
    this._listeners.push(callback);
    return () => {
      this._listeners = this._listeners.filter(l => l !== callback);
    };
  }

  /** @private */
  _notify(event, data) {
    for (const listener of this._listeners) {
      try {
        listener(event, data);
      } catch {
        // Non-blocking
      }
    }
  }

  // ────────────────────────────────────────────────────────────
  // RESET (for testing)
  // ────────────────────────────────────────────────────────────

  _resetForTesting() {
    this._mode = null;
    this._services.clear();
    this._components.clear();
    this._listeners = [];
    this._bootTime = Date.now();
  }
}

// ──────────────────────────────────────────────────────────────
// CELL REQUIREMENTS — what each 7×7 cell needs to be "active"
// Maps cell ID to required components/services
// ──────────────────────────────────────────────────────────────

const CELL_REQUIREMENTS = {
  // CODE × {analysis}
  'CODE.PERCEIVE':  ['codebase_tools'],          // brain_codebase, brain_search
  'CODE.JUDGE':     ['judge', 'dimensions'],      // Judge + 36 dimensions
  'CODE.DECIDE':    ['router', 'consensus'],      // KabbalisticRouter + Dogs
  'CODE.ACT':       ['automation'],               // AutomationExecutor
  'CODE.LEARN':     ['learning', 'ewc'],          // LearningService, EWC++
  'CODE.ACCOUNT':   [],                           // Not built
  'CODE.EMERGE':    ['emergence', 'residual'],    // EmergenceDetector, Residual

  // SOLANA × {analysis}
  'SOLANA.PERCEIVE': ['solanaWatcher'],
  'SOLANA.JUDGE':    ['judge', 'solanaWatcher'],
  'SOLANA.DECIDE':   ['router', 'solanaWatcher'],
  'SOLANA.ACT':      ['solanaAnchor'],
  'SOLANA.LEARN':    ['learning', 'solanaWatcher'],
  'SOLANA.ACCOUNT':  [],
  'SOLANA.EMERGE':   ['emergence', 'solanaWatcher'],

  // MARKET × {analysis}
  'MARKET.PERCEIVE': ['market_feed'],
  'MARKET.JUDGE':    ['judge', 'market_feed'],
  'MARKET.DECIDE':   ['router', 'market_feed'],
  'MARKET.ACT':      [],
  'MARKET.LEARN':    ['learning', 'market_feed'],
  'MARKET.ACCOUNT':  ['market_feed'],
  'MARKET.EMERGE':   ['emergence', 'market_feed'],

  // SOCIAL × {analysis}
  'SOCIAL.PERCEIVE': ['x_proxy', 'social_tools'],
  'SOCIAL.JUDGE':    ['judge', 'social_tools'],
  'SOCIAL.DECIDE':   ['router', 'social_tools'],
  'SOCIAL.ACT':      ['x_poster'],
  'SOCIAL.LEARN':    ['learning', 'social_tools'],
  'SOCIAL.ACCOUNT':  [],
  'SOCIAL.EMERGE':   ['emergence', 'social_tools'],

  // HUMAN × {analysis}
  'HUMAN.PERCEIVE':  ['psychology'],
  'HUMAN.JUDGE':     ['judge', 'psychology'],
  'HUMAN.DECIDE':    ['router', 'psychology'],
  'HUMAN.ACT':       ['humanAdvisor'],
  'HUMAN.LEARN':     ['learning', 'psychology'],
  'HUMAN.ACCOUNT':   ['humanAccountant'],
  'HUMAN.EMERGE':    ['humanEmergence'],

  // CYNIC × {analysis}
  'CYNIC.PERCEIVE':  ['topology', 'heartbeat'],
  'CYNIC.JUDGE':     ['judge'],
  'CYNIC.DECIDE':    ['router', 'consensus'],
  'CYNIC.ACT':       ['automation'],
  'CYNIC.LEARN':     ['learning', 'metaCognition', 'sona'],
  'CYNIC.ACCOUNT':   [],
  'CYNIC.EMERGE':    ['emergence', 'consciousness'],

  // COSMOS × {analysis}
  'COSMOS.PERCEIVE': ['ecosystem_tools'],
  'COSMOS.JUDGE':    ['judge', 'ecosystem_tools'],
  'COSMOS.DECIDE':   ['router', 'ecosystem_tools'],
  'COSMOS.ACT':      [],
  'COSMOS.LEARN':    ['learning', 'ecosystem_tools'],
  'COSMOS.ACCOUNT':  [],
  'COSMOS.EMERGE':   ['emergence', 'ecosystem_tools'],
};

// ──────────────────────────────────────────────────────────────
// SINGLETON EXPORT
// ──────────────────────────────────────────────────────────────

export const systemTopology = new SystemTopology();
export default systemTopology;
