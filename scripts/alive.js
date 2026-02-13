#!/usr/bin/env node
/**
 * CYNIC Alive Check
 *
 * 10-second answer: Is the organism breathing?
 * Run: npm run alive
 *
 * "Le chien respire ou ne respire pas. Pas de zone grise." - CYNIC
 *
 * @module scripts/alive
 */

'use strict';

import { getPool } from '@cynic/persistence';
import fs from 'fs';
import path from 'path';
import os from 'os';

const PHI_INV = 0.618;
const DAEMON_PID_FILE = path.join(os.homedir(), '.cynic', 'daemon', 'daemon.pid');

// Colors (no chalk dependency - use ANSI directly)
const R = '\x1b[31m';
const G = '\x1b[32m';
const Y = '\x1b[33m';
const C = '\x1b[36m';
const B = '\x1b[1m';
const DIM = '\x1b[2m';
const RST = '\x1b[0m';

const checks = [];
let pool;

function ok(name, detail) {
  checks.push({ name, pass: true, detail });
  console.log(`  ${G}✓${RST} ${name}${detail ? `  ${DIM}${detail}${RST}` : ''}`);
}

function fail(name, detail, fix) {
  checks.push({ name, pass: false, detail, fix });
  console.log(`  ${R}✗${RST} ${name}${detail ? `  ${DIM}${detail}${RST}` : ''}`);
  if (fix) console.log(`    ${Y}→ ${fix}${RST}`);
}

async function checkDatabase() {
  try {
    pool = getPool();
    const start = Date.now();
    await pool.query('SELECT 1');
    const ms = Date.now() - start;
    ok('Database', `${ms}ms`);
  } catch (e) {
    fail('Database', e.message, 'Check PostgreSQL is running');
    return false;
  }

  // Migrations
  try {
    const { rows } = await pool.query('SELECT COUNT(*) as c FROM _migrations');
    const count = parseInt(rows[0].c);
    if (count >= 40) ok('Migrations', `${count} applied`);
    else fail('Migrations', `${count}/40`, 'npm run migrate -w @cynic/persistence');
  } catch (e) {
    fail('Migrations', e.message);
  }

  return true;
}

function checkDaemon() {
  try {
    if (!fs.existsSync(DAEMON_PID_FILE)) {
      fail('Daemon', 'PID file not found', 'node packages/node/bin/cynic.js daemon start');
      return false;
    }
    const pid = parseInt(fs.readFileSync(DAEMON_PID_FILE, 'utf8').trim());

    // Check if process is actually running
    try {
      process.kill(pid, 0); // signal 0 = check existence
      ok('Daemon', `PID ${pid} running`);
      return true;
    } catch {
      fail('Daemon', `PID ${pid} stale (not running)`, 'node packages/node/bin/cynic.js daemon start');
      return false;
    }
  } catch (e) {
    fail('Daemon', e.message, 'node packages/node/bin/cynic.js daemon start');
    return false;
  }
}

async function checkWatchers() {
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(DISTINCT watcher_name) as active
      FROM watcher_heartbeats
      WHERE timestamp > NOW() - INTERVAL '5 minutes'
    `);
    const active = parseInt(rows[0].active);
    if (active >= 3) ok('Watchers', `${active}/3 active`);
    else fail('Watchers', `${active}/3 active`, 'Daemon must be running with watchers');
  } catch (e) {
    fail('Watchers', e.message);
  }
}

async function checkLearning() {
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(*) as c FROM learning_events
      WHERE timestamp > NOW() - INTERVAL '24 hours'
    `);
    const count = parseInt(rows[0].c);
    if (count >= 5) ok('Learning', `${count} events/24h`);
    else fail('Learning', `${count} events/24h (need ≥5)`, 'Daemon generates learning events');
  } catch (e) {
    fail('Learning', e.message);
  }
}

