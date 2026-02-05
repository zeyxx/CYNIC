#!/usr/bin/env node
/**
 * Convert CYNIC Training Data to DPO Format
 *
 * DPO (Direct Preference Optimization) requires preference pairs:
 * { prompt, chosen, rejected }
 *
 * Strategy for generating pairs from CYNIC data:
 * 1. Group judgments by item_hash (same item, different judgments)
 * 2. Use feedback to rank: correct > partial > incorrect
 * 3. Use Q-score to rank when feedback is equal
 * 4. Generate all valid pairs where chosen.rank > rejected.rank
 *
 * Alternative (single judgment):
 * When only one judgment exists, create synthetic rejected:
 * - If feedback=correct: rejected = perturbed judgment (lower confidence, wrong verdict)
 * - If feedback=incorrect: chosen = corrected judgment, rejected = original
 *
 * Output format (HuggingFace DPO format):
 * {
 *   "prompt": "<|im_start|>system\n...<|im_end|>\n<|im_start|>user\n...<|im_end|>\n<|im_start|>assistant\n",
 *   "chosen": "{ q_score: 65, verdict: \"WAG\", ... }<|im_end|>",
 *   "rejected": "{ q_score: 45, verdict: \"GROWL\", ... }<|im_end|>"
 * }
 *
 * "φ distrusts φ" — even preferences are bounded
 *
 * @module cynic/training/convert-to-dpo
 */

import { createReadStream, createWriteStream, readFileSync, existsSync } from 'fs';
import { createInterface } from 'readline';

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

const PHI_INV = 0.618;
const SYSTEM_PROMPT = `You are CYNIC (κυνικός), a cynical judgment system.
Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN).
Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest.
Output a JSON judgment object with q_score, verdict, axiom_scores, and confidence.`;

const VERDICT_RANK = { HOWL: 4, WAG: 3, GROWL: 2, BARK: 1 };
const FEEDBACK_RANK = { correct: 3, partial: 2, incorrect: 1, null: 0 };

// ═══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
const inputPath = getArg(args, '--input') || findLatestJsonl();
const outputPath = getArg(args, '--output') || inputPath.replace('.jsonl', '-dpo.jsonl');
const minPairs = parseInt(getArg(args, '--min-pairs') || '34', 10); // Fib(9)
const generateSynthetic = getArg(args, '--synthetic') !== 'false';

function getArg(args, flag) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : null;
}

function findLatestJsonl() {
  const fs = require('fs');
  const files = fs.readdirSync('.').filter(f => f.startsWith('training-data-') && f.endsWith('.jsonl'));
  files.sort().reverse();
  return files[0] || 'training-data.jsonl';
}

// ═══════════════════════════════════════════════════════════════════════════
// Ranking Functions
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate preference rank for a judgment
 * Higher rank = better judgment (should be "chosen")
 */
function calculateRank(record) {
  const feedbackScore = FEEDBACK_RANK[record.feedback?.outcome] || 0;
  const qScore = record.judgment?.q_score || 50;
  const confidence = record.judgment?.confidence || 0.5;

  // Primary: feedback (0-3), Secondary: Q-score (0-100), Tertiary: confidence (0-0.618)
  return feedbackScore * 10000 + qScore * 100 + confidence * 100;
}

/**
 * Format judgment as assistant response
 */
function formatJudgment(judgment) {
  return JSON.stringify({
    q_score: judgment.q_score,
    verdict: judgment.verdict,
    confidence: judgment.confidence,
    axiom_scores: judgment.axiom_scores,
  }, null, 0);
}

/**
 * Format prompt for DPO
 */
function formatPrompt(input) {
  const userContent = `Judge this ${input.item_type || 'item'}:
Hash: ${input.item_hash || 'unknown'}
Context: ${JSON.stringify(input.context || {})}`;

  return `<|im_start|>system
${SYSTEM_PROMPT}<|im_end|>
<|im_start|>user
${userContent}<|im_end|>
<|im_start|>assistant
`;
}

/**
 * Generate synthetic rejected judgment (perturbed version)
 */
function generateSyntheticRejected(judgment, feedback) {
  const perturbed = { ...judgment };

  if (feedback?.outcome === 'correct') {
    // Good judgment → create worse version
    perturbed.q_score = Math.max(0, judgment.q_score - 20 - Math.random() * 15);
    perturbed.confidence = Math.min(PHI_INV, judgment.confidence + 0.15); // Overconfident
    perturbed.verdict = perturbVerdict(judgment.verdict, -1);
  } else if (feedback?.outcome === 'incorrect') {
    // Bad judgment → this becomes rejected, synthetic becomes chosen
    // (handled in caller)
    perturbed.q_score = feedback.actual_score || judgment.q_score + 15;
    perturbed.confidence = Math.min(PHI_INV, 0.4 + Math.random() * 0.1);
    perturbed.verdict = deriveVerdict(perturbed.q_score);
  } else {
    // Neutral → small perturbation
    perturbed.q_score = Math.max(0, Math.min(100, judgment.q_score + (Math.random() - 0.5) * 20));
    perturbed.confidence = judgment.confidence * (0.8 + Math.random() * 0.4);
    perturbed.verdict = perturbVerdict(judgment.verdict, Math.random() > 0.5 ? 1 : -1);
  }

  return perturbed;
}

