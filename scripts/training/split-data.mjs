#!/usr/bin/env node
/**
 * CYNIC Training Data Splitter
 *
 * Splits exported JSONL into train/val/test sets with φ-aligned ratios.
 * Stratifies by verdict type to ensure balanced representation.
 *
 * Usage:
 *   node scripts/training/split-data.mjs [--input path.jsonl] [--output-dir dir] [--profile local|cloud]
 *
 * Reads config from training-config.mjs for split ratios and thresholds.
 *
 * Output files:
 *   {output-dir}/train.jsonl
 *   {output-dir}/val.jsonl
 *   {output-dir}/test.jsonl
 *   {output-dir}/split-stats.json  (metadata about the split)
 *
 * For llama.cpp local training, also generates:
 *   {output-dir}/train.txt  (plain text format for llama-finetune)
 *
 * "φ distrusts φ" — data quality > data quantity
 *
 * @module cynic/training/split
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'fs';
import { join, resolve } from 'path';
import { data as dataConfig, activeProfile, model } from './training-config.mjs';
import { formatSFTExample } from './reward-function.mjs';

// ═══════════════════════════════════════════════════════════════════════════
// CLI Arguments
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);

function getArg(flag, fallback = null) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : fallback;
}

const inputPath = getArg('--input');
const outputDir = getArg('--output-dir') || dataConfig.outputDir || 'training-splits';
const profile = getArg('--profile') || activeProfile;

// ═══════════════════════════════════════════════════════════════════════════
// Find Input File
// ═══════════════════════════════════════════════════════════════════════════

function findInputFile() {
  if (inputPath) return resolve(inputPath);

  // Find most recent training-data-*.jsonl in current directory
  const pattern = /^training-data-.*\.jsonl$/;
  const candidates = readdirSync('.').filter(f => pattern.test(f)).sort().reverse();

  if (candidates.length === 0) {
    console.error('[split] No training data found. Run export-training-data.mjs first.');
    process.exit(1);
  }

  console.error(`[split] Found ${candidates.length} data file(s), using: ${candidates[0]}`);
  return resolve(candidates[0]);
}

// ═══════════════════════════════════════════════════════════════════════════
// Load & Filter Records
// ═══════════════════════════════════════════════════════════════════════════

function loadRecords(filePath) {
  const raw = readFileSync(filePath, 'utf-8').trim().split('\n');
  const records = raw.map((line, i) => {
    try {
      return JSON.parse(line);
    } catch (e) {
      console.error(`[split] Skipping malformed line ${i + 1}: ${e.message}`);
      return null;
    }
  }).filter(Boolean);

  console.error(`[split] Loaded ${records.length} records from ${filePath}`);

  // Filter neutral rewards if configured
  if (dataConfig.excludeNeutralRewards) {
    const before = records.length;
    const filtered = records.filter(r => r.reward !== 0);
    console.error(`[split] Filtered neutral rewards: ${before} → ${filtered.length} (removed ${before - filtered.length})`);
    return filtered;
  }

  return records;
}

// ═══════════════════════════════════════════════════════════════════════════
// Stratified Split
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Split records into train/val/test with stratification.
 * Uses deterministic shuffle (seeded by record count) for reproducibility.
 */
function stratifiedSplit(records) {
  const { splits, stratifyBy } = dataConfig;

  // Group by stratification key
  const groups = {};
  for (const record of records) {
    const key = extractStratifyKey(record, stratifyBy);
    if (!groups[key]) groups[key] = [];
    groups[key].push(record);
  }

  console.error(`[split] Stratification groups (${stratifyBy}):`);
  for (const [key, recs] of Object.entries(groups)) {
    console.error(`  ${key}: ${recs.length} records`);
  }

  // Split each group proportionally
  const train = [];
  const val = [];
  const test = [];

  for (const [_key, recs] of Object.entries(groups)) {
    // Deterministic shuffle
    shuffle(recs, records.length);

    const trainEnd = Math.round(recs.length * splits.train);
    const valEnd = trainEnd + Math.round(recs.length * splits.validation);

    train.push(...recs.slice(0, trainEnd));
    val.push(...recs.slice(trainEnd, valEnd));
    test.push(...recs.slice(valEnd));
  }

  // Final shuffle within each split
  shuffle(train, 42);
  shuffle(val, 137);
  shuffle(test, 233);

  return { train, val, test };
}

/**
 * Extract stratification key from a record.
 */
function extractStratifyKey(record, field) {
  if (field === 'verdict') {
    return record.judgment?.verdict || 'UNKNOWN';
  }
  if (field === 'feedback') {
    return record.feedback?.outcome || 'none';
  }
  return 'all';
}

/**
 * Deterministic Fisher-Yates shuffle (seeded PRNG).
 * Not crypto-quality — just reproducible.
 */
