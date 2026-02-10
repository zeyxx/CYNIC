/**
 * Tests for ProcessRegistry — Cross-Process Discovery
 *
 * Gap #6 fix: Verifies announce/discover/depart lifecycle,
 * staleness pruning, peer queries, and file-based persistence.
 *
 * @module @cynic/core/test/process-registry
 */

'use strict';

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'fs';
import path from 'path';
import os from 'os';

import { processRegistry } from '../src/topology/process-registry.js';

// Use isolated path for test — prevents concurrent test files from clobbering
const TEST_REGISTRY_DIR = path.join(os.tmpdir(), '.cynic-test-pr', `${process.pid}`);
const TEST_REGISTRY_FILE = path.join(TEST_REGISTRY_DIR, 'registry.json');

describe('ProcessRegistry', () => {
  beforeEach(() => {
    processRegistry._resetForTesting();
    processRegistry._setRegistryPath(TEST_REGISTRY_FILE);
  });

  afterEach(() => {
    processRegistry._resetForTesting();
    // Clean up test dir
    try { fs.rmSync(TEST_REGISTRY_DIR, { recursive: true }); } catch { /* ok */ }
  });

  // ──────────────────────────────────────────────────────────
  // ANNOUNCE
  // ──────────────────────────────────────────────────────────

  describe('announce()', () => {
    it('should write process entry to registry file', () => {
      const id = processRegistry.announce({
        mode: 'mcp',
        endpoint: 'http://localhost:3000',
        capabilities: ['judge', 'consensus'],
      });

      assert.ok(id, 'Should return process ID');
      assert.ok(id.startsWith('mcp-'), 'ID should start with mode');
      assert.ok(fs.existsSync(TEST_REGISTRY_FILE), 'Registry file should exist');

      const data = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      assert.ok(data[id], 'Entry should exist in registry');
      assert.equal(data[id].mode, 'mcp');
      assert.equal(data[id].endpoint, 'http://localhost:3000');
      assert.equal(data[id].pid, process.pid);
    });

    it('should include capabilities in announcement', () => {
      const id = processRegistry.announce({
        mode: 'daemon',
        capabilities: ['persistence', 'learning', 'solana'],
      });

      const data = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      assert.deepEqual(data[id].capabilities, ['persistence', 'learning', 'solana']);
    });

    it('should include metadata', () => {
      const id = processRegistry.announce({
        mode: 'mcp',
        meta: { version: '1.0', env: 'local' },
      });

      const data = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      assert.equal(data[id].meta.version, '1.0');
      assert.equal(data[id].meta.env, 'local');
    });

    it('should track startedAt timestamp', () => {
      const before = Date.now();
      const id = processRegistry.announce({ mode: 'mcp' });
      const after = Date.now();

      const data = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      assert.ok(data[id].startedAt >= before);
      assert.ok(data[id].startedAt <= after);
    });
  });

  // ──────────────────────────────────────────────────────────
  // DISCOVER
  // ──────────────────────────────────────────────────────────

  describe('discover()', () => {
    it('should return empty when no processes registered', () => {
      const result = processRegistry.discover();
      assert.equal(result.processes.length, 0);
      assert.equal(result.stale.length, 0);
      assert.equal(result.self, null);
    });

    it('should find announced process', () => {
      processRegistry.announce({ mode: 'mcp', endpoint: 'http://localhost:3000' });

      const result = processRegistry.discover();
      assert.equal(result.processes.length, 1);
      assert.equal(result.processes[0].mode, 'mcp');
      assert.equal(result.processes[0].endpoint, 'http://localhost:3000');
    });

    it('should include self reference', () => {
      processRegistry.announce({ mode: 'mcp' });

      const result = processRegistry.discover();
      assert.ok(result.self, 'Self should be set after announcement');
      assert.ok(result.self.startsWith('mcp-'));
    });

    it('should filter by mode', () => {
      // Write two different entries manually
      const registry = {};
      registry['mcp-1234'] = {
        mode: 'mcp', pid: 1234, lastHeartbeat: Date.now(),
        endpoint: null, capabilities: [], meta: {}, startedAt: Date.now(),
      };
      registry['daemon-5678'] = {
        mode: 'daemon', pid: 5678, lastHeartbeat: Date.now(),
        endpoint: null, capabilities: [], meta: {}, startedAt: Date.now(),
      };
      if (!fs.existsSync(TEST_REGISTRY_DIR)) fs.mkdirSync(TEST_REGISTRY_DIR, { recursive: true });
      fs.writeFileSync(TEST_REGISTRY_FILE, JSON.stringify(registry));

      const mcpOnly = processRegistry.discover({ mode: 'mcp' });
      assert.equal(mcpOnly.processes.length, 1);
      assert.equal(mcpOnly.processes[0].mode, 'mcp');

      const daemonOnly = processRegistry.discover({ mode: 'daemon' });
      assert.equal(daemonOnly.processes.length, 1);
      assert.equal(daemonOnly.processes[0].mode, 'daemon');
    });

    it('should detect stale entries', () => {
      // Write entry with old heartbeat
      const registry = {};
      registry['mcp-9999'] = {
        mode: 'mcp', pid: 9999,
        lastHeartbeat: Date.now() - 120_000, // 2 minutes ago (stale but not max age)
        endpoint: null, capabilities: [], meta: {}, startedAt: Date.now() - 300_000,
      };
      if (!fs.existsSync(TEST_REGISTRY_DIR)) fs.mkdirSync(TEST_REGISTRY_DIR, { recursive: true });
      fs.writeFileSync(TEST_REGISTRY_FILE, JSON.stringify(registry));

      const result = processRegistry.discover();
      assert.equal(result.processes.length, 0, 'Stale entries excluded by default');
      assert.equal(result.stale.length, 1, 'Stale entries tracked');

      const withStale = processRegistry.discover({ includeStale: true });
      assert.equal(withStale.processes.length, 1, 'Stale included when requested');
    });

    it('should hard prune entries older than max age', () => {
      const registry = {};
      registry['mcp-dead'] = {
        mode: 'mcp', pid: 1111,
        lastHeartbeat: Date.now() - 15 * 60 * 1000, // 15 minutes ago
        endpoint: null, capabilities: [], meta: {}, startedAt: Date.now() - 20 * 60 * 1000,
      };
      if (!fs.existsSync(TEST_REGISTRY_DIR)) fs.mkdirSync(TEST_REGISTRY_DIR, { recursive: true });
      fs.writeFileSync(TEST_REGISTRY_FILE, JSON.stringify(registry));

      const result = processRegistry.discover({ includeStale: true });
      assert.equal(result.processes.length, 0, 'Max-age entries hard pruned');
      assert.equal(result.stale.length, 0, 'Not even in stale list');
    });
  });

  // ──────────────────────────────────────────────────────────
  // isAvailable
  // ──────────────────────────────────────────────────────────

  describe('isAvailable()', () => {
    it('should return unavailable when no process registered', () => {
      const result = processRegistry.isAvailable('mcp');
      assert.equal(result.available, false);
      assert.equal(result.endpoint, null);
      assert.equal(result.pid, null);
    });

    it('should return available with endpoint when registered', () => {
      processRegistry.announce({
        mode: 'mcp',
        endpoint: 'http://localhost:3000',
      });

      const result = processRegistry.isAvailable('mcp');
      assert.equal(result.available, true);
      assert.equal(result.endpoint, 'http://localhost:3000');
      assert.equal(result.pid, process.pid);
    });

    it('should return unavailable for different mode', () => {
      processRegistry.announce({ mode: 'daemon' });

      const result = processRegistry.isAvailable('mcp');
      assert.equal(result.available, false);
    });
  });

  // ──────────────────────────────────────────────────────────
  // getPeerSummary
  // ──────────────────────────────────────────────────────────

  describe('getPeerSummary()', () => {
    it('should return empty summary initially', () => {
      const summary = processRegistry.getPeerSummary();
      assert.equal(summary.total, 0);
      assert.equal(summary.healthy, 0);
      assert.equal(summary.stale, 0);
      assert.equal(summary.selfId, null);
    });

    it('should group by mode', () => {
      // Write two entries
      const registry = {};
      registry['mcp-100'] = {
        mode: 'mcp', pid: 100, lastHeartbeat: Date.now(),
        endpoint: 'http://localhost:3000', capabilities: [], meta: {}, startedAt: Date.now(),
      };
      registry['daemon-200'] = {
        mode: 'daemon', pid: 200, lastHeartbeat: Date.now(),
        endpoint: null, capabilities: [], meta: {}, startedAt: Date.now(),
      };
      if (!fs.existsSync(TEST_REGISTRY_DIR)) fs.mkdirSync(TEST_REGISTRY_DIR, { recursive: true });
      fs.writeFileSync(TEST_REGISTRY_FILE, JSON.stringify(registry));

      const summary = processRegistry.getPeerSummary();
      assert.equal(summary.total, 2);
      assert.ok(summary.byMode.mcp, 'MCP group should exist');
      assert.ok(summary.byMode.daemon, 'Daemon group should exist');
      assert.equal(summary.byMode.mcp.length, 1);
      assert.equal(summary.byMode.daemon.length, 1);
    });
  });

  // ──────────────────────────────────────────────────────────
  // DEPART
  // ──────────────────────────────────────────────────────────

  describe('depart()', () => {
    it('should remove entry from registry', () => {
      const id = processRegistry.announce({ mode: 'mcp' });
      assert.ok(processRegistry.isAvailable('mcp').available);

      processRegistry.depart();
      assert.equal(processRegistry.isAvailable('mcp').available, false);

      // File should still exist but entry removed
      if (fs.existsSync(TEST_REGISTRY_FILE)) {
        const data = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
        assert.equal(data[id], undefined);
      }
    });

    it('should be idempotent', () => {
      processRegistry.announce({ mode: 'mcp' });
      processRegistry.depart();
      processRegistry.depart(); // Should not throw
    });

    it('should noop if never announced', () => {
      processRegistry.depart(); // Should not throw
    });
  });

  // ──────────────────────────────────────────────────────────
  // HEARTBEAT
  // ──────────────────────────────────────────────────────────

  describe('heartbeat', () => {
    it('should update lastHeartbeat periodically', async () => {
      // Use short heartbeat for testing
      processRegistry.announce({
        mode: 'mcp',
        heartbeatInterval: 100, // 100ms
      });

      const data1 = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      const firstHeartbeat = Object.values(data1)[0].lastHeartbeat;

      // Wait for heartbeat
      await new Promise(resolve => setTimeout(resolve, 200));

      const data2 = JSON.parse(fs.readFileSync(TEST_REGISTRY_FILE, 'utf8'));
      const secondHeartbeat = Object.values(data2)[0].lastHeartbeat;

      assert.ok(secondHeartbeat > firstHeartbeat, 'Heartbeat should update');
    });
  });

  // ──────────────────────────────────────────────────────────
  // RESET
  // ──────────────────────────────────────────────────────────

  describe('_resetForTesting()', () => {
    it('should clean up registry file', () => {
      processRegistry.announce({ mode: 'mcp' });
      assert.ok(fs.existsSync(TEST_REGISTRY_FILE));

      processRegistry._resetForTesting();
      // After reset, path reverts to default — our test file should be gone
      assert.ok(!fs.existsSync(TEST_REGISTRY_FILE), 'Registry file should be deleted');
    });

    it('should stop heartbeat timer', () => {
      processRegistry.announce({
        mode: 'mcp',
        heartbeatInterval: 50,
      });

      processRegistry._resetForTesting();
      // If heartbeat timer leaked, process would hang
      // Test passing = timer cleaned up
    });
  });

  // ──────────────────────────────────────────────────────────
  // EDGE CASES
  // ──────────────────────────────────────────────────────────

  describe('edge cases', () => {
    it('should handle corrupt registry file gracefully', () => {
      if (!fs.existsSync(TEST_REGISTRY_DIR)) fs.mkdirSync(TEST_REGISTRY_DIR, { recursive: true });
      fs.writeFileSync(TEST_REGISTRY_FILE, 'NOT JSON{{{');

      // Should not throw
      const result = processRegistry.discover();
      assert.equal(result.processes.length, 0);
    });

    it('should handle missing registry directory gracefully', () => {
      // Use a path that definitely doesn't exist
      processRegistry._setRegistryPath(path.join(os.tmpdir(), '.cynic-nonexistent-' + Date.now(), 'reg.json'));

      // discover() should not throw even if dir doesn't exist
      const result = processRegistry.discover();
      assert.equal(result.processes.length, 0);
    });

    it('should create directory on first announce', () => {
      const uniqueDir = path.join(os.tmpdir(), `.cynic-test-create-${Date.now()}`);
      processRegistry._setRegistryPath(path.join(uniqueDir, 'registry.json'));

      processRegistry.announce({ mode: 'test' });
      assert.ok(fs.existsSync(uniqueDir), 'Directory should be created');

      // Cleanup
      try { fs.rmSync(uniqueDir, { recursive: true }); } catch { /* ok */ }
    });

    it('should expose registryPath for diagnostics', () => {
      assert.equal(processRegistry.registryPath, TEST_REGISTRY_FILE);
    });
  });
});
