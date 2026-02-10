/**
 * @cynic/core - System Topology Tests
 *
 * Tests CYNIC's self-awareness:
 * - Mode detection (hook, mcp, daemon, test)
 * - Service registry
 * - Component registry
 * - Capability matrix
 * - 7×7 matrix introspection
 *
 * "γνῶθι σεαυτόν" - but with φ⁻¹ doubt
 *
 * @module @cynic/core/test/topology
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import path from 'path';
import os from 'os';
import fs from 'fs';

import {
  systemTopology,
  ExecutionMode,
  ServiceStatus,
  RealityDimension,
  AnalysisDimension,
} from '../src/topology/system-topology.js';

import { processRegistry } from '../src/topology/process-registry.js';
import { PHI_INV } from '../src/axioms/constants.js';

// Isolated registry path for test
const TEST_REGISTRY_DIR = path.join(os.tmpdir(), '.cynic-test-topo', `${process.pid}`);
const TEST_REGISTRY_FILE = path.join(TEST_REGISTRY_DIR, 'registry.json');

describe('SystemTopology', () => {
  beforeEach(() => {
    systemTopology._resetForTesting();
    processRegistry._setRegistryPath(TEST_REGISTRY_FILE);
  });

  afterEach(() => {
    processRegistry._resetForTesting();
    try { fs.rmSync(TEST_REGISTRY_DIR, { recursive: true }); } catch { /* ok */ }
  });

  describe('Mode Detection', () => {
    it('detects test mode from NODE_ENV', () => {
      // NODE_ENV=test is set during testing
      const mode = systemTopology.detectMode();
      assert.equal(mode, ExecutionMode.TEST);
    });

    it('caches detected mode', () => {
      const mode1 = systemTopology.detectMode();
      const mode2 = systemTopology.detectMode();
      assert.equal(mode1, mode2);
    });

    it('allows explicit mode override', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      assert.equal(systemTopology.mode, ExecutionMode.MCP);
    });

    it('rejects invalid mode', () => {
      assert.throws(
        () => systemTopology.setMode('invalid'),
        /Invalid mode/
      );
    });

    it('exposes mode via getter', () => {
      assert.equal(systemTopology.mode, ExecutionMode.TEST);
    });
  });

  describe('Service Registry', () => {
    it('registers and retrieves services', () => {
      systemTopology.registerService('postgres', {
        status: ServiceStatus.HEALTHY,
        location: 'localhost:5432',
      });

      const svc = systemTopology.getService('postgres');
      assert.equal(svc.status, ServiceStatus.HEALTHY);
      assert.equal(svc.location, 'localhost:5432');
      assert.ok(svc.registeredAt > 0);
    });

    it('updates service status', () => {
      systemTopology.registerService('redis', {
        status: ServiceStatus.HEALTHY,
      });
      systemTopology.updateService('redis', {
        status: ServiceStatus.DEGRADED,
      });

      const svc = systemTopology.getService('redis');
      assert.equal(svc.status, ServiceStatus.DEGRADED);
    });

    it('returns null for unregistered service', () => {
      assert.equal(systemTopology.getService('nonexistent'), null);
    });

    it('lists all services', () => {
      systemTopology.registerService('postgres', { status: ServiceStatus.HEALTHY });
      systemTopology.registerService('redis', { status: ServiceStatus.DEAD });

      const services = systemTopology.getServices();
      assert.ok('postgres' in services);
      assert.ok('redis' in services);
    });
  });

  describe('Component Registry', () => {
    it('registers and checks components', () => {
      const mockJudge = { name: 'judge' };
      systemTopology.registerComponent('judge', mockJudge);

      assert.ok(systemTopology.hasComponent('judge'));
      assert.equal(systemTopology.getComponent('judge'), mockJudge);
    });

    it('lists component names', () => {
      systemTopology.registerComponent('judge', {});
      systemTopology.registerComponent('router', {});

      const names = systemTopology.getComponentNames();
      assert.ok(names.includes('judge'));
      assert.ok(names.includes('router'));
    });

    it('returns null for unregistered component', () => {
      assert.equal(systemTopology.getComponent('nonexistent'), null);
    });
  });

  describe('Capabilities', () => {
    it('returns capabilities for test mode', () => {
      const caps = systemTopology.getCapabilities();
      assert.equal(caps.judge, true);
      assert.equal(caps.persistence, false);
      assert.equal(caps.guardian, false);
    });

    it('enables persistence when postgres is healthy', () => {
      systemTopology.registerService('postgres', {
        status: ServiceStatus.HEALTHY,
      });
      assert.equal(systemTopology.can('persistence'), true);
    });

    it('enables consensus when collectivePack registered', () => {
      systemTopology.registerComponent('collectivePack', {});
      assert.equal(systemTopology.can('consensus'), true);
    });

    it('can() returns false for unknown capability', () => {
      assert.equal(systemTopology.can('teleportation'), false);
    });
  });

  describe('7×7 Matrix Introspection', () => {
    it('has 49 cells', () => {
      const matrix = systemTopology.getMatrixState();
      assert.equal(Object.keys(matrix).length, 49);
    });

    it('cells have correct structure', () => {
      const matrix = systemTopology.getMatrixState();
      const cell = matrix['CODE.JUDGE'];

      assert.equal(typeof cell.active, 'boolean');
      assert.equal(typeof cell.coverage, 'number');
      assert.ok(Array.isArray(cell.components));
      assert.ok(Array.isArray(cell.missing));
    });

    it('reports active cells when components registered', () => {
      systemTopology.registerComponent('judge', {});
      systemTopology.registerComponent('dimensions', {});

      const matrix = systemTopology.getMatrixState();
      assert.equal(matrix['CODE.JUDGE'].active, true);
      assert.equal(matrix['CODE.JUDGE'].coverage, 1); // Both requirements met
    });

    it('reports missing components', () => {
      const matrix = systemTopology.getMatrixState();
      // With no components, CODE.JUDGE should list judge and dimensions as missing
      assert.ok(matrix['CODE.JUDGE'].missing.includes('judge'));
    });

    it('completion never exceeds φ⁻¹ confidence', () => {
      const completion = systemTopology.getMatrixCompletion();
      assert.ok(completion.confidence <= PHI_INV);
    });
  });

  describe('Snapshot', () => {
    it('returns complete topology state', () => {
      systemTopology.registerComponent('judge', {});
      systemTopology.registerService('postgres', { status: ServiceStatus.HEALTHY });

      const snap = systemTopology.snapshot();
      assert.equal(snap.mode, ExecutionMode.TEST);
      assert.ok(snap.uptime >= 0);
      assert.ok('postgres' in snap.services);
      assert.ok(snap.components.includes('judge'));
      assert.ok(typeof snap.capabilities === 'object');
      assert.ok(typeof snap.matrix === 'object');
      assert.ok(snap.phi.selfAwareness <= PHI_INV);
    });
  });

  describe('Process Boundaries', () => {
    it('documents current process', () => {
      const boundaries = systemTopology.getProcessBoundaries();
      assert.equal(boundaries.current.mode, ExecutionMode.TEST);
      assert.equal(boundaries.current.pid, process.pid);
    });

    it('documents all three boundary types', () => {
      const boundaries = systemTopology.getProcessBoundaries();
      assert.ok('hooks' in boundaries.boundaries);
      assert.ok('mcp' in boundaries.boundaries);
      assert.ok('daemon' in boundaries.boundaries);
    });
  });

  describe('Event Listeners', () => {
    it('notifies on service registration', () => {
      const events = [];
      systemTopology.on((event, data) => events.push({ event, data }));

      systemTopology.registerService('test', { status: ServiceStatus.HEALTHY });
      assert.equal(events.length, 1);
      assert.equal(events[0].event, 'service:registered');
    });

    it('notifies on component registration', () => {
      const events = [];
      systemTopology.on((event, data) => events.push({ event, data }));

      systemTopology.registerComponent('test', {});
      assert.equal(events.length, 1);
      assert.equal(events[0].event, 'component:registered');
    });

    it('supports unsubscribe', () => {
      const events = [];
      const unsub = systemTopology.on((event, data) => events.push({ event, data }));

      systemTopology.registerComponent('a', {});
      unsub();
      systemTopology.registerComponent('b', {});

      assert.equal(events.length, 1); // Only first event
    });
  });

  describe('Constants', () => {
    it('exports all reality dimensions', () => {
      assert.equal(Object.keys(RealityDimension).length, 7);
      assert.equal(RealityDimension.CODE, 'CODE');
      assert.equal(RealityDimension.COSMOS, 'COSMOS');
    });

    it('exports all analysis dimensions', () => {
      assert.equal(Object.keys(AnalysisDimension).length, 7);
      assert.equal(AnalysisDimension.PERCEIVE, 'PERCEIVE');
      assert.equal(AnalysisDimension.EMERGE, 'EMERGE');
    });
  });

  // ────────────────────────────────────────────────────────
  // Gap #6: Cross-Process Discovery
  // ────────────────────────────────────────────────────────

  describe('Cross-Process Discovery (Gap #6)', () => {
    it('announceSelf() registers in ProcessRegistry', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      const id = systemTopology.announceSelf({ endpoint: 'http://localhost:3000' });

      assert.ok(id, 'Should return process ID');
      assert.ok(id.startsWith('mcp-'));
    });

    it('discoverPeers() returns announced processes', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      systemTopology.announceSelf();

      const peers = systemTopology.discoverPeers();
      assert.ok(peers.processes.length >= 1, 'Should find at least self');
    });

    it('isPeerAvailable() checks specific mode', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      systemTopology.announceSelf();

      const mcpCheck = systemTopology.isPeerAvailable('mcp');
      assert.equal(mcpCheck.available, true);
      assert.equal(mcpCheck.pid, process.pid);

      const daemonCheck = systemTopology.isPeerAvailable('daemon');
      assert.equal(daemonCheck.available, false);
    });

    it('getPeerSummary() groups by mode', () => {
      systemTopology.setMode(ExecutionMode.DAEMON);
      systemTopology.announceSelf();

      const summary = systemTopology.getPeerSummary();
      assert.ok(summary.total >= 1);
      assert.ok(summary.byMode.daemon);
    });

    it('departSelf() removes from registry', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      systemTopology.announceSelf();
      assert.equal(systemTopology.isPeerAvailable('mcp').available, true);

      systemTopology.departSelf();
      assert.equal(systemTopology.isPeerAvailable('mcp').available, false);
    });

    it('snapshot() includes peers', () => {
      systemTopology.setMode(ExecutionMode.MCP);
      systemTopology.announceSelf();

      const snap = systemTopology.snapshot();
      assert.ok(snap.peers, 'Snapshot should include peers');
      assert.ok(snap.peers.total >= 1);
    });
  });

  describe('Reset', () => {
    it('clears all state including ProcessRegistry', () => {
      systemTopology.registerService('test', { status: ServiceStatus.HEALTHY });
      systemTopology.registerComponent('test', {});
      systemTopology.setMode(ExecutionMode.MCP);
      systemTopology.announceSelf();

      systemTopology._resetForTesting();

      assert.equal(systemTopology.getService('test'), null);
      assert.equal(systemTopology.hasComponent('test'), false);
      assert.equal(systemTopology.getComponentNames().length, 0);
      assert.equal(systemTopology.isPeerAvailable('mcp').available, false);
    });
  });
});