async function checkQLearning() {
  try {
    const { rows } = await pool.query(`
      SELECT COUNT(*) as c FROM qlearning_episodes
      WHERE created_at > NOW() - INTERVAL '24 hours'
    `);
    const count = parseInt(rows[0].c);
    if (count >= 100) ok('Q-Episodes', `${count}/day`);
    else fail('Q-Episodes', `${count}/day (need ≥100)`, 'Q-Learning actively exploring');
  } catch (e) {
    fail('Q-Episodes', e.message);
  }
}

async function checkLoops() {
  try {
    const { rows } = await pool.query(`
      SELECT DISTINCT loop_name FROM learning_events
      WHERE timestamp > NOW() - INTERVAL '24 hours'
    `);
    const active = rows.length;
    const loopNames = rows.map(r => r.loop_name).join(', ');
    if (active >= 8) ok('Learning Loops', `${active}/11 active (${loopNames})`);
    else fail('Learning Loops', `${active}/11 active (need ≥8)`, 'Some loops may be dormant');
  } catch (e) {
    fail('Learning Loops', e.message);
  }
}

async function checkBudget() {
  try {
    const { rows } = await pool.query('SELECT budget_level, budget_remaining_usd, circuit_breaker_active FROM budget_state WHERE id = 1');
    if (rows.length > 0) {
      const { budget_level, budget_remaining_usd, circuit_breaker_active } = rows[0];
      if (circuit_breaker_active) {
        fail('Budget', `CIRCUIT BREAKER ACTIVE ($${budget_remaining_usd} remaining)`);
      } else {
        ok('Budget', `${budget_level} ($${budget_remaining_usd} remaining)`);
      }
    } else {
      fail('Budget', 'No budget state', 'Run migration 046_cost_ledger');
    }
  } catch (e) {
    fail('Budget', e.message);
  }
}

// Main
async function main() {
  console.log(`\n${B}${C}  CYNIC ALIVE CHECK${RST}  ${DIM}${new Date().toISOString()}${RST}\n`);

  // Core checks
  const dbOk = await checkDatabase();
  const daemonOk = checkDaemon();

  if (dbOk) {
    await checkWatchers();
    await checkLearning();
    await checkQLearning();
    await checkLoops();
    await checkBudget();
  }

  // Verdict
  const passed = checks.filter(c => c.pass).length;
  const total = checks.length;
  const ratio = total > 0 ? passed / total : 0;
  const breathing = dbOk && daemonOk && ratio >= PHI_INV;

  console.log(`\n${'─'.repeat(50)}`);

  if (breathing) {
    console.log(`\n  ${G}${B}CYNIC ALIVE?  YES ✓${RST}`);
    console.log(`  ${DIM}${passed}/${total} checks pass (${(ratio * 100).toFixed(0)}%)${RST}`);
    console.log(`  ${DIM}The organism is breathing.${RST}`);
  } else {
    console.log(`\n  ${R}${B}CYNIC ALIVE?  NO ✗${RST}`);
    console.log(`  ${DIM}${passed}/${total} checks pass (${(ratio * 100).toFixed(0)}%)${RST}`);

    // Show the first blocker
    const firstFail = checks.find(c => !c.pass);
    if (firstFail?.fix) {
      console.log(`\n  ${Y}${B}NEXT:${RST} ${firstFail.fix}`);
    }
  }

  console.log('');

  // Write state for other tools to consume
  const state = {
    alive: breathing,
    timestamp: new Date().toISOString(),
    checks: checks.map(c => ({ name: c.name, pass: c.pass, detail: c.detail })),
    passed,
    total,
    ratio,
  };

  try {
    const stateDir = path.join(os.homedir(), '.cynic');
    if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });
    fs.writeFileSync(path.join(stateDir, 'alive.json'), JSON.stringify(state, null, 2));
  } catch { /* non-critical */ }

  process.exit(breathing ? 0 : 1);
}

main().catch(err => {
  console.error(`\n  ${R}FATAL: ${err.message}${RST}\n`);
  process.exit(1);
});
