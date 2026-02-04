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
  <title>CYNIC Oracle - Solana Token Verdict</title>
  <style>
    :root {
      --bg: #06080c;
      --bg2: #0c1018;
      --card: #111820;
      --card-alt: #161e28;
      --border: #1c2838;
      --border-light: #253345;
      --text: #e0e6ef;
      --text-dim: #6880a0;
      --text-muted: #3d506a;
      --gold: #d4a847;
      --gold-dim: rgba(212,168,71,0.12);
      --gold-glow: rgba(212,168,71,0.08);
      --green: #4ade80;
      --green-dim: rgba(74,222,128,0.10);
      --blue: #60a5fa;
      --blue-dim: rgba(96,165,250,0.10);
      --yellow: #facc15;
      --yellow-dim: rgba(250,204,21,0.10);
      --red: #f87171;
      --red-dim: rgba(248,113,113,0.10);
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: var(--bg);
      color: var(--text);
      min-height: 100vh;
      overflow-x: hidden;
    }

    .container { max-width: 860px; margin: 0 auto; padding: 0 20px; }

    /* Header */
    .header {
      text-align: center;
      padding: 48px 20px 36px;
      position: relative;
    }
    .header::after {
      content: '';
      position: absolute;
      bottom: 0;
      left: 50%;
      transform: translateX(-50%);
      width: 120px;
      height: 1px;
      background: linear-gradient(90deg, transparent, var(--gold), transparent);
    }
    .logo { font-size: 0.7rem; letter-spacing: 0.4em; color: var(--text-muted); text-transform: uppercase; margin-bottom: 12px; }
    .header h1 {
      font-size: 2rem;
      font-weight: 300;
      letter-spacing: 0.2em;
      color: var(--gold);
    }
    .header .sub {
      color: var(--text-dim);
      font-size: 0.8rem;
      margin-top: 10px;
      letter-spacing: 0.05em;
    }
    .header .sub em { font-style: normal; color: var(--gold); }

    /* Input */
    .input-section {
      padding: 32px 0 24px;
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
      border-radius: 10px;
      color: var(--text);
      font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
      font-size: 0.9rem;
      outline: none;
      transition: border-color 0.2s, box-shadow 0.2s;
    }
    .mint-input:focus {
      border-color: var(--gold);
      box-shadow: 0 0 0 3px var(--gold-dim);
    }
    .mint-input::placeholder { color: var(--text-muted); }
    .judge-btn {
      padding: 14px 32px;
      background: var(--gold);
      border: none;
      border-radius: 10px;
      color: #000;
      font-family: inherit;
      font-size: 0.85rem;
      font-weight: 700;
      letter-spacing: 0.12em;
      cursor: pointer;
      transition: all 0.2s;
      white-space: nowrap;
    }
    .judge-btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .judge-btn:active { transform: translateY(0); }
    .judge-btn:disabled { opacity: 0.3; cursor: not-allowed; transform: none; }
    .quick-links {
      margin-top: 14px;
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }
    .quick-link {
      padding: 5px 14px;
      background: transparent;
      border: 1px solid var(--border);
      border-radius: 20px;
      color: var(--text-dim);
      font-size: 0.75rem;
      cursor: pointer;
      transition: all 0.15s;
      font-family: 'SF Mono', monospace;
    }
    .quick-link:hover { border-color: var(--gold); color: var(--gold); background: var(--gold-dim); }

    /* Result */
    .result { padding-bottom: 40px; }
    .loading-msg {
      text-align: center;
      padding: 60px 20px;
      color: var(--text-dim);
      font-size: 0.9rem;
    }
    .spinner {
      display: inline-block;
      width: 20px; height: 20px;
      border: 2px solid var(--border);
      border-top-color: var(--gold);
      border-radius: 50%;
      animation: spin 0.7s linear infinite;
      margin-right: 10px;
      vertical-align: middle;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
    .error-msg {
      text-align: center;
      padding: 24px;
      color: var(--red);
      background: var(--red-dim);
      border: 1px solid rgba(248,113,113,0.2);
      border-radius: 10px;
      margin-top: 20px;
      font-size: 0.9rem;
    }

    /* Verdict Card */
    .verdict-card {
      text-align: center;
      padding: 40px 30px 32px;
      border-radius: 16px;
      margin-top: 20px;
      position: relative;
    }
    .verdict-card.HOWL { background: var(--green-dim); border: 1px solid rgba(74,222,128,0.25); }
    .verdict-card.WAG { background: var(--blue-dim); border: 1px solid rgba(96,165,250,0.25); }
    .verdict-card.GROWL { background: var(--yellow-dim); border: 1px solid rgba(250,204,21,0.25); }
    .verdict-card.BARK { background: var(--red-dim); border: 1px solid rgba(248,113,113,0.25); }

    .verdict-icon { font-size: 2rem; margin-bottom: 8px; }
    .verdict-name {
      font-size: 2.8rem;
      font-weight: 800;
      letter-spacing: 0.2em;
      line-height: 1;
      margin-bottom: 4px;
    }
    .HOWL .verdict-name { color: var(--green); }
    .WAG .verdict-name { color: var(--blue); }
    .GROWL .verdict-name { color: var(--yellow); }
    .BARK .verdict-name { color: var(--red); }

    .verdict-desc {
      font-size: 0.85rem;
      color: var(--text-dim);
      margin-bottom: 6px;
    }
    .verdict-token {
      font-size: 1rem;
      color: var(--text);
      margin-bottom: 24px;
      font-weight: 500;
    }
    .verdict-token .sym { color: var(--text-dim); font-weight: 400; }

    .scores-row {
      display: flex;
      justify-content: center;
      gap: 2px;
    }
    .score-pill {
      padding: 10px 24px;
      background: rgba(0,0,0,0.25);
      text-align: center;
    }
    .score-pill:first-child { border-radius: 10px 0 0 10px; }
    .score-pill:last-child { border-radius: 0 10px 10px 0; }
    .score-pill .val { font-size: 1.5rem; font-weight: 700; font-family: 'SF Mono', monospace; }
    .score-pill .lbl { font-size: 0.65rem; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.1em; margin-top: 2px; }

    .verdict-meta {
      display: flex;
      justify-content: center;
      gap: 12px;
      margin-top: 16px;
      flex-wrap: wrap;
    }
    .meta-chip {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      padding: 5px 14px;
      background: rgba(0,0,0,0.2);
      border-radius: 20px;
      font-size: 0.78rem;
      color: var(--text-dim);
    }
    .meta-chip .val { color: var(--text); font-weight: 600; }

    .watch-btn {
      margin-top: 16px;
      padding: 8px 20px;
      background: transparent;
      border: 1px solid currentColor;
      border-radius: 8px;
      color: var(--text-dim);
      font-family: inherit;
      font-size: 0.8rem;
      cursor: pointer;
      transition: all 0.15s;
    }
    .watch-btn:hover { color: var(--gold); background: var(--gold-dim); }

    /* Trajectory */
    .trajectory {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 16px;
      margin-top: 16px;
      padding: 12px 20px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 10px;
      font-size: 0.85rem;
    }
    .traj-dir { font-size: 1.3rem; }
    .traj-dir.improving { color: var(--green); }
    .traj-dir.declining { color: var(--red); }
    .traj-dir.stable { color: var(--text-dim); }
    .traj-dir.new { color: var(--gold); }
    .traj-text { color: var(--text-dim); }
    .traj-text strong { color: var(--text); }

    /* Confidence */
    .confidence-row {
      margin-top: 20px;
      padding: 16px 20px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 10px;
    }
    .conf-header {
      display: flex;
      justify-content: space-between;
      font-size: 0.8rem;
      color: var(--text-dim);
      margin-bottom: 10px;
    }
    .conf-header .val { color: var(--gold); font-weight: 600; font-family: 'SF Mono', monospace; }
    .conf-track {
      height: 6px;
      background: var(--border);
      border-radius: 3px;
      position: relative;
    }
    .conf-fill {
      height: 100%;
      border-radius: 3px;
      background: linear-gradient(90deg, var(--gold), #e8c94a);
      transition: width 0.5s ease;
    }
    .conf-phi {
      position: absolute;
      left: 61.8%;
      top: -2px;
      bottom: -2px;
      width: 2px;
      background: var(--text-muted);
      border-radius: 1px;
    }
    .conf-phi-label {
      position: absolute;
      left: 61.8%;
      top: -18px;
      transform: translateX(-50%);
      font-size: 0.6rem;
      color: var(--text-muted);
      white-space: nowrap;
    }

    /* Dimensions */
    .section {
      margin-top: 24px;
    }
    .section-head {
      font-size: 0.75rem;
      color: var(--gold);
      text-transform: uppercase;
      letter-spacing: 0.2em;
      margin-bottom: 16px;
      padding-bottom: 8px;
      border-bottom: 1px solid var(--border);
    }

    .axiom-block { margin-bottom: 20px; }
    .axiom-label {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 10px;
      font-size: 0.8rem;
    }
    .axiom-name { color: var(--text-dim); font-weight: 500; }
    .axiom-val {
      font-weight: 700;
      font-size: 0.75rem;
      padding: 2px 10px;
      border-radius: 4px;
      font-family: 'SF Mono', monospace;
    }
    .dim-row {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 5px 0;
    }
    .dim-name {
      width: 150px;
      font-size: 0.78rem;
      color: var(--text-dim);
      flex-shrink: 0;
    }
    .dim-track {
      flex: 1;
      height: 4px;
      background: var(--border);
      border-radius: 2px;
      overflow: hidden;
    }
    .dim-fill {
      height: 100%;
      border-radius: 2px;
      transition: width 0.4s ease;
    }
    .dim-val {
      width: 30px;
      text-align: right;
      font-size: 0.78rem;
      font-weight: 600;
      font-family: 'SF Mono', monospace;
    }

    /* Unnameable */
    .unnameable-card {
      margin-top: 20px;
      padding: 16px 20px;
      background: var(--gold-glow);
      border: 1px dashed rgba(212,168,71,0.3);
      border-radius: 10px;
      text-align: center;
      font-size: 0.85rem;
      color: var(--text-dim);
    }
    .unnameable-card strong { color: var(--gold); }

    /* Weaknesses */
    .weakness-item {
      padding: 10px 16px;
      background: var(--red-dim);
      border-left: 3px solid var(--red);
      margin-bottom: 8px;
      border-radius: 0 8px 8px 0;
      font-size: 0.82rem;
    }
    .weakness-dim { color: var(--yellow); font-weight: 600; }
    .weakness-reason { color: var(--text-dim); }

    /* On-chain data */
    .raw-data {
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      padding: 16px 20px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 10px;
      font-size: 0.78rem;
    }
    .raw-item { }
    .raw-item .k { color: var(--text-muted); }
    .raw-item .v { color: var(--text-dim); font-family: 'SF Mono', monospace; }

    /* Watchlist */
    .watchlist-section {
      padding: 32px 0 20px;
    }
    .wl-head {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }
    .wl-head h3 {
      font-size: 0.75rem;
      color: var(--gold);
      text-transform: uppercase;
      letter-spacing: 0.2em;
      font-weight: 500;
    }
    .wl-count { color: var(--text-muted); font-size: 0.75rem; }
    .wl-item {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 12px 16px;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 10px;
      margin-bottom: 6px;
      cursor: pointer;
      transition: border-color 0.15s;
    }
    .wl-item:hover { border-color: var(--border-light); }
    .wl-left { display: flex; align-items: center; gap: 12px; }
    .wl-label { font-weight: 600; font-size: 0.85rem; }
    .wl-mint { color: var(--text-muted); font-size: 0.72rem; font-family: 'SF Mono', monospace; }
    .wl-right { display: flex; align-items: center; gap: 14px; }
    .wl-score { font-weight: 700; font-size: 0.85rem; font-family: 'SF Mono', monospace; }
    .wl-verdict-tag {
      font-size: 0.7rem;
      padding: 3px 10px;
      border-radius: 4px;
      font-weight: 700;
      letter-spacing: 0.05em;
    }
    .wl-time { color: var(--text-muted); font-size: 0.7rem; }
    .wl-remove {
      color: var(--text-muted);
      cursor: pointer;
      font-size: 1rem;
      padding: 0 4px;
      transition: color 0.15s;
    }
    .wl-remove:hover { color: var(--red); }
    .wl-empty {
      color: var(--text-muted);
      font-size: 0.8rem;
      padding: 16px;
      text-align: center;
    }

    /* Alerts */
    .alerts-section { margin-top: 16px; }
    .alert-item {
      padding: 10px 14px;
      background: var(--yellow-dim);
      border-left: 3px solid var(--yellow);
      margin-bottom: 6px;
      border-radius: 0 8px 8px 0;
      font-size: 0.8rem;
      color: var(--text-dim);
    }
    .alert-item strong { color: var(--text); }

    /* Footer */
    .footer {
      text-align: center;
      padding: 32px 20px;
      color: var(--text-muted);
      font-size: 0.72rem;
      letter-spacing: 0.03em;
    }
    .footer a { color: var(--text-dim); text-decoration: none; transition: color 0.15s; }
    .footer a:hover { color: var(--gold); }
    .footer .sep { margin: 0 8px; }

    /* Mobile */
    @media (max-width: 640px) {
      .input-row { flex-direction: column; }
      .scores-row { flex-direction: column; align-items: center; gap: 0; }
      .score-pill { border-radius: 0 !important; width: 100%; }
      .score-pill:first-child { border-radius: 10px 10px 0 0 !important; }
      .score-pill:last-child { border-radius: 0 0 10px 10px !important; }
      .dim-name { width: 110px; font-size: 0.72rem; }
      .header h1 { font-size: 1.5rem; }
      .verdict-name { font-size: 2rem; }
      .verdict-meta { flex-direction: column; align-items: center; }
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div class="logo">Solana Token Intelligence</div>
      <h1>CYNIC ORACLE</h1>
      <div class="sub"><em>\\u03C6 distrusts \\u03C6</em> &mdash; 17 on-chain dimensions, confidence capped at 61.8%</div>
    </div>

    <div class="input-section">
      <div class="input-row">
        <input id="mint" class="mint-input" placeholder="Enter Solana token mint address..." spellcheck="false" autocomplete="off" />
        <button id="judge-btn" class="judge-btn" onclick="judgeToken()">JUDGE</button>
      </div>
      <div class="quick-links">
        <span class="quick-link" onclick="setMint('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v')">USDC</span>
        <span class="quick-link" onclick="setMint('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB')">USDT</span>
        <span class="quick-link" onclick="setMint('So11111111111111111111111111111111')">SOL</span>
        <span class="quick-link" onclick="setMint('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263')">BONK</span>
      </div>
    </div>

    <div id="result" class="result"></div>

    <div class="watchlist-section">
      <div class="wl-head">
        <h3>Watchlist</h3>
        <span id="wl-status" class="wl-count"></span>
      </div>
      <div id="watchlist-items"></div>
      <div id="alerts-section" class="alerts-section" style="display:none;">
        <div style="font-size:0.72rem;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.15em;margin-bottom:8px;">Recent Alerts</div>
        <div id="alerts-items"></div>
      </div>
    </div>
  </div>

  <div class="footer">
    <a href="/api/oracle/health">API</a>
    <span class="sep">&middot;</span>
    <a href="/api/oracle/judge?mint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v">Example JSON</a>
    <span class="sep">&middot;</span>
    <a href="/">Observatory</a>
    <span class="sep">&middot;</span>
    CYNIC Oracle v2.0
    <span class="sep">&middot;</span>
    \\u03C6\\u207B\\u00B9 = 61.8% max confidence
  </div>

  <script>
    const PHI_INV = 0.618, PHI_INV_2 = 0.382, PHI_INV_3 = 0.236;
    let currentMint = null;

    function setMint(m) { document.getElementById('mint').value = m; judgeToken(); }
    document.getElementById('mint').addEventListener('keydown', e => { if (e.key === 'Enter') judgeToken(); });

    function esc(s) { return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }

    function dColor(s) {
      if (s >= 61.8) return 'var(--green)';
      if (s >= 38.2) return 'var(--yellow)';
      return 'var(--red)';
    }

    function vColor(v) {
      return v === 'HOWL' ? 'var(--green)' : v === 'WAG' ? 'var(--blue)' : v === 'GROWL' ? 'var(--yellow)' : 'var(--red)';
    }

    async function judgeToken() {
      const mint = document.getElementById('mint').value.trim();
      if (!mint) return;
      const btn = document.getElementById('judge-btn');
      const rd = document.getElementById('result');
      btn.disabled = true; btn.textContent = 'JUDGING...';
      rd.innerHTML = '<div class="loading-msg"><span class="spinner"></span>Scoring 17 on-chain dimensions...</div>';
      try {
        const r = await fetch('/api/oracle/judge?mint=' + encodeURIComponent(mint));
        const d = await r.json();
        if (d.error) { rd.innerHTML = '<div class="error-msg">' + esc(d.error) + '</div>'; return; }
        currentMint = mint;
        renderVerdict(d);
        loadWatchlist();
      } catch (e) {
        rd.innerHTML = '<div class="error-msg">Network error: ' + esc(e.message) + '</div>';
      } finally {
        btn.disabled = false; btn.textContent = 'JUDGE';
      }
    }

    function renderVerdict(d) {
      const vc = vColor(d.verdict);
      const axioms = {
        PHI:     { lbl: '\\u03C6 Harmony',  dims: ['supplyDistribution','liquidityDepth','priceStability','supplyMechanics'] },
        VERIFY:  { lbl: '\\u2713 Verify',    dims: ['mintAuthority','freezeAuthority','metadataIntegrity','programVerification'] },
        CULTURE: { lbl: '\\u2605 Culture',   dims: ['holderCount','tokenAge','ecosystemIntegration','organicGrowth'] },
        BURN:    { lbl: '\\u2737 Burn',      dims: ['burnActivity','creatorBehavior','feeRedistribution','realUtility'] },
      };
      const dimN = {
        supplyDistribution:'Supply Distribution', liquidityDepth:'Liquidity Depth',
        priceStability:'Price Stability', supplyMechanics:'Supply Mechanics',
        mintAuthority:'Mint Authority', freezeAuthority:'Freeze Authority',
        metadataIntegrity:'Metadata Integrity', programVerification:'Program Verification',
        holderCount:'Holder Count', tokenAge:'Token Age',
        ecosystemIntegration:'Ecosystem Integration', organicGrowth:'Organic Growth',
        burnActivity:'Burn Activity', creatorBehavior:'Creator Behavior',
        feeRedistribution:'Fee Redistribution', realUtility:'Real Utility',
      };

      let h = '';

      // ── Verdict Card
      h += '<div class="verdict-card ' + d.verdict + '">';
      h += '<div class="verdict-icon">' + (d.verdictIcon || '') + '</div>';
      h += '<div class="verdict-name">' + d.verdict + '</div>';
      h += '<div class="verdict-desc">' + esc(d.verdictDescription || '') + '</div>';
      h += '<div class="verdict-token">' + esc(d.name || 'Unknown') + ' <span class="sym">(' + esc(d.symbol || '?') + ')</span></div>';
      h += '<div class="scores-row">';
      h += '<div class="score-pill"><div class="val" style="color:' + vc + '">' + d.qScore + '</div><div class="lbl">Q-Score</div></div>';
      h += '<div class="score-pill"><div class="val">' + d.kScore + '</div><div class="lbl">K-Score</div></div>';
      h += '<div class="score-pill"><div class="val" style="color:var(--gold)">' + (d.confidence * 100).toFixed(1) + '%</div><div class="lbl">Confidence</div></div>';
      h += '</div>';

      // Meta chips
      h += '<div class="verdict-meta">';
      h += '<span class="meta-chip">K-Tier: <span class="val">' + esc(d.kTier || d.tier || '?') + '</span></span>';
      h += '<span class="meta-chip">D: <span class="val">' + (d.kComponents?.d?.toFixed(2) || '?') + '</span></span>';
      h += '<span class="meta-chip">O: <span class="val">' + (d.kComponents?.o?.toFixed(2) || '?') + '</span></span>';
      h += '<span class="meta-chip">L: <span class="val">' + (d.kComponents?.l?.toFixed(2) || '?') + '</span></span>';
      h += '</div>';
      h += '<button class="watch-btn" onclick="watchCurrentMint()">Add to watchlist</button>';
      h += '</div>';

      // ── Trajectory
      if (d.trajectory && !(d.trajectory.direction === 'new' && d.trajectory.previousJudgments <= 1)) {
        const t = d.trajectory;
        const arrows = { improving:'\\u2197\\uFE0F', declining:'\\u2198\\uFE0F', stable:'\\u2192', new:'\\u2728' };
        const labels = { improving:'Improving', declining:'Declining', stable:'Stable', new:'First judgment' };
        h += '<div class="trajectory">';
        h += '<span class="traj-dir ' + t.direction + '">' + (arrows[t.direction]||'') + '</span>';
        h += '<span class="traj-text"><strong>' + (labels[t.direction]||'') + '</strong>';
        if (t.delta) h += ' (Q ' + (t.delta > 0 ? '+' : '') + t.delta + ')';
        h += '</span>';
        if (t.previousJudgments > 1) h += '<span class="traj-text">' + t.previousJudgments + ' past judgments</span>';
        if (t.averageQScore) h += '<span class="traj-text">avg Q: ' + t.averageQScore + '</span>';
        h += '</div>';
      }

      // ── Confidence bar
      h += '<div class="confidence-row">';
      h += '<div class="conf-header"><span>Confidence</span><span class="val">' + (d.confidence * 100).toFixed(1) + '% / 61.8%</span></div>';
      h += '<div class="conf-track">';
      h += '<div class="conf-fill" style="width:' + Math.min(61.8, d.confidence * 100).toFixed(1) + '%"></div>';
      h += '<div class="conf-phi"></div>';
      h += '<div class="conf-phi-label">\\u03C6\\u207B\\u00B9</div>';
      h += '</div></div>';

      // ── Axiom dimensions
      h += '<div class="section"><div class="section-head">17 Dimensions</div>';
      for (const [ax, info] of Object.entries(axioms)) {
        const aScore = d.axiomScores[ax];
        const ac = dColor(aScore);
        h += '<div class="axiom-block">';
        h += '<div class="axiom-label"><span class="axiom-name">' + info.lbl + '</span>';
        h += '<span class="axiom-val" style="color:' + ac + ';background:' + ac + '15">' + aScore + '</span></div>';
        for (const dim of info.dims) {
          const s = d.dimensions[dim] || 0;
          const c = dColor(s);
          h += '<div class="dim-row">';
          h += '<span class="dim-name">' + (dimN[dim]||dim) + '</span>';
          h += '<div class="dim-track"><div class="dim-fill" style="width:' + s + '%;background:' + c + '"></div></div>';
          h += '<span class="dim-val" style="color:' + c + '">' + s + '</span>';
          h += '</div>';
        }
        h += '</div>';
      }
      h += '</div>';

      // ── THE_UNNAMEABLE
      h += '<div class="unnameable-card">';
      h += '<strong>THE UNNAMEABLE</strong> \\u2014 ' + d.theUnnameable + '% uncertainty \\u2014 ';
      h += d.theUnnameable > 50 ? 'Much we cannot measure. Judgment has limited grounding.' :
           d.theUnnameable > 25 ? 'Moderate gaps in available on-chain data.' :
           'Good data coverage. Judgment is well-grounded.';
      h += '</div>';

      // ── Weaknesses
      if (d.weaknesses && d.weaknesses.length > 0) {
        h += '<div class="section"><div class="section-head">Weaknesses (below \\u03C6\\u207B\\u00B2)</div>';
        for (const w of d.weaknesses) {
          h += '<div class="weakness-item">';
          h += '<span class="weakness-dim">[' + w.axiom + '] ' + (dimN[w.dimension]||w.dimension) + ': ' + w.score + '</span><br>';
          h += '<span class="weakness-reason">' + esc(w.reason) + '</span>';
          h += '</div>';
        }
        h += '</div>';
      }

      // ── On-chain data
      h += '<div class="section"><div class="section-head">On-Chain Data</div>';
      h += '<div class="raw-data">';
      h += '<div class="raw-item"><span class="k">Source </span><span class="v">' + esc(d._raw?.source || '?') + '</span></div>';
      h += '<div class="raw-item"><span class="k">Latency </span><span class="v">' + (d._raw?.latencyMs || '?') + 'ms</span></div>';
      h += '<div class="raw-item"><span class="k">Supply </span><span class="v">' + (d.supply?.total?.toLocaleString() || '?') + '</span></div>';
      h += '<div class="raw-item"><span class="k">Holders </span><span class="v">' + (d.distribution?.holderCount?.toLocaleString() || '?') + '</span></div>';
      h += '<div class="raw-item"><span class="k">Whale% </span><span class="v">' + ((d.distribution?.whaleConcentration || 0) * 100).toFixed(1) + '%</span></div>';
      h += '<div class="raw-item"><span class="k">Gini </span><span class="v">' + (d.distribution?.giniCoefficient?.toFixed(3) || '?') + '</span></div>';
      h += '</div></div>';

      document.getElementById('result').innerHTML = h;
    }

    // ── Watchlist
    async function loadWatchlist() {
      try {
        const r = await fetch('/api/oracle/watchlist');
        if (!r.ok) { document.getElementById('wl-status').textContent = 'offline'; return; }
        const d = await r.json();
        renderWatchlist(d.watchlist || [], d.recentAlerts || []);
      } catch { document.getElementById('wl-status').textContent = 'offline'; }
    }

    function renderWatchlist(items, alerts) {
      const c = document.getElementById('watchlist-items');
      document.getElementById('wl-status').textContent = items.length + ' watched';
      if (!items.length) {
        c.innerHTML = '<div class="wl-empty">No tokens watched yet. Judge a token and add it to your watchlist.</div>';
      } else {
        c.innerHTML = items.map(w => {
          const vc = vColor(w.lastVerdict);
          const ago = w.lastCheckedAt ? timeAgo(new Date(w.lastCheckedAt)) : '';
          return '<div class="wl-item" onclick="setMint(\\'' + w.mint + '\\')">' +
            '<div class="wl-left">' +
              '<span class="wl-label">' + esc(w.label || w.mint.slice(0,6)+'..') + '</span>' +
              '<span class="wl-mint">' + w.mint.slice(0,8) + '...' + w.mint.slice(-4) + '</span>' +
            '</div>' +
            '<div class="wl-right">' +
              (w.lastQScore ? '<span class="wl-score" style="color:'+vc+'">'+w.lastQScore+'</span>' : '') +
              (w.lastVerdict ? '<span class="wl-verdict-tag" style="color:'+vc+';background:'+vc+'15">'+w.lastVerdict+'</span>' : '') +
              (ago ? '<span class="wl-time">'+ago+'</span>' : '') +
              '<span class="wl-remove" onclick="event.stopPropagation();unwatchMint(\\''+w.mint+'\\')">\\u00D7</span>' +
            '</div></div>';
        }).join('');
      }

      const as = document.getElementById('alerts-section');
      const ai = document.getElementById('alerts-items');
      if (alerts.length > 0) {
        as.style.display = 'block';
        ai.innerHTML = alerts.slice(0,5).map(a =>
          '<div class="alert-item"><strong>' + esc(a.label || a.mint?.slice(0,8)) + '</strong>: ' +
          esc(a.message || a.alertType) +
          '<span style="float:right;color:var(--text-muted)">' + timeAgo(new Date(a.createdAt)) + '</span></div>'
        ).join('');
      } else { as.style.display = 'none'; }
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

        // Verify pool actually works before wiring agent capabilities
        await pool.query('SELECT 1');

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
          // Fetch on-chain data + DexScreener market data in parallel
          const [tokenData, dexScreener] = await Promise.all([
            tokenFetcher.getTokenData(mint),
            tokenFetcher.getDexScreenerData(mint),
          ]);

          // Attach DexScreener data for scorer
          tokenData.dexScreener = dexScreener;

          // OracleMemory price history: first-seen price from past judgments
          const currentPrice = tokenData.priceInfo?.pricePerToken || 0;
          let priceHistory = null;
          if (oracleMemory && currentPrice > 0) {
            const firstSeen = await oracleMemory.getFirstPrice(mint);
            if (firstSeen && firstSeen.price > 0) {
              const ownChange = (currentPrice - firstSeen.price) / firstSeen.price;
              priceHistory = {
                priceNow: currentPrice,
                priceFirstSeen: firstSeen.price,
                priceChange: ownChange,
                firstSeenAt: firstSeen.judgedAt,
                source: 'oracle_memory',
              };
            }
          }
          tokenData.priceHistory = priceHistory;

          const verdict = tokenScorer.score(tokenData);

          const response = {
            ...verdict,
            _raw: tokenData._raw,
            supply: tokenData.supply,
            priceInfo: tokenData.priceInfo,
            dexScreener: dexScreener,
            priceHistory: priceHistory,
            distribution: {
              holderCount: tokenData.distribution.holderCount,
              whaleConcentration: tokenData.distribution.whaleConcentration,
              giniCoefficient: tokenData.distribution.giniCoefficient,
            },
          };

          // Agent: store judgment + attach trajectory
          if (oracleMemory) {
            await oracleMemory.store({
              mint, name: tokenData.name, symbol: tokenData.symbol,
              ...verdict,
              pricePerToken: currentPrice || null,
            });
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
          const watchPrice = tokenData.priceInfo?.pricePerToken || null;
          await oracleMemory.store({ mint: body.mint, name: tokenData.name, symbol: tokenData.symbol, ...verdict, pricePerToken: watchPrice });
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
        let stats = null;
        let watchlist = [];
        try {
          if (oracleMemory) stats = await oracleMemory.getStats();
          if (oracleWatchlist) watchlist = await oracleWatchlist.list();
        } catch { /* DB not available — agent works without memory */ }
        return jsonResponse(res, {
          status: 'operational',
          service: 'CYNIC Oracle',
          version: '2.0.0',
          type: 'agent',
          capabilities: ['judge', 'remember', 'watch', 'alert', 'trajectory'],
          dataSources: {
            rpc: process.env.HELIUS_API_KEY ? 'helius_das' : 'public_rpc',
            market: 'dexscreener (free)',
            memory: oracleMemory ? 'postgresql' : 'unavailable',
          },
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
