/**
 * Consciousness Metrics Persistence Service
 *
 * Persists CYNIC's consciousness-level metrics:
 * - Distance D (per-prompt consciousness measurement)
 * - Thermodynamic state (heat, work, temperature, efficiency)
 * - Consciousness transitions (awareness level, state changes)
 *
 * "Le chien mesure et se souvient de sa conscience" - CYNIC
 *
 * @module @cynic/persistence/services/consciousness-metrics
 */

'use strict';

const PHI_INV = 0.618033988749895;

/**
 * Persist a CYNIC Distance measurement.
 * Called from perceive.js hook after calculateCYNICDistance().
 *
 * @param {Object} pool - PostgreSQL connection pool
 * @param {string} sessionId - Current session identifier
 * @param {Object} d - Distance calculation result
 * @returns {Promise<boolean>}
 */
export async function persistDistance(pool, sessionId, d) {
  if (!pool || !d) return false;
  try {
    const b = d.breakdown || {};
    await pool.query(
      `INSERT INTO cynic_distance_log
        (session_id, distance, state,
         delta_perception, delta_judgment, delta_memory,
         delta_consensus, delta_economics, delta_phi, delta_residual,
         active_axioms, lead_dog, source)
      VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)`,
      [sessionId, Math.min(PHI_INV, d.distance || 0), d.level || 'dormant',
       b.perception || 0, b.judgment || 0, b.memory || 0,
       b.consensus || 0, b.economics || 0, b.phi || 0, b.residual || 0,
       d.activeAxioms || [], d.leadDog || null, d.source || 'local']
    );
    return true;
  } catch (err) { return false; }
}

/**
 * Persist a thermodynamic snapshot.
 * @param {Object} pool - PostgreSQL connection pool
 * @param {string} sessionId - Current session identifier
 * @param {Object} t - Thermodynamic state data
 * @returns {Promise<boolean>}
 */
export async function persistThermodynamics(pool, sessionId, t) {
  if (!pool || !t) return false;
  try {
    await pool.query(
      `INSERT INTO thermodynamic_snapshots
        (session_id, heat, work, temperature, efficiency, entropy)
      VALUES ($1,$2,$3,$4,$5,$6)`,
      [sessionId, t.heat || 0, t.work || 0, t.temperature || 37,
       Math.min(PHI_INV, t.efficiency || 0), t.entropy || 0]
    );
    return true;
  } catch (err) { return false; }
}

/**
 * Persist a consciousness state transition.
 * @param {Object} pool - PostgreSQL connection pool
 * @param {string} sessionId - Current session identifier
 * @param {Object} c - Consciousness state data
 * @returns {Promise<boolean>}
 */
export async function persistConsciousnessTransition(pool, sessionId, c) {
  if (!pool || !c) return false;
  try {
    await pool.query(
      `INSERT INTO consciousness_transitions
        (session_id, awareness_level, state, avg_confidence,
         pattern_count, prediction_accuracy)
      VALUES ($1,$2,$3,$4,$5,$6)`,
      [sessionId, c.awarenessLevel || 0, c.state || 'DORMANT',
       c.avgConfidence || 0, c.patternCount || 0, c.predictionAccuracy || 0]
    );
    return true;
  } catch (err) { return false; }
}

/**
 * Get recent distance measurements for trend analysis.
 * @param {Object} pool - PostgreSQL connection pool
 * @param {string} sessionId - Session to query (null for all)
 * @param {number} [limit=50] - Maximum records to return
 * @returns {Promise<Object[]>}
 */
export async function getDistanceTrend(pool, sessionId, limit = 50) {
  if (!pool) return [];
  try {
    const q = sessionId
      ? { text: 'SELECT * FROM cynic_distance_log WHERE session_id=$1 ORDER BY timestamp DESC LIMIT $2', values: [sessionId, limit] }
      : { text: 'SELECT * FROM cynic_distance_log ORDER BY timestamp DESC LIMIT $1', values: [limit] };
    return (await pool.query(q)).rows;
  } catch (err) { return []; }
}

/**
 * Get recent efficiency measurements for trend analysis.
 * @param {Object} pool - PostgreSQL connection pool
 * @param {string} sessionId - Session to query (null for all)
 * @param {number} [limit=50] - Maximum records to return
 * @returns {Promise<Object[]>}
 */
export async function getEfficiencyTrend(pool, sessionId, limit = 50) {
  if (!pool) return [];
  try {
    const q = sessionId
      ? { text: 'SELECT * FROM thermodynamic_snapshots WHERE session_id=$1 ORDER BY timestamp DESC LIMIT $2', values: [sessionId, limit] }
      : { text: 'SELECT * FROM thermodynamic_snapshots ORDER BY timestamp DESC LIMIT $1', values: [limit] };
    return (await pool.query(q)).rows;
  } catch (err) { return []; }
}

/**
 * Fire-and-forget distance persistence via direct PostgreSQL.
 * Used from hooks where we cannot wait for results.
 * @param {string} databaseUrl - DATABASE_URL connection string
 * @param {string} sessionId - Session identifier
 * @param {Object} distanceData - Distance data to persist
 */
export function persistDistanceFireAndForget(databaseUrl, sessionId, distanceData) {
  if (!databaseUrl || !distanceData) return;
  import('pg').then(({ default: pg }) => {
    const pool = new pg.Pool({
      connectionString: databaseUrl,
      ssl: databaseUrl.includes('render.com') ? { rejectUnauthorized: false } : undefined,
      max: 1, idleTimeoutMillis: 5000, connectionTimeoutMillis: 3000,
    });
    persistDistance(pool, sessionId, distanceData)
      .catch(() => {})
      .finally(() => pool.end().catch(() => {}));
  }).catch(() => {});
}
