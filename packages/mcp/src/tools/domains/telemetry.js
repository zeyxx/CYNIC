/**
 * Telemetry MCP Tools
 *
 * Exposes telemetry data through MCP for dashboards and analysis.
 *
 * @module @cynic/mcp/tools/domains/telemetry
 */

'use strict';

import { getTelemetry } from '@cynic/persistence/services';

/**
 * Get current telemetry stats
 */
async function getStats() {
  const telemetry = getTelemetry();
  return telemetry.getStats();
}

/**
 * Export full telemetry data
 */
async function exportTelemetry() {
  const telemetry = getTelemetry();
  return telemetry.export();
}

/**
 * Get friction report
 */
async function getFrictions(options = {}) {
  const telemetry = getTelemetry();
  const data = telemetry.export();

  const frictions = data.frictions || [];

  // Filter by severity if specified
  const filtered = options.severity
    ? frictions.filter(f => f.severity === options.severity)
    : frictions;

  // Group by category
  const byCategory = {};
  for (const f of filtered) {
    const cat = f.category || 'unknown';
    if (!byCategory[cat]) byCategory[cat] = [];
    byCategory[cat].push(f);
  }

  return {
    total: filtered.length,
    byCategory,
    recent: filtered.slice(-10),
  };
}

/**
 * Get latency percentiles for a metric
 */
async function getLatencyStats(metricName) {
  const telemetry = getTelemetry();
  const data = telemetry.export();

  const timing = Object.entries(data.timings)
    .find(([key]) => key.includes(metricName));

  if (!timing) {
    return { error: `No timing data for ${metricName}` };
  }

  return {
    metric: timing[0],
    ...timing[1],
  };
}

/**
 * Format telemetry as TUI dashboard
 */
function formatDashboard(data) {
  const lines = [];

  lines.push('═══════════════════════════════════════════════════════════════');
  lines.push('📊 CYNIC TELEMETRY DASHBOARD');
  lines.push('═══════════════════════════════════════════════════════════════');
  lines.push('');

  // Session info
  lines.push('── SESSION ────────────────────────────────────────────────────');
  lines.push(`   ID: ${data.sessionId}`);
  lines.push(`   Uptime: ${Math.round(data.stats.uptime / 1000)}s`);
  lines.push(`   Actions: ${data.stats.actionCount}`);
  lines.push('');

  // Event counts
  lines.push('── EVENTS ─────────────────────────────────────────────────────');
  lines.push(`   Total: ${data.stats.totalEvents}`);
  lines.push(`   Errors: ${data.stats.totalErrors}`);

  if (Object.keys(data.stats.categories).length > 0) {
    lines.push('   By Category:');
    for (const [cat, count] of Object.entries(data.stats.categories).sort((a, b) => b[1] - a[1])) {
      const bar = '█'.repeat(Math.min(20, Math.round(count / 10)));
      lines.push(`     ${cat.padEnd(12)} ${bar} ${count}`);
    }
  }
  lines.push('');

  // Latency stats
  if (Object.keys(data.timings).length > 0) {
    lines.push('── LATENCY (ms) ───────────────────────────────────────────────');
    for (const [name, timing] of Object.entries(data.timings).slice(0, 10)) {
      const shortName = name.split('{')[0].substring(0, 20);
      lines.push(`   ${shortName.padEnd(20)} avg:${timing.avg.toFixed(0).padStart(5)} p95:${timing.p95.toFixed(0).padStart(5)} p99:${timing.p99.toFixed(0).padStart(5)}`);
    }
    lines.push('');
  }

  // Frictions
  if (data.frictions.length > 0) {
    lines.push('── FRICTIONS ──────────────────────────────────────────────────');
    lines.push(`   Total: ${data.frictions.length}`);

    const recent = data.frictions.slice(-5);
    for (const f of recent) {
      const time = new Date(f.timestamp).toISOString().split('T')[1].split('.')[0];
      lines.push(`   ${time} [${f.severity}] ${f.name}`);
    }
    lines.push('');
  }

  lines.push('═══════════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * MCP tool handler
 */
export async function handleTelemetry(params) {
  const { action, options = {} } = params;

  switch (action) {
    case 'stats':
      return await getStats();

    case 'export':
      return await exportTelemetry();

    case 'frictions':
      return await getFrictions(options);

    case 'latency':
      return await getLatencyStats(options.metric || 'llm_latency');

    case 'dashboard': {
      const data = await exportTelemetry();
      return { text: formatDashboard(data) };
    }

    case 'record': {
      const telemetry = getTelemetry();
      const { type, name, value, labels } = options;

      switch (type) {
        case 'counter':
          telemetry.increment(name, value || 1, labels || {});
          break;
        case 'gauge':
          telemetry.gauge(name, value, labels || {});
          break;
        case 'timing':
          telemetry.timing(name, value, labels || {});
          break;
        case 'friction':
          telemetry.friction(name, options.severity || 'medium', labels || {});
          break;
      }
      return { recorded: true };
    }

    default:
      return { error: `Unknown action: ${action}` };
  }
}

/**
 * Tool definition
 */
export const telemetryTool = {
  name: 'brain_telemetry',
  description: 'Access CYNIC telemetry: usage stats, latencies, frictions, dashboards',
  inputSchema: {
    type: 'object',
    properties: {
      action: {
        type: 'string',
        enum: ['stats', 'export', 'frictions', 'latency', 'dashboard', 'record'],
        description: 'Action to perform',
      },
      options: {
        type: 'object',
        description: 'Action-specific options',
      },
    },
    required: ['action'],
  },
  handler: handleTelemetry,
};

export default telemetryTool;
