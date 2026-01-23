/**
 * Prometheus Formatter
 *
 * Converts raw CYNIC metrics to Prometheus exposition format.
 * Pure function - no side effects.
 *
 * @module @cynic/mcp/metrics/PrometheusFormatter
 */

'use strict';

/**
 * Format metrics in Prometheus exposition format
 * @param {Object} metrics - Raw metrics object from MetricsCollector
 * @param {Object} [options={}] - Formatting options
 * @param {number} [options.alertCount=0] - Number of active alerts
 * @returns {string} Prometheus format string
 */
export function formatPrometheus(metrics, options = {}) {
  const { alertCount = 0 } = options;
  const lines = [];

  // ═══════════════════════════════════════════════════════════════════════════
  // Judgments
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_judgments_total Total number of judgments by verdict');
  lines.push('# TYPE cynic_judgments_total counter');

  const verdicts = metrics.judgments?.byVerdict || {};
  for (const [verdict, count] of Object.entries(verdicts)) {
    lines.push(`cynic_judgments_total{verdict="${verdict}"} ${count}`);
  }

  lines.push('# HELP cynic_avg_q_score Average Q-Score of all judgments');
  lines.push('# TYPE cynic_avg_q_score gauge');
  lines.push(`cynic_avg_q_score ${metrics.judgments?.avgQScore || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Sessions
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_active_sessions Number of active sessions');
  lines.push('# TYPE cynic_active_sessions gauge');
  lines.push(`cynic_active_sessions ${metrics.sessions?.active || 0}`);

  lines.push('# HELP cynic_sessions_total Total sessions created');
  lines.push('# TYPE cynic_sessions_total counter');
  lines.push(`cynic_sessions_total ${metrics.sessions?.total || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Cache (Librarian)
  // ═══════════════════════════════════════════════════════════════════════════

  if (metrics.cache?.library) {
    lines.push('# HELP cynic_library_cache_hits Total cache hits');
    lines.push('# TYPE cynic_library_cache_hits counter');
    lines.push(`cynic_library_cache_hits ${metrics.cache.library.hits}`);

    lines.push('# HELP cynic_library_cache_misses Total cache misses');
    lines.push('# TYPE cynic_library_cache_misses counter');
    lines.push(`cynic_library_cache_misses ${metrics.cache.library.misses}`);

    lines.push('# HELP cynic_library_cache_hit_rate Cache hit rate');
    lines.push('# TYPE cynic_library_cache_hit_rate gauge');
    lines.push(`cynic_library_cache_hit_rate ${metrics.cache.library.hitRate}`);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PoJ Chain
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_poj_chain_height Current PoJ chain height');
  lines.push('# TYPE cynic_poj_chain_height gauge');
  lines.push(`cynic_poj_chain_height ${metrics.chain?.height || 0}`);

  lines.push('# HELP cynic_poj_blocks_total Total PoJ blocks');
  lines.push('# TYPE cynic_poj_blocks_total counter');
  lines.push(`cynic_poj_blocks_total ${metrics.chain?.totalBlocks || metrics.chain?.blocksCreated || 0}`);

  lines.push('# HELP cynic_poj_pending_judgments Pending judgments in batch');
  lines.push('# TYPE cynic_poj_pending_judgments gauge');
  lines.push(`cynic_poj_pending_judgments ${metrics.chain?.pendingJudgments || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Ecosystem
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_ecosystem_docs_loaded Ecosystem docs loaded');
  lines.push('# TYPE cynic_ecosystem_docs_loaded gauge');
  lines.push(`cynic_ecosystem_docs_loaded ${metrics.ecosystem?.docsLoaded || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Integrator
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_integrator_drifts_current Current detected drifts');
  lines.push('# TYPE cynic_integrator_drifts_current gauge');
  lines.push(`cynic_integrator_drifts_current ${metrics.integrator?.currentDrifts || 0}`);

  lines.push('# HELP cynic_integrator_drifts_critical Critical drifts');
  lines.push('# TYPE cynic_integrator_drifts_critical gauge');
  lines.push(`cynic_integrator_drifts_critical ${metrics.integrator?.criticalDrifts || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // The Eleven Dogs (Collective)
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_dog_invocations Total invocations per dog');
  lines.push('# TYPE cynic_dog_invocations counter');

  const dogs = [
    'guardian', 'analyst', 'scholar', 'architect', 'sage',
    'janitor', 'scout', 'cartographer', 'oracle', 'deployer', 'cynic'
  ];

  for (const dog of dogs) {
    lines.push(`cynic_dog_invocations{dog="${dog}"} ${metrics.agents?.[dog]?.invocations || 0}`);
  }

  lines.push('# HELP cynic_guardian_blocks Total operations blocked by Guardian');
  lines.push('# TYPE cynic_guardian_blocks counter');
  lines.push(`cynic_guardian_blocks ${metrics.agents?.guardian?.blocks || 0}`);

  lines.push('# HELP cynic_guardian_warnings Total warnings from Guardian');
  lines.push('# TYPE cynic_guardian_warnings counter');
  lines.push(`cynic_guardian_warnings ${metrics.agents?.guardian?.warnings || 0}`);

  lines.push('# HELP cynic_analyst_patterns Patterns detected by Analyst');
  lines.push('# TYPE cynic_analyst_patterns counter');
  lines.push(`cynic_analyst_patterns ${metrics.agents?.analyst?.patterns || 0}`);

  lines.push('# HELP cynic_sage_wisdom Wisdom shared by Sage');
  lines.push('# TYPE cynic_sage_wisdom counter');
  lines.push(`cynic_sage_wisdom ${metrics.agents?.sage?.wisdomShared || 0}`);

  lines.push('# HELP cynic_collective_decisions Total collective decisions');
  lines.push('# TYPE cynic_collective_decisions counter');
  lines.push(`cynic_collective_decisions ${metrics.agents?.totalDecisions || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // System
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_uptime_seconds Server uptime in seconds');
  lines.push('# TYPE cynic_uptime_seconds gauge');
  lines.push(`cynic_uptime_seconds ${Math.floor(metrics.system?.uptime || 0)}`);

  lines.push('# HELP cynic_memory_used_bytes Heap memory used');
  lines.push('# TYPE cynic_memory_used_bytes gauge');
  lines.push(`cynic_memory_used_bytes ${metrics.system?.memoryUsed || 0}`);

  // ═══════════════════════════════════════════════════════════════════════════
  // Alerts
  // ═══════════════════════════════════════════════════════════════════════════

  lines.push('# HELP cynic_alerts_active Number of active alerts');
  lines.push('# TYPE cynic_alerts_active gauge');
  lines.push(`cynic_alerts_active ${alertCount}`);

  return lines.join('\n') + '\n';
}

export default { formatPrometheus };
