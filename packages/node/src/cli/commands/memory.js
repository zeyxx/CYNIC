/**
 * cynic memory - Memory Inspector
 *
 * Inspect CYNIC's memory tiers and stored knowledge.
 *
 * Usage:
 *   cynic memory                   Show memory overview
 *   cynic memory --tier <tier>     Inspect specific tier (working, episodic, semantic, vector)
 *   cynic memory --search <query>  Search across memories
 *   cynic memory --stats           Show memory statistics
 *   cynic memory --gc              Run garbage collection
 *
 * @module @cynic/node/cli/commands/memory
 */

'use strict';

import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Format bytes to human readable
 */
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Format progress bar
 */
function progressBar(value, max, width = 20) {
  const ratio = max > 0 ? Math.min(value / max, 1) : 0;
  const filled = Math.round(ratio * width);
  const empty = width - filled;
  const bar = '█'.repeat(filled) + '░'.repeat(empty);
  const percent = (ratio * 100).toFixed(0);
  return `[${bar}] ${percent}%`;
}

/**
 * Display memory tier info
 */
function displayTier(name, tier, verbose = false) {
  const icon = {
    working: '⚡',
    episodic: '📝',
    semantic: '🧠',
    vector: '🔢',
  }[name.toLowerCase()] || '📦';

  console.log(chalk.bold(`\n  ${icon} ${name.charAt(0).toUpperCase() + name.slice(1)} Memory`));
  console.log(chalk.gray('  ' + '─'.repeat(40)));

  if (!tier) {
    console.log(chalk.gray('    Not available'));
    return;
  }

  // Count and capacity
  const count = tier.count || tier.size || 0;
  const capacity = tier.capacity || tier.maxSize || 0;

  if (capacity > 0) {
    console.log(`    Items:    ${count}/${capacity} ${progressBar(count, capacity, 15)}`);
  } else {
    console.log(`    Items:    ${count}`);
  }

  // Memory size if available
  if (tier.memoryBytes) {
    console.log(`    Size:     ${formatBytes(tier.memoryBytes)}`);
  }

  // TTL if available
  if (tier.ttl) {
    const ttlMin = Math.round(tier.ttl / 60000);
    console.log(`    TTL:      ${ttlMin} minutes`);
  }

  // Hit rate if available
  if (tier.hitRate !== undefined) {
    const hitPct = (tier.hitRate * 100).toFixed(1);
    console.log(`    Hit Rate: ${hitPct}%`);
  }

  // Recent items
  if (verbose && tier.recent && tier.recent.length > 0) {
    console.log(chalk.gray('\n    Recent items:'));
    for (const item of tier.recent.slice(0, 5)) {
      const preview = typeof item === 'string'
        ? item.slice(0, 50)
        : JSON.stringify(item).slice(0, 50);
      console.log(chalk.gray(`      - ${preview}...`));
    }
  }
}

/**
 * Display overall memory statistics
 */
function displayOverview(stats) {
  console.log(chalk.bold('  ── Memory Overview ──\n'));

  // Process memory
  const heapUsed = process.memoryUsage().heapUsed;
  const heapTotal = process.memoryUsage().heapTotal;
  console.log(`    ${chalk.gray('Heap Used:')}  ${formatBytes(heapUsed)} / ${formatBytes(heapTotal)}`);
  console.log(`    ${chalk.gray('Usage:')}     ${progressBar(heapUsed, heapTotal, 20)}`);

  // Tier summary if available
  if (stats.tiers) {
    console.log(chalk.bold('\n  ── Tier Summary ──'));

    for (const [name, tier] of Object.entries(stats.tiers)) {
      const count = tier.count || tier.size || 0;
      const capacity = tier.capacity || 0;
      const icon = { working: '⚡', episodic: '📝', semantic: '🧠', vector: '🔢' }[name] || '📦';
      const bar = capacity > 0 ? progressBar(count, capacity, 10) : `${count} items`;
      console.log(`    ${icon} ${name.padEnd(10)} ${bar}`);
    }
  }

  // Global stats
  if (stats.total) {
    console.log(chalk.bold('\n  ── Totals ──'));
    console.log(`    ${chalk.gray('Total Items:')}  ${stats.total.items || 0}`);
    console.log(`    ${chalk.gray('Total Size:')}   ${formatBytes(stats.total.bytes || 0)}`);
  }
}

/**
 * Try to fetch memory stats from MCP server
 */
