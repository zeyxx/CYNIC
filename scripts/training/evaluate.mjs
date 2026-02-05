#!/usr/bin/env node
/**
 * CYNIC Dog 0 — Model Evaluation
 *
 * Evaluates the fine-tuned model on the test set and checks
 * against the deployment gate thresholds.
 *
 * Metrics:
 *   - Verdict agreement (predicted vs actual)
 *   - Format compliance (% parseable JSON)
 *   - Reward distribution (mean, positive ratio)
 *   - Confidence calibration (ECE)
 *   - Score accuracy (within threshold)
 *
 * Usage:
 *   node scripts/training/evaluate.mjs [--model name] [--test-data path]
 *
 * Prerequisites:
 *   - Ollama running with fine-tuned model (or base model)
 *   - training-splits/test.jsonl exists
 *
 * "φ distrusts φ" — the model must prove itself
 *
 * @module cynic/training/evaluate
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join } from 'path';
import { evaluation as evalConfig, deployment } from './training-config.mjs';
import { computeReward } from './reward-function.mjs';

// ═══════════════════════════════════════════════════════════════════════════
// CLI
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
function getArg(flag, fallback) {
  const idx = args.indexOf(flag);
  return idx >= 0 && idx + 1 < args.length ? args[idx + 1] : fallback;
}

const ollamaEndpoint = process.env.OLLAMA_URL || deployment.ollama.endpoint;
const ollamaModel = getArg('--model', process.env.CYNIC_DOG0_MODEL || 'cynic-dog0');
const testPath = getArg('--test-data', 'training-splits/test.jsonl');
const outputDir = getArg('--output-dir', 'training-checkpoints/eval');

// ═══════════════════════════════════════════════════════════════════════════
// Ollama
// ═══════════════════════════════════════════════════════════════════════════

async function generate(prompt) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 60000);
  try {
    const r = await fetch(`${ollamaEndpoint}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: ollamaModel,
        prompt,
        stream: false,
        options: { temperature: 0.3, num_predict: 256 },
      }),
      signal: controller.signal,
    });
    clearTimeout(timeout);
    if (!r.ok) throw new Error(`Ollama ${r.status}`);
    const data = await r.json();
    return data.response || '';
  } catch (e) {
    clearTimeout(timeout);
    throw e;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Parse & Compare
// ═══════════════════════════════════════════════════════════════════════════

function parseResponse(text) {
  try {
    const match = text.match(/\{[\s\S]*\}/);
    if (match) {
      const p = JSON.parse(match[0]);
      return {
        qScore: Math.min(100, Math.max(0, p.score || 50)),
        confidence: Math.min(0.618, Math.max(0, p.confidence || 0.3)),
        verdict: (p.verdict || '').toUpperCase(),
        reasoning: p.reasoning || '',
        valid: true,
      };
    }
  } catch { /* parse fail */ }
  return { qScore: 50, confidence: 0.3, verdict: '', reasoning: '', valid: false };
}

function buildPrompt(record) {
  const { input } = record;
  return [
    `<|im_start|>system`,
    `You are CYNIC (κυνικός), a cynical judgment system. Evaluate items and respond with JSON.`,
    ``,
    `Verdict rules: HOWL (score>=76, exceptional), WAG (score 50-75, normal/acceptable), GROWL (score 30-49, concerning), BARK (score<30, danger).`,
    `Most items score 50-65 and get WAG. Confidence max: 0.618.`,
    `<|im_end|>`,
    `<|im_start|>user`,
    `Judge this test:`,
    `{"type":"test","source":"batch-test"}`,
    `<|im_end|>`,
    `<|im_start|>assistant`,
    `{"score": 55, "confidence": 0.3, "verdict": "WAG", "reasoning": "Standard test item, no specific concerns."}`,
    `<|im_end|>`,
    `<|im_start|>user`,
    `Judge this code:`,
    `{"type":"code","source":"commit","quality":85}`,
    `<|im_end|>`,
    `<|im_start|>assistant`,
    `{"score": 62, "confidence": 0.45, "verdict": "WAG", "reasoning": "Good quality code, meets standards."}`,
    `<|im_end|>`,
    `<|im_start|>user`,
    `Judge this ${input.item_type}:`,
    JSON.stringify(input.context || {}, null, 2).slice(0, 500),
    `<|im_end|>`,
    `<|im_start|>assistant`,
  ].join('\n');
}

