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

// ═══════════════════════════════════════════════════════════════════════════
// CORS & RESPONSE HELPERS
// ═══════════════════════════════════════════════════════════════════════════

const CORS_HEADERS = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET, OPTIONS',
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
    <div class="sub">"\\u03C6 distrusts \\u03C6" — 17 dimensions, max 61.8% confidence</div>
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

        renderVerdict(data);
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
      html += '</div>';

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
          return jsonResponse(res, {
            ...verdict,
            _raw: tokenData._raw,
            supply: tokenData.supply,
            distribution: {
              holderCount: tokenData.distribution.holderCount,
              whaleConcentration: tokenData.distribution.whaleConcentration,
              giniCoefficient: tokenData.distribution.giniCoefficient,
            },
          });
        } catch (e) {
          return errorResponse(res, `Oracle error: ${e.message}`, 422);
        }
      }

      if (path === '/api/oracle/health') {
        return jsonResponse(res, {
          status: 'operational',
          service: 'CYNIC Oracle',
          version: '1.0.0',
          rpcSource: process.env.HELIUS_API_KEY ? 'helius' : 'public_rpc',
          dimensions: 17,
          maxConfidence: '61.8% (φ⁻¹)',
          endpoints: {
            judge: '/api/oracle/judge?mint=<ADDRESS>',
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
🐕 CYNIC Observatory
═══════════════════════════════════════════════════════════

Dashboard:  http://localhost:${port}/
Oracle:     http://localhost:${port}/oracle

API Endpoints:
  GET /api/oracle/judge?mint=X  TOKEN VERDICT (17 dimensions)
  GET /api/oracle/health        Oracle service status
  GET /api/qlearning/stats      Q-Table summary
  GET /api/qlearning/curve      Learning curve (is it learning?)
  GET /api/qlearning/heatmap    Q-values visualization
  GET /api/patterns/important   EWC++ locked patterns
  GET /api/patterns/anomalies   Detected anomalies
  GET /api/telemetry/health     System health
  GET /api/telemetry/frictions  Friction points
  GET /api/learning/proof       COMPREHENSIVE LEARNING PROOF

"\\u03C6 distrusts \\u03C6" — max confidence 61.8%
═══════════════════════════════════════════════════════════
    `);
  });

  return server;
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  start();
}
