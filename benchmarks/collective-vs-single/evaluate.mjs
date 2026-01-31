#!/usr/bin/env node
/**
 * Benchmark Evaluator
 *
 * Compares single-agent and collective results against ground truth.
 * Calculates precision, recall, F1, and other metrics.
 */

import { readFileSync, readdirSync, writeFileSync } from 'fs';

// Load ground truth
const dataset = JSON.parse(
  readFileSync('benchmarks/collective-vs-single/dataset/samples.json', 'utf8')
);

const groundTruth = new Map();
for (const sample of dataset.samples) {
  groundTruth.set(sample.id, sample.groundTruth);
}

// Find latest results files
function findLatestResults(prefix) {
  const files = readdirSync('benchmarks/collective-vs-single/results')
    .filter(f => f.startsWith(prefix) && f.endsWith('.json'))
    .sort()
    .reverse();

  if (files.length === 0) {
    throw new Error(`No ${prefix} results found. Run the benchmark first.`);
  }

  return JSON.parse(
    readFileSync(`benchmarks/collective-vs-single/results/${files[0]}`, 'utf8')
  );
}

// Normalize issue types for comparison
function normalizeIssueType(type) {
  const normalized = (type || '').toUpperCase()
    .replace(/[_-]/g, '_')
    .replace(/INJECTION/g, 'INJECTION')
    .replace(/CROSS.?SITE.?SCRIPTING/g, 'XSS')
    .replace(/REFLECTED_XSS|STORED_XSS|DOM_XSS/g, 'XSS');
  return normalized;
}

// Check if detected issue matches ground truth issue
function issueMatches(detected, truth) {
  const detectedType = normalizeIssueType(detected.type);
  const truthType = normalizeIssueType(truth.type);

  // Exact type match
  if (detectedType === truthType) return true;

  // Related types (SQL_INJECTION variants)
  const sqlTypes = ['SQL_INJECTION', 'SQLI', 'SQL'];
  if (sqlTypes.includes(detectedType) && sqlTypes.includes(truthType)) return true;

  // XSS variants
  const xssTypes = ['XSS', 'CROSS_SITE_SCRIPTING', 'REFLECTED_XSS'];
  if (xssTypes.includes(detectedType) && xssTypes.includes(truthType)) return true;

  return false;
}

// Evaluate a single result against ground truth
function evaluateResult(result, truth) {
  const detectedIssues = result.result?.issues || [];
  const truthIssues = truth.issues || [];

  let truePositives = 0;
  let falsePositives = 0;
  const matchedTruth = new Set();

  // Check each detected issue
  for (const detected of detectedIssues) {
    let matched = false;
    for (let i = 0; i < truthIssues.length; i++) {
      if (!matchedTruth.has(i) && issueMatches(detected, truthIssues[i])) {
        truePositives++;
        matchedTruth.add(i);
        matched = true;
        break;
      }
    }
    if (!matched) {
      falsePositives++;
    }
  }

  const falseNegatives = truthIssues.length - matchedTruth.size;

  // Verdict evaluation
  const detectedVerdict = result.result?.verdict;
  const expectedVerdict = truth.expectedVerdict;
  const verdictMatch = detectedVerdict === expectedVerdict;

  // Verdict severity match (BARK/GROWL = bad, WAG/HOWL = good)
  const detectedBad = ['BARK', 'GROWL'].includes(detectedVerdict);
  const expectedBad = ['BARK', 'GROWL'].includes(expectedVerdict);
  const verdictDirectionMatch = detectedBad === expectedBad;

  // Score evaluation
  const detectedScore = result.result?.score ?? 50;
  const scoreInRange = detectedScore >= truth.minScore && detectedScore <= truth.maxScore;

  return {
    sampleId: result.sampleId,
    truePositives,
    falsePositives,
    falseNegatives,
    detectedCount: detectedIssues.length,
    truthCount: truthIssues.length,
    verdictMatch,
    verdictDirectionMatch,
    detectedVerdict,
    expectedVerdict,
    scoreInRange,
    detectedScore,
    expectedScoreRange: [truth.minScore, truth.maxScore]
  };
}

