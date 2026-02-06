/**
 * @cynic/node - CollectiveCartographer Deep Tests
 *
 * Comprehensive tests for Cartographer (Malkhut - Kingdom):
 * - Constructor and initialization
 * - shouldTrigger for various events
 * - buildMap with caching, classification, connections
 * - Repo classification (CORE, INFRASTRUCTURE, INTELLIGENCE, TOOL, FORK, EXTERNAL)
 * - Connection mapping with known ecosystem connections
 * - Contributor mapping
 * - External dependency mapping
 * - Issue detection (circular deps, orphaned repos, stale forks, unsynced forks)
 * - Cache management with TTL and trimming
 * - Profile-based settings (NOVICE to MASTER)
 * - Event subscriptions (DISCOVERY_FOUND, DEPLOY_COMPLETED)
 * - getRepoConnections, getMap, getSummary
 * - toMermaid diagram generation
 * - voteOnConsensus
 * - calculateMappingConfidence with φ-alignment
 * - SUPERMEMORY enhancements
 *
 * Uses node:test (NOT vitest) for CI compatibility.
 *
 * "Je mappe la realite telle qu'elle est" - κυνικός Cartographer
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  CollectiveCartographer,
  CARTOGRAPHER_CONSTANTS,
  RepoType,
  ConnectionType,
  MapIssueType,
} from '../src/agents/collective/cartographer.js';
import { AgentEvent, AgentId } from '../src/agents/events.js';
import { ProfileLevel } from '../src/profile/calculator.js';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * Create a mock event bus
 */
function createMockEventBus() {
  return {
    subscriptions: [],
    published: [],
    subscribe(event, agentId, handler) {
      this.subscriptions.push({ event, agentId, handler });
    },
    publish(event) {
      this.published.push(event);
      return Promise.resolve();
    },
    emit(event, data) {
      this.published.push({ event, data });
    },
  };
}

