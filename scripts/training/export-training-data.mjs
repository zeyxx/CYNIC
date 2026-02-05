#!/usr/bin/env node
/**
 * Export CYNIC Training Data
 *
 * Joins judgments + feedback + reasoning_trajectories into JSONL format
 * suitable for SFT warm-up and GRPO fine-tuning.
 *
 * Output format (one JSON object per line):
 * {
 *   "id": "jdg_xxx",
 *   "input": { item_type, item_hash, context },
 *   "judgment": { q_score, verdict, axiom_scores, dimension_scores, reasoning_path },
 *   "feedback": { outcome, actual_score, reason } | null,
 *   "trajectory": { step_count, pivot_points, coherence_score } | null,
 *   "reward": number (computed by reward function)
 * }
 *
 * Usage:
 *   node scripts/training/export-training-data.mjs [--output path] [--min-feedback] [--since YYYY-MM-DD]
 *
 * "φ distrusts φ" — training data must be honest
 *
 * @module cynic/training/export
 */

import { getPool } from '../../packages/persistence/src/postgres/client.js';
import { computeReward } from './reward-function.mjs';
import { createWriteStream } from 'fs';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';

// ═══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
const outputPath = getArg(args, '--output') || `training-data-${new Date().toISOString().slice(0, 10)}.jsonl`;
const minFeedback = getArg(args, '--min-feedback') === 'true';
const since = getArg(args, '--since') || '2020-01-01';

function getArg(args, flag) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : null;
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Export
// ═══════════════════════════════════════════════════════════════════════════

async function exportTrainingData() {
  const pool = getPool();

  console.error(`[export] Connecting to PostgreSQL...`);
  console.error(`[export] Output: ${outputPath}`);
  console.error(`[export] Since: ${since}`);
  console.error(`[export] Min feedback required: ${minFeedback}`);

  // Query: LEFT JOIN judgments with feedback and trajectories
  const query = `
    SELECT
      j.judgment_id,
      j.item_type,
      j.item_hash,
      j.item_content,
      j.q_score,
      j.global_score,
      j.confidence,
      j.verdict,
      j.axiom_scores,
      j.dimension_scores,
      j.reasoning_path,
      j.context,
      j.created_at,
      -- Feedback (latest per judgment)
      f.outcome AS feedback_outcome,
      f.actual_score AS feedback_actual_score,
      f.reason AS feedback_reason,
      f.created_at AS feedback_at,
      -- Trajectory
      t.step_count AS traj_step_count,
      t.total_duration_ms AS traj_duration_ms,
      t.pivot_points AS traj_pivot_points,
      t.coherence_score AS traj_coherence,
      t.efficiency_score AS traj_efficiency,
      t.patterns_used AS traj_patterns,
      t.warnings_issued AS traj_warnings,
      t.outcome_feedback AS traj_outcome
    FROM judgments j
    LEFT JOIN LATERAL (
      SELECT * FROM feedback fb
      WHERE fb.judgment_id = j.judgment_id
      ORDER BY fb.created_at DESC
      LIMIT 1
    ) f ON true
    LEFT JOIN reasoning_trajectories t ON t.judgment_id = j.judgment_id
    WHERE j.created_at >= $1
    ${minFeedback ? 'AND f.outcome IS NOT NULL' : ''}
    ORDER BY j.created_at ASC
  `;

  const result = await pool.query(query, [since]);
  const rows = result.rows;

  console.error(`[export] Found ${rows.length} judgments`);

  let withFeedback = 0;
  let withTrajectory = 0;
  let withSkepticism = 0;
  let withDimensions = 0;

  const output = createWriteStream(outputPath);

  for (const row of rows) {
    const feedback = row.feedback_outcome ? {
      outcome: row.feedback_outcome,
      actual_score: row.feedback_actual_score ? parseFloat(row.feedback_actual_score) : null,
      reason: row.feedback_reason,
    } : null;

    const trajectory = row.traj_step_count ? {
      step_count: row.traj_step_count,
      duration_ms: row.traj_duration_ms,
      pivot_points: row.traj_pivot_points || [],
      coherence: row.traj_coherence ? parseFloat(row.traj_coherence) : null,
      efficiency: row.traj_efficiency ? parseFloat(row.traj_efficiency) : null,
      patterns_used: row.traj_patterns || [],
      warnings_issued: row.traj_warnings || [],
    } : null;

    if (feedback) withFeedback++;
    if (trajectory) withTrajectory++;
    if (skepticism) withSkepticism++;
    if (row.dimension_scores && Object.keys(row.dimension_scores).length > 0) withDimensions++;

    const qScore = parseFloat(row.q_score);
    const confidence = parseFloat(row.confidence);

    // Extract skepticism from context if present (D13: skepticism persistence)
    const contextObj = row.context || {};
    const skepticism = contextObj.skepticism || null;

    const record = {
      id: row.judgment_id,
      input: {
        item_type: row.item_type,
        item_hash: row.item_hash,
        context: typeof contextObj === 'object' ? { type: contextObj.type } : { type: contextObj },
      },
      judgment: {
        q_score: qScore,
        global_score: parseFloat(row.global_score),
        confidence,
        verdict: row.verdict,
        axiom_scores: row.axiom_scores || {},
        dimension_scores: row.dimension_scores || {},
        reasoning_path: row.reasoning_path || [],
      },
      // D13: Include skepticism as top-level field for training pipeline
      skepticism,
      feedback,
      trajectory,
      reward: computeReward(qScore, confidence, feedback),
    };

    output.write(JSON.stringify(record) + '\n');
  }

  output.end();

  console.error(`[export] Export complete:`);
  console.error(`  Total judgments:    ${rows.length}`);
  console.error(`  With feedback:      ${withFeedback} (${rows.length ? Math.round(withFeedback / rows.length * 100) : 0}%)`);
  console.error(`  With trajectory:    ${withTrajectory} (${rows.length ? Math.round(withTrajectory / rows.length * 100) : 0}%)`);
  console.error(`  With skepticism:    ${withSkepticism} (${rows.length ? Math.round(withSkepticism / rows.length * 100) : 0}%)`);
  console.error(`  With dimensions:    ${withDimensions} (${rows.length ? Math.round(withDimensions / rows.length * 100) : 0}%)`);
  console.error(`  Output:             ${outputPath}`);

  // Close pool (getPool returns a shared pool that may not have .end())
  try { await pool.end(); } catch { /* shared pool — skip */ }
}

// ═══════════════════════════════════════════════════════════════════════════
// Run
// ═══════════════════════════════════════════════════════════════════════════

exportTrainingData().catch(err => {
  console.error('[export] Fatal:', err.message);
  process.exit(1);
});