async function fetchMemoryStats(options = {}) {
  const port = process.env.CYNIC_MCP_PORT || 3000;
  const baseUrl = `http://localhost:${port}`;

  try {
    const params = new URLSearchParams();
    if (options.tier) params.set('tier', options.tier);
    if (options.search) params.set('query', options.search);

    const url = `${baseUrl}/api/memory?${params}`;
    const response = await fetch(url, { signal: AbortSignal.timeout(5000) });

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    return await response.json();
  } catch (error) {
    return null;
  }
}

/**
 * Get local memory stats (fallback)
 */
async function getLocalMemoryStats() {
  const mem = process.memoryUsage();

  return {
    process: {
      heapUsed: mem.heapUsed,
      heapTotal: mem.heapTotal,
      external: mem.external,
      rss: mem.rss,
    },
    tiers: {
      working: { count: 0, capacity: 7, ttl: 1800000, description: 'Active task focus (Miller\'s Law)' },
      episodic: { count: 0, capacity: 500, ttl: 604800000, description: 'Complete interaction records' },
      semantic: { count: 0, capacity: 5000, description: 'Factual knowledge & patterns' },
      vector: { count: 0, capacity: 10000, description: 'Dense embeddings for similarity' },
    },
    total: {
      items: 0,
      bytes: mem.heapUsed,
    },
  };
}

/**
 * Display search results
 */
function displaySearchResults(results, query) {
  console.log(chalk.bold(`\n  ── Search Results for "${query}" ──\n`));

  if (!results || results.length === 0) {
    console.log(chalk.gray('    No results found.'));
    return;
  }

  for (let i = 0; i < Math.min(results.length, 10); i++) {
    const result = results[i];
    const tier = result.tier || 'unknown';
    const score = result.score ? `(${(result.score * 100).toFixed(0)}%)` : '';
    const preview = (result.content || result.key || JSON.stringify(result)).slice(0, 60);

    console.log(`    ${chalk.cyan(`[${tier}]`)} ${preview}... ${chalk.gray(score)}`);
  }

  if (results.length > 10) {
    console.log(chalk.gray(`\n    ... and ${results.length - 10} more results`));
  }
}

/**
 * Memory command handler
 */
export async function memoryCommand(options) {
  const { tier, search, stats = false, gc = false, verbose = false, json = false } = options;

  if (!json) {
    console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
    console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Memory Inspector                ') + chalk.yellow('║'));
    console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));
  }

  // Garbage collection
  if (gc) {
    if (global.gc) {
      const before = process.memoryUsage().heapUsed;
      global.gc();
      const after = process.memoryUsage().heapUsed;
      const freed = before - after;
      console.log(chalk.green(`  ✓ Garbage collection complete`));
      console.log(chalk.gray(`    Freed: ${formatBytes(freed)}`));
      console.log(chalk.gray(`    Heap:  ${formatBytes(after)}\n`));
    } else {
      console.log(chalk.yellow('  ⚠ Garbage collection not available'));
      console.log(chalk.gray('  Run Node.js with --expose-gc flag\n'));
    }
    return;
  }

  // Try to fetch from server
  console.log(chalk.gray('  *sniff* Inspecting memory...\n'));

  let memoryData = await fetchMemoryStats({ tier, search });

  // Fallback to local stats
  if (!memoryData) {
    memoryData = await getLocalMemoryStats();
  }

  // JSON output
  if (json) {
    console.log(JSON.stringify(memoryData, null, 2));
    return;
  }

  // Search mode
  if (search) {
    displaySearchResults(memoryData.results || [], search);
    return;
  }

  // Specific tier
  if (tier) {
    const tierData = memoryData.tiers?.[tier];
    displayTier(tier, tierData, verbose);
    return;
  }

  // Overview (default)
  displayOverview(memoryData);

  // Show all tiers
  if (memoryData.tiers) {
    for (const [name, tierData] of Object.entries(memoryData.tiers)) {
      displayTier(name, tierData, verbose);
    }
  }

  // Summary
  console.log(chalk.yellow('\n═════════════════════════════════════════'));
  console.log(chalk.gray('\n  4-Tier Architecture (φ-aligned promotion):'));
  console.log(chalk.gray('    Working  → Episodic  → Semantic → Vector'));
  console.log(chalk.gray('    (7±2)      (500)       (5000)     (10000)'));
  console.log(chalk.cyan(`\n  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% promotion threshold\n`));
}

export default { memoryCommand };