function shuffle(arr, seed) {
  let s = seed;
  const rand = () => {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    return s / 0x7fffffff;
  };

  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

// ═══════════════════════════════════════════════════════════════════════════
// Format for llama.cpp (plain text)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Convert JSONL records to plain text format for llama.cpp finetune.
 * Uses ChatML format: <|im_start|>role\ncontent<|im_end|>
 */
function toLlamaCppText(records) {
  return records.map(record => {
    const example = formatSFTExample(record);
    return example.messages.map(m =>
      `<|im_start|>${m.role}\n${m.content}<|im_end|>`
    ).join('\n') + '\n';
  }).join('\n');
}

// ═══════════════════════════════════════════════════════════════════════════
// Write Output
// ═══════════════════════════════════════════════════════════════════════════

function writeOutputs(splits, outputDir) {
  mkdirSync(outputDir, { recursive: true });

  // Write JSONL splits
  for (const [name, records] of Object.entries(splits)) {
    const path = join(outputDir, `${name}.jsonl`);
    writeFileSync(path, records.map(r => JSON.stringify(r)).join('\n') + '\n');
    console.error(`[split] Wrote ${records.length} records → ${path}`);
  }

  // For local profile: also write plain text for llama.cpp
  if (profile === 'local') {
    const trainTxt = join(outputDir, 'train.txt');
    writeFileSync(trainTxt, toLlamaCppText(splits.train));
    console.error(`[split] Wrote llama.cpp train text → ${trainTxt}`);

    const valTxt = join(outputDir, 'val.txt');
    writeFileSync(valTxt, toLlamaCppText(splits.val));
    console.error(`[split] Wrote llama.cpp val text → ${valTxt}`);
  }

  // Write split statistics
  const stats = {
    profile,
    model: model.base,
    timestamp: new Date().toISOString(),
    counts: {
      total: splits.train.length + splits.val.length + splits.test.length,
      train: splits.train.length,
      val: splits.val.length,
      test: splits.test.length,
    },
    ratios: {
      train: round(splits.train.length / (splits.train.length + splits.val.length + splits.test.length)),
      val: round(splits.val.length / (splits.train.length + splits.val.length + splits.test.length)),
      test: round(splits.test.length / (splits.train.length + splits.val.length + splits.test.length)),
    },
    verdictDistribution: {
      train: countVerdicts(splits.train),
      val: countVerdicts(splits.val),
      test: countVerdicts(splits.test),
    },
    rewardStats: {
      train: computeRewardStats(splits.train),
      val: computeRewardStats(splits.val),
      test: computeRewardStats(splits.test),
    },
    feedbackCoverage: {
      train: round(splits.train.filter(r => r.feedback).length / splits.train.length),
      val: round(splits.val.filter(r => r.feedback).length / Math.max(1, splits.val.length)),
      test: round(splits.test.filter(r => r.feedback).length / Math.max(1, splits.test.length)),
    },
  };

  const statsPath = join(outputDir, 'split-stats.json');
  writeFileSync(statsPath, JSON.stringify(stats, null, 2) + '\n');
  console.error(`[split] Wrote stats → ${statsPath}`);

  return stats;
}

function countVerdicts(records) {
  const counts = {};
  for (const r of records) {
    const v = r.judgment?.verdict || 'UNKNOWN';
    counts[v] = (counts[v] || 0) + 1;
  }
  return counts;
}

function computeRewardStats(records) {
  const rewards = records.map(r => r.reward || 0);
  if (rewards.length === 0) return { mean: 0, min: 0, max: 0 };
  const sum = rewards.reduce((a, b) => a + b, 0);
  return {
    mean: round(sum / rewards.length),
    min: round(Math.min(...rewards)),
    max: round(Math.max(...rewards)),
    positive: rewards.filter(r => r > 0).length,
    negative: rewards.filter(r => r < 0).length,
  };
}

function round(n) {
  return Math.round(n * 1000) / 1000;
}

// ═══════════════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════════════

function main() {
  console.error(`[split] CYNIC Training Data Splitter`);
  console.error(`[split] Profile: ${profile} | Model: ${model.base}`);
  console.error(`[split] Ratios: train=${(dataConfig.splits.train * 100).toFixed(1)}% val=${(dataConfig.splits.validation * 100).toFixed(1)}% test=${round(dataConfig.splits.test * 100)}%`);

  const filePath = findInputFile();
  const records = loadRecords(filePath);

  // Validate minimum records
  if (records.length < dataConfig.minRecords) {
    console.error(`[split] WARNING: ${records.length} records < minimum ${dataConfig.minRecords} (Fib 11)`);
    console.error(`[split] Training may not generalize. Consider collecting more judgments first.`);
    // Continue anyway — let the user decide
  }

  // Check feedback coverage for GRPO
  const withFeedback = records.filter(r => r.feedback).length;
  if (withFeedback < dataConfig.minFeedbackRecords) {
    console.error(`[split] WARNING: ${withFeedback} feedback records < minimum ${dataConfig.minFeedbackRecords} (Fib 9)`);
    console.error(`[split] GRPO stage may produce weak rewards. Consider more /learn feedback.`);
  }

  const splits = stratifiedSplit(records);
  const stats = writeOutputs(splits, outputDir);

  // Print summary
  console.error(`\n[split] ═══ SPLIT COMPLETE ═══`);
  console.error(`  Total:    ${stats.counts.total}`);
  console.error(`  Train:    ${stats.counts.train} (${(stats.ratios.train * 100).toFixed(1)}%)`);
  console.error(`  Val:      ${stats.counts.val} (${(stats.ratios.val * 100).toFixed(1)}%)`);
  console.error(`  Test:     ${stats.counts.test} (${(stats.ratios.test * 100).toFixed(1)}%)`);
  console.error(`  Feedback: train=${(stats.feedbackCoverage.train * 100).toFixed(0)}% val=${(stats.feedbackCoverage.val * 100).toFixed(0)}% test=${(stats.feedbackCoverage.test * 100).toFixed(0)}%`);
  console.error(`  Output:   ${outputDir}/`);
  if (profile === 'local') {
    console.error(`  llama.cpp: train.txt + val.txt generated`);
  }
}

main();