describe('CollectiveCartographer (Deep)', () => {
  let cartographer;
  let eventBus;

  beforeEach(() => {
    eventBus = createMockEventBus();
    cartographer = new CollectiveCartographer({
      eventBus,
      profileLevel: ProfileLevel.PRACTITIONER,
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONSTRUCTOR & INITIALIZATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('constructor', () => {
    it('should set name to Cartographer', () => {
      assert.strictEqual(cartographer.name, 'Cartographer');
    });

    it('should set behavior to BACKGROUND', () => {
      assert.strictEqual(cartographer.behavior, 'background');
    });

    it('should default profileLevel to PRACTITIONER', () => {
      const c = new CollectiveCartographer();
      assert.strictEqual(c.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should initialize empty map with repos, connections, contributors', () => {
      assert.ok(cartographer.map.repos instanceof Map);
      assert.deepStrictEqual(cartographer.map.connections, []);
      assert.ok(cartographer.map.contributors instanceof Map);
      assert.strictEqual(cartographer.map.lastSync, 0);
    });

    it('should initialize empty cache', () => {
      assert.ok(cartographer.cache instanceof Map);
      assert.strictEqual(cartographer.cache.size, 0);
    });

    it('should initialize empty issues array', () => {
      assert.deepStrictEqual(cartographer.issues, []);
    });

    it('should initialize stats with all counters at 0', () => {
      assert.strictEqual(cartographer.stats.totalMappings, 0);
      assert.strictEqual(cartographer.stats.totalReposMapped, 0);
      assert.strictEqual(cartographer.stats.totalConnections, 0);
      assert.strictEqual(cartographer.stats.totalContributors, 0);
      assert.strictEqual(cartographer.stats.issuesDetected, 0);
      assert.strictEqual(cartographer.stats.driftsDetected, 0);
    });

    it('should subscribe to events if eventBus provided', () => {
      const bus = createMockEventBus();
      const c = new CollectiveCartographer({ eventBus: bus });
      assert.ok(bus.subscriptions.length > 0);
    });

    it('should handle missing eventBus gracefully', () => {
      const c = new CollectiveCartographer();
      assert.strictEqual(c.eventBus, null);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONSTANTS
  // ═══════════════════════════════════════════════════════════════════════

  describe('constants', () => {
    it('should have Fibonacci-aligned constants', () => {
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.MAX_REPOS, 233); // Fib(13)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.MAX_CONNECTIONS, 13); // Fib(7)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.MAX_PRS_PER_REPO, 21); // Fib(8)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.MAX_ISSUES_PER_REPO, 21); // Fib(8)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.MAX_CONTRIBUTORS, 233); // Fib(13)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.STALE_FORK_DAYS, 233); // Fib(13)
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.DRIFT_THRESHOLD_COMMITS, 5); // Fib(5)
    });

    it('should have cache TTL of 21 minutes', () => {
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.CACHE_TTL_MS, 21 * 60 * 1000);
    });

    it('should have sync interval of 233 seconds', () => {
      assert.strictEqual(CARTOGRAPHER_CONSTANTS.SYNC_INTERVAL_MS, 233000);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // shouldTrigger
  // ═══════════════════════════════════════════════════════════════════════

  describe('shouldTrigger', () => {
    it('should trigger on map event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'map' }), true);
    });

    it('should trigger on sync event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'sync' }), true);
    });

    it('should trigger on async event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'async' }), true);
    });

    it('should trigger on scheduled event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'scheduled' }), true);
    });

    it('should trigger on github_push event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'github_push' }), true);
    });

    it('should trigger on github_pr event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'github_pr' }), true);
    });

    it('should NOT trigger on unrelated event', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'compile' }), false);
    });

    it('should handle case-insensitive event types', () => {
      assert.strictEqual(cartographer.shouldTrigger({ type: 'MAP' }), true);
      assert.strictEqual(cartographer.shouldTrigger({ type: 'GitHub_Push' }), true);
    });

    it('should handle missing event type', () => {
      assert.strictEqual(cartographer.shouldTrigger({}), false);
      assert.strictEqual(cartographer.shouldTrigger(null), false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // REPO CLASSIFICATION
  // ═══════════════════════════════════════════════════════════════════════

  describe('repo classification', () => {
    it('should classify CYNIC as CORE', () => {
      const type = cartographer._classifyRepo({ name: 'CYNIC-new', fork: false });
      assert.strictEqual(type, RepoType.CORE);
    });

    it('should classify brain repos as CORE', () => {
      const type = cartographer._classifyRepo({ name: 'asdf-brain', fork: false });
      assert.strictEqual(type, RepoType.CORE);
    });

    it('should classify GASdf as INFRASTRUCTURE', () => {
      const type = cartographer._classifyRepo({ name: 'GASdf', fork: false });
      assert.strictEqual(type, RepoType.INFRASTRUCTURE);
    });

    it('should classify deploy tools as INFRASTRUCTURE', () => {
      const type = cartographer._classifyRepo({ name: 'deploy-tool', fork: false });
      assert.strictEqual(type, RepoType.INFRASTRUCTURE);
    });

    it('should classify infra repos as INFRASTRUCTURE', () => {
      const type = cartographer._classifyRepo({ name: 'infra-utils', fork: false });
      assert.strictEqual(type, RepoType.INFRASTRUCTURE);
    });

    it('should classify HolDex as INTELLIGENCE', () => {
      const type = cartographer._classifyRepo({ name: 'HolDex', fork: false });
      assert.strictEqual(type, RepoType.INTELLIGENCE);
    });

    it('should classify oracle repos as INTELLIGENCE', () => {
      const type = cartographer._classifyRepo({ name: 'price-oracle', fork: false });
      assert.strictEqual(type, RepoType.INTELLIGENCE);
    });

    it('should classify intel repos as INTELLIGENCE', () => {
      const type = cartographer._classifyRepo({ name: 'intel-gatherer', fork: false });
      assert.strictEqual(type, RepoType.INTELLIGENCE);
    });

    it('should classify tool repos as TOOL', () => {
      const type = cartographer._classifyRepo({ name: 'dev-tool', fork: false });
      assert.strictEqual(type, RepoType.TOOL);
    });

    it('should classify util repos as TOOL', () => {
      const type = cartographer._classifyRepo({ name: 'utils', fork: false });
      assert.strictEqual(type, RepoType.TOOL);
    });

    it('should classify grinder as TOOL', () => {
      const type = cartographer._classifyRepo({ name: 'grinder', fork: false });
      assert.strictEqual(type, RepoType.TOOL);
    });

    it('should classify mem repos as TOOL', () => {
      const type = cartographer._classifyRepo({ name: 'claude-mem', fork: false });
      assert.strictEqual(type, RepoType.TOOL);
    });

    it('should classify forks as FORK', () => {
      const type = cartographer._classifyRepo({ name: 'some-repo', fork: true });
      assert.strictEqual(type, RepoType.FORK);
    });

    it('should classify unknown repos as EXTERNAL', () => {
      const type = cartographer._classifyRepo({ name: 'unknown-project', fork: false });
      assert.strictEqual(type, RepoType.EXTERNAL);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // buildMap
  // ═══════════════════════════════════════════════════════════════════════

  describe('buildMap', () => {
    it('should fetch ecosystem repos by default', async () => {
      const result = await cartographer.buildMap();
      assert.ok(result.repos.length > 0);
      assert.strictEqual(cartographer.stats.totalMappings, 1);
    });

    it('should classify all repos', async () => {
      const result = await cartographer.buildMap();
      for (const repo of result.repos) {
        assert.ok(repo.type);
        assert.ok(Object.values(RepoType).includes(repo.type));
      }
    });

    it('should map connections between repos', async () => {
      const result = await cartographer.buildMap();
      assert.ok(result.connections.length > 0);
    });

    it('should include ecosystem repos in mock', async () => {
      const result = await cartographer.buildMap();
      assert.ok(result.repos.length >= 5);
    });

    it('should track lastSync timestamp', async () => {
      const before = Date.now();
      await cartographer.buildMap();
      const after = Date.now();
      assert.ok(cartographer.map.lastSync >= before);
      assert.ok(cartographer.map.lastSync <= after);
    });

    it('should increment totalReposMapped stat', async () => {
      await cartographer.buildMap();
      assert.strictEqual(cartographer.stats.totalReposMapped, 5);
    });

    it('should increment totalConnections stat', async () => {
      await cartographer.buildMap();
      assert.ok(cartographer.stats.totalConnections > 0);
    });

    it('should emit MAP_UPDATED event', async () => {
      await cartographer.buildMap();
      assert.ok(eventBus.published.length > 0);
      const mapEvent = eventBus.published.find(e => e.event === AgentEvent.MAP_UPDATED);
      assert.ok(mapEvent);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CACHE MANAGEMENT
  // ═══════════════════════════════════════════════════════════════════════

  describe('cache management', () => {
    it('should cache buildMap results', async () => {
      await cartographer.buildMap();
      assert.strictEqual(cartographer.cache.size, 1);
    });

    it('should return cached result on subsequent calls', async () => {
      const result1 = await cartographer.buildMap();
      const result2 = await cartographer.buildMap();
      assert.deepStrictEqual(result1, result2);
      assert.strictEqual(cartographer.stats.totalMappings, 1);
    });

    it('should bypass cache when force=true', async () => {
      await cartographer.buildMap();
      await cartographer.buildMap({ force: true });
      assert.strictEqual(cartographer.stats.totalMappings, 2);
    });

    it('should respect cache TTL', async () => {
      await cartographer.buildMap();
      // Simulate expired cache
      const cacheKey = 'map:all';
      const cached = cartographer.cache.get(cacheKey);
      cached.timestamp = Date.now() - (CARTOGRAPHER_CONSTANTS.CACHE_TTL_MS + 1000);
      cartographer.cache.set(cacheKey, cached);

      await cartographer.buildMap();
      assert.strictEqual(cartographer.stats.totalMappings, 2);
    });

    it('should trim cache at 100 entries', async () => {
      // Add 105 cache entries
      for (let i = 0; i < 105; i++) {
        cartographer.cache.set(`key${i}`, {
          data: { test: i },
          timestamp: Date.now() - i * 1000,
        });
      }

      // Trigger cache result which trims
      cartographer._cacheResult('newkey', { test: 'data' });

      assert.ok(cartographer.cache.size <= 100);
    });

    it('should keep most recent entries when trimming', async () => {
      // Add entries with different timestamps
      for (let i = 0; i < 105; i++) {
        cartographer.cache.set(`key${i}`, {
          data: i,
          timestamp: Date.now() - (105 - i) * 1000,
        });
      }

      // Trigger trim by adding new entry
      cartographer._cacheResult('newest', { test: 'data' });

      // Should keep at most 100 entries
      assert.ok(cartographer.cache.size <= 100);
      // Newest should be kept
      assert.ok(cartographer.cache.has('newest'));
    });

    it('should invalidate cache on _invalidateCache', () => {
      cartographer.cache.set('test:service1', { data: 'test', timestamp: Date.now() });
      cartographer.cache.set('other:service2', { data: 'test', timestamp: Date.now() });

      cartographer._invalidateCache('service1');

      assert.strictEqual(cartographer.cache.has('test:service1'), false);
      assert.strictEqual(cartographer.cache.has('other:service2'), true);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONNECTION MAPPING
  // ═══════════════════════════════════════════════════════════════════════

  describe('connection mapping', () => {
    it('should map 5 known ecosystem connections', async () => {
      const result = await cartographer.buildMap();
      assert.strictEqual(result.connections.length, 5);
    });

    it('should only include connections where both repos exist', async () => {
      const result = await cartographer.buildMap();
      for (const conn of result.connections) {
        assert.ok(cartographer.map.repos.has(conn.source));
        assert.ok(cartographer.map.repos.has(conn.target));
      }
    });

    it('should calculate connection strength', async () => {
      const result = await cartographer.buildMap();
      for (const conn of result.connections) {
        assert.ok(conn.strength);
        assert.ok(conn.strength > 0 && conn.strength <= 1);
      }
    });

    it('should assign DEPENDENCY strength of 0.9', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.DEPENDENCY,
      });
      assert.strictEqual(strength, 0.9);
    });

    it('should assign IMPORT strength of 0.8', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.IMPORT,
      });
      assert.strictEqual(strength, 0.8);
    });

    it('should assign SHARED_CODE strength of 0.8', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.SHARED_CODE,
      });
      assert.strictEqual(strength, 0.8);
    });

    it('should assign UPSTREAM strength of 0.7', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.UPSTREAM,
      });
      assert.strictEqual(strength, 0.7);
    });

    it('should assign DOWNSTREAM strength of 0.6', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.DOWNSTREAM,
      });
      assert.strictEqual(strength, 0.6);
    });

    it('should assign FORK strength of 0.5', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.FORK,
      });
      assert.strictEqual(strength, 0.5);
    });

    it('should assign CONTRIBUTOR strength of 0.4', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: ConnectionType.CONTRIBUTOR,
      });
      assert.strictEqual(strength, 0.4);
    });

    it('should default to 0.5 for unknown type', () => {
      const strength = cartographer._calculateConnectionStrength({
        type: 'unknown',
      });
      assert.strictEqual(strength, 0.5);
    });

    it('should include lastUsed timestamp in connections', async () => {
      const result = await cartographer.buildMap();
      for (const conn of result.connections) {
        assert.ok(conn.lastUsed);
        assert.ok(typeof conn.lastUsed === 'number');
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // CONTRIBUTOR MAPPING
  // ═══════════════════════════════════════════════════════════════════════

  describe('contributor mapping', () => {
    it('should track contributors when enabled', async () => {
      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      const result = await cartographer.buildMap();
      assert.ok(result.contributors.length > 0);
    });

    it('should NOT track contributors for NOVICE', async () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const result = await cartographer.buildMap();
      assert.strictEqual(result.contributors.length, 0);
    });

    it('should map jeanterre552 as contributor', async () => {
      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      await cartographer.buildMap();
      assert.ok(cartographer.map.contributors.has('jeanterre552'));
    });

    it('should include contributor details', async () => {
      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      await cartographer.buildMap();
      const contributor = cartographer.map.contributors.get('jeanterre552');
      assert.ok(contributor.username);
      assert.ok(contributor.repos);
      assert.ok(contributor.commits);
      assert.ok(contributor.role);
    });

    it('should increment totalContributors stat', async () => {
      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      await cartographer.buildMap();
      assert.ok(cartographer.stats.totalContributors > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // EXTERNAL DEPENDENCIES
  // ═══════════════════════════════════════════════════════════════════════

  describe('external dependencies', () => {
    it('should map external deps when includeExternal=true', async () => {
      const result = await cartographer.buildMap({ includeExternal: true });
      const external = result.repos.find(r => r.external);
      assert.ok(external);
    });

    it('should NOT map external deps when includeExternal=false', async () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const result = await cartographer.buildMap({ includeExternal: false });
      const external = result.repos.find(r => r.external);
      assert.strictEqual(external, undefined);
    });

    it('should include npm packages as external repos', async () => {
      const result = await cartographer.buildMap({ includeExternal: true });
      const npmRepo = result.repos.find(r => r.full_name.startsWith('npm/'));
      assert.ok(npmRepo);
    });

    it('should mark external repos with external=true', async () => {
      const result = await cartographer.buildMap({ includeExternal: true });
      const external = result.repos.find(r => r.external);
      assert.strictEqual(external.external, true);
    });

    it('should queue external deps on DISCOVERY_FOUND event', async () => {
      const handler = eventBus.subscriptions.find(
        s => s.event === AgentEvent.DISCOVERY_FOUND
      )?.handler;

      handler({
        payload: {
          type: 'dependency',
          path: '/test',
          details: { name: 'new-package', version: '1.0.0', external: true },
        },
      });

      assert.ok(cartographer.cache.has('ext:new-package'));
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // ISSUE DETECTION
  // ═══════════════════════════════════════════════════════════════════════

  describe('issue detection', () => {
    it('should detect orphaned repos', async () => {
      // Add a repo with no connections
      cartographer.map.repos.set('asdfasdfa/orphaned', {
        full_name: 'asdfasdfa/orphaned',
        name: 'orphaned',
        external: false,
      });

      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      const issues = await cartographer.detectIssues();
      const orphaned = issues.find(i => i.type === MapIssueType.ORPHANED_REPO);
      assert.ok(orphaned);
    });

    it('should NOT flag external repos as orphaned', async () => {
      await cartographer.buildMap({ includeExternal: true });
      const issues = await cartographer.detectIssues();
      const orphanedExternal = issues.find(
        i => i.type === MapIssueType.ORPHANED_REPO && i.repo.includes('npm/')
      );
      assert.strictEqual(orphanedExternal, undefined);
    });

    it('should detect stale forks', async () => {
      // Add a stale fork
      const staleFork = {
        name: 'old-fork',
        full_name: 'user/old-fork',
        type: RepoType.FORK,
        updated_at: new Date(Date.now() - 300 * 24 * 60 * 60 * 1000).toISOString(), // 300 days old
      };
      cartographer.map.repos.set(staleFork.full_name, staleFork);

      const issues = await cartographer.detectIssues();
      const stale = issues.find(i => i.type === MapIssueType.STALE_FORK);
      assert.ok(stale);
    });

    it('should detect unsynced forks', async () => {
      // Add an unsynced fork
      const unsyncedFork = {
        name: 'fork',
        full_name: 'user/fork',
        type: RepoType.FORK,
        commits_behind: 10,
      };
      cartographer.map.repos.set(unsyncedFork.full_name, unsyncedFork);

      const issues = await cartographer.detectIssues();
      const unsynced = issues.find(i => i.type === MapIssueType.UNSYNCED_FORK);
      assert.ok(unsynced);
    });

    it('should NOT detect unsynced if behind < DRIFT_THRESHOLD_COMMITS', async () => {
      const fork = {
        name: 'fork',
        full_name: 'user/fork',
        type: RepoType.FORK,
        commits_behind: 3, // Below threshold of 5
      };
      cartographer.map.repos.set(fork.full_name, fork);

      const issues = await cartographer.detectIssues();
      const unsynced = issues.find(
        i => i.type === MapIssueType.UNSYNCED_FORK && i.repo === fork.full_name
      );
      assert.strictEqual(unsynced, undefined);
    });

    it('should detect circular dependencies', async () => {
      // Create circular dependency
      cartographer.map.repos.set('a/repo1', { full_name: 'a/repo1' });
      cartographer.map.repos.set('a/repo2', { full_name: 'a/repo2' });
      cartographer.map.connections = [
        { source: 'a/repo1', target: 'a/repo2', type: ConnectionType.DEPENDENCY },
        { source: 'a/repo2', target: 'a/repo1', type: ConnectionType.DEPENDENCY },
      ];

      const issues = await cartographer.detectIssues();
      const circular = issues.find(i => i.type === MapIssueType.CIRCULAR_DEPENDENCY);
      assert.ok(circular);
    });

    it('should emit REALITY_DRIFT_DETECTED for HIGH severity issues', async () => {
      // Create circular dependency (HIGH severity)
      cartographer.map.repos.set('a/repo1', { full_name: 'a/repo1' });
      cartographer.map.repos.set('a/repo2', { full_name: 'a/repo2' });
      cartographer.map.connections = [
        { source: 'a/repo1', target: 'a/repo2', type: ConnectionType.DEPENDENCY },
        { source: 'a/repo2', target: 'a/repo1', type: ConnectionType.DEPENDENCY },
      ];

      await cartographer.detectIssues();

      const driftEvent = eventBus.published.find(
        e => e.event === AgentEvent.REALITY_DRIFT_DETECTED
      );
      assert.ok(driftEvent);
    });

    it('should increment driftsDetected stat', async () => {
      // Create circular dependency
      cartographer.map.repos.set('a/repo1', { full_name: 'a/repo1' });
      cartographer.map.repos.set('a/repo2', { full_name: 'a/repo2' });
      cartographer.map.connections = [
        { source: 'a/repo1', target: 'a/repo2', type: ConnectionType.DEPENDENCY },
        { source: 'a/repo2', target: 'a/repo1', type: ConnectionType.DEPENDENCY },
      ];

      await cartographer.detectIssues();

      assert.ok(cartographer.stats.driftsDetected > 0);
    });

    it('should NOT detect issues if detectDrifts=false', async () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const result = await cartographer.buildMap();
      assert.strictEqual(result.issues.length, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // getRepoConnections
  // ═══════════════════════════════════════════════════════════════════════

  describe('getRepoConnections', () => {
    it('should return connections for a repo as source', async () => {
      await cartographer.buildMap();
      const connections = cartographer.getRepoConnections('asdfasdfa/CYNIC-new');
      assert.ok(connections.length > 0);
    });

    it('should return connections for a repo as target', async () => {
      await cartographer.buildMap();
      const connections = cartographer.getRepoConnections('asdfasdfa/asdf-brain');
      assert.ok(connections.length > 0);
    });

    it('should return empty array for repo with no connections', () => {
      const connections = cartographer.getRepoConnections('nonexistent/repo');
      assert.deepStrictEqual(connections, []);
    });

    it('should include both inbound and outbound connections', async () => {
      await cartographer.buildMap();
      const connections = cartographer.getRepoConnections('asdfasdfa/GASdf');
      const asSource = connections.filter(c => c.source === 'asdfasdfa/GASdf');
      const asTarget = connections.filter(c => c.target === 'asdfasdfa/GASdf');
      assert.ok(asSource.length > 0 || asTarget.length > 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // PROFILE-BASED SETTINGS
  // ═══════════════════════════════════════════════════════════════════════

  describe('profile-based settings', () => {
    it('should limit NOVICE to 21 repos', async () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.maxRepos, 21);
    });

    it('should disable external for NOVICE', () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.includeExternal, false);
    });

    it('should disable contributors for NOVICE', () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.trackContributors, false);
    });

    it('should disable drifts for NOVICE', () => {
      cartographer.profileLevel = ProfileLevel.NOVICE;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.detectDrifts, false);
    });

    it('should allow MASTER to map 233 repos', () => {
      cartographer.profileLevel = ProfileLevel.MASTER;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.maxRepos, 233);
    });

    it('should enable all features for MASTER', () => {
      cartographer.profileLevel = ProfileLevel.MASTER;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.includeExternal, true);
      assert.strictEqual(settings.trackContributors, true);
      assert.strictEqual(settings.detectDrifts, true);
      assert.strictEqual(settings.deepAnalysis, true);
    });

    it('should enable features for PRACTITIONER', () => {
      cartographer.profileLevel = ProfileLevel.PRACTITIONER;
      const settings = cartographer._getProfileSettings();
      assert.strictEqual(settings.maxRepos, 89);
      assert.strictEqual(settings.includeExternal, true);
      assert.strictEqual(settings.trackContributors, true);
      assert.strictEqual(settings.detectDrifts, true);
    });

    it('should setProfileLevel correctly', () => {
      cartographer.setProfileLevel(ProfileLevel.EXPERT);
      assert.strictEqual(cartographer.profileLevel, ProfileLevel.EXPERT);
    });

    it('should accept ProfileLevel enum values', () => {
      cartographer.setProfileLevel(ProfileLevel.EXPERT);
      assert.strictEqual(cartographer.profileLevel, ProfileLevel.EXPERT);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // EVENT SUBSCRIPTIONS
  // ═══════════════════════════════════════════════════════════════════════

  describe('event subscriptions', () => {
    it('should subscribe to DISCOVERY_FOUND', () => {
      const sub = eventBus.subscriptions.find(
        s => s.event === AgentEvent.DISCOVERY_FOUND && s.agentId === AgentId.CARTOGRAPHER
      );
      assert.ok(sub);
    });

    it('should subscribe to DEPLOY_COMPLETED', () => {
      const sub = eventBus.subscriptions.find(
        s => s.event === AgentEvent.DEPLOY_COMPLETED && s.agentId === AgentId.CARTOGRAPHER
      );
      assert.ok(sub);
    });

    it('should handle DISCOVERY_FOUND event', async () => {
      const handler = eventBus.subscriptions.find(
        s => s.event === AgentEvent.DISCOVERY_FOUND
      )?.handler;

      handler({
        payload: {
          type: 'dependency',
          path: '/src/test.js',
          details: {
            name: 'test-package',
            version: '1.0.0',
            external: true,
          },
        },
      });

      assert.ok(cartographer.cache.has('ext:test-package'));
    });

    it('should handle DEPLOY_COMPLETED event', async () => {
      cartographer.cache.set('map:service1', { data: 'test', timestamp: Date.now() });

      const handler = eventBus.subscriptions.find(
        s => s.event === AgentEvent.DEPLOY_COMPLETED
      )?.handler;

      handler({
        payload: {
          service: 'service1',
          version: '1.0.0',
        },
      });

      assert.strictEqual(cartographer.cache.has('map:service1'), false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // getMap
  // ═══════════════════════════════════════════════════════════════════════

  describe('getMap', () => {
    it('should return repos as array', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(Array.isArray(map.repos));
    });

    it('should return connections array', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(Array.isArray(map.connections));
    });

    it('should return contributors as array', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(Array.isArray(map.contributors));
    });

    it('should include stats', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(map.stats);
      assert.ok('totalRepos' in map.stats);
      assert.ok('totalConnections' in map.stats);
      assert.ok('totalContributors' in map.stats);
    });

    it('should include stats.byType breakdown', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(map.stats.byType);
      assert.ok('core' in map.stats.byType);
      assert.ok('infra' in map.stats.byType);
      assert.ok('intel' in map.stats.byType);
      assert.ok('tool' in map.stats.byType);
      assert.ok('external' in map.stats.byType);
      assert.ok('fork' in map.stats.byType);
    });

    it('should include issues array', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(Array.isArray(map.issues));
    });

    it('should include lastSync timestamp', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      assert.ok(map.lastSync);
      assert.ok(typeof map.lastSync === 'number');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // getSummary
  // ═══════════════════════════════════════════════════════════════════════

  describe('getSummary', () => {
    it('should return summary with name', () => {
      const summary = cartographer.getSummary();
      assert.strictEqual(summary.name, 'Cartographer');
    });

    it('should return sefirah Malkhut', () => {
      const summary = cartographer.getSummary();
      assert.strictEqual(summary.sefirah, 'Malkhut');
    });

    it('should return role Reality Mapping', () => {
      const summary = cartographer.getSummary();
      assert.strictEqual(summary.role, 'Reality Mapping');
    });

    it('should include profileLevel', () => {
      const summary = cartographer.getSummary();
      assert.strictEqual(summary.profileLevel, ProfileLevel.PRACTITIONER);
    });

    it('should include stats', () => {
      const summary = cartographer.getSummary();
      assert.ok(summary.stats);
      assert.ok('totalMappings' in summary.stats);
      assert.ok('totalReposMapped' in summary.stats);
      assert.ok('currentRepos' in summary.stats);
      assert.ok('cacheSize' in summary.stats);
    });

    it('should include lastSync', () => {
      const summary = cartographer.getSummary();
      assert.ok('lastSync' in summary);
    });

    it('should include constants', () => {
      const summary = cartographer.getSummary();
      assert.ok(summary.constants);
      assert.strictEqual(summary.constants.maxRepos, 233);
      assert.strictEqual(summary.constants.maxConnections, 13);
      assert.strictEqual(summary.constants.cacheTtlMs, 21 * 60 * 1000);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // clear
  // ═══════════════════════════════════════════════════════════════════════

  describe('clear', () => {
    it('should reset map', async () => {
      await cartographer.buildMap();
      cartographer.clear();
      assert.strictEqual(cartographer.map.repos.size, 0);
      assert.strictEqual(cartographer.map.connections.length, 0);
      assert.strictEqual(cartographer.map.contributors.size, 0);
      assert.strictEqual(cartographer.map.lastSync, 0);
    });

    it('should clear cache', async () => {
      await cartographer.buildMap();
      cartographer.clear();
      assert.strictEqual(cartographer.cache.size, 0);
    });

    it('should clear issues', async () => {
      await cartographer.buildMap();
      cartographer.clear();
      assert.deepStrictEqual(cartographer.issues, []);
    });

    it('should reset stats', async () => {
      await cartographer.buildMap();
      cartographer.clear();
      assert.strictEqual(cartographer.stats.totalMappings, 0);
      assert.strictEqual(cartographer.stats.totalReposMapped, 0);
      assert.strictEqual(cartographer.stats.totalConnections, 0);
      assert.strictEqual(cartographer.stats.totalContributors, 0);
      assert.strictEqual(cartographer.stats.issuesDetected, 0);
      assert.strictEqual(cartographer.stats.driftsDetected, 0);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // toMermaid
  // ═══════════════════════════════════════════════════════════════════════

  describe('toMermaid', () => {
    it('should generate Mermaid diagram', async () => {
      await cartographer.buildMap();
      const diagram = cartographer.toMermaid();
      assert.ok(diagram.includes('graph TD'));
    });

    it('should include repo nodes', async () => {
      await cartographer.buildMap();
      const diagram = cartographer.toMermaid();
      assert.ok(diagram.includes('CYNIC_new'));
    });

    it('should include connections', async () => {
      await cartographer.buildMap();
      const diagram = cartographer.toMermaid();
      assert.ok(diagram.includes('-->') || diagram.includes('-.->'));
    });

    it('should include classDef styles', async () => {
      await cartographer.buildMap();
      const diagram = cartographer.toMermaid();
      assert.ok(diagram.includes('classDef core'));
      assert.ok(diagram.includes('classDef infra'));
      assert.ok(diagram.includes('classDef intel'));
      assert.ok(diagram.includes('classDef tool'));
    });

    it('should sanitize repo names', () => {
      const sanitized = cartographer._sanitize('test-repo/name');
      assert.strictEqual(sanitized, 'test_repo_name');
    });

    it('should NOT include external repos in diagram', async () => {
      await cartographer.buildMap({ includeExternal: true });
      const diagram = cartographer.toMermaid();
      assert.ok(!diagram.includes('npm_'));
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // voteOnConsensus
  // ═══════════════════════════════════════════════════════════════════════

  describe('voteOnConsensus', () => {
    it('should APPROVE ecosystem coherence', () => {
      const vote = cartographer.voteOnConsensus('Should we improve ecosystem mapping?');
      assert.strictEqual(vote.vote, 'approve');
      assert.ok(vote.reason.includes('tail wag'));
    });

    it('should APPROVE mapping questions', () => {
      const vote = cartographer.voteOnConsensus('Should we map the repository structure?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should APPROVE connection questions', () => {
      const vote = cartographer.voteOnConsensus('Should we connect these services?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should APPROVE dependency tracking', () => {
      const vote = cartographer.voteOnConsensus('Should we track ecosystem dependencies?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should APPROVE sync questions', () => {
      const vote = cartographer.voteOnConsensus('Should we sync the repositories?');
      assert.strictEqual(vote.vote, 'approve');
    });

    it('should REJECT fragmentation', () => {
      const vote = cartographer.voteOnConsensus('Should we isolate this repository?');
      assert.strictEqual(vote.vote, 'reject');
      assert.ok(vote.reason.includes('GROWL'));
    });

    it('should REJECT fragmentation', () => {
      const vote = cartographer.voteOnConsensus('Should we fragment the services?');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should REJECT orphaning repos', () => {
      const vote = cartographer.voteOnConsensus('Should we orphan this service?');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should REJECT drift', () => {
      const vote = cartographer.voteOnConsensus('Allow drift from upstream?');
      assert.strictEqual(vote.vote, 'reject');
    });

    it('should ABSTAIN on ambiguous questions', () => {
      const vote = cartographer.voteOnConsensus('What should we do?');
      assert.strictEqual(vote.vote, 'abstain');
      assert.ok(vote.reason.includes('head tilt'));
    });

    it('should handle empty question', () => {
      const vote = cartographer.voteOnConsensus('');
      assert.strictEqual(vote.vote, 'abstain');
    });

    it('should handle null question', () => {
      const vote = cartographer.voteOnConsensus(null);
      assert.strictEqual(vote.vote, 'abstain');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // φ-ALIGNMENT
  // ═══════════════════════════════════════════════════════════════════════

  describe('φ-alignment', () => {
    it('should never exceed φ⁻¹ confidence', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      const confidence = cartographer._calculateMappingConfidence(map);
      assert.ok(confidence <= PHI_INV + 0.001);
    });

    it('should return φ⁻² confidence for empty repos', () => {
      const confidence = cartographer._calculateMappingConfidence({ repos: [], connections: [], issues: [] });
      assert.strictEqual(confidence, PHI_INV_2);
    });

    it('should blend connection and issue ratios', async () => {
      await cartographer.buildMap();
      const map = cartographer.getMap();
      const confidence = cartographer._calculateMappingConfidence(map);
      assert.ok(confidence > 0);
      assert.ok(confidence <= PHI_INV);
    });

    it('should cap confidence at φ⁻¹', () => {
      const perfectMap = {
        repos: [1, 2, 3, 4, 5],
        connections: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        issues: [],
      };
      const confidence = cartographer._calculateMappingConfidence(perfectMap);
      assert.ok(confidence <= PHI_INV);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════
  // PROCESS
  // ═══════════════════════════════════════════════════════════════════════

  describe('process', () => {
    it('should return result with agent ID', async () => {
      const result = await cartographer.process({ type: 'map' }, {});
      assert.strictEqual(result.agent, AgentId.CARTOGRAPHER);
      assert.ok(result.result);
    });

    it('should include result', async () => {
      const result = await cartographer.process({ type: 'map' }, {});
      assert.ok(result.result);
      assert.ok(result.result.repos);
    });

    it('should include confidence', async () => {
      const result = await cartographer.process({ type: 'map' }, {});
      assert.ok(result.confidence);
      assert.ok(result.confidence <= PHI_INV);
    });

    it('should handle errors gracefully', async () => {
      // Force an error by mocking
      const originalFetch = cartographer._fetchAllRepos;
      cartographer._fetchAllRepos = async () => {
        throw new Error('Test error');
      };

      const result = await cartographer.process({ type: 'map' }, {});
      assert.ok(result.error);

      cartographer._fetchAllRepos = originalFetch;
    });

    it('should respect force option', async () => {
      await cartographer.process({ type: 'map' }, {});
      await cartographer.process({ type: 'map', force: true }, {});
      assert.strictEqual(cartographer.stats.totalMappings, 2);
    });

    it('should use owner from event', async () => {
      const result = await cartographer.process({ type: 'map', owner: 'asdfasdfa' }, {});
      assert.ok(result.result);
    });

    it('should use includeExternal from event', async () => {
      const result = await cartographer.process({ type: 'map', includeExternal: true }, {});
      const external = result.result.repos.find(r => r.external);
      assert.ok(external);
    });
  });
});
