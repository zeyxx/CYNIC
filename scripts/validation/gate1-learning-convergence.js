#!/usr/bin/env node
/**
 * Gate 1: Learning Convergence Validation
 *
 * **Goal**: Validate all 11 learning loops @ >61.8% maturity
 * **Method**: Query loop maturity metrics from database
 * **Success**: All 11 loops stable @ >φ⁻¹ (61.8%) for 7 consecutive days
 *
 * From: docs/architecture/completion-criteria.md
 *
 * The 11 learning loops:
 * 1. Q-Learning (routing optimization)
 * 2. DPO (Direct Preference Optimization)
 * 3. Thompson Sampling (exploration/exploitation)
 * 4. Calibration (confidence adjustment)
 * 5. EWC++ (catastrophic forgetting prevention)
 * 6. SONA (novelty adaptation)
 * 7. Meta-Cognition (self-monitoring)
 * 8. Behavior Modifier (Dog personality tuning)
 * 9. Residual Detector (dimension discovery)
 * 10. Context Compressor (user expertise tracking)
 * 11. Performance Tracker (metrics aggregation)
 *
 * Usage: node scripts/validation/gate1-learning-convergence.js
 */

'use strict';

import pg from 'pg';
const { Pool } = pg;

const PHI_INV = 0.618; // φ⁻¹ = 61.8%

console.log('🐕 Gate 1: Learning Convergence Validation');
console.log('==========================================\n');

// Database connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'localhost',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'cynic',
  user: process.env.POSTGRES_USER || 'cynic',
  password: process.env.POSTGRES_PASSWORD || 'cynic',
});

/**
 * The 11 learning loops to check
 */
const LEARNING_LOOPS = [
  { id: 'q_learning', name: 'Q-Learning', service: 'qlearning' },
  { id: 'dpo', name: 'DPO', service: 'dpo' },
  { id: 'thompson', name: 'Thompson Sampling', service: 'thompson' },
  { id: 'calibration', name: 'Calibration', service: 'calibration' },
  { id: 'ewc', name: 'EWC++', service: 'ewc' },
  { id: 'sona', name: 'SONA', service: 'sona' },
  { id: 'metacognition', name: 'Meta-Cognition', service: 'metacognition' },
  { id: 'behavior', name: 'Behavior Modifier', service: 'behavior' },
  { id: 'residual', name: 'Residual Detector', service: 'residual' },
  { id: 'context', name: 'Context Compressor', service: 'context' },
  { id: 'performance', name: 'Performance Tracker', service: 'performance' },
];

/**
 * Calculate maturity for a learning loop
 *
 * Maturity = φ-weighted average of:
 * - dataCoverage (how much data collected)
 * - convergence (weight stability)
 * - accuracy (prediction quality)
 * - adaptability (response to drift)
 */
function calculateMaturity(metrics) {
  if (!metrics) return 0;

  const {
    dataCoverage = 0,
    convergence = 0,
    accuracy = 0,
    adaptability = 0,
  } = metrics;

  // φ-weighted geometric mean
  const maturity = Math.pow(
    dataCoverage * convergence * accuracy * adaptability,
    1 / 4
  );

  return Math.min(maturity, PHI_INV); // Cap at φ⁻¹
}

/**
 * Get loop maturity from database
 */
