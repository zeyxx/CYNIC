/**
 * HTML Reporter
 *
 * Generates HTML dashboard from CYNIC metrics.
 * Pure function - no side effects.
 *
 * @module @cynic/mcp/metrics/HtmlReporter
 */

'use strict';

const PHI_INV = 0.618033988749895;

/**
 * Generate alert HTML
 * @param {Object[]} alerts - Active alerts
 * @returns {string} Alert HTML
 */
function renderAlerts(alerts) {
  if (!alerts || alerts.length === 0) {
    return '<div class="alert alert-ok">No active alerts</div>';
  }

  return alerts.map(a => `
    <div class="alert alert-${a.level}">
      <strong>${a.level.toUpperCase()}</strong>: ${a.message}
    </div>
  `).join('');
}

/**
 * Generate metric card HTML
 * @param {string} title - Card title
 * @param {string|number} value - Main value
 * @param {string} [subtitle] - Subtitle text
 * @returns {string} Card HTML
 */
function card(title, value, subtitle = '') {
  return `
    <div class="card">
      <div class="card-title">${title}</div>
      <div class="card-value">${value}</div>
      ${subtitle ? `<div class="card-subtitle">${subtitle}</div>` : ''}
    </div>
  `;
}

/**
 * CSS styles for the dashboard
 */
const DASHBOARD_STYLES = `
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: 'SF Mono', Monaco, 'Cascadia Code', monospace;
    background: #0d1117;
    color: #c9d1d9;
    padding: 20px;
  }
  h1 { color: #58a6ff; margin-bottom: 20px; }
  h2 { color: #8b949e; margin: 20px 0 10px; font-size: 14px; text-transform: uppercase; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; }
  .card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 15px;
  }
  .card-title { color: #8b949e; font-size: 12px; margin-bottom: 8px; }
  .card-value { color: #58a6ff; font-size: 28px; font-weight: bold; }
  .card-subtitle { color: #6e7681; font-size: 11px; margin-top: 5px; }
  .alert { padding: 10px; border-radius: 4px; margin-bottom: 10px; }
  .alert-critical { background: #f8514933; border-left: 3px solid #f85149; }
  .alert-warning { background: #d2992233; border-left: 3px solid #d29922; }
  .alert-info { background: #58a6ff33; border-left: 3px solid #58a6ff; }
  .alert-ok { background: #238636aa; border-left: 3px solid #238636; }
  .phi { color: #c9b458; font-size: 12px; }
  footer { margin-top: 30px; color: #6e7681; font-size: 11px; }
`;

/**
 * Format HTML dashboard from metrics
 * @param {Object} metrics - Raw metrics object
 * @param {Object} [options={}] - Formatting options
 * @param {Object[]} [options.alerts=[]] - Active alerts
 * @param {number} [options.lastCollectMs=0] - Last collection time in ms
 * @returns {string} HTML string
 */
export function formatHtml(metrics, options = {}) {
  const { alerts = [], lastCollectMs = 0 } = options;

  return `<!DOCTYPE html>
<html>
<head>
  <title>CYNIC Dashboard</title>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>${DASHBOARD_STYLES}</style>
</head>
<body>
  <h1>🐕 CYNIC Dashboard</h1>
  <p class="phi">κυνικός - "φ distrusts φ" - Max confidence: ${(PHI_INV * 100).toFixed(1)}%</p>

  <h2>Alerts</h2>
  ${renderAlerts(alerts)}

  <h2>Judgments</h2>
  <div class="grid">
    ${card('Total Judgments', metrics.judgments?.total || 0, 'All time')}
    ${card('Avg Q-Score', (metrics.judgments?.avgQScore || 0).toFixed(1), 'Quality score')}
    ${card('WAG', metrics.judgments?.byVerdict?.WAG || 0, 'Approved')}
    ${card('HOWL', metrics.judgments?.byVerdict?.HOWL || 0, 'Excellent')}
  </div>

  <h2>Sessions</h2>
  <div class="grid">
    ${card('Active Sessions', metrics.sessions?.active || 0)}
    ${card('Total Sessions', metrics.sessions?.total || 0)}
  </div>

  <h2>PoJ Chain</h2>
  <div class="grid">
    ${card('Chain Height', metrics.chain?.height || 0, 'Current slot')}
    ${card('Total Blocks', metrics.chain?.totalBlocks || metrics.chain?.blocksCreated || 0)}
    ${card('Pending', metrics.chain?.pendingJudgments || 0, 'Awaiting batch')}
  </div>

  <h2>Cache</h2>
  <div class="grid">
    ${card('Hit Rate', ((metrics.cache?.library?.hitRate || 0) * 100).toFixed(1) + '%')}
    ${card('Hits / Misses', `${metrics.cache?.library?.hits || 0} / ${metrics.cache?.library?.misses || 0}`)}
  </div>

  <h2>Integration</h2>
  <div class="grid">
    ${card('Current Drifts', metrics.integrator?.currentDrifts || 0, `${metrics.integrator?.criticalDrifts || 0} critical`)}
    ${card('Modules Tracked', metrics.integrator?.modulesTracked || 0)}
  </div>

  <h2>The Eleven Dogs (Collective)</h2>
  <div class="grid">
    ${card('Guardian (Gevurah)', metrics.agents?.guardian?.blocks || 0, `Blocked (${metrics.agents?.guardian?.warnings || 0} warnings)`)}
    ${card('Analyst (Chesed)', metrics.agents?.analyst?.patterns || 0, 'Patterns detected')}
    ${card('Scholar (Binah)', metrics.agents?.scholar?.invocations || 0, 'Invocations')}
    ${card('Architect (Chokmah)', metrics.agents?.architect?.invocations || 0, 'Invocations')}
    ${card('Sage (Da\'at)', metrics.agents?.sage?.wisdomShared || 0, 'Wisdom shared')}
    ${card('Janitor (Yesod)', metrics.agents?.janitor?.invocations || 0, 'Invocations')}
    ${card('Scout (Netzach)', metrics.agents?.scout?.invocations || 0, 'Invocations')}
    ${card('Cartographer (Malkhut)', metrics.agents?.cartographer?.invocations || 0, 'Invocations')}
    ${card('Oracle (Tiferet)', metrics.agents?.oracle?.invocations || 0, 'Invocations')}
    ${card('Deployer (Hod)', metrics.agents?.deployer?.invocations || 0, 'Invocations')}
    ${card('CYNIC (Keter)', metrics.agents?.cynic?.eventsObserved || 0, `Events observed (${metrics.agents?.cynicState || 'dormant'})`)}
  </div>

  <h2>System</h2>
  <div class="grid">
    ${card('Uptime', Math.floor((metrics.system?.uptime || 0) / 60) + 'm')}
    ${card('Memory', Math.round((metrics.system?.memoryUsed || 0) / 1024 / 1024) + 'MB', `of ${Math.round((metrics.system?.memoryTotal || 0) / 1024 / 1024)}MB`)}
  </div>

  <footer>
    Last updated: ${new Date(metrics.timestamp || Date.now()).toISOString()} |
    Collect time: ${lastCollectMs}ms |
    phi^-1 = ${PHI_INV}
  </footer>
</body>
</html>`;
}

export default { formatHtml };
