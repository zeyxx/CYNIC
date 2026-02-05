#!/usr/bin/env node
/**
 * CYNIC Dog 0 — Local GRPO via Rejection Sampling
 *
 * "Poor man's GRPO" for CPU-only training:
 *   1. Load test prompts from training splits
 *   2. Generate N completions per prompt via Ollama
 *   3. Score each completion with reward function
 *   4. Keep completions with positive reward
 *   5. Write filtered set as new training data
 *   6. Re-run SFT on filtered set (human triggers)
 *   7. Repeat until convergence or max iterations
 *
 * This approximates GRPO's "group relative" comparison by:
 *   - Generating multiple outputs (the "group")
 *   - Keeping only the best (relative selection)
 *   - Re-training on winners (policy optimization)
 *
 * Usage:
 *   node scripts/training/run-grpo-local.mjs [--iterations N] [--group-size N]
 *
 * Prerequisites:
 *   - Ollama running with base model (or SFT model after stage 3)
 *   - training-splits/train.jsonl exists
 *
 * "φ distrusts φ" — rejection is a form of judgment
 *
 * @module cynic/training/grpo-local
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { grpo as grpoConfig, model as modelConfig, deployment } from './training-config.mjs';
import { computeReward, rewardStats } from './reward-function.mjs';

// ═══════════════════════════════════════════════════════════════════════════
// CLI
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
function getArg(flag, fallback) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : fallback;
}

const maxIterations = parseInt(getArg('--iterations', '3'), 10);
const groupSize = parseInt(getArg('--group-size', String(grpoConfig.groupSize)), 10);
const ollamaEndpoint = process.env.OLLAMA_URL || deployment.ollama.endpoint;
const ollamaModel = getArg('--model', process.env.CYNIC_DOG0_MODEL || 'cynic-dog0');
const splitsDir = getArg('--splits-dir', 'training-splits');
const outputDir = getArg('--output-dir', 'training-checkpoints/grpo-local');

// ═══════════════════════════════════════════════════════════════════════════
// Ollama Client
// ═══════════════════════════════════════════════════════════════════════════

async function ollamaGenerate(prompt, temperature = grpoConfig.temperature) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 60000);

  try {
    const response = await fetch(`${ollamaEndpoint}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: ollamaModel,
        prompt,
        stream: false,
        options: {
          temperature,
          top_p: grpoConfig.topP,
          num_predict: grpoConfig.maxNewTokens,
        },
      }),
      signal: controller.signal,
    });

    clearTimeout(timeout);
    if (!response.ok) throw new Error(`Ollama ${response.status}`);
    const data = await response.json();
    return data.response || '';
  } catch (e) {
    clearTimeout(timeout);
    throw e;
  }
}

async function checkOllama() {
  try {
    const r = await fetch(`${ollamaEndpoint}/api/tags`, { signal: AbortSignal.timeout(5000) });
    if (!r.ok) return false;
    const data = await r.json();
    const models = (data.models || []).map(m => m.name);
    console.error(`[grpo] Ollama models: ${models.join(', ')}`);
    return models.some(m => m.includes(ollamaModel) || m.includes('qwen'));
  } catch {
    return false;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Load Prompts
// ═══════════════════════════════════════════════════════════════════════════

function loadPrompts(path) {
  const lines = readFileSync(path, 'utf-8').trim().split('\n');
  return lines.map(line => {
    try { return JSON.parse(line); } catch { return null; }
  }).filter(Boolean);
}

/**
 * Build a judgment prompt from a training record.
 */
function buildPrompt(record) {
  const { input } = record;
  return [
    `<|im_start|>system`,
    `You are CYNIC (κυνικός), a cynical judgment system. Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN). Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest.`,
    ``,
    `Verdict definitions based on score:`,
    `- HOWL: score >= 76 (exceptional quality, rare — less than 5% of items)`,
    `- WAG: score 61-75 (acceptable quality, MOST items land here — ~70% of items)`,
    `- GROWL: score 38-60 (needs improvement, moderate concern)`,
    `- BARK: score < 38 (critical danger, reject immediately)`,
    ``,
    `Important: Most items are ordinary and score 50-70 (WAG range). Only truly exceptional items score above 76. Be skeptical — default toward WAG unless strong evidence otherwise.`,
    ``,
    `Respond with JSON only: {"score": 0-100, "confidence": 0.0-0.618, "verdict": "HOWL|WAG|GROWL|BARK", "reasoning": "brief"}`,
    `<|im_end|>`,
    `<|im_start|>user`,
    `Judge this ${input.item_type}:`,
    JSON.stringify(input.context || {}, null, 2).slice(0, 500),
    `<|im_end|>`,
    `<|im_start|>assistant`,
  ].join('\n');
}

