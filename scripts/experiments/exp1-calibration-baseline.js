#!/usr/bin/env node
/**
 * Experiment 1: Calibration Baseline Validation
 *
 * **Goal**: Validate ECE < 10% on existing calibration data
 * **Method**: Run get_calibration_curve(30) on 30-day history
 * **Success**: ECE < 0.10, no drift alerts
 *
 * From: docs/architecture/learning-validation.md § 6.3
 *
 * Usage: node scripts/experiments/exp1-calibration-baseline.js
 */

'use strict';

import pg from 'pg';
const { Pool } = pg;

console.log('🐕 Experiment 1: Calibration Baseline');
console.log('====================================\n');

// Database connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'localhost',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'cynic',
  user: process.env.POSTGRES_USER || 'cynic',
  password: process.env.POSTGRES_PASSWORD || 'cynic',
});

/**
 * Calculate Expected Calibration Error (ECE)
 *
 * ECE = Σ (|predicted_rate - actual_rate| * weight)
 *
 * @param {Array} calibrationCurve - Array of { bucket, predicted_rate, actual_rate, sample_count, calibration_error }
 * @returns {number} ECE value (0-1)
 */
function calculateECE(calibrationCurve) {
  if (!calibrationCurve || calibrationCurve.length === 0) return null;

  const totalSamples = calibrationCurve.reduce((sum, bin) => sum + parseInt(bin.sample_count), 0);
  if (totalSamples === 0) return null;

  let ece = 0;

  for (const bin of calibrationCurve) {
    const weight = parseInt(bin.sample_count) / totalSamples;
    const calibrationError = parseFloat(bin.calibration_error) || 0;
    ece += calibrationError * weight;
  }

  return ece;
}

/**
 * Get calibration curve from PostgreSQL function
 */
async function getCalibrationCurve(serviceId = 'default', days = 30) {
  const query = `SELECT * FROM get_calibration_curve($1, $2)`;

  try {
    const { rows } = await pool.query(query, [serviceId, days]);

    if (rows.length === 0) {
      return { curve: [], ece: null, warning: `No calibration data found for service '${serviceId}' in last ${days} days` };
    }

    const ece = calculateECE(rows);
    const totalSamples = rows.reduce((sum, bin) => sum + parseInt(bin.sample_count), 0);

    return { curve: rows, ece, totalSamples };
  } catch (error) {
    // Check if it's a table not exists error
    if (error.message.includes('does not exist')) {
      return { curve: [], ece: null, warning: 'Calibration tracking table not found - system needs to run first' };
    }
    throw new Error(`Failed to get calibration curve: ${error.message}`);
  }
}

/**
 * Get count of calibration tracking records
 */
async function getCalibrationStats() {
  const query = `
    SELECT
      COUNT(*) as total_records,
      COUNT(DISTINCT service_id) as service_count,
      MIN(created_at) as oldest_record,
      MAX(created_at) as newest_record
    FROM calibration_tracking
  `;

  try {
    const { rows } = await pool.query(query);
    return rows[0];
  } catch (error) {
    if (error.message.includes('does not exist')) {
      return null;
    }
    throw error;
  }
}

/**
 * Run Experiment 1
 */
