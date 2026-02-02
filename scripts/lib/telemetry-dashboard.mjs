#!/usr/bin/env node
/**
 * Telemetry Dashboard
 *
 * Displays CYNIC telemetry data in a TUI-friendly format.
 *
 * "phi mesure tout, phi apprend de tout"
 *
 * @module scripts/lib/telemetry-dashboard
 */

import { createRequire } from 'module';
const require = createRequire(import.meta.url);

// ANSI colors
const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  brightRed: '\x1b[91m',
  brightGreen: '\x1b[92m',
  brightYellow: '\x1b[93m',
  brightCyan: '\x1b[96m',
};

/**
 * Build a progress bar
 */
function bar(value, max = 100, width = 10) {
  const normalized = Math.min(1, Math.max(0, value / max));
  const filled = Math.round(normalized * width);
  return '\u2588'.repeat(filled) + '\u2591'.repeat(width - filled);
}

/**
 * Format duration in human-readable form
 */
function formatDuration(ms) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
  return `${(ms / 3600000).toFixed(1)}h`;
}

/**
 * Format severity with color
 */
function formatSeverity(severity) {
  switch (severity) {
    case 'critical': return `${C.brightRed}CRITICAL${C.reset}`;
    case 'high': return `${C.red}HIGH${C.reset}`;
    case 'medium': return `${C.yellow}MEDIUM${C.reset}`;
    case 'low': return `${C.green}LOW${C.reset}`;
    default: return severity;
  }
}

/**
 * Main dashboard function
 */
