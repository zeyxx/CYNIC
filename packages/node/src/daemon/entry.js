#!/usr/bin/env node
/**
 * CYNIC Daemon Entry Point
 *
 * Spawned by `cynic daemon start` or auto-started by thin hooks.
 * Boots collective-singleton, starts DaemonServer, runs forever.
 *
 * "Le chien se lève" - CYNIC
 *
 * @module @cynic/node/daemon/entry
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import { bootDaemon } from '@cynic/core/boot';
import { DaemonServer } from './index.js';
import { processRegistry, createLogger } from '@cynic/core';
import { wireDaemonServices, wireLearningSystem, wireOrchestrator, wireWatchers, wireConsciousnessReflection, cleanupDaemonServices } from './service-wiring.js';
import { Watchdog, checkRestartSentinel } from './watchdog.js';

const log = createLogger('DaemonEntry');

const DAEMON_DIR = path.join(os.homedir(), '.cynic', 'daemon');
const PID_FILE = path.join(DAEMON_DIR, 'daemon.pid');
const LOG_FILE = path.join(DAEMON_DIR, 'daemon.log');

// Parse port from args or env
const args = process.argv.slice(2);
const portIdx = args.indexOf('--port');
const port = portIdx >= 0 ? parseInt(args[portIdx + 1], 10) : parseInt(process.env.CYNIC_DAEMON_PORT || '6180', 10);

/**
 * Append to log file (daemon has no console)
 */
function logToFile(level, message) {
  try {
    if (!fs.existsSync(DAEMON_DIR)) {
      fs.mkdirSync(DAEMON_DIR, { recursive: true });
    }
    const timestamp = new Date().toISOString();
    fs.appendFileSync(LOG_FILE, `[${timestamp}] [${level}] ${message}\n`);
  } catch { /* ignore */ }
}

/**
 * Main daemon boot sequence
 */
async function main() {
  logToFile('INFO', `Daemon starting (PID: ${process.pid}, port: ${port})`);

  // Write PID file
  try {
    if (!fs.existsSync(DAEMON_DIR)) {
      fs.mkdirSync(DAEMON_DIR, { recursive: true });
    }
    fs.writeFileSync(PID_FILE, process.pid.toString());
  } catch (err) {
    logToFile('ERROR', `Failed to write PID file: ${err.message}`);
  }

  // Check for crash recovery (stale restart sentinel)
  const recovery = checkRestartSentinel();
  if (recovery.recovered) {
    logToFile('INFO', `Crash recovery detected — previous PID: ${recovery.previousCrash?.pid}, reason: ${recovery.previousCrash?.reason}`);
  }

  let server;
  let watchdog;

  try {
    // Create and start daemon server
    server = new DaemonServer({ port, host: '127.0.0.1' });
    await server.start();
    logToFile('INFO', `Daemon server listening on 127.0.0.1:${port}`);

    // Boot CYNIC subsystems (exclude P2P, MCP, transport)
    try {
      const cynic = await bootDaemon({ silent: true });
      logToFile('INFO', `Boot completed: ${cynic.components?.length || 0} components`);
    } catch (err) {
      logToFile('WARN', `Boot partial: ${err.message}`);
      // Daemon still runs — it can serve hooks even without full boot
    }

    // Wire daemon-essential services (LLM, CostLedger — warm boot)
    let daemonServices = {};
    try {
      daemonServices = wireDaemonServices();
      server.services = daemonServices; // Expose for /health endpoint
      logToFile('INFO', 'Daemon services wired (ModelIntelligence + CostLedger warm)');
    } catch (err) {
      logToFile('WARN', `Service wiring partial: ${err.message}`);
    }

    // Wire learning system (collective-singleton, SONA, BehaviorModifier, MetaCognition)
    // Now that watchdog uses correct heap calculation, try to wire
    try {
      const learningServices = await wireLearningSystem();
      // Merge learning services into server.services
      server.services = { ...server.services, ...learningServices };
      logToFile('INFO', 'Learning system wired — organism breathing');
    } catch (err) {
      logToFile('WARN', `Learning system wiring failed — daemon still operational: ${err.message}`);
    }

    // Wire orchestrator (UnifiedOrchestrator + KabbalisticRouter + DogOrchestrator)
    // GAP-1: Enables event routing through Tree of Life → Dogs → Consensus
    try {
      await wireOrchestrator();
      logToFile('INFO', 'Orchestrator wired — event routing through KabbalisticRouter → Dogs → Consensus');
    } catch (err) {
      logToFile('WARN', `Orchestrator wiring failed — routing degraded: ${err.message}`);
    }

    // Wire watchers (FileWatcher + SolanaWatcher — perception layer)
    try {
      await wireWatchers();
      logToFile('INFO', 'Watchers wired — perception layer active (FilesystemWatcher + SolanaWatcher)');
    } catch (err) {
      logToFile('WARN', `Watcher wiring failed — perception degraded: ${err.message}`);
    }

    // Reset postgres circuit breaker after watcher init
    // Watcher initialization can cause event loop lag → query timeouts → CB trips
    // Now that watchers are scoped to packages/ + scripts/, reset the slate
    try {
      const { getPool } = await import('@cynic/persistence');
      const pool = getPool();
      if (pool) {
        pool.resetCircuitBreaker();
        logToFile('INFO', 'Postgres circuit breaker reset after watcher init');
      }
    } catch (err) {
      logToFile('WARN', `CB reset failed (non-critical): ${err.message}`);
    }

    // Wire consciousness reflection (R3: self-reflection loop)
    try {
      await wireConsciousnessReflection();
      logToFile('INFO', 'Consciousness reflection wired — φ observes φ (60 min cycles)');
    } catch (err) {
      logToFile('WARN', `Consciousness reflection failed — meta-cognition degraded: ${err.message}`);
    }

    // Start watchdog (self-monitoring)
    try {
      watchdog = new Watchdog();
      server.watchdog = watchdog; // Expose for /health endpoint
      watchdog.start();
      logToFile('INFO', 'Watchdog started (30s health checks)');
    } catch (err) {
      logToFile('WARN', `Watchdog failed to start: ${err.message}`);
    }

    logToFile('INFO', 'Daemon fully operational');
  } catch (err) {
    logToFile('ERROR', `Daemon failed to start: ${err.message}`);
    cleanup();
    process.exit(1);
  }

  // Graceful shutdown handlers
  async function cleanup() {
    logToFile('INFO', 'Daemon shutting down...');
    try { if (watchdog) watchdog.stop(); } catch { /* ignore */ }
    try { await cleanupDaemonServices(); } catch { /* ignore */ }
    try { fs.unlinkSync(PID_FILE); } catch { /* ignore */ }
    try { processRegistry.depart(); } catch { /* ignore */ }
    if (server) {
      await server.stop().catch(() => {});
    }
  }

  process.on('SIGTERM', () => { cleanup(); process.exit(0); });
  process.on('SIGINT', () => { cleanup(); process.exit(0); });
  process.on('uncaughtException', (err) => {
    logToFile('ERROR', `Uncaught exception: ${err.message}\n${err.stack}`);
    cleanup();
    process.exit(1);
  });
  process.on('unhandledRejection', (err) => {
    logToFile('WARN', `Unhandled rejection: ${err?.message || err}`);
  });
}

main().catch((err) => {
  logToFile('ERROR', `Fatal: ${err.message}`);
  process.exit(1);
});