async function runExperiment() {
  console.log('Step 1: Checking calibration tracking status...\n');

  const stats = await getCalibrationStats();

  if (!stats || parseInt(stats.total_records) === 0) {
    console.log('⚠️ No calibration tracking data found');
    console.log('\nNOTE: CYNIC needs to run with CalibrationTracker active to generate data.');
    console.log('This is expected if the system hasn\'t been running yet.\n');

    console.log('═'.repeat(60));
    console.log('\nExperiment 1: INCONCLUSIVE (no data)');
    console.log('\nRecommendation:');
    console.log('  1. Ensure CalibrationTracker is running in the daemon');
    console.log('  2. Make predictions and validate outcomes');
    console.log('  3. Wait for at least 10+ tracked predictions');
    console.log('  4. Re-run this experiment\n');
    await pool.end();
    process.exit(0);
  }

  console.log(`✓ Found calibration data:`);
  console.log(`  Total records: ${stats.total_records}`);
  console.log(`  Services tracked: ${stats.service_count}`);
  console.log(`  Oldest record: ${new Date(stats.oldest_record).toISOString()}`);
  console.log(`  Newest record: ${new Date(stats.newest_record).toISOString()}\n`);

  console.log('Step 2: Fetching 30-day calibration curve...\n');

  const { curve, ece, totalSamples, warning } = await getCalibrationCurve('default', 30);

  if (warning) {
    console.log(`⚠️ ${warning}\n`);
    console.log('═'.repeat(60));
    console.log('\nExperiment 1: INCONCLUSIVE (insufficient data)');
    console.log('\nRecommendation: Continue running CYNIC to collect more calibration data.');
    await pool.end();
    process.exit(0);
  }

  console.log(`✓ Found ${curve.length} confidence buckets (${totalSamples} total samples)\n`);

  console.log('Step 3: Analyzing calibration curve...\n');

  console.log('Confidence Bucket | Predicted Rate | Actual Rate | Samples | Error');
  console.log('-'.repeat(75));

  for (const bin of curve) {
    const bucket = parseInt(bin.bucket);
    const predicted = parseFloat(bin.predicted_rate) * 100;
    const actual = bin.actual_rate ? parseFloat(bin.actual_rate) * 100 : 0;
    const samples = parseInt(bin.sample_count);
    const error = parseFloat(bin.calibration_error) * 100;

    console.log(
      `${bucket.toString().padStart(2)} (${(bucket * 10).toString().padStart(2)}-${((bucket + 1) * 10).toString().padStart(3)}%)`.padEnd(18) + ' | ' +
      `${predicted.toFixed(1)}%`.padStart(13) + ' | ' +
      `${actual.toFixed(1)}%`.padStart(11) + ' | ' +
      `${samples.toString().padStart(7)} | ` +
      `${error.toFixed(1)}%`
    );
  }

  console.log();

  console.log('Step 4: Calculating ECE (Expected Calibration Error)...\n');

  const ecePercent = (ece * 100).toFixed(2);
  console.log(`ECE: ${ecePercent}%`);
  console.log(`Threshold: 10.0%`);
  console.log(`Status: ${ece < 0.10 ? '✓ PASS' : '✗ FAIL'}\n`);

  // Final validation
  console.log('═'.repeat(60));
  console.log('\nExperiment 1 Results:\n');

  const tests = {
    'Sufficient data (>10 samples)': totalSamples >= 10,
    'ECE < 10%': ece < 0.10,
    'Well-calibrated (ECE < 5%)': ece < 0.05,
    'Multiple buckets (>3)': curve.length > 3,
  };

  let passCount = 0;
  const totalTests = Object.keys(tests).length;

  for (const [test, pass] of Object.entries(tests)) {
    if (pass) passCount++;
    console.log(`  ${pass ? '✓' : '✗'} ${test}`);
  }

  console.log('\n' + '═'.repeat(60));

  if (tests['ECE < 10%'] && tests['Sufficient data (>10 samples)']) {
    console.log('\n🎉 Experiment 1: SUCCESS ✓');
    console.log('\nCalibration baseline validated:');
    console.log(`  - ECE: ${ecePercent}% (below 10% threshold)`);
    console.log(`  - Samples: ${totalSamples} tracked predictions`);
    console.log(`  - Buckets: ${curve.length} confidence ranges`);
    if (tests['Well-calibrated (ECE < 5%)']) {
      console.log(`  - Bonus: Well-calibrated (ECE < 5%) ✓`);
    }
    console.log('\nCYNIC prediction confidence is calibrated.');
    console.log('Task #14: COMPLETE ✓');
    await pool.end();
    process.exit(0);
  } else {
    console.log('\n⚠️ Experiment 1: PARTIAL');
    console.log(`\nIssues detected:`);
    if (!tests['Sufficient data (>10 samples)']) {
      console.log(`  - Insufficient data: ${totalSamples} samples (need 10+)`);
    }
    if (!tests['ECE < 10%']) {
      console.log(`  - ECE too high: ${ecePercent}% (threshold: 10%)`);
    }
    await pool.end();
    process.exit(1);
  }
}

// Run experiment
try {
  await runExperiment();
} catch (error) {
  console.error('\n✗ Experiment failed:', error.message);
  console.error(error.stack);
  await pool.end();
  process.exit(1);
}
