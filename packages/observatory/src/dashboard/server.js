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
<title>CYNIC Oracle</title>
<style>
:root{
  --bg:#0a0e14;--bg2:#111820;--card:#151d28;
  --border:#1e2a3a;--border-hi:#2a3a50;
  --text:#cdd6e4;--dim:#5a7090;--muted:#2e4060;
  --gold:#c9a84c;--gold-10:rgba(201,168,76,.1);--gold-20:rgba(201,168,76,.2);
  --green:#22c55e;--blue:#3b82f6;--yellow:#eab308;--red:#ef4444;--orange:#f97316;
}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:var(--bg);color:var(--text);min-height:100vh}
.mono{font-family:'SF Mono','Fira Code','Cascadia Code',Consolas,monospace}
.ctr{max-width:820px;margin:0 auto;padding:0 20px}

/* ── Header ── */
.hdr{text-align:center;padding:36px 0 24px;position:relative}
.hdr::after{content:'';position:absolute;bottom:0;left:50%;transform:translateX(-50%);width:100px;height:1px;background:linear-gradient(90deg,transparent,var(--gold),transparent)}
.hdr h1{font-size:1.5rem;font-weight:300;letter-spacing:.35em;color:var(--gold)}
.hdr .sub{color:var(--dim);font-size:.76rem;margin-top:6px}
.hdr .sub b{color:var(--gold);font-weight:400}

