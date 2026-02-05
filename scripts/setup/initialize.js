#!/usr/bin/env node

/**
 * CYNIC Post-Install Initialization
 *
 * Runs after `cowork install cynic` to verify and configure the environment.
 * Checks for required dependencies, optional API keys, and database connectivity.
 *
 * @module @cynic/scripts/setup/initialize
 */

'use strict';

import { existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, '..', '..');

const PHI_INV = 0.618033988749895;

// ═══════════════════════════════════════════════════════════════════════════
// DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

function banner() {
  console.log(`
═══════════════════════════════════════════════════════════
  CYNIC — Post-Install Setup
  "Loyal to truth, not to comfort"
═══════════════════════════════════════════════════════════
`);
}

function ok(msg) { console.log(`  ✅ ${msg}`); }
function warn(msg) { console.log(`  ⚠️  ${msg}`); }
function fail(msg) { console.log(`  ❌ ${msg}`); }
function info(msg) { console.log(`  ℹ️  ${msg}`); }

// ═══════════════════════════════════════════════════════════════════════════
// CHECKS
// ═══════════════════════════════════════════════════════════════════════════

function checkFiles() {
  console.log('\n── FILE STRUCTURE ──────────────────────────────────────');

  const required = [
    '.claude/plugin.json',
    '.claude/hooks/hooks.json',
    'packages/mcp/src/server.js',
    'packages/node/src/collective-singleton.js',
    'packages/llm/src/router.js',
    'packages/llm/src/perception-router.js',
    'packages/llm/src/adapters/gemini.js',
    'packages/mcp/src/tools/domains/browser.js',
    'scripts/hooks/awaken.js',
    'scripts/hooks/observe.js',
  ];

  let passed = 0;
  for (const file of required) {
    const full = join(ROOT, file);
    if (existsSync(full)) {
      ok(file);
      passed++;
    } else {
      fail(`${file} — MISSING`);
    }
  }

  return passed === required.length;
}

function checkEnv() {
  console.log('\n── ENVIRONMENT ────────────────────────────────────────');

  // Required: none (CYNIC degrades gracefully)
  info('No required env vars — CYNIC degrades gracefully');

  // Optional
  const optional = [
    { key: 'DATABASE_URL', desc: 'PostgreSQL (persistence, Q-Learning)' },
    { key: 'GEMINI_API_KEY', desc: 'Google Gemini (multi-model routing)' },
    { key: 'OLLAMA_ENDPOINT', desc: 'Ollama (local model validation)' },
    { key: 'HELIUS_API_KEY', desc: 'Helius (Solana RPC)' },
    { key: 'RENDER_API_KEY', desc: 'Render (deployment management)' },
    { key: 'GITHUB_TOKEN', desc: 'GitHub (ecosystem tracking)' },
  ];

  let configured = 0;
  for (const { key, desc } of optional) {
    if (process.env[key]) {
      ok(`${key} — ${desc}`);
      configured++;
    } else {
      warn(`${key} not set — ${desc} (optional)`);
    }
  }

  const ratio = configured / optional.length;
  console.log(`\n  Configured: ${configured}/${optional.length} (${(ratio * 100).toFixed(0)}%)`);

  return true; // All optional
}

async function checkDatabase() {
  console.log('\n── DATABASE ───────────────────────────────────────────');

  const dbUrl = process.env.DATABASE_URL;
  if (!dbUrl) {
    warn('DATABASE_URL not set — using in-memory fallback');
    info('Set DATABASE_URL for persistent Q-Learning and psychology');
    return true;
  }

  try {
    const { default: pg } = await import('pg');
    const pool = new pg.Pool({ connectionString: dbUrl, connectionTimeoutMillis: 5000 });
    const result = await pool.query('SELECT NOW() as time, current_database() as db');
    ok(`PostgreSQL connected: ${result.rows[0].db} at ${result.rows[0].time}`);

    // Check for CYNIC tables
    const tables = await pool.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'public' AND table_name LIKE '%cynic%' OR table_name LIKE '%qlearning%'
      ORDER BY table_name
    `);
    if (tables.rows.length > 0) {
      ok(`Found ${tables.rows.length} CYNIC tables`);
    } else {
      warn('No CYNIC tables found — they will be created on first use');
    }

    await pool.end();
    return true;
  } catch (e) {
    fail(`Database connection failed: ${e.message}`);
    info('CYNIC will use in-memory fallback');
    return true; // Non-fatal
  }
}

function checkCapabilities() {
  console.log('\n── CAPABILITIES ───────────────────────────────────────');

  const caps = [];

  // Perception layers
  caps.push({ name: 'Layer 1: APIs', active: !!(process.env.HELIUS_API_KEY || process.env.GEMINI_API_KEY) });
  caps.push({ name: 'Layer 2: MCP Tools', active: true }); // Always available
  caps.push({ name: 'Layer 3: Browser', active: true }); // Playwright MCP
  caps.push({ name: 'Layer 4: Filesystem', active: true }); // Always available

  // LLM Routing
  caps.push({ name: 'LLM: Claude (primary)', active: true });
  caps.push({ name: 'LLM: Gemini (validator)', active: !!process.env.GEMINI_API_KEY });
  caps.push({ name: 'LLM: Ollama (validator)', active: !!process.env.OLLAMA_ENDPOINT });

  // Persistence
  caps.push({ name: 'Q-Learning persistence', active: !!process.env.DATABASE_URL });
  caps.push({ name: 'Psychology tracking', active: true }); // In-memory always works

  let active = 0;
  for (const cap of caps) {
    if (cap.active) {
      ok(cap.name);
      active++;
    } else {
      warn(`${cap.name} — not configured`);
    }
  }

  const ratio = active / caps.length;
  console.log(`\n  Active: ${active}/${caps.length} (${(ratio * 100).toFixed(0)}%)`);
  if (ratio >= PHI_INV) {
    ok(`Operational capacity above φ⁻¹ (${(PHI_INV * 100).toFixed(1)}%)`);
  } else {
    warn(`Below φ⁻¹ threshold — consider configuring more capabilities`);
  }

  return true;
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  banner();

  const fileOk = checkFiles();
  const envOk = checkEnv();
  const dbOk = await checkDatabase();
  const capsOk = checkCapabilities();

  console.log('\n═══════════════════════════════════════════════════════════');

  if (fileOk && envOk && dbOk && capsOk) {
    console.log('  CYNIC is ready. φ guides all ratios.');
    console.log('  Run your Claude Code session to awaken the collective.');
  } else {
    console.log('  Setup complete with warnings. CYNIC degrades gracefully.');
  }

  console.log('═══════════════════════════════════════════════════════════\n');
}

main().catch(e => {
  console.error('Setup error:', e.message);
  process.exit(1);
});