async function getLoopMaturity(loop) {
  try {
    // Try to get maturity from learning_metrics table
    const query = `
      SELECT
        service_id,
        data_coverage,
        convergence,
        accuracy,
        adaptability,
        maturity,
        last_updated
      FROM learning_metrics
      WHERE service_id = $1
      ORDER BY last_updated DESC
      LIMIT 1
    `;

    const { rows } = await pool.query(query, [loop.service]);

    if (rows.length === 0) {
      return {
        loop: loop.name,
        maturity: 0,
        status: 'NO_DATA',
        metrics: null,
      };
    }

    const metrics = rows[0];
    const maturity = metrics.maturity || calculateMaturity({
      dataCoverage: parseFloat(metrics.data_coverage) || 0,
      convergence: parseFloat(metrics.convergence) || 0,
      accuracy: parseFloat(metrics.accuracy) || 0,
      adaptability: parseFloat(metrics.adaptability) || 0,
    });

    return {
      loop: loop.name,
      maturity,
      status: maturity >= PHI_INV ? 'CONVERGED' : 'LEARNING',
      metrics: {
        dataCoverage: parseFloat(metrics.data_coverage) || 0,
        convergence: parseFloat(metrics.convergence) || 0,
        accuracy: parseFloat(metrics.accuracy) || 0,
        adaptability: parseFloat(metrics.adaptability) || 0,
      },
      lastUpdated: metrics.last_updated,
    };
  } catch (error) {
    if (error.message.includes('does not exist')) {
      return {
        loop: loop.name,
        maturity: 0,
        status: 'TABLE_MISSING',
        metrics: null,
      };
    }
    throw error;
  }
}

/**
 * Check if loop has been stable for 7 days
 */
async function checkStability(loop, days = 7) {
  try {
    const query = `
      SELECT
        maturity,
        last_updated
      FROM learning_metrics
      WHERE service_id = $1
        AND last_updated > NOW() - INTERVAL '${days} days'
      ORDER BY last_updated ASC
    `;

    const { rows } = await pool.query(query, [loop.service]);

    if (rows.length === 0) {
      return { stable: false, reason: 'insufficient_data' };
    }

    // Check if all samples are above threshold
    const allConverged = rows.every(r => parseFloat(r.maturity) >= PHI_INV);

    if (!allConverged) {
      return { stable: false, reason: 'below_threshold' };
    }

    // Check variance (should be low if stable)
    const maturities = rows.map(r => parseFloat(r.maturity));
    const mean = maturities.reduce((a, b) => a + b, 0) / maturities.length;
    const variance = maturities.reduce((sum, m) => sum + Math.pow(m - mean, 2), 0) / maturities.length;
    const stdDev = Math.sqrt(variance);

    if (stdDev > 0.05) {
      return { stable: false, reason: 'high_variance', stdDev };
    }

    return { stable: true, samples: rows.length, stdDev };
  } catch (error) {
    return { stable: false, reason: 'error', error: error.message };
  }
}

/**
 * Run Gate 1 validation
 */