async function main() {
  let telemetry = null;

  // Try to load telemetry from persistence services
  try {
    const { getTelemetry } = await import('@cynic/persistence/services');
    telemetry = getTelemetry();
  } catch (e) {
    // Telemetry not available
  }

  // If no telemetry, show placeholder
  if (!telemetry) {
    console.log(`
${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}
${C.magenta}\ud83d\udcca CYNIC TELEMETRY${C.reset} - ${C.dim}"phi mesure tout"${C.reset}
${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}

${C.yellow}*sniff* No telemetry data yet.${C.reset}

Telemetry collection starts automatically with the session.
Run some commands and check back.

${C.dim}Telemetry tracks:${C.reset}
- LLM usage (tokens, latency, cache hits)
- Judgments (verdicts, Q-scores, confidence)
- Tool usage (success/fail, latency)
- Frictions (errors, timeouts, failures)
- Session events (start, end, actions)

${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}
`);
    return;
  }

  // Get telemetry data
  const stats = telemetry.getStats();
  const data = telemetry.export();

  // Header
  console.log(`
${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}
${C.magenta}\ud83d\udcca CYNIC TELEMETRY${C.reset} - ${C.dim}"phi mesure tout"${C.reset}
${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}
`);

  // Session section
  console.log(`${C.cyan}\u2500\u2500 SESSION \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
  console.log(`   ID: ${C.brightCyan}${stats.sessionId}${C.reset}`);
  console.log(`   Uptime: ${C.brightGreen}${formatDuration(stats.uptime)}${C.reset}`);
  console.log(`   Actions: ${C.brightYellow}${stats.actionCount}${C.reset}`);
  console.log();

  // Events by category
  if (Object.keys(stats.categories).length > 0) {
    console.log(`${C.cyan}\u2500\u2500 EVENTS BY CATEGORY \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    const maxCount = Math.max(...Object.values(stats.categories));
    for (const [category, count] of Object.entries(stats.categories).sort((a, b) => b[1] - a[1])) {
      const barStr = bar(count, maxCount, 20);
      console.log(`   ${category.padEnd(12)} [${barStr}] ${count}`);
    }
    console.log();
  }

  // Latencies
  if (Object.keys(data.timings).length > 0) {
    console.log(`${C.cyan}\u2500\u2500 LATENCY (ms) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    for (const [name, timing] of Object.entries(data.timings).slice(0, 10)) {
      const displayName = name.replace(/_histogram$/, '').slice(0, 20).padEnd(20);
      const avgColor = timing.avg < 500 ? C.green : (timing.avg < 1500 ? C.yellow : C.red);
      const p95Color = timing.p95 < 1500 ? C.green : (timing.p95 < 3000 ? C.yellow : C.red);
      const p99Color = timing.p99 < 3000 ? C.green : (timing.p99 < 5000 ? C.yellow : C.red);
      console.log(`   ${displayName} avg:${avgColor}${Math.round(timing.avg)}${C.reset} p95:${p95Color}${Math.round(timing.p95)}${C.reset} p99:${p99Color}${Math.round(timing.p99)}${C.reset}`);
    }
    console.log();
  }

  // Frictions
  if (data.frictions.length > 0) {
    console.log(`${C.cyan}\u2500\u2500 FRICTIONS \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    console.log(`   Total: ${C.yellow}${stats.frictions}${C.reset}`);
    console.log(`   Recent:`);

    // Group by severity
    const bySeverity = { critical: 0, high: 0, medium: 0, low: 0 };
    for (const f of data.frictions) {
      bySeverity[f.severity] = (bySeverity[f.severity] || 0) + 1;
    }
    console.log(`   ${C.brightRed}CRITICAL: ${bySeverity.critical}${C.reset}  ${C.red}HIGH: ${bySeverity.high}${C.reset}  ${C.yellow}MEDIUM: ${bySeverity.medium}${C.reset}  ${C.green}LOW: ${bySeverity.low}${C.reset}`);

    // Show last 5 frictions
    for (const f of data.frictions.slice(-5).reverse()) {
      const time = new Date(f.timestamp).toLocaleTimeString();
      console.log(`   ${C.dim}${time}${C.reset} [${formatSeverity(f.severity)}] ${f.name}`);
    }
    console.log();
  }

  // LLM Usage (from counters)
  const llmCounters = Object.entries(data.counters)
    .filter(([k]) => k.startsWith('llm_'))
    .reduce((acc, [k, v]) => { acc[k] = v.value; return acc; }, {});

  if (Object.keys(llmCounters).length > 0) {
    console.log(`${C.cyan}\u2500\u2500 LLM USAGE \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    const calls = llmCounters['llm_calls_total'] || 0;
    const inputTokens = llmCounters['llm_input_tokens_total'] || 0;
    const outputTokens = llmCounters['llm_output_tokens_total'] || 0;
    const cacheHits = llmCounters['llm_cache_hits'] || 0;
    const cacheRate = calls > 0 ? Math.round((cacheHits / calls) * 100) : 0;

    console.log(`   Calls: ${C.brightGreen}${calls}${C.reset}`);
    console.log(`   Tokens: ${C.brightCyan}${inputTokens.toLocaleString()}${C.reset} in / ${C.brightCyan}${outputTokens.toLocaleString()}${C.reset} out`);
    console.log(`   Cache hits: ${C.brightYellow}${cacheRate}%${C.reset}`);

    // LLM latency if available
    const llmLatency = data.timings['llm_latency'];
    if (llmLatency) {
      console.log(`   Avg latency: ${C.brightGreen}${Math.round(llmLatency.avg)}ms${C.reset}`);
    }
    console.log();
  }

  // Judgments (from counters)
  const judgmentCounters = Object.entries(data.counters)
    .filter(([k]) => k.startsWith('judgments_'))
    .reduce((acc, [k, v]) => {
      // Extract verdict from label
      const match = k.match(/verdict=(\w+)/);
      if (match) {
        acc[match[1]] = (acc[match[1]] || 0) + v.value;
      }
      return acc;
    }, {});

  if (Object.keys(judgmentCounters).length > 0) {
    console.log(`${C.cyan}\u2500\u2500 JUDGMENTS \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    const total = Object.values(judgmentCounters).reduce((a, b) => a + b, 0);
    console.log(`   Total: ${C.brightGreen}${total}${C.reset}`);
    console.log(`   By verdict:`);
    const verdictColors = {
      HOWL: C.brightGreen,
      WAG: C.green,
      GROWL: C.yellow,
      BARK: C.red,
    };
    for (const [verdict, count] of Object.entries(judgmentCounters)) {
      const color = verdictColors[verdict] || C.white;
      console.log(`     ${color}${verdict}: ${count}${C.reset}`);
    }

    // Q-Score histogram if available
    const qHist = data.histograms['judgment_qscore'];
    if (qHist) {
      console.log(`   Avg Q-Score: ${C.brightCyan}${Math.round(qHist.sum / qHist.count)}${C.reset}`);
    }

    // Confidence histogram if available
    const confHist = data.histograms['judgment_confidence'];
    if (confHist) {
      console.log(`   Avg confidence: ${C.brightYellow}${Math.round(confHist.sum / confHist.count)}%${C.reset}`);
    }
    console.log();
  }

  // System metrics (from gauges)
  const systemGauges = Object.entries(data.gauges)
    .filter(([k]) => k.startsWith('system_'))
    .reduce((acc, [k, v]) => { acc[k] = v.value; return acc; }, {});

  if (Object.keys(systemGauges).length > 0) {
    console.log(`${C.cyan}\u2500\u2500 SYSTEM \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500${C.reset}`);
    const heapUsed = systemGauges['system_memory_heap_used'] || 0;
    const heapTotal = systemGauges['system_memory_heap_total'] || 0;
    const heapPct = heapTotal > 0 ? Math.round((heapUsed / heapTotal) * 100) : 0;
    const heapColor = heapPct < 50 ? C.green : (heapPct < 80 ? C.yellow : C.red);
    console.log(`   Memory: ${heapColor}${Math.round(heapUsed)}MB${C.reset} / ${Math.round(heapTotal)}MB`);

    const cpuUser = systemGauges['system_cpu_user'] || 0;
    const cpuSystem = systemGauges['system_cpu_system'] || 0;
    console.log(`   CPU: user ${C.brightCyan}${Math.round(cpuUser)}ms${C.reset} / system ${C.brightCyan}${Math.round(cpuSystem)}ms${C.reset}`);
    console.log();
  }

  // Footer
  console.log(`${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}`);

  // CYNIC voice
  const frictionCount = stats.frictions;
  if (frictionCount === 0) {
    console.log(`   ${C.green}*sniff* Telemetry nominal. phi sees all.${C.reset}`);
  } else if (frictionCount < 5) {
    console.log(`   ${C.yellow}*sniff* ${frictionCount} frictions detected. Minor issues.${C.reset}`);
  } else {
    console.log(`   ${C.red}*concerned sniff* ${frictionCount} frictions. Review needed.${C.reset}`);
  }

  console.log(`${C.cyan}\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550${C.reset}
`);
}

main().catch(console.error);
