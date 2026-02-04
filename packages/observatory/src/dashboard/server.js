/**
 * Observatory Dashboard Server
 *
 * Web server for real-time CYNIC observability.
 * "Observer pour comprendre, comprendre pour améliorer"
 *
 * @module @cynic/observatory/dashboard/server
 */

'use strict';

import http from 'http';
import { URL } from 'url';
import { getPool } from '@cynic/persistence';
import { PHI_INV } from '@cynic/core';

import { QLearningQueries } from '../queries/qlearning.js';
import { PatternsQueries } from '../queries/patterns.js';
import { TelemetryQueries } from '../queries/telemetry.js';
import { LearningProofQueries } from '../queries/learning-proof.js';
import { TokenFetcher } from '../oracle/token-fetcher.js';
import { TokenScorer } from '../oracle/scorer.js';
import { OracleMemory } from '../oracle/memory.js';
import { OracleWatchlist } from '../oracle/watchlist.js';

// ═══════════════════════════════════════════════════════════════════════════
// CORS & RESPONSE HELPERS
// ═══════════════════════════════════════════════════════════════════════════

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET, POST, DELETE, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type',
};

function jsonResponse(res, data, status = 200) {
  res.writeHead(status, {
    ...CORS_HEADERS,
    'Content-Type': 'application/json',
  });
  res.end(JSON.stringify(data, null, 2));
}

function errorResponse(res, message, status = 500) {
  jsonResponse(res, { error: message, timestamp: new Date().toISOString() }, status);
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = '';
    req.on('data', chunk => { data += chunk; if (data.length > 1e5) reject(new Error('Body too large')); });
    req.on('end', () => { try { resolve(data ? JSON.parse(data) : {}); } catch { reject(new Error('Invalid JSON')); } });
    req.on('error', reject);
  });
}

// ═══════════════════════════════════════════════════════════════════════════
// DASHBOARD HTML
// ═══════════════════════════════════════════════════════════════════════════

