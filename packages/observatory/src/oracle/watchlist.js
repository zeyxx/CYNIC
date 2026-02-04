/**
 * Oracle Watchlist — Autonomous token monitoring
 *
 * "An agent observes. A calculator waits."
 *
 * Maintains a watchlist of mints, re-judges them periodically,
 * detects changes, and tracks alerts.
 *
 * @module @cynic/observatory/oracle/watchlist
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

const INIT_SQL = `
CREATE TABLE IF NOT EXISTS oracle_watchlist (
  id SERIAL PRIMARY KEY,
  mint TEXT UNIQUE NOT NULL,
  label TEXT,
  last_verdict TEXT,
  last_q_score REAL,
  last_k_score REAL,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  last_checked_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS oracle_alerts (
  id SERIAL PRIMARY KEY,
  mint TEXT NOT NULL,
  alert_type TEXT NOT NULL,
  old_verdict TEXT,
  new_verdict TEXT,
  old_q_score REAL,
  new_q_score REAL,
  message TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oracle_watchlist_mint ON oracle_watchlist(mint);
CREATE INDEX IF NOT EXISTS idx_oracle_alerts_mint ON oracle_alerts(mint);
CREATE INDEX IF NOT EXISTS idx_oracle_alerts_created ON oracle_alerts(created_at DESC);
`;

// ═══════════════════════════════════════════════════════════════════════════
// WATCHLIST CLASS
// ═══════════════════════════════════════════════════════════════════════════

export class OracleWatchlist {
  constructor(pool, memory, fetcher, scorer) {
    this.pool = pool;
    this.memory = memory;
    this.fetcher = fetcher;
    this.scorer = scorer;
    this._initialized = false;
    this._interval = null;
  }

  async ensureSchema() {
    if (this._initialized) return;
    try {
      await this.pool.query(INIT_SQL);
      this._initialized = true;
    } catch (e) {
      console.warn('[OracleWatchlist] Schema init failed:', e.message);
    }
  }

  /**
   * Add a mint to the watchlist
   * @param {string} mint
   * @param {string} [label] - Human label (e.g. "SOL", "$asdfasdfa")
   */
  async add(mint, label = null) {
    await this.ensureSchema();
    await this.pool.query(
      `INSERT INTO oracle_watchlist (mint, label)
       VALUES ($1, $2)
       ON CONFLICT (mint) DO UPDATE SET label = COALESCE($2, oracle_watchlist.label)`,
      [mint, label]
    );
  }

  /**
   * Remove a mint from the watchlist
   * @param {string} mint
   */
  async remove(mint) {
    await this.ensureSchema();
    await this.pool.query('DELETE FROM oracle_watchlist WHERE mint = $1', [mint]);
  }

  /**
   * Get all watched mints
   * @returns {Array}
   */
  async list() {
    await this.ensureSchema();
    const { rows } = await this.pool.query(
      `SELECT mint, label, last_verdict, last_q_score, last_k_score, added_at, last_checked_at
       FROM oracle_watchlist
       ORDER BY added_at DESC`
    );
    return rows.map(r => ({
      mint: r.mint,
      label: r.label,
      lastVerdict: r.last_verdict,
      lastQScore: r.last_q_score,
      lastKScore: r.last_k_score,
      addedAt: r.added_at,
      lastCheckedAt: r.last_checked_at,
    }));
  }

  /**
   * Get recent alerts
   * @param {number} limit
   * @returns {Array}
   */
  async getAlerts(limit = 50) {
    await this.ensureSchema();
    const { rows } = await this.pool.query(
      `SELECT a.mint, a.alert_type, a.old_verdict, a.new_verdict,
              a.old_q_score, a.new_q_score, a.message, a.created_at,
              w.label
       FROM oracle_alerts a
       LEFT JOIN oracle_watchlist w ON w.mint = a.mint
       ORDER BY a.created_at DESC
       LIMIT $1`,
      [limit]
    );
    return rows.map(r => ({
      mint: r.mint,
      label: r.label,
      alertType: r.alert_type,
      oldVerdict: r.old_verdict,
      newVerdict: r.new_verdict,
      oldQScore: r.old_q_score,
      newQScore: r.new_q_score,
      message: r.message,
      createdAt: r.created_at,
    }));
  }

  /**
   * Check all watched mints — the agent's heartbeat
   * Re-judges each, stores result, generates alerts on changes.
   */
  async checkAll() {
    await this.ensureSchema();
    const { rows } = await this.pool.query('SELECT mint, label, last_verdict, last_q_score FROM oracle_watchlist');

    if (rows.length === 0) return { checked: 0, alerts: 0 };

    let alertCount = 0;
    console.log(`[OracleWatchlist] Checking ${rows.length} watched tokens...`);

    for (const row of rows) {
      try {
        const tokenData = await this.fetcher.getTokenData(row.mint);
        const verdict = this.scorer.score(tokenData);

        // Store judgment
        await this.memory.store({
          mint: row.mint,
          name: tokenData.name,
          symbol: tokenData.symbol,
          ...verdict,
        });

        // Detect changes
        if (row.last_verdict && row.last_verdict !== verdict.verdict) {
          await this._createAlert(row, verdict, 'verdict_change',
            `${row.label || row.mint.slice(0, 8)}: ${row.last_verdict} → ${verdict.verdict} (Q: ${row.last_q_score} → ${verdict.qScore})`
          );
          alertCount++;
        } else if (row.last_q_score !== null) {
          const delta = verdict.qScore - row.last_q_score;
          if (Math.abs(delta) >= 10) {
            const dir = delta > 0 ? 'improved' : 'declined';
            await this._createAlert(row, verdict, `score_${dir}`,
              `${row.label || row.mint.slice(0, 8)}: Q-Score ${dir} by ${Math.abs(delta).toFixed(0)} (${row.last_q_score} → ${verdict.qScore})`
            );
            alertCount++;
          }
        }

        // Update watchlist entry
        await this.pool.query(
          `UPDATE oracle_watchlist
           SET last_verdict = $2, last_q_score = $3, last_k_score = $4, last_checked_at = NOW()
           WHERE mint = $1`,
          [row.mint, verdict.verdict, verdict.qScore, verdict.kScore]
        );

        // Rate-limit: pause between checks to avoid hammering RPC
        await new Promise(r => setTimeout(r, 2000));
      } catch (e) {
        console.warn(`[OracleWatchlist] Failed to check ${row.mint.slice(0, 8)}:`, e.message);
      }
    }

    console.log(`[OracleWatchlist] Done. ${rows.length} checked, ${alertCount} alerts.`);
    return { checked: rows.length, alerts: alertCount };
  }

  async _createAlert(row, verdict, type, message) {
    try {
      await this.pool.query(
        `INSERT INTO oracle_alerts (mint, alert_type, old_verdict, new_verdict, old_q_score, new_q_score, message)
         VALUES ($1, $2, $3, $4, $5, $6, $7)`,
        [row.mint, type, row.last_verdict, verdict.verdict, row.last_q_score, verdict.qScore, message]
      );
    } catch (e) {
      console.warn('[OracleWatchlist] Alert creation failed:', e.message);
    }
  }

  /**
   * Start background monitoring
   * @param {number} intervalMs - Check interval (default: 1 hour)
   */
  startMonitoring(intervalMs = 3600000) {
    if (this._interval) return;
    console.log(`[OracleWatchlist] Background monitoring started (every ${intervalMs / 60000} min)`);
    // Initial check after 30s
    setTimeout(() => this.checkAll().catch(e => console.warn('[OracleWatchlist] Check error:', e.message)), 30000);
    // Then periodic
    this._interval = setInterval(() => {
      this.checkAll().catch(e => console.warn('[OracleWatchlist] Check error:', e.message));
    }, intervalMs);
  }

  stopMonitoring() {
    if (this._interval) {
      clearInterval(this._interval);
      this._interval = null;
    }
  }
}