/**
 * Parse model response into score/confidence for reward computation.
 */
function parseResponse(text) {
  try {
    const match = text.match(/\{[\s\S]*\}/);
    if (match) {
      const parsed = JSON.parse(match[0]);
      return {
        qScore: Math.min(100, Math.max(0, parsed.score || 50)),
        confidence: Math.min(0.618, Math.max(0, parsed.confidence || 0.3)),
        verdict: parsed.verdict || 'GROWL',
        reasoning: parsed.reasoning || '',
        valid: true,
      };
    }
  } catch { /* parse failure */ }

  return { qScore: 50, confidence: 0.1, verdict: null, reasoning: '', valid: false };
}

// ═══════════════════════════════════════════════════════════════════════════
// GRPO Iteration
// ═══════════════════════════════════════════════════════════════════════════

async function grpoIteration(records, iteration) {
  console.error(`\n[grpo] ═══ Iteration ${iteration + 1}/${maxIterations} ═══`);
  console.error(`[grpo] Prompts: ${records.length} | Group size: ${groupSize}`);

  const accepted = [];
  const allRewards = [];
  let formatValid = 0;
  let formatTotal = 0;

  // Sample a subset of prompts per iteration (avoid exhausting all data)
  const sampleSize = Math.min(records.length, grpoConfig.maxSteps || 50);
  const sampled = records.slice(0, sampleSize);

  for (let i = 0; i < sampled.length; i++) {
    const record = sampled[i];
    const prompt = buildPrompt(record);
    const reference = record.judgment;

    if ((i + 1) % 10 === 0 || i === 0) {
      console.error(`[grpo]   Processing ${i + 1}/${sampled.length}...`);
    }

    // Generate N completions
    const completions = [];
    for (let g = 0; g < groupSize; g++) {
      try {
        const text = await ollamaGenerate(prompt);
        const parsed = parseResponse(text);
        formatTotal++;
        if (parsed.valid) formatValid++;

        // Compute reward using reference judgment as "actual"
        const feedback = reference ? {
          outcome: 'partial',
          actual_score: reference.q_score,
        } : null;

        const reward = computeReward(parsed.qScore, parsed.confidence, feedback);

        completions.push({
          text,
          parsed,
          reward,
        });
        allRewards.push(reward);
      } catch (e) {
        console.error(`[grpo]   Generation failed: ${e.message}`);
      }
    }

    // Keep completions with positive reward (rejection sampling)
    const winners = completions
      .filter(c => c.reward > 0 && c.parsed.valid)
      .sort((a, b) => b.reward - a.reward);

    if (winners.length > 0) {
      // Take the best completion as the new training example
      const best = winners[0];
      accepted.push({
        ...record,
        judgment: {
          q_score: best.parsed.qScore,
          confidence: best.parsed.confidence,
          verdict: best.parsed.verdict,
          axiom_scores: record.judgment?.axiom_scores || {},
          dimension_scores: record.judgment?.dimension_scores || {},
          reasoning_path: [{ step: 0, content: best.parsed.reasoning }],
        },
        reward: best.reward,
        grpo_iteration: iteration,
      });
    }
  }

  // Stats
  const stats = rewardStats(allRewards);
  const acceptRate = sampled.length > 0 ? (accepted.length / sampled.length * 100).toFixed(1) : 0;
  const formatRate = formatTotal > 0 ? (formatValid / formatTotal * 100).toFixed(1) : 0;

  console.error(`[grpo]   Results:`);
  console.error(`    Accepted:       ${accepted.length}/${sampled.length} (${acceptRate}%)`);
  console.error(`    Format valid:   ${formatValid}/${formatTotal} (${formatRate}%)`);
  console.error(`    Mean reward:    ${stats.mean}`);
  console.error(`    Reward range:   [${stats.min}, ${stats.max}]`);
  console.error(`    Positive/Neg:   ${stats.positive}/${stats.negative}`);

  return { accepted, stats, formatRate: parseFloat(formatRate) };
}

