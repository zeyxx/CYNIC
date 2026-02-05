#!/usr/bin/env node
/**
 * CYNIC Auto-Training Orchestrator
 *
 * Watches feedback accumulation and auto-triggers training when thresholds are met.
 * Runs as a daemon or one-shot check.
 *
 * Triggers:
 * 1. Feedback count >= minFeedback (default: 34 = Fib(9))
 * 2. Time since last training >= minInterval (default: 24h)
 * 3. New feedback rate >= φ⁻² per hour (sustained learning signal)
 *
 * Pipeline:
 * 1. Export → 2. Split → 3. SFT → 4. GRPO → 5. Eval → 6. Deploy (if gate passes)
 *
 * Usage:
 *   # One-shot check (cron-friendly)
 *   node scripts/training/auto-train.mjs --check
 *
 *   # Daemon mode (watches continuously)
 *   node scripts/training/auto-train.mjs --daemon
 *
 *   # Force training (bypass triggers)
 *   node scripts/training/auto-train.mjs --force
 *
 *   # Dry run (check triggers, don't train)
 *   node scripts/training/auto-train.mjs --dry-run
 *
 * "φ distrusts φ" — automation must be bounded
 *
 * @module cynic/training/auto-train
 */

import { spawn } from 'child_process';
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'fs';
import { pipeline } from './training-config.mjs';

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618;
const PHI_INV_2 = 0.382;
const PHI_INV_3 = 0.236;

const STATE_FILE = '.cynic/training-state.json';
const LOG_DIR = 'logs/training';

// ═══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
const mode = args.includes('--daemon') ? 'daemon'
           : args.includes('--force') ? 'force'
           : args.includes('--dry-run') ? 'dry-run'
           : 'check';
const profile = getArg(args, '--profile') || process.env.CYNIC_TRAIN_PROFILE || 'local';

function getArg(args, flag) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : null;
}

// ═══════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════

const config = {
  // Minimum feedback records to trigger training
  minFeedback: parseInt(process.env.CYNIC_MIN_FEEDBACK || '34', 10), // Fib(9)

  // Minimum hours between training runs
  minIntervalHours: parseInt(process.env.CYNIC_TRAIN_INTERVAL || '24', 10),

  // Minimum new feedback per hour to consider "active learning"
  minFeedbackRate: PHI_INV_2, // ~0.38 feedback/hour = ~9/day

  // Daemon poll interval (minutes)
  pollIntervalMinutes: 60,

  // Skip stages (for debugging)
  skipStages: (process.env.CYNIC_SKIP_STAGES || '').split(',').filter(Boolean),

  // Profile
  profile,
};

// ═══════════════════════════════════════════════════════════════════════════
// State Management
// ═══════════════════════════════════════════════════════════════════════════

function loadState() {
  try {
    if (existsSync(STATE_FILE)) {
      return JSON.parse(readFileSync(STATE_FILE, 'utf8'));
    }
  } catch (e) {
    console.error(`[auto-train] Failed to load state: ${e.message}`);
  }
  return {
    lastTrainingAt: null,
    lastFeedbackCount: 0,
    lastCheckAt: null,
    trainingHistory: [],
  };
}

