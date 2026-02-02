/**
 * Collective Cartographer - Malkhut (Kingdom)
 *
 * "Je mappe la realite telle qu'elle est, pas telle qu'on voudrait.
 *  Le royaume est vaste, mes cartes sont precises." - κυνικός Cartographer
 *
 * Malkhut represents the physical manifestation, the kingdom of reality.
 * Cartographer maps the entire GitHub ecosystem to ground CYNIC in reality.
 *
 * Responsibilities:
 * 1. GitHub Reality Mapping - Map all repositories, track forks, PRs, issues
 * 2. Ecosystem Visualization - Dependency graphs, contribution flows
 * 3. Reality Verification - Verify what's deployed, check what code is used
 *
 * @module @cynic/node/agents/collective/cartographer
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { BaseAgent, AgentTrigger, AgentBehavior, AgentResponse } from '../base.js';
import {
  AgentEvent,
  AgentId,
  MapUpdatedEvent,
  RealityDriftEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Cartographer
 */
export const CARTOGRAPHER_CONSTANTS = {
  /** Max repos to track (Fib(13) = 233) */
  MAX_REPOS: 233,

  /** Cache TTL (Fib(8) = 21 minutes) */
  CACHE_TTL_MS: 21 * 60 * 1000,

  /** Max connections per repo (Fib(7) = 13) */
  MAX_CONNECTIONS: 13,

  /** Sync interval (Fib(13) = 233 seconds) */
  SYNC_INTERVAL_MS: 233000,

  /** Max PRs to track per repo (Fib(8) = 21) */
  MAX_PRS_PER_REPO: 21,

  /** Max issues to track per repo (Fib(8) = 21) */
  MAX_ISSUES_PER_REPO: 21,

  /** Max contributors to track (Fib(13) = 233) */
  MAX_CONTRIBUTORS: 233,

  /** Stale fork threshold (Fib(13) = 233 days) */
  STALE_FORK_DAYS: 233,

  /** Drift detection threshold (Fib(5) = 5 commits behind) */
  DRIFT_THRESHOLD_COMMITS: 5,
};

/**
 * Repository types
 */
export const RepoType = {
  CORE: 'core',              // Main CYNIC
  INFRASTRUCTURE: 'infra',   // GASdf, deploy tools
  INTELLIGENCE: 'intel',     // HolDex, analytics
  TOOL: 'tool',              // Utilities
  EXTERNAL: 'external',      // Dependencies
  FORK: 'fork',              // Forks of external repos
};

/**
 * Connection types between repos
 */
export const ConnectionType = {
  FORK: 'fork',
  DEPENDENCY: 'dependency',
  IMPORT: 'import',
  UPSTREAM: 'upstream',
  DOWNSTREAM: 'downstream',
  SHARED_CODE: 'shared_code',
  CONTRIBUTOR: 'contributor',
};

/**
 * Issue types detected during mapping
 */
export const MapIssueType = {
  CIRCULAR_DEPENDENCY: 'circular_dependency',
  ORPHANED_REPO: 'orphaned_repo',
  STALE_FORK: 'stale_fork',
  UNSYNCED_FORK: 'unsynced_fork',
  MISSING_UPSTREAM: 'missing_upstream',
  BROKEN_DEPENDENCY: 'broken_dependency',
};

/**
 * Profile-based mapping settings
 */
const PROFILE_SETTINGS = {
  [ProfileLevel.NOVICE]: {
    maxRepos: 21,           // Fib(8)
    includeExternal: false,
    trackContributors: false,
    detectDrifts: false,
  },
  [ProfileLevel.APPRENTICE]: {
    maxRepos: 55,           // Fib(10)
    includeExternal: false,
    trackContributors: true,
    detectDrifts: false,
  },
  [ProfileLevel.PRACTITIONER]: {
    maxRepos: 89,           // Fib(11)
    includeExternal: true,
    trackContributors: true,
    detectDrifts: true,
  },
  [ProfileLevel.EXPERT]: {
    maxRepos: 144,          // Fib(12)
    includeExternal: true,
    trackContributors: true,
    detectDrifts: true,
  },
  [ProfileLevel.MASTER]: {
    maxRepos: 233,          // Fib(13)
    includeExternal: true,
    trackContributors: true,
    detectDrifts: true,
    deepAnalysis: true,
  },
};