const DASHBOARD_HTML = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CYNIC Observatory</title>
  <style>
    :root {
      --bg: #0d1117;
      --card: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --text-dim: #8b949e;
      --green: #3fb950;
      --yellow: #d29922;
      --red: #f85149;
      --blue: #58a6ff;
      --phi: 0.618;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      padding: 20px;
      line-height: 1.5;
    }
    .header {
      display: flex;
      align-items: center;
      gap: 15px;
      margin-bottom: 30px;
      border-bottom: 1px solid var(--border);
      padding-bottom: 20px;
    }
    .header h1 { font-size: 1.5rem; }
    .header .status { padding: 4px 12px; border-radius: 20px; font-size: 0.8rem; }
    .status.healthy { background: var(--green); color: #000; }
    .status.degraded { background: var(--yellow); color: #000; }
    .status.critical { background: var(--red); color: #fff; }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
      gap: 20px;
    }
    .card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 20px;
    }
    .card h2 {
      font-size: 1rem;
      color: var(--text-dim);
      margin-bottom: 15px;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .metric {
      display: flex;
      justify-content: space-between;
      padding: 8px 0;
      border-bottom: 1px solid var(--border);
    }
    .metric:last-child { border-bottom: none; }
    .metric .label { color: var(--text-dim); }
    .metric .value { font-weight: 600; font-family: monospace; }
    .metric .value.good { color: var(--green); }
    .metric .value.warn { color: var(--yellow); }
    .metric .value.bad { color: var(--red); }
    .proof-card { grid-column: span 2; }
    .proof-item {
      display: flex;
      align-items: center;
      gap: 15px;
      padding: 12px;
      background: var(--bg);
      border-radius: 6px;
      margin-bottom: 10px;
    }
    .proof-score {
      width: 50px;
      height: 50px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: bold;
      font-size: 1.1rem;
    }
    .score-good { background: var(--green); color: #000; }
    .score-warn { background: var(--yellow); color: #000; }
    .score-bad { background: var(--red); color: #fff; }
    .score-none { background: var(--border); color: var(--text-dim); }
    .proof-details { flex: 1; }
    .proof-name { font-weight: 600; }
    .proof-interp { font-size: 0.85rem; color: var(--text-dim); }
    .verdict-banner {
      padding: 20px;
      border-radius: 8px;
      text-align: center;
      margin-bottom: 20px;
    }
    .verdict-banner.learning { background: rgba(63, 185, 80, 0.2); border: 2px solid var(--green); }
    .verdict-banner.stable { background: rgba(210, 153, 34, 0.2); border: 2px solid var(--yellow); }
    .verdict-banner.not_learning { background: rgba(248, 81, 73, 0.2); border: 2px solid var(--red); }
    .verdict-banner h3 { font-size: 1.3rem; margin-bottom: 8px; }
    .refresh-time { color: var(--text-dim); font-size: 0.8rem; margin-top: 20px; text-align: center; }
    .loading { text-align: center; padding: 40px; color: var(--text-dim); }
  </style>
</head>
<body>
  <div class="header">
    <h1>🐕 CYNIC Observatory</h1>
    <span id="status" class="status">Loading...</span>
  </div>

  <div id="content" class="loading">Loading observatory data...</div>

  <div id="refresh-time" class="refresh-time"></div>

  <script>
    const PHI_INV = 0.618;
    const PHI_INV_2 = 0.382;

    function scoreClass(score) {
      if (score === null) return 'score-none';
      if (score >= PHI_INV) return 'score-good';
      if (score >= PHI_INV_2) return 'score-warn';
      return 'score-bad';
    }

    function valueClass(value, goodThreshold, warnThreshold) {
      if (value >= goodThreshold) return 'good';
      if (value >= warnThreshold) return 'warn';
      return 'bad';
    }

    async function fetchData() {
      try {
        const [health, proof, qstats, patterns] = await Promise.all([
          fetch('/api/telemetry/health').then(r => r.json()),
          fetch('/api/learning/proof').then(r => r.json()),
          fetch('/api/qlearning/stats').then(r => r.json()),
          fetch('/api/patterns/important').then(r => r.json()),
        ]);

        renderDashboard({ health, proof, qstats, patterns });
      } catch (e) {
        document.getElementById('content').innerHTML = '<div class="loading">Error: ' + e.message + '</div>';
      }
    }

    function renderDashboard({ health, proof, qstats, patterns }) {
      // Status
      document.getElementById('status').textContent = health.status?.toUpperCase() || 'UNKNOWN';
      document.getElementById('status').className = 'status ' + (health.status || 'degraded');

      // Main content
      const verdictClass = proof.verdict || 'unknown';
      const html = \`
        <div class="verdict-banner \${verdictClass}">
          <h3>\${proof.interpretation || 'Loading...'}</h3>
          <div>Overall Score: \${proof.overallScore !== null ? (proof.overallScore * 100).toFixed(1) + '%' : 'N/A'}</div>
        </div>

        <div class="grid">
          <!-- Health Card -->
          <div class="card">
            <h2>📊 System Health</h2>
            <div class="metric">
              <span class="label">Success Rate</span>
              <span class="value \${valueClass(health.successRate, PHI_INV, PHI_INV_2)}">\${(health.successRate * 100).toFixed(1)}%</span>
            </div>
            <div class="metric">
              <span class="label">Tool Calls (1h)</span>
              <span class="value">\${health.toolCalls || 0}</span>
            </div>
            <div class="metric">
              <span class="label">Avg Latency</span>
              <span class="value">\${(health.avgLatencyMs || 0).toFixed(0)}ms</span>
            </div>
            <div class="metric">
              <span class="label">Frictions (1h)</span>
              <span class="value \${health.frictionCount > 10 ? 'bad' : health.frictionCount > 5 ? 'warn' : 'good'}">\${health.frictionCount || 0}</span>
            </div>
            <div class="metric">
              <span class="label">Active Sessions</span>
              <span class="value">\${health.activeSessions || 0}</span>
            </div>
          </div>

          <!-- Q-Learning Card -->
          <div class="card">
            <h2>🧠 Q-Learning State</h2>
            \${qstats.services?.length > 0 ? qstats.services.map(s => \`
              <div class="metric">
                <span class="label">\${s.service_id}</span>
                <span class="value">\${s.total_episodes || 0} episodes</span>
              </div>
              <div class="metric">
                <span class="label">Exploration Rate</span>
                <span class="value \${valueClass(1 - s.exploration_rate, 0.7, 0.3)}">\${(s.exploration_rate * 100).toFixed(1)}%</span>
              </div>
              <div class="metric">
                <span class="label">Q-Table Entries</span>
                <span class="value">\${s.entry_count || 0}</span>
              </div>
            \`).join('') : '<div class="metric"><span class="label">No Q-Learning data yet</span></div>'}
          </div>

          <!-- Patterns Card -->
          <div class="card">
            <h2>🔍 Pattern Memory (EWC++)</h2>
            <div class="metric">
              <span class="label">Locked Patterns</span>
              <span class="value \${valueClass(patterns.lockedCount, 5, 1)}">\${patterns.lockedCount || 0}</span>
            </div>
            <div class="metric">
              <span class="label">Important Patterns</span>
              <span class="value">\${patterns.importantCount || 0}</span>
            </div>
            <div class="metric">
              <span class="label">Total Patterns</span>
              <span class="value">\${patterns.patterns?.length || 0}</span>
            </div>
            <div class="metric">
              <span class="label">EWC Threshold</span>
              <span class="value">\${(patterns.ewcThreshold * 100).toFixed(1)}% (φ⁻¹)</span>
            </div>
          </div>

          <!-- Learning Proof Card -->
          <div class="card proof-card">
            <h2>📈 Learning Proof</h2>
            \${Object.values(proof.proofs || {}).map(p => \`
              <div class="proof-item">
                <div class="proof-score \${scoreClass(p.score)}">
                  \${p.score !== null ? (p.score * 100).toFixed(0) + '%' : '?'}
                </div>
                <div class="proof-details">
                  <div class="proof-name">\${p.name || 'Unknown'}</div>
                  <div class="proof-interp">\${p.interpretation || p.description || ''}</div>
                </div>
              </div>
            \`).join('')}
          </div>
        </div>
      \`;

      document.getElementById('content').innerHTML = html;
      document.getElementById('refresh-time').textContent = 'Last updated: ' + new Date().toLocaleString();
    }

    // Initial load
    fetchData();

    // Auto-refresh every 30s
    setInterval(fetchData, 30000);
  </script>
</body>
</html>`;

// ═══════════════════════════════════════════════════════════════════════════
// ORACLE HTML
// ═══════════════════════════════════════════════════════════════════════════

const ORACLE_HTML = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>CYNIC Oracle — Token Verdict</title>
  <style>
    :root {
      --bg: #0a0e14;
      --card: #131920;
      --card-hover: #1a2130;
      --border: #1e2a3a;
      --text: #d4dae4;
      --text-dim: #6b7d93;
      --gold: #c9a84c;
      --gold-dim: rgba(201,168,76,0.15);
      --green: #3fb950;
      --yellow: #d29922;
      --red: #f85149;
      --blue: #58a6ff;
      --phi: 0.618;
      --phi2: 0.382;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'SF Mono', 'Fira Code', 'JetBrains Mono', monospace;
      background: var(--bg);
      color: var(--text);
      min-height: 100vh;
      overflow-x: hidden;
    }

    /* Header */
    .header {
      text-align: center;
      padding: 40px 20px 30px;
      border-bottom: 1px solid var(--border);
    }
    .header h1 {
      font-size: 1.8rem;
      letter-spacing: 0.15em;
      color: var(--gold);
      margin-bottom: 8px;
    }
    .header .sub {
      color: var(--text-dim);
      font-size: 0.85rem;
      font-style: italic;
    }

    /* Input section */
    .input-section {
      max-width: 700px;
      margin: 30px auto;
      padding: 0 20px;
    }
    .input-row {
      display: flex;
      gap: 10px;
    }
    .mint-input {
      flex: 1;
      padding: 14px 18px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 8px;
      color: var(--text);
      font-family: inherit;
      font-size: 0.95rem;
      outline: none;
      transition: border-color 0.2s;
    }
    .mint-input:focus {
      border-color: var(--gold);
    }
    .mint-input::placeholder {
      color: var(--text-dim);
    }
    .judge-btn {
      padding: 14px 28px;
      background: var(--gold-dim);
      border: 1px solid var(--gold);
      border-radius: 8px;
      color: var(--gold);
      font-family: inherit;
      font-size: 0.95rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }
    .judge-btn:hover { background: rgba(201,168,76,0.25); }
    .judge-btn:disabled { opacity: 0.4; cursor: not-allowed; }
    .quick-links {
      margin-top: 12px;
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }
    .quick-link {
      padding: 4px 12px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 20px;
      color: var(--text-dim);
      font-size: 0.75rem;
      cursor: pointer;
      transition: all 0.2s;
    }
    .quick-link:hover { border-color: var(--gold); color: var(--gold); }

    /* Result */
    .result { max-width: 800px; margin: 0 auto; padding: 0 20px 60px; }
    .loading-msg {
      text-align: center;
      padding: 40px;
      color: var(--text-dim);
      font-size: 0.9rem;
    }
    .loading-msg .spinner {
      display: inline-block;
      width: 20px;
      height: 20px;
      border: 2px solid var(--border);
      border-top-color: var(--gold);
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-right: 10px;
      vertical-align: middle;
    }
    @keyframes spin { to { transform: rotate(360deg); } }

    .error-msg {
      text-align: center;
      padding: 30px;
      color: var(--red);
      background: rgba(248,81,73,0.08);
      border: 1px solid rgba(248,81,73,0.3);
      border-radius: 8px;
      margin-top: 20px;
    }

    /* Verdict banner */
    .verdict-card {
      text-align: center;
      padding: 30px;
      border-radius: 12px;
      margin-top: 20px;
      position: relative;
      overflow: hidden;
    }
    .verdict-card.HOWL { background: rgba(63,185,80,0.08); border: 2px solid var(--green); }
    .verdict-card.WAG { background: rgba(88,166,255,0.08); border: 2px solid var(--blue); }
    .verdict-card.GROWL { background: rgba(210,153,34,0.08); border: 2px solid var(--yellow); }
    .verdict-card.BARK { background: rgba(248,81,73,0.08); border: 2px solid var(--red); }

    .verdict-verdict {
      font-size: 2.5rem;
      font-weight: 800;
      letter-spacing: 0.2em;
      margin-bottom: 5px;
    }
    .HOWL .verdict-verdict { color: var(--green); }
    .WAG .verdict-verdict { color: var(--blue); }
    .GROWL .verdict-verdict { color: var(--yellow); }
    .BARK .verdict-verdict { color: var(--red); }

    .verdict-token {
      font-size: 1.1rem;
      color: var(--text-dim);
      margin-bottom: 15px;
    }
    .verdict-scores {
      display: flex;
      justify-content: center;
      gap: 30px;
      flex-wrap: wrap;
    }
    .score-block { text-align: center; }
    .score-value {
      font-size: 1.8rem;
      font-weight: 700;
    }
    .score-label {
      font-size: 0.7rem;
      color: var(--text-dim);
      text-transform: uppercase;
      letter-spacing: 0.1em;
    }

    /* Confidence gauge */
    .confidence-section {
      margin-top: 25px;
      padding: 20px;
      background: var(--card);
      border-radius: 8px;
      border: 1px solid var(--border);
    }
    .confidence-bar-bg {
      width: 100%;
      height: 8px;
      background: var(--border);
      border-radius: 4px;
      position: relative;
      margin-top: 10px;
    }
    .confidence-bar {
      height: 100%;
      border-radius: 4px;
      background: var(--gold);
      transition: width 0.6s ease;
    }
    .confidence-mark {
      position: absolute;
      right: 38.2%;
      top: -18px;
      font-size: 0.65rem;
      color: var(--text-dim);
    }
    .confidence-label {
      display: flex;
      justify-content: space-between;
      font-size: 0.8rem;
      color: var(--text-dim);
    }

    /* Dimensions grid */
    .dimensions-section {
      margin-top: 20px;
    }
    .section-title {
      font-size: 0.85rem;
      color: var(--gold);
      text-transform: uppercase;
      letter-spacing: 0.15em;
      margin-bottom: 15px;
      padding-bottom: 8px;
      border-bottom: 1px solid var(--border);
    }
    .axiom-group {
      margin-bottom: 20px;
    }
    .axiom-header {
      font-size: 0.8rem;
      color: var(--text-dim);
      margin-bottom: 8px;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .axiom-score {
      font-weight: 600;
      padding: 2px 8px;
      border-radius: 4px;
      font-size: 0.75rem;
    }
    .dim-row {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 6px 0;
    }
    .dim-name {
      width: 160px;
      font-size: 0.8rem;
      color: var(--text-dim);
      flex-shrink: 0;
    }
    .dim-bar-bg {
      flex: 1;
      height: 6px;
      background: var(--border);
      border-radius: 3px;
    }
    .dim-bar {
      height: 100%;
      border-radius: 3px;
      transition: width 0.4s ease;
    }
    .dim-val {
      width: 35px;
      text-align: right;
      font-size: 0.8rem;
      font-weight: 600;
    }

    /* Weaknesses */
    .weakness-item {
      padding: 10px 14px;
      background: rgba(248,81,73,0.05);
      border-left: 3px solid var(--red);
      margin-bottom: 8px;
      border-radius: 0 6px 6px 0;
      font-size: 0.85rem;
    }
    .weakness-dim {
      color: var(--yellow);
      font-weight: 600;
    }
    .weakness-reason { color: var(--text-dim); }

    /* Tier badge */
    .tier-badge {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 6px 16px;
      border-radius: 20px;
      background: var(--card);
      border: 1px solid var(--border);
      font-size: 0.85rem;
      margin-top: 10px;
    }

    /* THE_UNNAMEABLE */
    .unnameable {
      margin-top: 20px;
      padding: 15px;
      background: rgba(201,168,76,0.05);
      border: 1px dashed var(--gold);
      border-radius: 8px;
      text-align: center;
      font-size: 0.85rem;
      color: var(--text-dim);
    }
    .unnameable strong { color: var(--gold); }

    /* Footer */
    .footer {
      text-align: center;
      padding: 30px;
      color: var(--text-dim);
      font-size: 0.75rem;
      border-top: 1px solid var(--border);
      margin-top: 40px;
    }
    .footer a { color: var(--gold); text-decoration: none; }

    /* Trajectory */
    .trajectory {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 12px;
      margin-top: 12px;
      padding: 10px;
      background: var(--card);
      border-radius: 8px;
      font-size: 0.85rem;
    }
    .traj-arrow { font-size: 1.4rem; }
    .traj-arrow.improving { color: var(--green); }
    .traj-arrow.declining { color: var(--red); }
    .traj-arrow.stable { color: var(--text-dim); }
    .traj-arrow.new { color: var(--gold); }
    .traj-detail { color: var(--text-dim); }
    .traj-detail strong { color: var(--text); }

    /* History mini-list */
    .history-section { margin-top: 20px; }
    .history-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 8px 12px;
      border-bottom: 1px solid var(--border);
      font-size: 0.8rem;
    }
    .history-item:last-child { border-bottom: none; }
    .history-verdict { font-weight: 700; width: 55px; }

    /* Watchlist panel */
    .watchlist-panel {
      max-width: 800px;
      margin: 30px auto 0;
      padding: 0 20px;
    }
    .watchlist-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 10px;
    }
    .watchlist-header h3 { color: var(--gold); font-size: 0.9rem; text-transform: uppercase; letter-spacing: 0.1em; }
    .watch-btn {
      padding: 6px 14px;
      background: transparent;
      border: 1px solid var(--gold);
      border-radius: 6px;
      color: var(--gold);
      font-family: inherit;
      font-size: 0.8rem;
      cursor: pointer;
    }
    .watch-btn:hover { background: var(--gold-dim); }
    .watch-btn.active { background: var(--gold-dim); }
    .watchlist-items { }
    .wl-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 10px 14px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 6px;
      margin-bottom: 6px;
      cursor: pointer;
      transition: border-color 0.2s;
    }
    .wl-item:hover { border-color: var(--gold); }
    .wl-left { display: flex; align-items: center; gap: 10px; }
    .wl-label { font-weight: 600; font-size: 0.85rem; }
    .wl-mint { color: var(--text-dim); font-size: 0.75rem; }
    .wl-right { display: flex; align-items: center; gap: 12px; }
    .wl-score { font-weight: 700; font-size: 0.9rem; }
    .wl-verdict { font-size: 0.75rem; padding: 2px 8px; border-radius: 4px; font-weight: 600; }
    .wl-remove {
      color: var(--text-dim);
      cursor: pointer;
      font-size: 1.1rem;
      padding: 0 4px;
    }
    .wl-remove:hover { color: var(--red); }
    .wl-time { color: var(--text-dim); font-size: 0.7rem; }

    /* Alerts */
    .alert-item {
      padding: 8px 12px;
      background: rgba(210,153,34,0.08);
      border-left: 3px solid var(--yellow);
      margin-bottom: 6px;
      border-radius: 0 6px 6px 0;
      font-size: 0.8rem;
      color: var(--text-dim);
    }
    .alert-item strong { color: var(--text); }

    /* Mobile */
    @media (max-width: 600px) {
      .input-row { flex-direction: column; }
      .verdict-scores { gap: 15px; }
      .dim-name { width: 120px; font-size: 0.75rem; }
      .header h1 { font-size: 1.3rem; }
    }
  </style>
</head>
<body>
  <div class="header">
    <h1>CYNIC ORACLE</h1>
    <div class="sub">"\\u03C6 distrusts \\u03C6" — Agent v2.0 — judges, remembers, watches, alerts</div>
  </div>

  <div class="input-section">
    <div class="input-row">
      <input id="mint" class="mint-input" placeholder="Paste Solana mint address..." spellcheck="false" autocomplete="off" />
      <button id="judge-btn" class="judge-btn" onclick="judgeToken()">JUDGE</button>
    </div>
    <div class="quick-links">
      <span class="quick-link" onclick="setMint('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v')">USDC</span>
      <span class="quick-link" onclick="setMint('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB')">USDT</span>
      <span class="quick-link" onclick="setMint('So11111111111111111111111111111111')">SOL</span>
    </div>
  </div>

  <div id="result" class="result"></div>

  <div class="watchlist-panel">
    <div class="watchlist-header">
      <h3>\\uD83D\\uDC41 Watchlist</h3>
      <span id="wl-status" style="color:var(--text-dim);font-size:0.75rem;"></span>
    </div>
    <div id="watchlist-items" class="watchlist-items"></div>
    <div id="alerts-section" style="margin-top:15px;display:none;">
      <div style="font-size:0.8rem;color:var(--gold);margin-bottom:8px;text-transform:uppercase;letter-spacing:0.1em;">Recent Alerts</div>
      <div id="alerts-items"></div>
    </div>
  </div>

  <div class="footer">
    <a href="/api/oracle/health">API Health</a> &middot;
    <a href="/">Observatory</a> &middot;
    CYNIC Oracle v1.0 &middot;
    \\u03C6\\u207B\\u00B9 = 61.8% max confidence
  </div>

  <script>
    const PHI_INV = 0.618;
    const PHI_INV_2 = 0.382;
    const PHI_INV_3 = 0.236;

    function setMint(m) {
      document.getElementById('mint').value = m;
      judgeToken();
    }

    document.getElementById('mint').addEventListener('keydown', e => {
      if (e.key === 'Enter') judgeToken();
    });

    async function judgeToken() {
      const mint = document.getElementById('mint').value.trim();
      if (!mint) return;

      const btn = document.getElementById('judge-btn');
      const resultDiv = document.getElementById('result');

      btn.disabled = true;
      btn.textContent = 'JUDGING...';
      resultDiv.innerHTML = '<div class="loading-msg"><span class="spinner"></span>Fetching on-chain data & scoring 17 dimensions...</div>';

      try {
        const res = await fetch('/api/oracle/judge?mint=' + encodeURIComponent(mint));
        const data = await res.json();

        if (data.error) {
          resultDiv.innerHTML = '<div class="error-msg">' + escapeHtml(data.error) + '</div>';
          return;
        }

        currentMint = mint;
        renderVerdict(data);
        loadWatchlist();
      } catch (e) {
        resultDiv.innerHTML = '<div class="error-msg">Network error: ' + escapeHtml(e.message) + '</div>';
      } finally {
        btn.disabled = false;
        btn.textContent = 'JUDGE';
      }
    }

    function escapeHtml(s) {
      return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    }

    function dimColor(score) {
      if (score >= 61.8) return 'var(--green)';
      if (score >= 38.2) return 'var(--yellow)';
      return 'var(--red)';
    }

    function renderVerdict(d) {
      const axiomMap = {
        PHI: { label: '\\u03C6 HARMONY', dims: ['supplyDistribution','liquidityDepth','priceStability','supplyMechanics'] },
        VERIFY: { label: '\\u2713 VERIFY', dims: ['mintAuthority','freezeAuthority','metadataIntegrity','programVerification'] },
        CULTURE: { label: '\\u2B50 CULTURE', dims: ['holderCount','tokenAge','ecosystemIntegration','organicGrowth'] },
        BURN: { label: '\\uD83D\\uDD25 BURN', dims: ['burnActivity','creatorBehavior','feeRedistribution','realUtility'] },
      };

      const dimLabels = {
        supplyDistribution: 'Supply Distribution',
        liquidityDepth: 'Liquidity Depth',
        priceStability: 'Price Stability',
        supplyMechanics: 'Supply Mechanics',
        mintAuthority: 'Mint Authority',
        freezeAuthority: 'Freeze Authority',
        metadataIntegrity: 'Metadata Integrity',
        programVerification: 'Program Verification',
        holderCount: 'Holder Count',
        tokenAge: 'Token Age',
        ecosystemIntegration: 'Ecosystem Integration',
        organicGrowth: 'Organic Growth',
        burnActivity: 'Burn Activity',
        creatorBehavior: 'Creator Behavior',
        feeRedistribution: 'Fee Redistribution',
        realUtility: 'Real Utility',
      };

      let html = '';

      // Verdict banner
      html += '<div class="verdict-card ' + d.verdict + '">';
      html += '<div class="verdict-verdict">' + d.verdict + '</div>';
      html += '<div class="verdict-token">' + escapeHtml(d.name || 'Unknown') + ' (' + escapeHtml(d.symbol || '???') + ')</div>';
      html += '<div class="verdict-scores">';
      html += '<div class="score-block"><div class="score-value">' + d.qScore + '</div><div class="score-label">Q-Score</div></div>';
      html += '<div class="score-block"><div class="score-value">' + d.kScore + '</div><div class="score-label">K-Score</div></div>';
      html += '<div class="score-block"><div class="score-value">' + (d.confidence * 100).toFixed(1) + '%</div><div class="score-label">Confidence</div></div>';
      html += '</div>';
      html += '<div class="tier-badge">' + d.tierIcon + ' ' + d.tier + ' — ' + escapeHtml(d.tierDescription) + '</div>';
      html += '<div style="margin-top:12px"><button class="watch-btn" onclick="watchCurrentMint()">\\uD83D\\uDC41 Watch this token</button></div>';
      html += '</div>';

      // Trajectory
      if (d.trajectory) {
        html += renderTrajectory(d.trajectory);
      }

      // Confidence gauge
      const confPct = Math.min(100, (d.confidence / PHI_INV) * 100);
      html += '<div class="confidence-section">';
      html += '<div class="confidence-label"><span>Confidence</span><span>' + (d.confidence * 100).toFixed(1) + '% / 61.8% max</span></div>';
      html += '<div class="confidence-bar-bg">';
      html += '<div class="confidence-bar" style="width:' + (d.confidence * 100).toFixed(1) + '%;max-width:61.8%"></div>';
      html += '<div class="confidence-mark">\\u2502 \\u03C6\\u207B\\u00B9</div>';
      html += '</div>';
      html += '</div>';

      // Dimension scores by axiom
      html += '<div class="dimensions-section">';
      html += '<div class="section-title">17 Dimensions</div>';

      for (const [axiom, info] of Object.entries(axiomMap)) {
        const aScore = d.axiomScores[axiom];
        const aColor = dimColor(aScore);
        html += '<div class="axiom-group">';
        html += '<div class="axiom-header"><span>' + info.label + '</span>';
        html += '<span class="axiom-score" style="color:' + aColor + ';background:' + aColor + '20">' + aScore + '</span></div>';

        for (const dim of info.dims) {
          const score = d.dimensions[dim] || 0;
          const color = dimColor(score);
          html += '<div class="dim-row">';
          html += '<span class="dim-name">' + (dimLabels[dim] || dim) + '</span>';
          html += '<div class="dim-bar-bg"><div class="dim-bar" style="width:' + score + '%;background:' + color + '"></div></div>';
          html += '<span class="dim-val" style="color:' + color + '">' + score + '</span>';
          html += '</div>';
        }
        html += '</div>';
      }
      html += '</div>';

      // THE_UNNAMEABLE
      html += '<div class="unnameable">';
      html += '<strong>THE UNNAMEABLE</strong>: ' + d.theUnnameable + '% — ';
      html += d.theUnnameable > 50 ? 'High uncertainty. Much we cannot measure.' :
              d.theUnnameable > 25 ? 'Moderate gaps in on-chain data.' :
              'Good data coverage. Judgment has grounding.';
      html += '</div>';

      // Weaknesses
      if (d.weaknesses && d.weaknesses.length > 0) {
        html += '<div class="dimensions-section">';
        html += '<div class="section-title">Weaknesses (below \\u03C6\\u207B\\u00B2 threshold)</div>';
        for (const w of d.weaknesses) {
          html += '<div class="weakness-item">';
          html += '<span class="weakness-dim">[' + w.axiom + '] ' + (dimLabels[w.dimension] || w.dimension) + ': ' + w.score + '</span><br>';
          html += '<span class="weakness-reason">' + escapeHtml(w.reason) + '</span>';
          html += '</div>';
        }
        html += '</div>';
      }

      // Raw data
      html += '<div class="dimensions-section">';
      html += '<div class="section-title">On-Chain Data</div>';
      html += '<div style="font-size:0.8rem;color:var(--text-dim)">';
      html += 'Source: ' + (d._raw?.source || 'unknown') + ' &middot; ';
      html += 'Latency: ' + (d._raw?.latencyMs || '?') + 'ms &middot; ';
      html += 'Supply: ' + (d.supply?.total?.toLocaleString() || '?') + ' &middot; ';
      html += 'Holders (est): ' + (d.distribution?.holderCount || '?') + ' &middot; ';
      html += 'Whale %: ' + ((d.distribution?.whaleConcentration || 0) * 100).toFixed(1) + '% &middot; ';
      html += 'Gini: ' + (d.distribution?.giniCoefficient?.toFixed(3) || '?');
      html += '</div></div>';

      document.getElementById('result').innerHTML = html;
    }

    function renderTrajectory(t) {
      if (!t || t.direction === 'new' && t.previousJudgments <= 1) return '';
      const arrows = { improving: '\\u2197', declining: '\\u2198', stable: '\\u2192', new: '\\u2728' };
      const labels = { improving: 'Improving', declining: 'Declining', stable: 'Stable', new: 'First judgment' };
      let html = '<div class="trajectory">';
      html += '<span class="traj-arrow ' + t.direction + '">' + (arrows[t.direction] || '') + '</span>';
      html += '<span class="traj-detail"><strong>' + labels[t.direction] + '</strong>';
      if (t.delta) html += ' (Q ' + (t.delta > 0 ? '+' : '') + t.delta + ')';
      html += '</span>';
      if (t.previousJudgments > 1) html += '<span class="traj-detail">' + t.previousJudgments + ' past judgments</span>';
      if (t.averageQScore) html += '<span class="traj-detail">avg Q: ' + t.averageQScore + '</span>';
      html += '</div>';
      return html;
    }

    // ─── Watchlist ───

    let currentMint = null;

    async function loadWatchlist() {
      try {
        const res = await fetch('/api/oracle/watchlist');
        if (!res.ok) { document.getElementById('wl-status').textContent = 'Memory offline'; return; }
        const data = await res.json();
        renderWatchlist(data.watchlist || [], data.recentAlerts || []);
      } catch(e) {
        document.getElementById('wl-status').textContent = 'Agent offline';
      }
    }

    function renderWatchlist(items, alerts) {
      const container = document.getElementById('watchlist-items');
      if (items.length === 0) {
        container.innerHTML = '<div style="color:var(--text-dim);font-size:0.8rem;padding:10px;">No tokens watched. Judge a token then click "Watch" to monitor it.</div>';
      } else {
        container.innerHTML = items.map(w => {
          const vColor = w.lastVerdict === 'HOWL' ? 'var(--green)' : w.lastVerdict === 'WAG' ? 'var(--blue)' : w.lastVerdict === 'GROWL' ? 'var(--yellow)' : 'var(--red)';
          const ago = w.lastCheckedAt ? timeAgo(new Date(w.lastCheckedAt)) : 'never';
          return '<div class="wl-item" onclick="setMint(\\'' + w.mint + '\\')">' +
            '<div class="wl-left">' +
              '<span class="wl-label">' + escapeHtml(w.label || w.mint.slice(0,6) + '..') + '</span>' +
              '<span class="wl-mint">' + w.mint.slice(0,8) + '..' + w.mint.slice(-4) + '</span>' +
            '</div>' +
            '<div class="wl-right">' +
              (w.lastQScore ? '<span class="wl-score" style="color:' + vColor + '">Q:' + w.lastQScore + '</span>' : '') +
              (w.lastVerdict ? '<span class="wl-verdict" style="color:' + vColor + ';border:1px solid ' + vColor + '">' + w.lastVerdict + '</span>' : '') +
              '<span class="wl-time">' + ago + '</span>' +
              '<span class="wl-remove" onclick="event.stopPropagation();unwatchMint(\\'' + w.mint + '\\')" title="Remove">&times;</span>' +
            '</div>' +
          '</div>';
        }).join('');
      }
      document.getElementById('wl-status').textContent = items.length + ' watched';

      // Alerts
      const alertsDiv = document.getElementById('alerts-items');
      const alertsSection = document.getElementById('alerts-section');
      if (alerts.length > 0) {
        alertsSection.style.display = 'block';
        alertsDiv.innerHTML = alerts.slice(0,5).map(a =>
          '<div class="alert-item"><strong>' + escapeHtml(a.label || a.mint.slice(0,8)) + '</strong>: ' + escapeHtml(a.message || a.alertType) + ' <span style="float:right">' + timeAgo(new Date(a.createdAt)) + '</span></div>'
        ).join('');
      } else {
        alertsSection.style.display = 'none';
      }
    }

    async function watchCurrentMint() {
      if (!currentMint) return;
      const label = document.querySelector('.verdict-token')?.textContent?.split('(')[0]?.trim() || null;
      await fetch('/api/oracle/watch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ mint: currentMint, label })
      });
      loadWatchlist();
    }

    async function unwatchMint(mint) {
      await fetch('/api/oracle/watch?mint=' + encodeURIComponent(mint), { method: 'DELETE' });
      loadWatchlist();
    }

    function timeAgo(date) {
      const s = Math.floor((Date.now() - date.getTime()) / 1000);
      if (s < 60) return s + 's ago';
      if (s < 3600) return Math.floor(s/60) + 'm ago';
      if (s < 86400) return Math.floor(s/3600) + 'h ago';
      return Math.floor(s/86400) + 'd ago';
    }

    // Load watchlist on page load
    loadWatchlist();
    setInterval(loadWatchlist, 60000);
  </script>
</body>
</html>`;

// ═══════════════════════════════════════════════════════════════════════════
// SERVER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create observatory server
 * @param {Object} [options] - Server options
 * @param {number} [options.port=3847] - Port (φ * 6220 ≈ 3847)
 * @returns {http.Server}
 */
export function createServer(options = {}) {
  const port = options.port || parseInt(process.env.PORT || process.env.OBSERVATORY_PORT || '3847', 10);

  let pool = null;
  let dbInitialized = false;
  let qlearning = null;
  let patterns = null;
  let telemetry = null;
  let learningProof = null;
  let tokenFetcher = null;
  let tokenScorer = null;
  let oracleMemory = null;
  let oracleWatchlist = null;

  // Simple rate limiter for oracle (60 req/min per IP)
  const oracleRateLimit = new Map();
  function checkOracleRateLimit(ip) {
    const now = Date.now();
    const entry = oracleRateLimit.get(ip);
    if (!entry || now - entry.windowStart > 60000) {
      oracleRateLimit.set(ip, { windowStart: now, count: 1 });
      return true;
    }
    entry.count++;
    return entry.count <= 60;
  }

  // Initialize Oracle (no database needed)
  function ensureOracleInitialized() {
    if (!tokenFetcher) {
      tokenFetcher = new TokenFetcher(process.env.HELIUS_API_KEY);
      tokenScorer = new TokenScorer();
    }
  }

  // Initialize database-dependent services (may fail gracefully)
  async function ensureDbInitialized() {
    if (!dbInitialized) {
      dbInitialized = true;
      try {
        pool = getPool();
        qlearning = new QLearningQueries(pool);
        patterns = new PatternsQueries(pool);
        telemetry = new TelemetryQueries(pool);
        learningProof = new LearningProofQueries(pool);

        // Wire Oracle memory + watchlist (agent capabilities)
        ensureOracleInitialized();
        oracleMemory = new OracleMemory(pool);
        oracleWatchlist = new OracleWatchlist(pool, oracleMemory, tokenFetcher, tokenScorer);
        await oracleMemory.ensureSchema();
        await oracleWatchlist.ensureSchema();
        oracleWatchlist.startMonitoring();
        console.log('[Observatory] Oracle agent capabilities initialized (memory + watchlist)');
      } catch (e) {
        console.warn('[Observatory] Database not available:', e.message);
      }
    }
  }

  const server = http.createServer(async (req, res) => {
    // Handle CORS preflight
    if (req.method === 'OPTIONS') {
      res.writeHead(204, CORS_HEADERS);
      res.end();
      return;
    }

    const url = new URL(req.url, `http://localhost:${port}`);
    const path = url.pathname;

    // Dashboard HTML
    if (path === '/' || path === '/dashboard') {
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(DASHBOARD_HTML);
      return;
    }

    // API routes
    try {
      ensureOracleInitialized();
      await ensureDbInitialized();

      // Database-dependent endpoints (require pool)
      const needsDb = path.startsWith('/api/qlearning') || path.startsWith('/api/patterns') ||
                       path.startsWith('/api/telemetry') || path.startsWith('/api/learning');
      if (needsDb && !pool) {
        return errorResponse(res, 'Database not configured. Oracle endpoints available at /api/oracle/*', 503);
      }

      // Q-Learning endpoints
      if (path === '/api/qlearning/stats') {
        const data = await qlearning.getQTableStats();
        return jsonResponse(res, data);
      }
      if (path === '/api/qlearning/episodes') {
        const limit = parseInt(url.searchParams.get('limit') || '100', 10);
        const data = await qlearning.getEpisodeHistory({ limit });
        return jsonResponse(res, data);
      }
      if (path === '/api/qlearning/curve') {
        const interval = url.searchParams.get('interval') || 'hour';
        const data = await qlearning.getLearningCurve({ interval });
        return jsonResponse(res, data);
      }
      if (path === '/api/qlearning/heatmap') {
        const serviceId = url.searchParams.get('serviceId');
        const data = await qlearning.getQValuesHeatmap(serviceId);
        return jsonResponse(res, data);
      }
      if (path === '/api/qlearning/top-actions') {
        const data = await qlearning.getTopActions();
        return jsonResponse(res, data);
      }

      // Patterns endpoints
      if (path === '/api/patterns/recent') {
        const limit = parseInt(url.searchParams.get('limit') || '50', 10);
        const type = url.searchParams.get('type');
        const data = await patterns.getRecentPatterns({ limit, type });
        return jsonResponse(res, data);
      }
      if (path === '/api/patterns/important') {
        const data = await patterns.getImportantPatterns();
        return jsonResponse(res, data);
      }
      if (path === '/api/patterns/distribution') {
        const data = await patterns.getPatternDistribution();
        return jsonResponse(res, data);
      }
      if (path === '/api/patterns/timeline') {
        const interval = url.searchParams.get('interval') || 'hour';
        const data = await patterns.getPatternTimeline({ interval });
        return jsonResponse(res, data);
      }
      if (path === '/api/patterns/anomalies') {
        const data = await patterns.getAnomalies();
        return jsonResponse(res, data);
      }

      // Telemetry endpoints
      if (path === '/api/telemetry/health') {
        const data = await telemetry.getHealthMetrics();
        return jsonResponse(res, data);
      }
      if (path === '/api/telemetry/tools') {
        const interval = url.searchParams.get('interval') || 'hour';
        const data = await telemetry.getToolUsage({ interval });
        return jsonResponse(res, data);
      }
      if (path === '/api/telemetry/frictions') {
        const severity = url.searchParams.get('severity');
        const data = await telemetry.getFrictions({ severity });
        return jsonResponse(res, data);
      }
      if (path === '/api/telemetry/hotspots') {
        const data = await telemetry.getFrictionHotspots();
        return jsonResponse(res, data);
      }
      if (path === '/api/telemetry/sessions') {
        const data = await telemetry.getSessionMetrics();
        return jsonResponse(res, data);
      }
      if (path === '/api/telemetry/llm') {
        const interval = url.searchParams.get('interval') || 'hour';
        const data = await telemetry.getLLMMetrics(interval);
        return jsonResponse(res, data);
      }

      // Learning proof endpoints
      if (path === '/api/learning/proof') {
        const data = await learningProof.getLearningProof();
        return jsonResponse(res, data);
      }
      if (path === '/api/learning/timeline') {
        const days = parseInt(url.searchParams.get('days') || '30', 10);
        const data = await learningProof.getLearningTimeline(days);
        return jsonResponse(res, data);
      }

      // ═══════════════════════════════════════════════════════════════════
      // Oracle endpoints — Token Verdict API
      // ═══════════════════════════════════════════════════════════════════

      if (path === '/api/oracle/judge') {
        const clientIp = req.headers['x-forwarded-for'] || req.socket.remoteAddress || 'unknown';
        if (!checkOracleRateLimit(clientIp)) {
          return errorResponse(res, 'Rate limit exceeded (60 req/min)', 429);
        }

        const mint = url.searchParams.get('mint');
        if (!mint) {
          return errorResponse(res, 'mint parameter required. Usage: /api/oracle/judge?mint=<SOLANA_MINT_ADDRESS>', 400);
        }

        try {
          const tokenData = await tokenFetcher.getTokenData(mint);
          const verdict = tokenScorer.score(tokenData);

          const response = {
            ...verdict,
            _raw: tokenData._raw,
            supply: tokenData.supply,
            distribution: {
              holderCount: tokenData.distribution.holderCount,
              whaleConcentration: tokenData.distribution.whaleConcentration,
              giniCoefficient: tokenData.distribution.giniCoefficient,
            },
          };

          // Agent: store judgment + attach trajectory
          if (oracleMemory) {
            await oracleMemory.store({ mint, name: tokenData.name, symbol: tokenData.symbol, ...verdict });
            const trajectory = await oracleMemory.getTrajectory(mint);
            response.trajectory = {
              direction: trajectory.direction,
              delta: trajectory.delta,
              trend: trajectory.trend,
              previousJudgments: trajectory.previousJudgments,
              verdictChanged: trajectory.verdictChanged,
              averageQScore: trajectory.averageQScore,
            };
          }

          return jsonResponse(res, response);
        } catch (e) {
          return errorResponse(res, `Oracle error: ${e.message}`, 422);
        }
      }

      // ═══════════════════════════════════════════════════════════════════
      // Oracle Agent endpoints — Memory + Watchlist
      // ═══════════════════════════════════════════════════════════════════

      if (path === '/api/oracle/history') {
        const mint = url.searchParams.get('mint');
        if (!mint) return errorResponse(res, 'mint parameter required', 400);
        if (!oracleMemory) return errorResponse(res, 'Oracle memory not available (no database)', 503);
        const trajectory = await oracleMemory.getTrajectory(mint);
        return jsonResponse(res, trajectory);
      }

      if (path === '/api/oracle/recent') {
        if (!oracleMemory) return errorResponse(res, 'Oracle memory not available', 503);
        const limit = parseInt(url.searchParams.get('limit') || '20', 10);
        const mints = await oracleMemory.getRecentMints(limit);
        return jsonResponse(res, { mints });
      }

      if (path === '/api/oracle/stats') {
        if (!oracleMemory) return errorResponse(res, 'Oracle memory not available', 503);
        const stats = await oracleMemory.getStats();
        return jsonResponse(res, stats);
      }

      if (path === '/api/oracle/watchlist' && req.method === 'GET') {
        if (!oracleWatchlist) return errorResponse(res, 'Watchlist not available (no database)', 503);
        const list = await oracleWatchlist.list();
        const alerts = await oracleWatchlist.getAlerts(20);
        return jsonResponse(res, { watchlist: list, recentAlerts: alerts });
      }

      if (path === '/api/oracle/watch' && req.method === 'POST') {
        if (!oracleWatchlist) return errorResponse(res, 'Watchlist not available (no database)', 503);
        const body = await readBody(req);
        if (!body.mint) return errorResponse(res, 'mint required in body', 400);
        await oracleWatchlist.add(body.mint, body.label || null);
        // Immediately judge and store
        try {
          const tokenData = await tokenFetcher.getTokenData(body.mint);
          const verdict = tokenScorer.score(tokenData);
          await oracleMemory.store({ mint: body.mint, name: tokenData.name, symbol: tokenData.symbol, ...verdict });
          await oracleWatchlist.pool.query(
            'UPDATE oracle_watchlist SET last_verdict = $2, last_q_score = $3, last_k_score = $4, last_checked_at = NOW() WHERE mint = $1',
            [body.mint, verdict.verdict, verdict.qScore, verdict.kScore]
          );
          return jsonResponse(res, { added: true, mint: body.mint, verdict: verdict.verdict, qScore: verdict.qScore });
        } catch (e) {
          // Added but couldn't judge yet
          return jsonResponse(res, { added: true, mint: body.mint, judgeError: e.message });
        }
      }

      if (path === '/api/oracle/watch' && req.method === 'DELETE') {
        if (!oracleWatchlist) return errorResponse(res, 'Watchlist not available (no database)', 503);
        const mint = url.searchParams.get('mint');
        if (!mint) return errorResponse(res, 'mint parameter required', 400);
        await oracleWatchlist.remove(mint);
        return jsonResponse(res, { removed: true, mint });
      }

      if (path === '/api/oracle/alerts') {
        if (!oracleWatchlist) return errorResponse(res, 'Watchlist not available (no database)', 503);
        const limit = parseInt(url.searchParams.get('limit') || '50', 10);
        const alerts = await oracleWatchlist.getAlerts(limit);
        return jsonResponse(res, { alerts });
      }

      if (path === '/api/oracle/check-now') {
        if (!oracleWatchlist) return errorResponse(res, 'Watchlist not available (no database)', 503);
        const result = await oracleWatchlist.checkAll();
        return jsonResponse(res, result);
      }

      if (path === '/api/oracle/health') {
        const stats = oracleMemory ? await oracleMemory.getStats() : null;
        const watchlist = oracleWatchlist ? await oracleWatchlist.list() : [];
        return jsonResponse(res, {
          status: 'operational',
          service: 'CYNIC Oracle',
          version: '2.0.0',
          type: 'agent',
          capabilities: ['judge', 'remember', 'watch', 'alert', 'trajectory'],
          rpcSource: process.env.HELIUS_API_KEY ? 'helius' : 'public_rpc',
          memory: stats ? { totalJudgments: stats.totalJudgments, uniqueTokens: stats.uniqueTokens } : 'unavailable',
          watchlist: { watching: watchlist.length },
          dimensions: 17,
          maxConfidence: '61.8% (φ⁻¹)',
          endpoints: {
            judge: '/api/oracle/judge?mint=<ADDRESS>',
            history: '/api/oracle/history?mint=<ADDRESS>',
            recent: '/api/oracle/recent',
            stats: '/api/oracle/stats',
            watchlist: 'GET /api/oracle/watchlist',
            watch: 'POST /api/oracle/watch {mint, label?}',
            unwatch: 'DELETE /api/oracle/watch?mint=<ADDRESS>',
            alerts: '/api/oracle/alerts',
            checkNow: '/api/oracle/check-now',
            health: '/api/oracle/health',
            ui: '/oracle',
          },
          timestamp: new Date().toISOString(),
        });
      }

      if (path === '/oracle') {
        res.writeHead(200, { ...CORS_HEADERS, 'Content-Type': 'text/html' });
        res.end(ORACLE_HTML);
        return;
      }

      // 404
      errorResponse(res, 'Not found', 404);

    } catch (e) {
      console.error('[Observatory] Error:', e.message);
      errorResponse(res, e.message);
    }
  });

  return server;
}

/**
 * Start observatory server
 */
export async function start() {
  const port = parseInt(process.env.OBSERVATORY_PORT || '3847', 10);
  const server = createServer({ port });

  server.listen(port, () => {
    console.log(`
═══════════════════════════════════════════════════════════
🐕 CYNIC Oracle Agent v2.0
═══════════════════════════════════════════════════════════

Dashboard:  http://localhost:${port}/
Oracle:     http://localhost:${port}/oracle

Agent Capabilities:
  JUDGE     /api/oracle/judge?mint=X      17-dimension verdict
  REMEMBER  /api/oracle/history?mint=X    Past judgments + trajectory
  WATCH     POST /api/oracle/watch        Add to watchlist
  ALERT     /api/oracle/alerts            Verdict change alerts
  STATS     /api/oracle/stats             Aggregate memory

Observatory:
  GET /api/qlearning/stats      Q-Table summary
  GET /api/patterns/important   EWC++ locked patterns
  GET /api/telemetry/health     System health
  GET /api/learning/proof       Learning proof

"φ distrusts φ" — max confidence 61.8%
═══════════════════════════════════════════════════════════
    `);
  });

  return server;
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  start();
}