// Calculate aggregate metrics
function calculateMetrics(evaluations) {
  let totalTP = 0, totalFP = 0, totalFN = 0;
  let verdictMatches = 0, verdictDirectionMatches = 0;
  let scoreInRange = 0;

  for (const e of evaluations) {
    totalTP += e.truePositives;
    totalFP += e.falsePositives;
    totalFN += e.falseNegatives;
    if (e.verdictMatch) verdictMatches++;
    if (e.verdictDirectionMatch) verdictDirectionMatches++;
    if (e.scoreInRange) scoreInRange++;
  }

  const precision = totalTP / (totalTP + totalFP) || 0;
  const recall = totalTP / (totalTP + totalFN) || 0;
  const f1 = 2 * (precision * recall) / (precision + recall) || 0;

  return {
    precision,
    recall,
    f1,
    truePositives: totalTP,
    falsePositives: totalFP,
    falseNegatives: totalFN,
    verdictAccuracy: verdictMatches / evaluations.length,
    verdictDirectionAccuracy: verdictDirectionMatches / evaluations.length,
    scoreAccuracy: scoreInRange / evaluations.length,
    sampleCount: evaluations.length
  };
}

function formatPercent(n) {
  return (n * 100).toFixed(1) + '%';
}

function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  BENCHMARK EVALUATION');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');

  // Load results
  let singleResults, collectiveResults;
  try {
    singleResults = findLatestResults('single');
    console.log(`  Single-agent results: ${singleResults.timestamp}`);
  } catch (e) {
    console.log(`  Single-agent: ${e.message}`);
    singleResults = null;
  }

  try {
    collectiveResults = findLatestResults('collective');
    console.log(`  Collective results:   ${collectiveResults.timestamp}`);
  } catch (e) {
    console.log(`  Collective: ${e.message}`);
    collectiveResults = null;
  }

  if (!singleResults && !collectiveResults) {
    console.log('\n  No results to evaluate. Run benchmarks first.');
    return;
  }

  console.log('');
  console.log('───────────────────────────────────────────────────────────────');
  console.log('  METRICS COMPARISON');
  console.log('───────────────────────────────────────────────────────────────');
  console.log('');

  const allMetrics = {};

  // Evaluate single-agent
  if (singleResults) {
    const evaluations = [];
    for (const result of singleResults.results.filter(r => r.success)) {
      const truth = groundTruth.get(result.sampleId);
      if (truth) {
        evaluations.push(evaluateResult(result, truth));
      }
    }
    const metrics = calculateMetrics(evaluations);
    allMetrics.single = {
      ...metrics,
      avgLatencyMs: singleResults.summary.avgLatencyMs,
      totalTokens: singleResults.summary.totalTokensIn + singleResults.summary.totalTokensOut
    };

    console.log('  SINGLE-AGENT:');
    console.log(`    Precision:           ${formatPercent(metrics.precision)}`);
    console.log(`    Recall:              ${formatPercent(metrics.recall)}`);
    console.log(`    F1 Score:            ${formatPercent(metrics.f1)}`);
    console.log(`    Verdict Accuracy:    ${formatPercent(metrics.verdictAccuracy)}`);
    console.log(`    Verdict Direction:   ${formatPercent(metrics.verdictDirectionAccuracy)}`);
    console.log(`    Score in Range:      ${formatPercent(metrics.scoreAccuracy)}`);
    console.log(`    Avg Latency:         ${singleResults.summary.avgLatencyMs}ms`);
    console.log(`    Total Tokens:        ${allMetrics.single.totalTokens}`);
    console.log('');
  }

  // Evaluate collective
  if (collectiveResults) {
    const evaluations = [];
    for (const result of collectiveResults.results.filter(r => r.success)) {
      const truth = groundTruth.get(result.sampleId);
      if (truth) {
        evaluations.push(evaluateResult(result, truth));
      }
    }
    const metrics = calculateMetrics(evaluations);
    allMetrics.collective = {
      ...metrics,
      avgLatencyMs: collectiveResults.summary.avgLatencyMs
    };

    console.log('  COLLECTIVE (11 Dogs):');
    console.log(`    Precision:           ${formatPercent(metrics.precision)}`);
    console.log(`    Recall:              ${formatPercent(metrics.recall)}`);
    console.log(`    F1 Score:            ${formatPercent(metrics.f1)}`);
    console.log(`    Verdict Accuracy:    ${formatPercent(metrics.verdictAccuracy)}`);
    console.log(`    Verdict Direction:   ${formatPercent(metrics.verdictDirectionAccuracy)}`);
    console.log(`    Score in Range:      ${formatPercent(metrics.scoreAccuracy)}`);
    console.log(`    Avg Latency:         ${collectiveResults.summary.avgLatencyMs}ms`);
    console.log('');
  }

  // Comparison
  if (singleResults && collectiveResults) {
    console.log('───────────────────────────────────────────────────────────────');
    console.log('  VERDICT: Collective vs Single');
    console.log('───────────────────────────────────────────────────────────────');
    console.log('');

    const f1Diff = allMetrics.collective.f1 - allMetrics.single.f1;
    const precisionDiff = allMetrics.collective.precision - allMetrics.single.precision;
    const recallDiff = allMetrics.collective.recall - allMetrics.single.recall;
    const latencyRatio = allMetrics.collective.avgLatencyMs / allMetrics.single.avgLatencyMs;

    console.log(`    F1 Difference:       ${f1Diff >= 0 ? '+' : ''}${formatPercent(f1Diff)}`);
    console.log(`    Precision Diff:      ${precisionDiff >= 0 ? '+' : ''}${formatPercent(precisionDiff)}`);
    console.log(`    Recall Diff:         ${recallDiff >= 0 ? '+' : ''}${formatPercent(recallDiff)}`);
    console.log(`    Latency Ratio:       ${latencyRatio.toFixed(2)}x`);
    console.log('');

    // Kill criteria check
    console.log('  KILL CRITERIA CHECK:');

    // 1. Single agent F1 >= Collective F1
    if (allMetrics.single.f1 >= allMetrics.collective.f1) {
      console.log('    ❌ FAIL: Single F1 >= Collective F1 (no quality gain)');
    } else {
      console.log('    ✅ PASS: Collective F1 > Single F1');
    }

    // 2. Latency > 10x with F1 diff < 0.1
    if (latencyRatio > 10 && Math.abs(f1Diff) < 0.1) {
      console.log('    ❌ FAIL: Latency > 10x with marginal F1 gain');
    } else {
      console.log('    ✅ PASS: Acceptable latency/quality tradeoff');
    }

    // 3. Collective precision < Single precision
    if (allMetrics.collective.precision < allMetrics.single.precision) {
      console.log('    ⚠️  WARN: Collective precision < Single (more false positives)');
    } else {
      console.log('    ✅ PASS: Collective precision >= Single');
    }

    console.log('');

    // Final verdict
    const passed = allMetrics.collective.f1 > allMetrics.single.f1 &&
                   !(latencyRatio > 10 && Math.abs(f1Diff) < 0.1);

    if (passed) {
      console.log('  ════════════════════════════════════════════════════════════');
      console.log('  ✅ COLLECTIVE VALIDATED: Measurable improvement over baseline');
      console.log('  ════════════════════════════════════════════════════════════');
    } else {
      console.log('  ════════════════════════════════════════════════════════════');
      console.log('  ❌ COLLECTIVE NOT VALIDATED: No clear advantage over single');
      console.log('  ════════════════════════════════════════════════════════════');
    }
    console.log('');
  }

  // Save evaluation
  const outputPath = `benchmarks/collective-vs-single/results/evaluation-${Date.now()}.json`;
  writeFileSync(outputPath, JSON.stringify({
    timestamp: new Date().toISOString(),
    metrics: allMetrics,
    groundTruthSamples: dataset.samples.length
  }, null, 2));

  console.log(`  Full evaluation saved to: ${outputPath}`);
  console.log('');
}

main();