/* ── Input ── */
.iw{display:flex;gap:8px;margin:20px 0 10px}
.mi{flex:1;padding:13px 16px;background:var(--card);border:1px solid var(--border);border-radius:10px;color:var(--text);font-family:'SF Mono','Fira Code',Consolas,monospace;font-size:.85rem;outline:none;transition:border .2s,box-shadow .2s}
.mi:focus{border-color:var(--gold);box-shadow:0 0 0 3px var(--gold-10)}
.mi::placeholder{color:var(--muted)}
.jb{padding:13px 28px;background:var(--gold);border:none;border-radius:10px;color:#000;font-weight:700;font-size:.8rem;letter-spacing:.14em;cursor:pointer;white-space:nowrap;transition:all .15s;font-family:inherit}
.jb:hover{filter:brightness(1.12);transform:translateY(-1px)}
.jb:active{transform:none}
.jb:disabled{opacity:.3;cursor:not-allowed;transform:none}
.qt{display:flex;gap:6px;flex-wrap:wrap;margin-bottom:20px}
.qt button{padding:4px 14px;background:transparent;border:1px solid var(--border);border-radius:20px;color:var(--dim);font-size:.7rem;cursor:pointer;font-family:inherit;transition:all .15s}
.qt button:hover{border-color:var(--gold);color:var(--gold);background:var(--gold-10)}

/* ── Loading / Error ── */
.ld{text-align:center;padding:48px 20px;color:var(--dim);font-size:.85rem}
.sp{display:inline-block;width:18px;height:18px;border:2px solid var(--border);border-top-color:var(--gold);border-radius:50%;animation:spin .6s linear infinite;vertical-align:middle;margin-right:8px}
@keyframes spin{to{transform:rotate(360deg)}}
.err{text-align:center;padding:18px;color:var(--red);background:rgba(239,68,68,.06);border:1px solid rgba(239,68,68,.12);border-radius:10px;font-size:.82rem}

/* ── Verdict Hero ── */
.vhero{text-align:center;padding:32px 24px 28px;border-radius:14px;margin-bottom:16px;animation:fadeUp .4s ease}
.vhero.HOWL{background:linear-gradient(160deg,rgba(34,197,94,.07),rgba(34,197,94,.01));border:1px solid rgba(34,197,94,.18)}
.vhero.WAG{background:linear-gradient(160deg,rgba(59,130,246,.07),rgba(59,130,246,.01));border:1px solid rgba(59,130,246,.18)}
.vhero.GROWL{background:linear-gradient(160deg,rgba(234,179,8,.07),rgba(234,179,8,.01));border:1px solid rgba(234,179,8,.18)}
.vhero.BARK{background:linear-gradient(160deg,rgba(239,68,68,.07),rgba(239,68,68,.01));border:1px solid rgba(239,68,68,.18)}

/* Ring gauge */
.ring-w{display:inline-block;width:156px;height:156px;margin-bottom:10px}
.ring-track{fill:none;stroke:var(--border);stroke-width:10}
.ring-prog{fill:none;stroke-width:10;stroke-linecap:round;transition:stroke-dashoffset 1s cubic-bezier(.35,0,.15,1);filter:drop-shadow(0 0 5px currentColor)}
.ring-lbl{font-size:.6rem;fill:var(--dim);letter-spacing:.15em;text-transform:uppercase}
.ring-val{font-size:2.8rem;font-weight:800;font-family:'SF Mono','Fira Code',Consolas,monospace}

/* Badge & token */
.vbadge{font-size:1.6rem;font-weight:800;letter-spacing:.3em;line-height:1;margin-bottom:4px}
.HOWL .vbadge{color:var(--green)}.WAG .vbadge{color:var(--blue)}.GROWL .vbadge{color:var(--yellow)}.BARK .vbadge{color:var(--red)}
.vdesc{font-size:.78rem;color:var(--dim);margin-bottom:2px}
.vtok{font-size:.95rem;font-weight:600;margin-bottom:18px}.vtok .sym{color:var(--dim);font-weight:400}

/* Score row */
.srow{display:flex;justify-content:center;gap:2px}
.sp2{padding:10px 18px;background:rgba(0,0,0,.3);text-align:center;min-width:80px}
.sp2:first-child{border-radius:10px 0 0 10px}.sp2:last-child{border-radius:0 10px 10px 0}
.sp2 .v{font-size:1.2rem;font-weight:700;font-family:'SF Mono',Consolas,monospace}.sp2 .l{font-size:.58rem;color:var(--dim);text-transform:uppercase;letter-spacing:.08em;margin-top:2px}

/* Watch button */
.wbtn{margin-top:14px;padding:6px 16px;background:transparent;border:1px solid var(--border);border-radius:8px;color:var(--dim);font-size:.72rem;cursor:pointer;font-family:inherit;transition:all .15s}
.wbtn:hover{color:var(--gold);border-color:var(--gold);background:var(--gold-10)}

/* ── Market Data ── */
.mkt{display:flex;flex-wrap:wrap;gap:1px;background:var(--border);border-radius:12px;overflow:hidden;margin-bottom:16px;animation:fadeUp .4s ease .08s both}
.mkt-i{flex:1;min-width:90px;padding:11px 12px;background:var(--card);text-align:center}
.mkt-i .l{font-size:.55rem;color:var(--muted);text-transform:uppercase;letter-spacing:.06em;margin-bottom:2px}
.mkt-i .v{font-size:.88rem;font-weight:600;font-family:'SF Mono',Consolas,monospace}
.up{color:var(--green)}.dn{color:var(--red)}

/* ── Confidence ── */
.conf{padding:12px 16px;background:var(--card);border:1px solid var(--border);border-radius:10px;margin-bottom:16px;animation:fadeUp .4s ease .12s both}
.conf-hd{display:flex;justify-content:space-between;font-size:.72rem;color:var(--dim);margin-bottom:6px}
.conf-hd .v{color:var(--gold);font-weight:600;font-family:'SF Mono',monospace}
.conf-tk{height:5px;background:var(--border);border-radius:3px;position:relative}
.conf-fl{height:100%;border-radius:3px;background:linear-gradient(90deg,var(--gold),#e2c05a);transition:width .7s ease}
.conf-ph{position:absolute;left:61.8%;top:-3px;bottom:-3px;width:2px;background:var(--muted);border-radius:1px}
.conf-pl{position:absolute;left:61.8%;top:-15px;transform:translateX(-50%);font-size:.52rem;color:var(--muted)}

/* ── Trajectory ── */
.traj{display:flex;align-items:center;justify-content:center;gap:10px;padding:10px 14px;background:var(--card);border:1px solid var(--border);border-radius:10px;margin-bottom:16px;font-size:.8rem}
.traj-a{font-size:1.1rem}.traj-a.improving{color:var(--green)}.traj-a.declining{color:var(--red)}.traj-a.stable{color:var(--dim)}.traj-a.new{color:var(--gold)}
.traj span{color:var(--dim)}.traj strong{color:var(--text)}

/* ── Axiom Grid ── */
.agrid{display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-bottom:16px;animation:fadeUp .4s ease .16s both}
.acard{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:12px 14px;border-left:3px solid var(--gold)}
.acard.a-v{border-left-color:var(--blue)}.acard.a-c{border-left-color:var(--green)}.acard.a-b{border-left-color:var(--orange)}
.ac-top{display:flex;justify-content:space-between;align-items:center;margin-bottom:8px}
.ac-nm{font-size:.68rem;font-weight:600;color:var(--dim);text-transform:uppercase;letter-spacing:.08em}
.ac-sc{font-size:.7rem;font-weight:700;font-family:'SF Mono',monospace;padding:2px 8px;border-radius:4px}
.dr{display:flex;align-items:center;gap:6px;padding:2px 0}
.dr .n{width:82px;font-size:.67rem;color:var(--dim);flex-shrink:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.dr .t{flex:1;height:3px;background:var(--border);border-radius:2px;overflow:hidden}
.dr .f{height:100%;border-radius:2px;transition:width .5s ease}
.dr .s{width:22px;text-align:right;font-size:.67rem;font-weight:600;font-family:'SF Mono',monospace}

/* ── Unnameable ── */
.unm{padding:10px 14px;background:var(--gold-10);border:1px dashed var(--gold-20);border-radius:10px;text-align:center;font-size:.76rem;color:var(--dim);margin-bottom:16px}
.unm strong{color:var(--gold)}

/* ── Weaknesses ── */
.wsec{margin-bottom:16px;animation:fadeUp .4s ease .2s both}
.wsec-t{font-size:.65rem;color:var(--dim);text-transform:uppercase;letter-spacing:.12em;margin-bottom:8px}
.wpl{display:flex;flex-wrap:wrap;gap:5px}
.wp{padding:4px 9px;background:rgba(239,68,68,.04);border:1px solid rgba(239,68,68,.12);border-radius:6px;font-size:.65rem;color:var(--dim)}
.wp .a{font-weight:600;margin-right:3px;color:var(--yellow)}.wp .sc{font-weight:700;margin-left:3px;color:var(--red)}

/* ── On-chain data ── */
.cdata{display:flex;flex-wrap:wrap;gap:10px;padding:12px 14px;background:var(--card);border:1px solid var(--border);border-radius:10px;margin-bottom:16px;font-size:.7rem}
.cdata .k{color:var(--muted)}.cdata .v{color:var(--dim);font-family:'SF Mono',monospace}

/* ── Watchlist ── */
.wlsec{padding-top:16px;border-top:1px solid var(--border)}
.wl-hd{display:flex;justify-content:space-between;align-items:center;margin-bottom:8px}
.wl-hd h3{font-size:.65rem;color:var(--gold);text-transform:uppercase;letter-spacing:.2em;font-weight:500}
.wl-ct{color:var(--muted);font-size:.68rem}
.wl-i{display:flex;justify-content:space-between;align-items:center;padding:9px 12px;background:var(--card);border:1px solid var(--border);border-radius:10px;margin-bottom:4px;cursor:pointer;transition:border .15s}
.wl-i:hover{border-color:var(--border-hi)}
.wl-l{display:flex;align-items:center;gap:8px}.wl-lb{font-weight:600;font-size:.8rem}.wl-m{color:var(--muted);font-size:.65rem;font-family:'SF Mono',monospace}
.wl-r{display:flex;align-items:center;gap:8px}
.wl-sc{font-weight:700;font-size:.8rem;font-family:'SF Mono',monospace}
.wl-vt{font-size:.62rem;padding:2px 8px;border-radius:4px;font-weight:700;letter-spacing:.04em}
.wl-tm{color:var(--muted);font-size:.62rem}
.wl-x{color:var(--muted);cursor:pointer;font-size:.95rem;padding:0 3px;transition:color .15s}.wl-x:hover{color:var(--red)}
.wl-e{color:var(--muted);font-size:.76rem;padding:12px;text-align:center}

/* ── Alerts ── */
.al-sec{margin-top:10px}
.al-i{padding:7px 11px;background:rgba(234,179,8,.05);border-left:3px solid var(--yellow);margin-bottom:4px;border-radius:0 8px 8px 0;font-size:.75rem;color:var(--dim)}
.al-i strong{color:var(--text)}

/* ── Footer ── */
.ft{text-align:center;padding:24px 20px;color:var(--muted);font-size:.65rem;letter-spacing:.02em}
.ft a{color:var(--dim);text-decoration:none;transition:color .15s}.ft a:hover{color:var(--gold)}.ft .d{margin:0 6px}

/* ── Animations ── */
@keyframes fadeUp{from{opacity:0;transform:translateY(10px)}to{opacity:1;transform:none}}

/* ── Mobile ── */
@media(max-width:640px){
  .iw{flex-direction:column}
  .srow{flex-direction:column;align-items:center;gap:0}
  .sp2{width:100%;border-radius:0!important}.sp2:first-child{border-radius:10px 10px 0 0!important}.sp2:last-child{border-radius:0 0 10px 10px!important}
  .agrid{grid-template-columns:1fr}
  .hdr h1{font-size:1.2rem;letter-spacing:.25em}
  .vbadge{font-size:1.2rem}
  .dr .n{width:70px;font-size:.62rem}
  .mkt{flex-direction:column}
  .ring-w{width:130px;height:130px}
  .ring-val{font-size:2.2rem}
}
</style>
</head>
<body>
<div class="ctr">
  <header class="hdr">
    <h1>CYNIC ORACLE</h1>
    <p class="sub"><b>\\u03C6 distrusts \\u03C6</b> \\u2014 17 on-chain dimensions, max 61.8% confidence</p>
  </header>

  <div class="iw">
    <input id="mint" class="mi mono" placeholder="Solana token mint address..." spellcheck="false" autocomplete="off"/>
    <button id="jb" class="jb" onclick="judgeToken()">JUDGE</button>
  </div>
  <div class="qt">
    <button onclick="qm('So11111111111111111111111111111111')">SOL</button>
    <button onclick="qm('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263')">BONK</button>
    <button onclick="qm('EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm')">WIF</button>
    <button onclick="qm('6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN')">TRUMP</button>
    <button onclick="qm('JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN')">JUP</button>
    <button onclick="qm('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v')">USDC</button>
  </div>

  <div id="result"></div>

  <div class="wlsec">
    <div class="wl-hd"><h3>Watchlist</h3><span id="wl-st" class="wl-ct"></span></div>
    <div id="wl-items"></div>
    <div id="al-sec" class="al-sec" style="display:none">
      <div style="font-size:.6rem;color:var(--muted);text-transform:uppercase;letter-spacing:.12em;margin-bottom:5px">Recent Alerts</div>
      <div id="al-items"></div>
    </div>
  </div>
</div>

<footer class="ft">
  <a href="/api/oracle/health">API</a><span class="d">\\u00B7</span>
  <a href="/api/oracle/judge?mint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v">Example</a><span class="d">\\u00B7</span>
  <a href="/">Observatory</a><span class="d">\\u00B7</span>
  CYNIC Oracle v2<span class="d">\\u00B7</span>
  \\u03C6\\u207B\\u00B9 = 61.8%
</footer>

<script>
var cur=null,CIRC=402;
function qm(m){document.getElementById('mint').value=m;judgeToken()}
document.getElementById('mint').addEventListener('keydown',function(e){if(e.key==='Enter')judgeToken()});
function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')}
function dc(s){return s>=61.8?'var(--green)':s>=38.2?'var(--yellow)':'var(--red)'}
function vc(v){return v==='HOWL'?'var(--green)':v==='WAG'?'var(--blue)':v==='GROWL'?'var(--yellow)':'var(--red)'}
function fu(n){if(n==null)return'\\u2014';if(n>=1e9)return'$'+(n/1e9).toFixed(1)+'B';if(n>=1e6)return'$'+(n/1e6).toFixed(1)+'M';if(n>=1e3)return'$'+(n/1e3).toFixed(1)+'K';if(n>=1)return'$'+n.toFixed(2);if(n>=.001)return'$'+n.toFixed(4);return'$'+n.toFixed(8)}

async function judgeToken(){
  var mint=document.getElementById('mint').value.trim();
  if(!mint)return;
  var btn=document.getElementById('jb'),rd=document.getElementById('result');
  btn.disabled=true;btn.textContent='JUDGING...';
  rd.innerHTML='<div class="ld"><span class="sp"></span>Fetching on-chain data & scoring 17 dimensions...</div>';
  try{
    var r=await fetch('/api/oracle/judge?mint='+encodeURIComponent(mint));
    var d=await r.json();
    if(d.error){rd.innerHTML='<div class="err">'+esc(d.error)+'</div>';return}
    cur=mint;renderV(d);loadWL();
  }catch(e){rd.innerHTML='<div class="err">'+esc(e.message)+'</div>'}
  finally{btn.disabled=false;btn.textContent='JUDGE'}
}

function renderV(d){
  var c=vc(d.verdict),q=d.qScore,off=CIRC-(q/100*CIRC);
  var axs=[
    {k:'PHI',cls:'',lbl:'\\u03C6 Harmony',ds:['supplyDistribution','liquidityDepth','priceStability','supplyMechanics']},
    {k:'VERIFY',cls:'a-v',lbl:'\\u2713 Verify',ds:['mintAuthority','freezeAuthority','metadataIntegrity','programVerification']},
    {k:'CULTURE',cls:'a-c',lbl:'\\u2605 Culture',ds:['holderCount','tokenAge','ecosystemIntegration','organicGrowth']},
    {k:'BURN',cls:'a-b',lbl:'\\u2737 Burn',ds:['burnActivity','creatorBehavior','feeRedistribution','realUtility']}
  ];
  var dn={supplyDistribution:'Supply Dist',liquidityDepth:'Liquidity',priceStability:'Price Stability',supplyMechanics:'Supply Mech',mintAuthority:'Mint Auth',freezeAuthority:'Freeze Auth',metadataIntegrity:'Metadata',programVerification:'Prog Verify',holderCount:'Holders',tokenAge:'Token Age',ecosystemIntegration:'Ecosystem',organicGrowth:'Organic Growth',burnActivity:'Burn Activity',creatorBehavior:'Creator Behav',feeRedistribution:'Fee Redist',realUtility:'Real Utility'};

  var h='';

  // ── Verdict Hero with SVG ring
  h+='<div class="vhero '+d.verdict+'">';
  h+='<div class="ring-w"><svg viewBox="0 0 160 160" width="100%" height="100%">';
  h+='<circle cx="80" cy="80" r="64" class="ring-track"/>';
  h+='<circle cx="80" cy="80" r="64" class="ring-prog" style="stroke:'+c+';stroke-dasharray:'+CIRC+';stroke-dashoffset:'+CIRC+'" transform="rotate(-90 80 80)" data-off="'+off+'"/>';
  h+='<text x="80" y="70" text-anchor="middle" dominant-baseline="auto" class="ring-val" fill="'+c+'">'+q+'</text>';
  h+='<text x="80" y="90" text-anchor="middle" class="ring-lbl">Q-SCORE</text>';
  h+='</svg></div>';
  h+='<div class="vbadge">'+d.verdict+'</div>';
  h+='<div class="vdesc">'+esc(d.verdictDescription||'')+'</div>';
  h+='<div class="vtok">'+esc(d.name||'Unknown')+' <span class="sym">('+esc(d.symbol||'?')+')</span></div>';

  // Score pills
  h+='<div class="srow">';
  h+='<div class="sp2"><div class="v" style="color:'+c+'">'+q+'</div><div class="l">Q-Score</div></div>';
  h+='<div class="sp2"><div class="v">'+d.kScore+'</div><div class="l">K-Score</div></div>';
  h+='<div class="sp2"><div class="v" style="color:var(--gold)">'+(d.confidence*100).toFixed(1)+'%</div><div class="l">Confidence</div></div>';
  h+='<div class="sp2"><div class="v">'+esc(d.kTier||'?')+'</div><div class="l">K-Tier</div></div>';
  h+='</div>';
  h+='<button class="wbtn" onclick="watchC()">+ Watchlist</button>';
  h+='</div>';

  // ── Market Data
  var ds=d.dexScreener;
  if(ds){
    var pc=ds.priceChange24h;
    h+='<div class="mkt">';
    h+='<div class="mkt-i"><div class="l">Price</div><div class="v">'+fu(ds.priceUsd)+'</div></div>';
    h+='<div class="mkt-i"><div class="l">24h</div><div class="v '+(pc>=0?'up':'dn')+'">'+(pc>=0?'+':'')+((pc||0).toFixed(1))+'%</div></div>';
    h+='<div class="mkt-i"><div class="l">Liquidity</div><div class="v">'+fu(ds.liquidityUsd)+'</div></div>';
    h+='<div class="mkt-i"><div class="l">Volume 24h</div><div class="v">'+fu(ds.volume24h)+'</div></div>';
    if(ds.sellBuyRatio!=null)h+='<div class="mkt-i"><div class="l">Sell/Buy</div><div class="v'+(ds.sellBuyRatio>1.5?' dn':ds.sellBuyRatio<=1?' up':'')+'">'+ds.sellBuyRatio.toFixed(2)+'</div></div>';
    h+='</div>';
  }

  // ── Trajectory
  if(d.trajectory&&!(d.trajectory.direction==='new'&&d.trajectory.previousJudgments<=1)){
    var t=d.trajectory;
    var ar={improving:'\\u2197',declining:'\\u2198',stable:'\\u2192',new:'\\u2728'};
    var lb={improving:'Improving',declining:'Declining',stable:'Stable',new:'First'};
    h+='<div class="traj"><span class="traj-a '+t.direction+'">'+(ar[t.direction]||'')+'</span>';
    h+='<strong>'+(lb[t.direction]||'')+'</strong>';
    if(t.delta)h+='<span>Q '+(t.delta>0?'+':'')+t.delta+'</span>';
    if(t.previousJudgments>1)h+='<span>'+t.previousJudgments+' past</span>';
    if(t.averageQScore)h+='<span>avg Q:'+t.averageQScore+'</span>';
    h+='</div>';
  }

  // ── Confidence bar
  h+='<div class="conf"><div class="conf-hd"><span>Confidence</span><span class="v">'+(d.confidence*100).toFixed(1)+'% / 61.8%</span></div>';
  h+='<div class="conf-tk"><div class="conf-fl" style="width:'+Math.min(61.8,d.confidence*100).toFixed(1)+'%"></div>';
  h+='<div class="conf-ph"></div><div class="conf-pl">\\u03C6\\u207B\\u00B9</div></div></div>';

  // ── Axiom Grid (2x2)
  h+='<div class="agrid">';
  for(var i=0;i<axs.length;i++){
    var ax=axs[i],aS=d.axiomScores[ax.k],ac=dc(aS);
    h+='<div class="acard '+ax.cls+'">';
    h+='<div class="ac-top"><span class="ac-nm">'+ax.lbl+'</span><span class="ac-sc" style="color:'+ac+';background:'+ac+'12">'+aS+'</span></div>';
    for(var j=0;j<ax.ds.length;j++){
      var dm=ax.ds[j],s=d.dimensions[dm]||0,sc=dc(s);
      h+='<div class="dr"><span class="n">'+(dn[dm]||dm)+'</span><div class="t"><div class="f" style="width:'+s+'%;background:'+sc+'"></div></div><span class="s" style="color:'+sc+'">'+s+'</span></div>';
    }
    h+='</div>';
  }
  h+='</div>';

  // ── THE UNNAMEABLE
  h+='<div class="unm"><strong>THE UNNAMEABLE</strong> \\u2014 '+d.theUnnameable+'% uncertainty \\u2014 ';
  h+=d.theUnnameable>50?'Much we cannot measure.':d.theUnnameable>25?'Moderate data gaps.':'Good data coverage.';
  h+='</div>';

  // ── Weaknesses (compact pills)
  if(d.weaknesses&&d.weaknesses.length>0){
    h+='<div class="wsec"><div class="wsec-t">Weaknesses (below \\u03C6\\u207B\\u00B2)</div><div class="wpl">';
    for(var k=0;k<d.weaknesses.length;k++){
      var w=d.weaknesses[k];
      h+='<div class="wp"><span class="a">'+w.axiom+'</span>'+(dn[w.dimension]||w.dimension)+'<span class="sc">'+w.score+'</span></div>';
    }
    h+='</div></div>';
  }

  // ── On-chain data
  h+='<div class="cdata">';
  h+='<div><span class="k">Source </span><span class="v">'+esc(d._raw?.source||'helius')+'</span></div>';
  h+='<div><span class="k">Latency </span><span class="v">'+(d._raw?.latencyMs||'?')+'ms</span></div>';
  h+='<div><span class="k">Supply </span><span class="v">'+(d.supply?.total?d.supply.total.toLocaleString():'?')+'</span></div>';
  h+='<div><span class="k">Holders </span><span class="v">'+(d.distribution?.holderCount?d.distribution.holderCount.toLocaleString():'?')+'</span></div>';
  h+='<div><span class="k">Top1% </span><span class="v">'+((d.distribution?.whaleConcentration||0)*100).toFixed(1)+'%</span></div>';
  h+='</div>';

  document.getElementById('result').innerHTML=h;
  // Animate ring
  setTimeout(function(){var r=document.querySelector('.ring-prog');if(r)r.style.strokeDashoffset=r.getAttribute('data-off')},60);
}

// ── Watchlist
async function loadWL(){
  try{var r=await fetch('/api/oracle/watchlist');if(!r.ok){document.getElementById('wl-st').textContent='offline';return}
    var d=await r.json();rWL(d.watchlist||[],d.recentAlerts||[])
  }catch(e){document.getElementById('wl-st').textContent='offline'}}

function rWL(items,alerts){
  var el=document.getElementById('wl-items');
  document.getElementById('wl-st').textContent=items.length+' watched';
  if(!items.length){el.innerHTML='<div class="wl-e">No tokens watched. Judge a token and add it.</div>';return}
  el.innerHTML=items.map(function(w){
    var c=vc(w.lastVerdict),ago=w.lastCheckedAt?tAgo(new Date(w.lastCheckedAt)):'';
    return '<div class="wl-i" onclick="qm(\\''+w.mint+'\\')">'+'<div class="wl-l"><span class="wl-lb">'+esc(w.label||w.mint.slice(0,6)+'..')+'</span><span class="wl-m">'+w.mint.slice(0,8)+'...'+w.mint.slice(-4)+'</span></div>'+'<div class="wl-r">'+(w.lastQScore?'<span class="wl-sc" style="color:'+c+'">'+w.lastQScore+'</span>':'')+(w.lastVerdict?'<span class="wl-vt" style="color:'+c+';background:'+c+'12">'+w.lastVerdict+'</span>':'')+(ago?'<span class="wl-tm">'+ago+'</span>':'')+'<span class="wl-x" onclick="event.stopPropagation();uwM(\\''+w.mint+'\\')">\\u00D7</span></div></div>'
  }).join('');
  var as=document.getElementById('al-sec'),ai=document.getElementById('al-items');
  if(alerts.length>0){as.style.display='block';ai.innerHTML=alerts.slice(0,5).map(function(a){return '<div class="al-i"><strong>'+esc(a.label||a.mint?.slice(0,8))+'</strong>: '+esc(a.message||a.alertType)+'<span style="float:right;color:var(--muted)">'+tAgo(new Date(a.createdAt))+'</span></div>'}).join('')}else{as.style.display='none'}
}

async function watchC(){if(!cur)return;var l=document.querySelector('.vtok')?.textContent?.split('(')[0]?.trim()||null;await fetch('/api/oracle/watch',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({mint:cur,label:l})});loadWL()}
async function uwM(m){await fetch('/api/oracle/watch?mint='+encodeURIComponent(m),{method:'DELETE'});loadWL()}
function tAgo(d){var s=Math.floor((Date.now()-d.getTime())/1000);if(s<60)return s+'s';if(s<3600)return Math.floor(s/60)+'m';if(s<86400)return Math.floor(s/3600)+'h';return Math.floor(s/86400)+'d'}

loadWL();setInterval(loadWL,60000);
var p=new URLSearchParams(window.location.search);if(p.get('mint')){document.getElementById('mint').value=p.get('mint');judgeToken()}
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