// ═══════════════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  CYNIC Dog 0 — GRPO Local (Rejection Sampling)`);
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  Model:       ${ollamaModel}`);
  console.error(`  Endpoint:    ${ollamaEndpoint}`);
  console.error(`  Iterations:  ${maxIterations}`);
  console.error(`  Group size:  ${groupSize}`);
  console.error(``);

  // Check Ollama
  const available = await checkOllama();
  if (!available) {
    console.error(`[grpo] ERROR: Ollama not available or model '${ollamaModel}' not found.`);
    console.error(`[grpo] Make sure Ollama is running and the model is loaded:`);
    console.error(`  ollama pull qwen2.5:1.5b`);
    console.error(`  # or after SFT: use the fine-tuned model`);
    process.exit(1);
  }

  // Load training data
  const trainPath = join(splitsDir, 'train.jsonl');
  let records;
  try {
    records = loadPrompts(trainPath);
  } catch (e) {
    console.error(`[grpo] ERROR: Cannot load ${trainPath}: ${e.message}`);
    console.error(`[grpo] Run: node scripts/training/split-data.mjs`);
    process.exit(1);
  }

  console.error(`[grpo] Loaded ${records.length} training records`);

  mkdirSync(outputDir, { recursive: true });

  // Run iterations
  let cumulativeAccepted = [];
  const iterationStats = [];

  for (let iter = 0; iter < maxIterations; iter++) {
    const result = await grpoIteration(records, iter);
    cumulativeAccepted.push(...result.accepted);
    iterationStats.push(result.stats);

    // Write iteration output
    const iterPath = join(outputDir, `grpo-iter-${iter + 1}.jsonl`);
    writeFileSync(iterPath, result.accepted.map(r => JSON.stringify(r)).join('\n') + '\n');
    console.error(`[grpo]   Wrote ${result.accepted.length} records → ${iterPath}`);

    // Early stop if rewards are consistently positive
    if (result.stats.mean > 0.2 && result.formatRate > 80) {
      console.error(`[grpo]   Converged: mean reward ${result.stats.mean} > 0.2, format ${result.formatRate}% > 80%`);
      break;
    }
  }

  // Write combined GRPO-refined training data
  const combinedPath = join(outputDir, 'grpo-refined.jsonl');
  writeFileSync(combinedPath, cumulativeAccepted.map(r => JSON.stringify(r)).join('\n') + '\n');

  // Write plain text version for llama.cpp re-training
  const textPath = join(outputDir, 'grpo-refined.txt');
  const { formatSFTExample } = await import('./reward-function.mjs');
  const textLines = cumulativeAccepted.map(record => {
    const example = formatSFTExample(record);
    return example.messages.map(m =>
      `<|im_start|>${m.role}\n${m.content}<|im_end|>`
    ).join('\n');
  });
  writeFileSync(textPath, textLines.join('\n\n') + '\n');

  // Write summary
  const summary = {
    timestamp: new Date().toISOString(),
    model: ollamaModel,
    iterations: iterationStats.length,
    groupSize,
    totalAccepted: cumulativeAccepted.length,
    totalPrompts: records.length,
    acceptRate: records.length > 0 ? Math.round(cumulativeAccepted.length / records.length * 100) : 0,
    iterationStats,
    outputFiles: {
      refined: combinedPath,
      text: textPath,
    },
  };
  writeFileSync(join(outputDir, 'grpo-summary.json'), JSON.stringify(summary, null, 2) + '\n');

  console.error(`\n═══════════════════════════════════════════════════════════════`);
  console.error(`  GRPO Complete`);
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  Iterations:    ${iterationStats.length}`);
  console.error(`  Accepted:      ${cumulativeAccepted.length}/${records.length} records`);
  console.error(`  Refined data:  ${combinedPath}`);
  console.error(`  llama.cpp txt: ${textPath}`);
  console.error(``);
  console.error(`  Next steps:`);
  console.error(`    1. Re-run SFT on refined data:`);
  console.error(`       bash scripts/training/run-sft-local.sh --data ${textPath}`);
  console.error(`    2. Evaluate:`);
  console.error(`       node scripts/training/evaluate.mjs`);
  console.error(`═══════════════════════════════════════════════════════════════`);
}

main().catch(e => {
  console.error(`[grpo] Fatal: ${e.message}`);
  process.exit(1);
});