// ═══════════════════════════════════════════════════════════════════════════
// Metrics
// ═══════════════════════════════════════════════════════════════════════════

function computeECE(predictions, bins = 10) {
  // Expected Calibration Error
  const buckets = Array.from({ length: bins }, () => ({ confSum: 0, accSum: 0, count: 0 }));

  for (const p of predictions) {
    const binIdx = Math.min(bins - 1, Math.floor(p.confidence * bins));
    buckets[binIdx].confSum += p.confidence;
    buckets[binIdx].accSum += p.correct ? 1 : 0;
    buckets[binIdx].count++;
  }

  let ece = 0;
  const total = predictions.length;
  for (const b of buckets) {
    if (b.count === 0) continue;
    const avgConf = b.confSum / b.count;
    const avgAcc = b.accSum / b.count;
    ece += (b.count / total) * Math.abs(avgConf - avgAcc);
  }

  return Math.round(ece * 1000) / 1000;
}

// ═══════════════════════════════════════════════════════════════════════════
// Main Evaluation
// ═══════════════════════════════════════════════════════════════════════════

async function main() {
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  CYNIC Dog 0 — Model Evaluation`);
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  Model:     ${ollamaModel}`);
  console.error(`  Endpoint:  ${ollamaEndpoint}`);
  console.error(`  Test data: ${testPath}`);
  console.error(``);

  // Load test data
  let records;
  try {
    records = readFileSync(testPath, 'utf-8').trim().split('\n')
      .map(l => { try { return JSON.parse(l); } catch { return null; } })
      .filter(Boolean);
  } catch (e) {
    console.error(`[eval] ERROR: Cannot load ${testPath}: ${e.message}`);
    process.exit(1);
  }

  console.error(`[eval] Loaded ${records.length} test records`);

  mkdirSync(outputDir, { recursive: true });

  // Evaluate each record
  const results = [];
  let formatValid = 0;
  let verdictMatch = 0;
  const scoreErrors = [];
  const rewards = [];
  const calibrationData = [];

  for (let i = 0; i < records.length; i++) {
    const record = records[i];
    const prompt = buildPrompt(record);
    const reference = record.judgment;

    if ((i + 1) % 5 === 0 || i === 0) {
      console.error(`[eval] Evaluating ${i + 1}/${records.length}...`);
    }

    try {
      const text = await generate(prompt);
      const pred = parseResponse(text);

      // Format compliance
      if (pred.valid) formatValid++;

      // Verdict agreement
      const refVerdict = (reference?.verdict || '').toUpperCase();
      const predVerdict = pred.verdict;
      const verdictOk = predVerdict === refVerdict;
      if (verdictOk) verdictMatch++;

      // Score accuracy
      const scoreDiff = Math.abs(pred.qScore - (reference?.q_score || 50));
      scoreErrors.push(scoreDiff);

      // Reward
      const feedback = reference ? { outcome: 'partial', actual_score: reference.q_score } : null;
      const reward = computeReward(pred.qScore, pred.confidence, feedback);
      rewards.push(reward);

      // Calibration
      calibrationData.push({
        confidence: pred.confidence,
        correct: verdictOk,
      });

      results.push({
        id: record.id,
        predicted: pred,
        reference: { qScore: reference?.q_score, verdict: refVerdict },
        scoreDiff,
        verdictMatch: verdictOk,
        reward,
      });
    } catch (e) {
      console.error(`[eval]   Failed on ${record.id}: ${e.message}`);
      results.push({ id: record.id, error: e.message });
    }
  }

  // ── Compute metrics ─────────────────────────────────────────────────────

  const total = records.length;
  const formatRate = total > 0 ? formatValid / total : 0;
  const verdictRate = total > 0 ? verdictMatch / total : 0;
  const meanReward = rewards.length > 0 ? rewards.reduce((a, b) => a + b, 0) / rewards.length : 0;
  const ece = computeECE(calibrationData, evalConfig.metrics.confidenceCalibration.bins);

  const scoreWithin5 = scoreErrors.filter(e => e <= 5).length / Math.max(1, scoreErrors.length);
  const scoreWithin10 = scoreErrors.filter(e => e <= 10).length / Math.max(1, scoreErrors.length);
  const scoreWithin20 = scoreErrors.filter(e => e <= 20).length / Math.max(1, scoreErrors.length);

  const negativeRatio = rewards.filter(r => r < 0).length / Math.max(1, rewards.length);

  // ── Deployment gate ───────────────────────────────────────────────────

  const gate = evalConfig.deploymentGate;
  const gateResults = {
    verdictAgreement: { value: verdictRate, threshold: gate.verdictAgreement, pass: verdictRate >= gate.verdictAgreement },
    formatCompliance: { value: formatRate, threshold: gate.formatCompliance, pass: formatRate >= gate.formatCompliance },
    meanReward: { value: meanReward, threshold: gate.meanReward, pass: meanReward >= gate.meanReward },
    ece: { value: ece, threshold: gate.eceBelow, pass: ece <= gate.eceBelow },
  };

  const allGatesPass = Object.values(gateResults).every(g => g.pass);

  // ── Output ────────────────────────────────────────────────────────────

  const report = {
    timestamp: new Date().toISOString(),
    model: ollamaModel,
    testRecords: total,
    metrics: {
      formatCompliance: Math.round(formatRate * 1000) / 1000,
      verdictAgreement: Math.round(verdictRate * 1000) / 1000,
      meanReward: Math.round(meanReward * 1000) / 1000,
      ece,
      scoreAccuracy: {
        within5: Math.round(scoreWithin5 * 1000) / 1000,
        within10: Math.round(scoreWithin10 * 1000) / 1000,
        within20: Math.round(scoreWithin20 * 1000) / 1000,
      },
      rewardDistribution: {
        positive: rewards.filter(r => r > 0).length,
        negative: rewards.filter(r => r < 0).length,
        neutral: rewards.filter(r => r === 0).length,
        negativeRatio: Math.round(negativeRatio * 1000) / 1000,
      },
    },
    deploymentGate: gateResults,
    deployable: allGatesPass,
    results,
  };

  writeFileSync(join(outputDir, 'eval-report.json'), JSON.stringify(report, null, 2) + '\n');

  // ── Display ───────────────────────────────────────────────────────────

  console.error(`\n═══════════════════════════════════════════════════════════════`);
  console.error(`  Evaluation Results`);
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  Test records:    ${total}`);
  console.error(``);
  console.error(`  ── METRICS ──────────────────────────────────────────────`);
  console.error(`  Format compliance: ${(formatRate * 100).toFixed(1)}%`);
  console.error(`  Verdict agreement: ${(verdictRate * 100).toFixed(1)}%`);
  console.error(`  Mean reward:       ${meanReward.toFixed(3)}`);
  console.error(`  ECE (calibration): ${(ece * 100).toFixed(1)}%`);
  console.error(`  Score within 5:    ${(scoreWithin5 * 100).toFixed(1)}%`);
  console.error(`  Score within 10:   ${(scoreWithin10 * 100).toFixed(1)}%`);
  console.error(`  Score within 20:   ${(scoreWithin20 * 100).toFixed(1)}%`);
  console.error(``);
  console.error(`  ── DEPLOYMENT GATE ──────────────────────────────────────`);

  for (const [name, g] of Object.entries(gateResults)) {
    const icon = g.pass ? '✅' : '❌';
    const op = name === 'ece' ? '<=' : '>=';
    console.error(`  ${icon} ${name}: ${typeof g.value === 'number' ? (g.value * 100).toFixed(1) + '%' : g.value} (${op} ${(g.threshold * 100).toFixed(1)}%)`);
  }

  console.error(``);
  if (allGatesPass) {
    console.error(`  ✅ ALL GATES PASS — Model is deployable`);
    console.error(`  Run: bash scripts/training/deploy-ollama.sh`);
  } else {
    console.error(`  ❌ GATES FAILED — Model needs more training`);
    console.error(`  Consider:`);
    console.error(`    - More GRPO iterations`);
    console.error(`    - More training data (use /learn to provide feedback)`);
    console.error(`    - Adjust deployment gate thresholds`);
  }
  console.error(`═══════════════════════════════════════════════════════════════`);
  console.error(`  Report: ${join(outputDir, 'eval-report.json')}`);

  process.exit(allGatesPass ? 0 : 1);
}

main().catch(e => {
  console.error(`[eval] Fatal: ${e.message}`);
  process.exit(1);
});