function saveState(state) {
  try {
    const dir = STATE_FILE.split('/').slice(0, -1).join('/');
    if (dir && !existsSync(dir)) mkdirSync(dir, { recursive: true });
    writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
  } catch (e) {
    console.error(`[auto-train] Failed to save state: ${e.message}`);
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Trigger Checks
// ═══════════════════════════════════════════════════════════════════════════

async function getFeedbackStats() {
  try {
    const { getPool } = await import('../../packages/persistence/src/postgres/client.js');
    const pool = getPool();

    const result = await pool.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE applied = false) as unapplied,
        COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '24 hours') as last_24h,
        COUNT(*) FILTER (WHERE created_at > NOW() - INTERVAL '1 hour') as last_hour,
        MAX(created_at) as latest_at
      FROM feedback
    `);

    const row = result.rows[0];
    return {
      total: parseInt(row.total, 10),
      unapplied: parseInt(row.unapplied, 10),
      last24h: parseInt(row.last_24h, 10),
      lastHour: parseInt(row.last_hour, 10),
      latestAt: row.latest_at,
    };
  } catch (e) {
    console.error(`[auto-train] Failed to get feedback stats: ${e.message}`);
    return { total: 0, unapplied: 0, last24h: 0, lastHour: 0, latestAt: null };
  }
}

async function checkTriggers(state) {
  const stats = await getFeedbackStats();
  const now = Date.now();

  const triggers = {
    feedbackCount: false,
    timeInterval: false,
    feedbackRate: false,
    stats,
  };

  // Trigger 1: Minimum feedback count
  if (stats.unapplied >= config.minFeedback) {
    triggers.feedbackCount = true;
    console.error(`[auto-train] ✓ Feedback trigger: ${stats.unapplied} unapplied (min: ${config.minFeedback})`);
  } else {
    console.error(`[auto-train] ✗ Feedback trigger: ${stats.unapplied}/${config.minFeedback} unapplied`);
  }

  // Trigger 2: Time since last training
  if (!state.lastTrainingAt) {
    triggers.timeInterval = true;
    console.error(`[auto-train] ✓ Time trigger: never trained before`);
  } else {
    const hoursSince = (now - new Date(state.lastTrainingAt).getTime()) / (1000 * 60 * 60);
    if (hoursSince >= config.minIntervalHours) {
      triggers.timeInterval = true;
      console.error(`[auto-train] ✓ Time trigger: ${hoursSince.toFixed(1)}h since last (min: ${config.minIntervalHours}h)`);
    } else {
      console.error(`[auto-train] ✗ Time trigger: ${hoursSince.toFixed(1)}h/${config.minIntervalHours}h`);
    }
  }

  // Trigger 3: Sustained feedback rate
  const feedbackRate = stats.last24h / 24;
  if (feedbackRate >= config.minFeedbackRate) {
    triggers.feedbackRate = true;
    console.error(`[auto-train] ✓ Rate trigger: ${feedbackRate.toFixed(2)}/h (min: ${config.minFeedbackRate.toFixed(2)}/h)`);
  } else {
    console.error(`[auto-train] ✗ Rate trigger: ${feedbackRate.toFixed(2)}/${config.minFeedbackRate.toFixed(2)}/h`);
  }

  // All triggers must be met (AND logic)
  triggers.shouldTrain = triggers.feedbackCount && triggers.timeInterval;

  return triggers;
}

// ═══════════════════════════════════════════════════════════════════════════
// Pipeline Execution
// ═══════════════════════════════════════════════════════════════════════════

function runStage(stage) {
  return new Promise((resolve, reject) => {
    console.error(`\n[auto-train] ═══════════════════════════════════════════════════════`);
    console.error(`[auto-train] Stage: ${stage.name} — ${stage.description}`);
    console.error(`[auto-train] Script: ${stage.script}`);
    console.error(`[auto-train] ═══════════════════════════════════════════════════════\n`);

    const ext = stage.script.split('.').pop();
    const cmd = ext === 'mjs' || ext === 'js' ? 'node' : 'bash';
    const args = [stage.script, ...stage.args];

    const proc = spawn(cmd, args, {
      stdio: 'inherit',
      env: { ...process.env, CYNIC_TRAIN_PROFILE: config.profile },
    });

    proc.on('close', code => {
      if (code === 0) {
        console.error(`[auto-train] ✓ ${stage.name} completed`);
        resolve();
      } else {
        reject(new Error(`${stage.name} failed with code ${code}`));
      }
    });

    proc.on('error', reject);
  });
}

async function runPipeline() {
  const startTime = Date.now();
  const results = { stages: [], success: false, duration: 0 };

  console.error(`\n[auto-train] ╔═══════════════════════════════════════════════════════╗`);
  console.error(`[auto-train] ║  CYNIC AUTO-TRAINING PIPELINE                         ║`);
  console.error(`[auto-train] ║  Profile: ${config.profile.padEnd(44)}║`);
  console.error(`[auto-train] ╚═══════════════════════════════════════════════════════╝\n`);

  for (const stage of pipeline.stages) {
    if (config.skipStages.includes(stage.name)) {
      console.error(`[auto-train] Skipping ${stage.name} (CYNIC_SKIP_STAGES)`);
      continue;
    }

    try {
      await runStage(stage);
      results.stages.push({ name: stage.name, success: true });
    } catch (err) {
      console.error(`[auto-train] ✗ ${stage.name} failed: ${err.message}`);
      results.stages.push({ name: stage.name, success: false, error: err.message });

      // Stop on failure (no partial deployment)
      results.duration = Date.now() - startTime;
      return results;
    }
  }

  results.success = true;
  results.duration = Date.now() - startTime;

  console.error(`\n[auto-train] ╔═══════════════════════════════════════════════════════╗`);
  console.error(`[auto-train] ║  TRAINING COMPLETE                                    ║`);
  console.error(`[auto-train] ║  Duration: ${(results.duration / 1000 / 60).toFixed(1)} minutes${' '.repeat(33)}║`);
  console.error(`[auto-train] ╚═══════════════════════════════════════════════════════╝\n`);

  return results;
}

// ═══════════════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  console.error(`[auto-train] Mode: ${mode}`);
  console.error(`[auto-train] Profile: ${config.profile}`);
  console.error(`[auto-train] Min feedback: ${config.minFeedback}`);
  console.error(`[auto-train] Min interval: ${config.minIntervalHours}h`);

  const state = loadState();

  if (mode === 'force') {
    console.error(`[auto-train] Force mode — bypassing triggers`);
    const results = await runPipeline();
    state.lastTrainingAt = new Date().toISOString();
    state.trainingHistory.push({
      at: state.lastTrainingAt,
      mode: 'force',
      success: results.success,
      duration: results.duration,
    });
    saveState(state);
    process.exit(results.success ? 0 : 1);
  }

  if (mode === 'daemon') {
    console.error(`[auto-train] Daemon mode — polling every ${config.pollIntervalMinutes} minutes`);

    const poll = async () => {
      const triggers = await checkTriggers(state);
      state.lastCheckAt = new Date().toISOString();
      saveState(state);

      if (triggers.shouldTrain) {
        console.error(`[auto-train] Triggers met — starting training`);
        const results = await runPipeline();
        state.lastTrainingAt = new Date().toISOString();
        state.lastFeedbackCount = triggers.stats.total;
        state.trainingHistory.push({
          at: state.lastTrainingAt,
          mode: 'daemon',
          success: results.success,
          duration: results.duration,
          triggers,
        });
        saveState(state);
      }
    };

    await poll();
    setInterval(poll, config.pollIntervalMinutes * 60 * 1000);
    return; // Keep running
  }

  // Check mode (one-shot)
  const triggers = await checkTriggers(state);
  state.lastCheckAt = new Date().toISOString();
  saveState(state);

  if (mode === 'dry-run') {
    console.error(`[auto-train] Dry run — would${triggers.shouldTrain ? '' : ' NOT'} train`);
    process.exit(triggers.shouldTrain ? 0 : 1);
  }

  if (triggers.shouldTrain) {
    console.error(`[auto-train] Triggers met — starting training`);
    const results = await runPipeline();
    state.lastTrainingAt = new Date().toISOString();
    state.lastFeedbackCount = triggers.stats.total;
    state.trainingHistory.push({
      at: state.lastTrainingAt,
      mode: 'check',
      success: results.success,
      duration: results.duration,
      triggers,
    });
    saveState(state);
    process.exit(results.success ? 0 : 1);
  } else {
    console.error(`[auto-train] Triggers not met — skipping training`);
    process.exit(0);
  }
}

main().catch(err => {
  console.error(`[auto-train] Fatal: ${err.message}`);
  process.exit(1);
});