async function runValidation() {
  console.log('Step 1: Checking database connectivity...\n');

  try {
    await pool.query('SELECT NOW()');
    console.log('✓ Database connected\n');
  } catch (error) {
    console.log('✗ Database not available');
    console.log(`  Error: ${error.message}\n`);
    console.log('═'.repeat(60));
    console.log('\nGate 1: BLOCKED (no database)');
    console.log('\nRecommendation: Start PostgreSQL and ensure tables exist.');
    await pool.end();
    process.exit(1);
  }

  console.log('Step 2: Checking learning loop maturity...\n');

  const results = [];

  for (const loop of LEARNING_LOOPS) {
    const result = await getLoopMaturity(loop);
    results.push(result);

    const statusSymbol = result.status === 'CONVERGED' ? '✓' :
                        result.status === 'LEARNING' ? '⚡' :
                        result.status === 'NO_DATA' ? '○' : '✗';

    console.log(`  ${statusSymbol} ${result.loop.padEnd(20)} ${(result.maturity * 100).toFixed(1)}% ${result.status}`);
  }

  console.log();

  console.log('Step 3: Checking 7-day stability...\n');

  const stabilityResults = [];

  for (const loop of LEARNING_LOOPS) {
    const stability = await checkStability(loop, 7);
    stabilityResults.push({ loop: loop.name, ...stability });

    if (stability.stable) {
      console.log(`  ✓ ${loop.name.padEnd(20)} STABLE (${stability.samples} samples, σ=${(stability.stdDev * 100).toFixed(2)}%)`);
    } else {
      console.log(`  ✗ ${loop.name.padEnd(20)} ${stability.reason}`);
    }
  }

  console.log();

  // Final validation
  console.log('═'.repeat(60));
  console.log('\nGate 1 Results:\n');

  const convergedCount = results.filter(r => r.status === 'CONVERGED').length;
  const stableCount = stabilityResults.filter(r => r.stable).length;
  const avgMaturity = results.reduce((sum, r) => sum + r.maturity, 0) / results.length;

  const tests = {
    'All 11 loops instrumented': results.every(r => r.status !== 'TABLE_MISSING'),
    'Data collected for all loops': results.every(r => r.status !== 'NO_DATA'),
    [`All loops converged (>61.8%)`]: convergedCount === 11,
    'All loops stable (7 days)': stableCount === 11,
    'Average maturity >61.8%': avgMaturity >= PHI_INV,
  };

  let passCount = 0;
  const totalTests = Object.keys(tests).length;

  for (const [test, pass] of Object.entries(tests)) {
    if (pass) passCount++;
    console.log(`  ${pass ? '✓' : '✗'} ${test}`);
  }

  console.log();
  console.log(`  Converged: ${convergedCount}/11 loops`);
  console.log(`  Stable: ${stableCount}/11 loops`);
  console.log(`  Average maturity: ${(avgMaturity * 100).toFixed(1)}%`);

  console.log('\n' + '═'.repeat(60));

  if (tests['All loops converged (>61.8%)'] && tests['All loops stable (7 days)']) {
    console.log('\n🎉 Gate 1: OPEN ✓');
    console.log('\nLearning convergence validated:');
    console.log(`  - All 11 loops @ >${(PHI_INV * 100).toFixed(1)}% maturity`);
    console.log(`  - Stable for 7+ consecutive days`);
    console.log(`  - Average maturity: ${(avgMaturity * 100).toFixed(1)}%`);
    console.log('\nCYNIC v1.0 learning criterion MET.');
    console.log('Ready for Gate 2 (Matrix Completion).');
    console.log('\nTask #20: COMPLETE ✓');
    await pool.end();
    process.exit(0);
  } else if (!tests['All 11 loops instrumented'] || !tests['Data collected for all loops']) {
    console.log('\n⚠️ Gate 1: BLOCKED (infrastructure incomplete)');
    console.log('\nMissing infrastructure:');
    if (!tests['All 11 loops instrumented']) {
      console.log('  - learning_metrics table not found');
    }
    if (!tests['Data collected for all loops']) {
      const noData = results.filter(r => r.status === 'NO_DATA').map(r => r.loop);
      console.log(`  - No data for: ${noData.join(', ')}`);
    }
    console.log('\nRecommendation: Run CYNIC daemon to collect loop metrics.');
    await pool.end();
    process.exit(0);
  } else {
    console.log('\n⚠️ Gate 1: IN PROGRESS');
    console.log(`\nCurrent state:`);
    console.log(`  - Converged: ${convergedCount}/11 loops`);
    console.log(`  - Stable: ${stableCount}/11 loops`);
    console.log(`  - Average: ${(avgMaturity * 100).toFixed(1)}% (target: 61.8%)`);
    console.log('\nRecommendation:');
    console.log(`  - Continue running CYNIC daemon`);
    console.log(`  - Estimated sessions needed: ${Math.max(0, Math.ceil((PHI_INV - avgMaturity) * 300))}`);
    console.log(`  - ETA to convergence: ${Math.ceil((PHI_INV - avgMaturity) * 500 / 100)} weeks (at ~100 sessions/week)`);
    await pool.end();
    process.exit(0);
  }
}

// Run validation
try {
  await runValidation();
} catch (error) {
  console.error('\n✗ Validation failed:', error.message);
  console.error(error.stack);
  await pool.end();
  process.exit(1);
}