function perturbVerdict(verdict, direction) {
  const order = ['BARK', 'GROWL', 'WAG', 'HOWL'];
  const idx = order.indexOf(verdict);
  const newIdx = Math.max(0, Math.min(3, idx + direction));
  return order[newIdx];
}

function deriveVerdict(qScore) {
  if (qScore >= 70) return 'HOWL';
  if (qScore >= 50) return 'WAG';
  if (qScore >= 30) return 'GROWL';
  return 'BARK';
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Conversion
// ═══════════════════════════════════════════════════════════════════════════

async function convertToDpo() {
  console.error(`[dpo] Converting ${inputPath} to DPO format...`);
  console.error(`[dpo] Output: ${outputPath}`);
  console.error(`[dpo] Synthetic pairs: ${generateSynthetic}`);

  // Phase 1: Load and group by item_hash
  const byHash = new Map();
  const rl = createInterface({ input: createReadStream(inputPath) });

  for await (const line of rl) {
    if (!line.trim()) continue;
    try {
      const record = JSON.parse(line);
      const hash = record.input?.item_hash || record.id;
      if (!byHash.has(hash)) byHash.set(hash, []);
      byHash.get(hash).push(record);
    } catch (e) {
      console.error(`[dpo] Parse error: ${e.message}`);
    }
  }

  console.error(`[dpo] Loaded ${byHash.size} unique items`);

  // Phase 2: Generate pairs
  const output = createWriteStream(outputPath);
  let pairCount = 0;
  let syntheticCount = 0;

  for (const [hash, records] of byHash) {
    // Sort by rank (best first)
    records.sort((a, b) => calculateRank(b) - calculateRank(a));

    if (records.length >= 2) {
      // Multiple judgments for same item → natural pairs
      for (let i = 0; i < records.length - 1; i++) {
        for (let j = i + 1; j < records.length; j++) {
          const chosen = records[i];
          const rejected = records[j];

          // Only create pair if meaningfully different
          const rankDiff = calculateRank(chosen) - calculateRank(rejected);
          if (rankDiff < 500) continue; // Too similar

          const pair = {
            prompt: formatPrompt(chosen.input),
            chosen: formatJudgment(chosen.judgment) + '<|im_end|>',
            rejected: formatJudgment(rejected.judgment) + '<|im_end|>',
          };
          output.write(JSON.stringify(pair) + '\n');
          pairCount++;
        }
      }
    } else if (generateSynthetic && records.length === 1) {
      // Single judgment → synthetic pair
      const record = records[0];
      const feedback = record.feedback;

      if (feedback?.outcome === 'correct') {
        // Original is chosen, synthetic is rejected
        const rejected = generateSyntheticRejected(record.judgment, feedback);
        const pair = {
          prompt: formatPrompt(record.input),
          chosen: formatJudgment(record.judgment) + '<|im_end|>',
          rejected: formatJudgment(rejected) + '<|im_end|>',
        };
        output.write(JSON.stringify(pair) + '\n');
        pairCount++;
        syntheticCount++;
      } else if (feedback?.outcome === 'incorrect') {
        // Original is rejected, corrected is chosen
        const chosen = generateSyntheticRejected(record.judgment, feedback);
        const pair = {
          prompt: formatPrompt(record.input),
          chosen: formatJudgment(chosen) + '<|im_end|>',
          rejected: formatJudgment(record.judgment) + '<|im_end|>',
        };
        output.write(JSON.stringify(pair) + '\n');
        pairCount++;
        syntheticCount++;
      }
      // Skip neutral feedback (no clear preference signal)
    }
  }

  output.end();

  console.error(`[dpo] Conversion complete:`);
  console.error(`  Total pairs:      ${pairCount}`);
  console.error(`  Synthetic pairs:  ${syntheticCount}`);
  console.error(`  Natural pairs:    ${pairCount - syntheticCount}`);
  console.error(`  Output:           ${outputPath}`);

  if (pairCount < minPairs) {
    console.error(`[dpo] WARNING: Only ${pairCount} pairs (min: ${minPairs})`);
    console.error(`[dpo] DPO training may not be effective yet.`);
    console.error(`[dpo] Consider using GRPO (reward-based) instead.`);
    process.exit(1);
  }

  return { pairCount, syntheticCount };
}

// ═══════════════════════════════════════════════════════════════════════════
// Run
// ═══════════════════════════════════════════════════════════════════════════

convertToDpo().catch(err => {
  console.error('[dpo] Fatal:', err.message);
  process.exit(1);
});