/**
 * Collective Cartographer Agent
 */
export class CollectiveCartographer extends BaseAgent {
  /**
   * Create Cartographer agent
   * @param {Object} [options] - Options
   * @param {Object} [options.eventBus] - Event bus for communication
   * @param {number} [options.profileLevel] - User profile level
   * @param {Object} [options.githubClient] - GitHub API client
   */
  constructor(options = {}) {
    super({
      name: 'Cartographer',
      trigger: AgentTrigger.ASYNC,
      behavior: AgentBehavior.BACKGROUND,
      sefirah: 'Malkhut',
      ...options,
    });

    /** @type {Object} */
    this.map = {
      repos: new Map(),
      connections: [],
      contributors: new Map(),
      lastSync: 0,
    };

    /** @type {number} */
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    /** @type {Object} */
    this.eventBus = options.eventBus || null;

    /** @type {Object|null} */
    this.githubClient = options.githubClient || null;

    /** @type {Map<string, Object>} */
    this.cache = new Map();

    /** @type {Object[]} */
    this.issues = [];

    /** @type {Object} */
    this.stats = {
      totalMappings: 0,
      totalReposMapped: 0,
      totalConnections: 0,
      totalContributors: 0,
      issuesDetected: 0,
      driftsDetected: 0,
    };

    // Subscribe to events if event bus available
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to relevant events
   * @private
   */
  _subscribeToEvents() {
    // Listen for discovery events from Scout
    this.eventBus.subscribe(
      AgentEvent.DISCOVERY_FOUND,
      AgentId.CARTOGRAPHER,
      this._handleDiscovery.bind(this)
    );

    // Listen for deploy events
    this.eventBus.subscribe(
      AgentEvent.DEPLOY_COMPLETED,
      AgentId.CARTOGRAPHER,
      this._handleDeploy.bind(this)
    );
  }

  /**
   * Handle discovery from Scout
   * @private
   */
  _handleDiscovery(event) {
    const { type, path, details } = event.payload;

    // If external dependency discovered, add to mapping queue
    if (type === 'dependency' && details?.external) {
      this._queueExternalMapping(details.name, details.version);
    }
  }

  /**
   * Handle deployment completion
   * @private
   */
  _handleDeploy(event) {
    const { service, version } = event.payload;
    // Trigger re-mapping to verify deployment state
    this._invalidateCache(service);
  }

  /**
   * Queue external repo for mapping
   * @private
   */
  _queueExternalMapping(name, version) {
    // Would be implemented with actual GitHub API
    // For now, track it in cache
    this.cache.set(`ext:${name}`, {
      name,
      version,
      queued: Date.now(),
    });
  }

  /**
   * Invalidate cache for a service
   * @private
   */
  _invalidateCache(service) {
    for (const [key, value] of this.cache.entries()) {
      if (key.includes(service)) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Check if Cartographer should trigger for event
   * @param {Object} event - Event to check
   * @returns {boolean} Whether to trigger
   */
  shouldTrigger(event) {
    const type = event?.type?.toLowerCase() || '';
    return (
      type === 'map' ||
      type === 'sync' ||
      type === 'async' ||
      type === 'scheduled' ||
      // Also trigger on GitHub-related events
      type === 'github_push' ||
      type === 'github_pr'
    );
  }

  /**
   * Process mapping request
   * @param {Object} event - Event to process
   * @param {Object} [context] - Context
   * @returns {Promise<Object>} Processing result
   */
  async process(event, context = {}) {
    const options = {
      owner: event.owner || context.owner,
      includeExternal: event.includeExternal ?? context.includeExternal,
      force: event.force || context.force || false,
      ...context,
    };

    try {
      const result = await this.buildMap(options);
      return {
        response: AgentResponse.CONTINUE,
        agent: AgentId.CARTOGRAPHER,
        result,
        confidence: this._calculateMappingConfidence(result),
      };
    } catch (error) {
      return {
        response: AgentResponse.ERROR,
        agent: AgentId.CARTOGRAPHER,
        error: error.message,
      };
    }
  }

  /**
   * Build complete ecosystem map
   * @param {Object} options - Mapping options
   * @returns {Promise<Object>} Map result
   */
  async buildMap(options = {}) {
    const settings = this._getProfileSettings();
    const { owner, includeExternal = settings.includeExternal, force = false } = options;

    // Check cache
    const cacheKey = `map:${owner || 'all'}`;
    const cached = this._checkCache(cacheKey);
    if (cached && !force) {
      return cached;
    }

    this.stats.totalMappings++;

    // Fetch all repos (simulated)
    const repos = await this._fetchAllRepos(owner);

    // Classify repos
    for (const repo of repos.slice(0, settings.maxRepos)) {
      repo.type = this._classifyRepo(repo);
      this.map.repos.set(repo.full_name, repo);
    }
    this.stats.totalReposMapped = this.map.repos.size;

    // Map connections
    this.map.connections = await this._mapConnections(Array.from(this.map.repos.values()));
    this.stats.totalConnections = this.map.connections.length;

    // Map contributors
    if (settings.trackContributors) {
      await this._mapContributors(Array.from(this.map.repos.values()));
      this.stats.totalContributors = this.map.contributors.size;
    }

    // Map external dependencies if requested
    if (includeExternal) {
      await this._mapExternalDependencies(Array.from(this.map.repos.values()));
    }

    // Detect issues
    if (settings.detectDrifts) {
      this.issues = await this.detectIssues();
      this.stats.issuesDetected = this.issues.length;
    }

    this.map.lastSync = Date.now();

    const result = this.getMap();

    // Cache result
    this._cacheResult(cacheKey, result);

    // Emit map updated event
    this._emitMapUpdated();

    return result;
  }

  /**
   * Fetch all repos for owner (simulated)
   * @private
   */
  async _fetchAllRepos(owner) {
    // In real implementation, would use GitHub API
    // For now, return simulated ecosystem repos
    if (!owner) {
      return this._getMockEcosystemRepos();
    }

    return this._getMockOwnerRepos(owner);
  }

  /**
   * Get mock ecosystem repos
   * @private
   */
  _getMockEcosystemRepos() {
    return [
      {
        name: 'CYNIC-new',
        full_name: 'asdfasdfa/CYNIC-new',
        description: 'CYNIC consciousness system',
        fork: false,
        size: 5000,
        language: 'JavaScript',
        default_branch: 'main',
        updated_at: new Date().toISOString(),
      },
      {
        name: 'HolDex',
        full_name: 'asdfasdfa/HolDex',
        description: 'Token holder analysis',
        fork: false,
        size: 3000,
        language: 'JavaScript',
        default_branch: 'main',
        updated_at: new Date().toISOString(),
      },
      {
        name: 'GASdf',
        full_name: 'asdfasdfa/GASdf',
        description: 'Gasless transaction infrastructure',
        fork: false,
        size: 2000,
        language: 'JavaScript',
        default_branch: 'main',
        updated_at: new Date().toISOString(),
      },
      {
        name: 'asdf-brain',
        full_name: 'asdfasdfa/asdf-brain',
        description: 'MCP server for CYNIC',
        fork: false,
        size: 1500,
        language: 'JavaScript',
        default_branch: 'main',
        updated_at: new Date().toISOString(),
      },
      {
        name: 'claude-mem',
        full_name: 'asdfasdfa/claude-mem',
        description: 'Memory persistence',
        fork: false,
        size: 1000,
        language: 'JavaScript',
        default_branch: 'main',
        updated_at: new Date().toISOString(),
      },
    ];
  }

  /**
   * Get mock repos for specific owner
   * @private
   */
  _getMockOwnerRepos(owner) {
    return this._getMockEcosystemRepos().filter(r =>
      r.full_name.startsWith(`${owner}/`)
    );
  }

  /**
   * Classify repo by content
   * @private
   */
  _classifyRepo(repo) {
    const name = repo.name.toLowerCase();

    if (name.includes('cynic') || name.includes('brain')) {
      return RepoType.CORE;
    }
    if (name.includes('gasdf') || name.includes('deploy') || name.includes('infra')) {
      return RepoType.INFRASTRUCTURE;
    }
    if (name.includes('holdex') || name.includes('oracle') || name.includes('intel')) {
      return RepoType.INTELLIGENCE;
    }
    if (name.includes('tool') || name.includes('util') || name.includes('grinder') || name.includes('mem')) {
      return RepoType.TOOL;
    }
    if (repo.fork) {
      return RepoType.FORK;
    }

    return RepoType.EXTERNAL;
  }

  /**
   * Map connections between repos
   * @private
   */
  async _mapConnections(repos) {
    const connections = [];

    // Map known ecosystem connections (simulated)
    const knownConnections = [
      { source: 'asdfasdfa/CYNIC-new', target: 'asdfasdfa/asdf-brain', type: ConnectionType.DEPENDENCY },
      { source: 'asdfasdfa/CYNIC-new', target: 'asdfasdfa/HolDex', type: ConnectionType.IMPORT },
      { source: 'asdfasdfa/CYNIC-new', target: 'asdfasdfa/GASdf', type: ConnectionType.IMPORT },
      { source: 'asdfasdfa/asdf-brain', target: 'asdfasdfa/claude-mem', type: ConnectionType.DEPENDENCY },
      { source: 'asdfasdfa/HolDex', target: 'asdfasdfa/GASdf', type: ConnectionType.SHARED_CODE },
    ];

    for (const conn of knownConnections) {
      // Only add if both repos exist in our map
      if (this.map.repos.has(conn.source) && this.map.repos.has(conn.target)) {
        connections.push({
          ...conn,
          strength: this._calculateConnectionStrength(conn),
          lastUsed: Date.now(),
        });
      }
    }

    return connections;
  }

  /**
   * Calculate connection strength
   * @private
   */
  _calculateConnectionStrength(connection) {
    const weights = {
      [ConnectionType.FORK]: 0.5,
      [ConnectionType.DEPENDENCY]: 0.9,
      [ConnectionType.IMPORT]: 0.8,
      [ConnectionType.UPSTREAM]: 0.7,
      [ConnectionType.DOWNSTREAM]: 0.6,
      [ConnectionType.SHARED_CODE]: 0.8,
      [ConnectionType.CONTRIBUTOR]: 0.4,
    };

    return weights[connection.type] || 0.5;
  }

  /**
   * Map contributors across repos
   * @private
   */
  async _mapContributors(repos) {
    // Simulated contributors
    const mockContributors = [
      { username: 'jeanterre552', repos: ['CYNIC-new', 'HolDex', 'GASdf', 'asdf-brain'], commits: 500, role: 'owner' },
    ];

    for (const contributor of mockContributors) {
      this.map.contributors.set(contributor.username, contributor);
    }
  }

  /**
   * Map external dependencies
   * @private
   */
  async _mapExternalDependencies(repos) {
    // Would analyze package.json files in real implementation
    // For now, add common known dependencies
    const externalDeps = [
      { name: '@anthropic-ai/sdk', version: '^0.30.0', type: RepoType.EXTERNAL },
    ];

    for (const dep of externalDeps) {
      const fullName = `npm/${dep.name}`;
      if (!this.map.repos.has(fullName)) {
        this.map.repos.set(fullName, {
          name: dep.name,
          full_name: fullName,
          type: RepoType.EXTERNAL,
          version: dep.version,
          external: true,
        });
      }
    }
  }

  /**
   * Detect architectural issues
   * @returns {Promise<Object[]>} Issues found
   */
  async detectIssues() {
    const issues = [];

    // Circular dependencies
    const cycles = this._findCycles();
    for (const cycle of cycles) {
      issues.push({
        type: MapIssueType.CIRCULAR_DEPENDENCY,
        severity: 'HIGH',
        repos: cycle,
        message: `Circular dependency detected: ${cycle.join(' -> ')}`,
      });
    }

    // Orphaned repos (no connections)
    for (const repo of this.map.repos.values()) {
      if (repo.external) continue;

      const connections = this.getRepoConnections(repo.full_name);
      if (connections.length === 0) {
        issues.push({
          type: MapIssueType.ORPHANED_REPO,
          severity: 'MEDIUM',
          repo: repo.full_name,
          message: `Repo has no connections to ecosystem: ${repo.full_name}`,
        });
      }
    }

    // Stale forks
    const staleForks = await this._findStaleForks();
    issues.push(...staleForks);

    // Unsynced forks
    const unsynced = await this._findUnsyncedForks();
    issues.push(...unsynced);

    // Emit drift events for critical issues
    for (const issue of issues) {
      if (issue.severity === 'HIGH') {
        this._emitDriftDetected(issue);
      }
    }

    return issues;
  }

  /**
   * Find circular dependencies
   * @private
   */
  _findCycles() {
    const cycles = [];
    const visited = new Set();
    const recursionStack = new Set();

    const dfs = (node, path) => {
      visited.add(node);
      recursionStack.add(node);
      path.push(node);

      const outgoing = this.map.connections.filter(c => c.source === node);

      for (const conn of outgoing) {
        const target = conn.target;

        if (!visited.has(target)) {
          const cycle = dfs(target, [...path]);
          if (cycle) cycles.push(cycle);
        } else if (recursionStack.has(target)) {
          // Found cycle
          const cycleStart = path.indexOf(target);
          if (cycleStart !== -1) {
            cycles.push([...path.slice(cycleStart), target]);
          }
        }
      }

      recursionStack.delete(node);
      return null;
    };

    for (const repo of this.map.repos.keys()) {
      if (!visited.has(repo)) {
        dfs(repo, []);
      }
    }

    return cycles;
  }

  /**
   * Find stale forks
   * @private
   */
  async _findStaleForks() {
    const issues = [];
    const now = Date.now();
    const staleThreshold = CARTOGRAPHER_CONSTANTS.STALE_FORK_DAYS * 24 * 60 * 60 * 1000;

    for (const repo of this.map.repos.values()) {
      if (repo.type === RepoType.FORK) {
        const lastUpdated = new Date(repo.updated_at).getTime();
        if (now - lastUpdated > staleThreshold) {
          issues.push({
            type: MapIssueType.STALE_FORK,
            severity: 'LOW',
            repo: repo.full_name,
            daysSinceUpdate: Math.floor((now - lastUpdated) / (24 * 60 * 60 * 1000)),
            message: `Fork is stale: ${repo.full_name}`,
          });
        }
      }
    }

    return issues;
  }

  /**
   * Find unsynced forks
   * @private
   */
  async _findUnsyncedForks() {
    const issues = [];

    for (const repo of this.map.repos.values()) {
      if (repo.type === RepoType.FORK && repo.commits_behind > CARTOGRAPHER_CONSTANTS.DRIFT_THRESHOLD_COMMITS) {
        issues.push({
          type: MapIssueType.UNSYNCED_FORK,
          severity: 'MEDIUM',
          repo: repo.full_name,
          commitsBehind: repo.commits_behind,
          message: `Fork is behind upstream: ${repo.full_name} (${repo.commits_behind} commits)`,
        });
      }
    }

    return issues;
  }

  /**
   * Check cache
   * @private
   */
  _checkCache(key) {
    const cached = this.cache.get(key);
    if (!cached) return null;

    const age = Date.now() - cached.timestamp;
    if (age > CARTOGRAPHER_CONSTANTS.CACHE_TTL_MS) {
      this.cache.delete(key);
      return null;
    }

    return cached.data;
  }

  /**
   * Cache result
   * @private
   */
  _cacheResult(key, data) {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });

    // Trim cache if needed
    if (this.cache.size > 100) {
      const entries = Array.from(this.cache.entries())
        .sort((a, b) => a[1].timestamp - b[1].timestamp);
      const toRemove = entries.slice(0, entries.length - 100);
      for (const [k] of toRemove) {
        this.cache.delete(k);
      }
    }
  }

  /**
   * Get profile settings
   * @private
   */
  _getProfileSettings() {
    return PROFILE_SETTINGS[this.profileLevel] || PROFILE_SETTINGS[ProfileLevel.PRACTITIONER];
  }

  /**
   * Get current ecosystem map
   * @returns {Object} Map data
   */
  getMap() {
    return {
      repos: Array.from(this.map.repos.values()),
      connections: this.map.connections,
      contributors: Array.from(this.map.contributors.values()),
      stats: this._calculateStats(),
      issues: this.issues,
      lastSync: this.map.lastSync,
    };
  }

  /**
   * Calculate map statistics
   * @private
   */
  _calculateStats() {
    const repos = Array.from(this.map.repos.values());

    return {
      totalRepos: repos.length,
      totalConnections: this.map.connections.length,
      totalContributors: this.map.contributors.size,
      byType: {
        core: repos.filter(r => r.type === RepoType.CORE).length,
        infra: repos.filter(r => r.type === RepoType.INFRASTRUCTURE).length,
        intel: repos.filter(r => r.type === RepoType.INTELLIGENCE).length,
        tool: repos.filter(r => r.type === RepoType.TOOL).length,
        external: repos.filter(r => r.type === RepoType.EXTERNAL).length,
        fork: repos.filter(r => r.type === RepoType.FORK).length,
      },
      issueCount: this.issues.length,
    };
  }

  /**
   * Find all connections for a repo
   * @param {string} repoName - Repo full name
   * @returns {Object[]} Connections
   */
  getRepoConnections(repoName) {
    return this.map.connections.filter(
      c => c.source === repoName || c.target === repoName
    );
  }

  /**
   * Calculate mapping confidence
   * @private
   */
  _calculateMappingConfidence(result) {
    if (!result.repos || result.repos.length === 0) {
      return PHI_INV_2;
    }

    // Higher confidence with more connections and fewer issues
    const connRatio = Math.min(result.connections.length / result.repos.length, 1);
    const issueRatio = 1 - Math.min(result.issues.length / result.repos.length, 1);

    const confidence = (connRatio * 0.6 + issueRatio * 0.4) * PHI_INV;
    return Math.min(confidence, PHI_INV);
  }

  /**
   * Emit map updated event
   * @private
   */
  _emitMapUpdated() {
    if (!this.eventBus) return;

    const event = new MapUpdatedEvent({
      agentId: AgentId.CARTOGRAPHER,
      repoCount: this.map.repos.size,
      connectionCount: this.map.connections.length,
      issues: this.issues.length,
    });

    this.eventBus.emit(AgentEvent.MAP_UPDATED, event);
  }

  /**
   * Emit reality drift detected event
   * @private
   */
  _emitDriftDetected(issue) {
    if (!this.eventBus) return;

    this.stats.driftsDetected++;

    const event = new RealityDriftEvent({
      type: issue.type,
      severity: issue.severity,
      expected: 'clean',
      actual: issue.repo || issue.repos?.join(', '),
      recommendation: issue.message,
    });

    this.eventBus.emit(AgentEvent.REALITY_DRIFT_DETECTED, event);
  }

  /**
   * Generate Mermaid diagram of ecosystem
   * @returns {string} Mermaid diagram
   */
  toMermaid() {
    let diagram = 'graph TD\n';

    // Add repos as nodes
    for (const repo of this.map.repos.values()) {
      if (repo.external) continue;
      const safeName = this._sanitize(repo.name);
      diagram += `    ${safeName}[${repo.name}]:::${repo.type}\n`;
    }

    // Add connections
    for (const conn of this.map.connections) {
      const sourceName = this._sanitize(conn.source.split('/')[1]);
      const targetName = this._sanitize(conn.target.split('/')[1]);
      const arrow = conn.type === 'fork' ? '-.->|fork|' : `-->|${conn.type}|`;
      diagram += `    ${sourceName} ${arrow} ${targetName}\n`;
    }

    // Add styles
    diagram += '\n    classDef core fill:#ffd700\n';
    diagram += '    classDef infra fill:#ff6b6b\n';
    diagram += '    classDef intel fill:#4ecdc4\n';
    diagram += '    classDef tool fill:#95e1d3\n';
    diagram += '    classDef external fill:#dfe6e9\n';
    diagram += '    classDef fork fill:#b2bec3\n';

    return diagram;
  }

  /**
   * Sanitize string for Mermaid
   * @private
   */
  _sanitize(str) {
    return str.replace(/[^a-zA-Z0-9_]/g, '_');
  }

  /**
   * Set profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    if (ProfileLevel[level] !== undefined || Object.values(ProfileLevel).includes(level)) {
      this.profileLevel = level;
    }
  }

  /**
   * Vote on consensus request from Cartographer's reality-mapping perspective
   * @param {string} question - The question to vote on
   * @param {Object} context - Context for the decision
   * @returns {Object} Vote result
   */
  voteOnConsensus(question, context = {}) {
    const questionLower = (question || '').toLowerCase();

    // Cartographer cares about ecosystem coherence, mapping reality, connections
    const coherencePatterns = ['ecosystem', 'map', 'structure', 'connect', 'dependency', 'sync'];
    const fragmentationPatterns = ['isolate', 'fragment', 'break connection', 'orphan', 'drift'];

    const isCoherent = coherencePatterns.some(p => questionLower.includes(p));
    const isFragmenting = fragmentationPatterns.some(p => questionLower.includes(p));

    if (isCoherent) {
      return {
        vote: 'approve',
        reason: '*tail wag* Cartographer approves - maintains ecosystem coherence.',
      };
    }

    if (isFragmenting) {
      return {
        vote: 'reject',
        reason: '*GROWL* Cartographer rejects - this fragments the reality map.',
      };
    }

    return {
      vote: 'abstain',
      reason: '*head tilt* Cartographer abstains - no significant mapping implications.',
    };
  }

// =============================================================================
  // SUPERMEMORY Enhancement Methods
  // =============================================================================

  /**
   * Map local codebase using CodebaseIndexer (SUPERMEMORY enhancement)
   *
   * Indexes all JavaScript files in the current project and extracts
   * dependencies for graph queries.
   *
   * @param {Object} options
   * @param {string} options.rootDir - Root directory to index (default: cwd)
   * @param {Object} options.factsRepo - FactsRepository for storage
   * @param {boolean} options.extractDeps - Extract dependencies (default: true)
   * @returns {Promise<Object>} Indexing results
   */
  async mapLocalCodebase(options = {}) {
    const {
      rootDir = process.cwd(),
      factsRepo = null,
      extractDeps = true,
    } = options;

    // Try to import CodebaseIndexer dynamically
    let CodebaseIndexer;
    try {
      const mod = await import('@cynic/persistence/services/codebase-indexer');
      CodebaseIndexer = mod.CodebaseIndexer || mod.default;
    } catch (e) {
      // Fallback: try relative import
      try {
        const mod = await import('../../../../persistence/src/services/codebase-indexer.js');
        CodebaseIndexer = mod.CodebaseIndexer || mod.default;
      } catch (e2) {
        return { error: 'CodebaseIndexer not available', details: e2.message };
      }
    }

    const indexer = new CodebaseIndexer({
      factsRepo,
      rootDir,
      userId: 'cartographer',
      sessionId: `carto-${Date.now()}`,
      projectName: 'local',
      onProgress: (progress) => {
        // Store progress for status reporting
        this._indexProgress = progress;
      },
    });

    try {
      const result = await indexer.indexAll({
        extractDeps,
        includeKeystone: true,
      });

      // Store reference to indexer for further queries
      this._localIndexer = indexer;

      return {
        success: true,
        filesIndexed: result.filesIndexed,
        factsGenerated: result.factsGenerated,
        dependenciesExtracted: result.dependenciesExtracted,
        durationMs: result.timing?.durationMs,
      };
    } catch (e) {
      return { error: e.message };
    }
  }

  /**
   * Find where a symbol is used across the codebase (SUPERMEMORY enhancement)
   *
   * @param {string} symbol - Symbol name (function, class, variable)
   * @param {Object} options
   * @param {number} options.maxResults - Maximum results (default: 20)
   * @returns {Promise<Array>} Usage locations
   */
  async findSymbolUsage(symbol, options = {}) {
    const { maxResults = 20 } = options;

    if (!this._localIndexer) {
      return { error: 'Call mapLocalCodebase() first' };
    }

    // Use the indexer's getReverseDependencies as a starting point
    const usages = await this._localIndexer.getReverseDependencies(symbol);

    return usages.slice(0, maxResults).map(u => ({
      path: u.path,
      fullPath: u.fullPath,
      type: 'import',
    }));
  }

  /**
   * Get dependency tree for a file (SUPERMEMORY enhancement)
   *
   * @param {string} filePath - File path to analyze
   * @param {Object} options
   * @param {number} options.maxDepth - Maximum depth (default: 3)
   * @returns {Promise<Object>} Dependency tree
   */
  async getDependencyTree(filePath, options = {}) {
    const { maxDepth = 3 } = options;

    if (!this._localIndexer) {
      return { error: 'Call mapLocalCodebase() first' };
    }

    const result = await this._localIndexer.queryDependencyGraph(filePath, {
      maxDepth,
      direction: 'imports',
    });

    return result;
  }

  /**
   * Get reverse dependencies (what depends on this file) (SUPERMEMORY enhancement)
   *
   * @param {string} filePath - File path to analyze
   * @returns {Promise<Array>} Files that depend on this
   */
  async getFileDependents(filePath) {
    if (!this._localIndexer) {
      return { error: 'Call mapLocalCodebase() first' };
    }

    return this._localIndexer.getReverseDependencies(filePath);
  }

  /**
   * Query the local dependency graph (SUPERMEMORY enhancement)
   *
   * @param {string} query - Symbol or file to search for
   * @param {Object} options
   * @returns {Promise<Object>} Graph result
   */
  async queryLocalGraph(query, options = {}) {
    if (!this._localIndexer) {
      return { error: 'Call mapLocalCodebase() first' };
    }

    return this._localIndexer.queryDependencyGraph(query, options);
  }

  /**
   * Get a specific file's info (SUPERMEMORY enhancement)
   *
   * @param {string} query - File path or symbol
   * @returns {Promise<Object|null>} File info
   */
  async getFileInfo(query) {
    if (!this._localIndexer) {
      return { error: 'Call mapLocalCodebase() first' };
    }

    return this._localIndexer.getFile(query);
  }

  // =============================================================================
  // End SUPERMEMORY Enhancement Methods
  // =============================================================================

  /**
   * Get agent summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      name: this.name,
      sefirah: 'Malkhut',
      role: 'Reality Mapping',
      profileLevel: this.profileLevel,
      stats: {
        ...this.stats,
        currentRepos: this.map.repos.size,
        currentConnections: this.map.connections.length,
        currentContributors: this.map.contributors.size,
        cacheSize: this.cache.size,
      },
      lastSync: this.map.lastSync,
      constants: {
        maxRepos: CARTOGRAPHER_CONSTANTS.MAX_REPOS,
        maxConnections: CARTOGRAPHER_CONSTANTS.MAX_CONNECTIONS,
        cacheTtlMs: CARTOGRAPHER_CONSTANTS.CACHE_TTL_MS,
      },
    };
  }

  /**
   * Clear agent state
   */
  clear() {
    this.map = {
      repos: new Map(),
      connections: [],
      contributors: new Map(),
      lastSync: 0,
    };
    this.cache.clear();
    this.issues = [];
    this.stats = {
      totalMappings: 0,
      totalReposMapped: 0,
      totalConnections: 0,
      totalContributors: 0,
      issuesDetected: 0,
      driftsDetected: 0,
    };
  }
}

export default CollectiveCartographer;
