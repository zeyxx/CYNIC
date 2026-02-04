/**
 * Oracle Memory — Judgment persistence + trajectory
 *
 * "Culture is a moat" — An agent that forgets is just a calculator.
 *
 * Stores every judgment, computes trajectory over time.
 * Auto-creates tables on first use.
 *
 * @module @cynic/observatory/oracle/memory
 */

'use strict';

// ═══════════════════════════════════════════════════════════════════════════
// SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

const INIT_SQL = `
CREATE TABLE IF NOT EXISTS oracle_judgments (
  id SERIAL PRIMARY KEY,
  mint TEXT NOT NULL,
  name TEXT,
  symbol TEXT,
  verdict TEXT NOT NULL,
  q_score REAL NOT NULL,
  k_score REAL NOT NULL,
  confidence REAL NOT NULL,
  tier TEXT,
  dimensions JSONB,
  weaknesses JSONB,
  judged_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oracle_judgments_mint ON oracle_judgments(mint);
CREATE INDEX IF NOT EXISTS idx_oracle_judgments_judged_at ON oracle_judgments(judged_at DESC);
CREATE INDEX IF NOT EXISTS idx_oracle_judgments_mint_time ON oracle_judgments(mint, judged_at DESC);
`;

// ═══════════════════════════════════════════════════════════════════════════
// MEMORY CLASS
// ═══════════════════════════════════════════════════════════════════════════

export class OracleMemory {
  constructor(pool) {
    this.pool = pool;
    this._initialized = false;
  }

  async ensureSchema() {
    if (this._initialized) return;
    try {
      await this.pool.query(INIT_SQL);
      this._initialized = true;
    } catch (e) {
      console.warn('[OracleMemory] Schema init failed:', e.message);
    }
  }

  /**
   * Store a judgment
   * @param {Object} judgment - Full verdict response from scorer
   */
  async store(judgment) {
    await this.ensureSchema();
    try {
      await this.pool.query(
        `INSERT INTO oracle_judgments (mint, name, symbol, verdict, q_score, k_score, confidence, tier, dimensions, weaknesses)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
        [
          judgment.mint,
          judgment.name || null,
          judgment.symbol || null,
          judgment.verdict,
          judgment.qScore,
          judgment.kScore,
          judgment.confidence,
          judgment.tier || null,
          JSON.stringify(judgment.dimensions || {}),
          JSON.stringify(judgment.weaknesses || []),
        ]
      );
    } catch (e) {
      console.warn('[OracleMemory] Store failed:', e.message);
    }
  }

  /**
   * Get judgment history for a mint
   * @param {string} mint
   * @param {number} limit
   * @returns {Array} Past judgments, newest first
   */
  async getHistory(mint, limit = 20) {
    await this.ensureSchema();
    const { rows } = await this.pool.query(
      `SELECT verdict, q_score, k_score, confidence, tier, dimensions, weaknesses, judged_at
       FROM oracle_judgments
       WHERE mint = $1
       ORDER BY judged_at DESC
       LIMIT $2`,
      [mint, limit]
    );
    return rows.map(r => ({
      verdict: r.verdict,
      qScore: r.q_score,
      kScore: r.k_score,
      confidence: r.confidence,
      tier: r.tier,
      dimensions: r.dimensions,
      weaknesses: r.weaknesses,
      judgedAt: r.judged_at,
    }));
  }

  /**
   * Calculate trajectory for a mint
   * Compares current vs previous judgments to determine direction
   * @param {string} mint
   * @returns {Object} { direction, delta, history }
   */
  async getTrajectory(mint) {
    const history = await this.getHistory(mint, 10);

    if (history.length === 0) {
      return { direction: 'new', delta: 0, previousJudgments: 0, history: [] };
    }

    if (history.length === 1) {
      return {
        direction: 'new',
        delta: 0,
        previousJudgments: 1,
        lastVerdict: history[0].verdict,
        lastQScore: history[0].qScore,
        lastJudgedAt: history[0].judgedAt,
        history,
      };
    }

    // Compare most recent stored vs second most recent
    const latest = history[0];
    const previous = history[1];
    const delta = latest.qScore - previous.qScore;

    // Calculate average Q-Score across all history
    const avgQ = history.reduce((sum, h) => sum + h.qScore, 0) / history.length;

    // Trend: look at last 5 entries
    const recent = history.slice(0, Math.min(5, history.length));
    let trend = 0;
    for (let i = 0; i < recent.length - 1; i++) {
      trend += recent[i].qScore - recent[i + 1].qScore;
    }
    trend = recent.length > 1 ? trend / (recent.length - 1) : 0;

    let direction;
    if (Math.abs(delta) < 3) direction = 'stable';
    else if (delta > 0) direction = 'improving';
    else direction = 'declining';

    // Verdict changes
    const verdictChanged = latest.verdict !== previous.verdict;

    return {
      direction,
      delta: Math.round(delta * 10) / 10,
      trend: Math.round(trend * 10) / 10,
      averageQScore: Math.round(avgQ * 10) / 10,
      previousJudgments: history.length,
      verdictChanged,
      lastVerdict: latest.verdict,
      lastQScore: latest.qScore,
      lastJudgedAt: latest.judgedAt,
      previousVerdict: previous.verdict,
      previousQScore: previous.qScore,
      history,
    };
  }

  /**
   * Get recently judged unique mints
   * @param {number} limit
   * @returns {Array}
   */
  async getRecentMints(limit = 20) {
    await this.ensureSchema();
    const { rows } = await this.pool.query(
      `SELECT DISTINCT ON (mint) mint, name, symbol, verdict, q_score, k_score, tier, judged_at
       FROM oracle_judgments
       ORDER BY mint, judged_at DESC`,
      []
    );
    // Sort by judged_at desc after distinct
    rows.sort((a, b) => new Date(b.judged_at) - new Date(a.judged_at));
    return rows.slice(0, limit).map(r => ({
      mint: r.mint,
      name: r.name,
      symbol: r.symbol,
      verdict: r.verdict,
      qScore: r.q_score,
      kScore: r.k_score,
      tier: r.tier,
      judgedAt: r.judged_at,
    }));
  }

  /**
   * Get aggregate stats
   * @returns {Object}
   */
  async getStats() {
    await this.ensureSchema();
    const { rows } = await this.pool.query(
      `SELECT
         COUNT(*) as total_judgments,
         COUNT(DISTINCT mint) as unique_tokens,
         AVG(q_score) as avg_q_score,
         AVG(k_score) as avg_k_score,
         MIN(judged_at) as first_judgment,
         MAX(judged_at) as last_judgment
       FROM oracle_judgments`
    );
    const r = rows[0];
    return {
      totalJudgments: parseInt(r.total_judgments),
      uniqueTokens: parseInt(r.unique_tokens),
      avgQScore: r.avg_q_score ? Math.round(parseFloat(r.avg_q_score) * 10) / 10 : null,
      avgKScore: r.avg_k_score ? Math.round(parseFloat(r.avg_k_score) * 10) / 10 : null,
      firstJudgment: r.first_judgment,
      lastJudgment: r.last_judgment,
    };
  }
}
