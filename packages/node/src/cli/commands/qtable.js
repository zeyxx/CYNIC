/**
 * cynic qtable - Q-Table Visualizer
 *
 * Visualize the Q-Learning router's learned state-action values.
 *
 * Usage:
 *   cynic qtable                   Show Q-table overview
 *   cynic qtable --heatmap         Display as heatmap
 *   cynic qtable --top <n>         Show top N state-action pairs
 *   cynic qtable --state <state>   Show values for specific state
 *   cynic qtable --export <file>   Export Q-table to JSON
 *
 * @module @cynic/node/cli/commands/qtable
 */

'use strict';

import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';

const PHI_INV = 0.618033988749895;

/**
 * Dog colors for visualization
 */
const DOG_COLORS = {
  GUARDIAN: chalk.red,
  ANALYST: chalk.cyan,
  SCHOLAR: chalk.yellow,
  SAGE: chalk.cyan,
  ARCHITECT: chalk.blue,
  ORACLE: chalk.yellow,
  JANITOR: chalk.magenta,
  SCOUT: chalk.green,
  CARTOGRAPHER: chalk.green,
  DEPLOYER: chalk.yellow,
  CYNIC: chalk.white,
};

/**
 * State feature short names
 */
const FEATURE_SHORT = {
  'task:security': 'SEC',
  'task:code_change': 'COD',
  'task:analysis': 'ANA',
  'task:exploration': 'EXP',
  'task:deployment': 'DEP',
  'task:cleanup': 'CLN',
  'task:documentation': 'DOC',
  'task:test': 'TST',
  'task:debug': 'DBG',
  'task:design': 'DES',
  'ctx:error': 'ERR',
  'ctx:urgent': 'URG',
  'ctx:complex': 'CPX',
  'ctx:simple': 'SMP',
  'tool:bash': 'BSH',
  'tool:write': 'WRT',
  'tool:edit': 'EDT',
  'tool:read': 'RD',
  'tool:task': 'TSK',
};

/**
 * Format Q-value with color
 */
function formatQValue(value, width = 6) {
  const str = value.toFixed(2).padStart(width);
  if (value >= 0.5) {
    return chalk.green(str);
  } else if (value >= 0) {
    return chalk.white(str);
  } else if (value >= -0.3) {
    return chalk.yellow(str);
  } else {
    return chalk.red(str);
  }
}

/**
 * Format state key for display
 */
function formatStateKey(stateKey, maxLen = 30) {
  if (!stateKey) return '(empty)';

  const features = stateKey.split('|');
  const shorts = features.map(f => FEATURE_SHORT[f] || f.slice(0, 3).toUpperCase());
  const result = shorts.join('+');

  if (result.length > maxLen) {
    return result.slice(0, maxLen - 3) + '...';
  }
  return result.padEnd(maxLen);
}

/**
 * Display Q-table as heatmap
 */
function displayHeatmap(qTable) {
  const dogs = [
    'GUARDIAN', 'ANALYST', 'ARCHITECT', 'SCOUT', 'SCHOLAR',
    'SAGE', 'ORACLE', 'JANITOR', 'DEPLOYER', 'CARTOGRAPHER', 'CYNIC'
  ];

  console.log(chalk.bold('\n  ── Q-Table Heatmap ──\n'));

  // Header
  const header = '  ' + 'State'.padEnd(32);
  const dogHeader = dogs.map(d => d.slice(0, 4).padStart(5)).join(' ');
  console.log(header + dogHeader);
  console.log('  ' + '─'.repeat(32 + dogs.length * 6));

  // Get entries sorted by most learned
  const entries = Array.from(qTable.table.entries())
    .map(([state, actions]) => {
      const maxQ = Math.max(...Object.values(actions));
      return { state, actions, maxQ };
    })
    .sort((a, b) => b.maxQ - a.maxQ)
    .slice(0, 15); // Top 15 states

  for (const { state, actions } of entries) {
    const stateStr = formatStateKey(state, 30);
    const values = dogs.map(dog => {
      const v = actions[dog] || 0;
      return formatQValue(v, 5);
    }).join(' ');
    console.log(`  ${stateStr} ${values}`);
  }

  if (qTable.table.size > 15) {
    console.log(chalk.gray(`\n  ... and ${qTable.table.size - 15} more states`));
  }
}

/**
 * Display top state-action pairs
 */
function displayTopPairs(qTable, count = 20) {
  console.log(chalk.bold(`\n  ── Top ${count} Learned Pairs ──\n`));

  const pairs = [];
  for (const [state, actions] of qTable.table.entries()) {
    for (const [action, value] of Object.entries(actions)) {
      pairs.push({ state, action, value });
    }
  }

  // Sort by absolute value (most learned)
  pairs.sort((a, b) => Math.abs(b.value) - Math.abs(a.value));

  for (let i = 0; i < Math.min(count, pairs.length); i++) {
    const { state, action, value } = pairs[i];
    const stateStr = formatStateKey(state, 25);
    const dogColor = DOG_COLORS[action] || chalk.gray;
    const actionStr = dogColor(action.padEnd(12));
    const valueStr = formatQValue(value, 8);
    const rank = `${i + 1}.`.padStart(4);

    console.log(`  ${chalk.gray(rank)} ${stateStr} → ${actionStr} ${valueStr}`);
  }
}

/**
 * Display values for a specific state
 */
function displayStateValues(qTable, stateQuery) {
  console.log(chalk.bold(`\n  ── Values for State: ${stateQuery} ──\n`));

  // Find matching states
  const matches = [];
  for (const [state, actions] of qTable.table.entries()) {
    if (state.includes(stateQuery) || stateQuery === '*') {
      matches.push({ state, actions });
    }
  }

  if (matches.length === 0) {
    console.log(chalk.gray('  No matching states found.'));
    console.log(chalk.gray(`\n  Available features: ${Object.keys(FEATURE_SHORT).join(', ')}`));
    return;
  }

  for (const { state, actions } of matches.slice(0, 5)) {
    console.log(chalk.gray(`  State: ${state}`));
    console.log();

    // Sort by value
    const sorted = Object.entries(actions)
      .sort((a, b) => b[1] - a[1]);

    for (const [action, value] of sorted) {
      const dogColor = DOG_COLORS[action] || chalk.gray;
      const bar = value > 0
        ? chalk.green('█'.repeat(Math.min(Math.round(value * 20), 20)))
        : chalk.red('█'.repeat(Math.min(Math.round(Math.abs(value) * 20), 20)));
      console.log(`    ${dogColor(action.padEnd(12))} ${formatQValue(value, 6)} ${bar}`);
    }
    console.log();
  }
}

/**
 * Display Q-table overview
 */
function displayOverview(qTable) {
  console.log(chalk.bold('  ── Q-Table Overview ──\n'));

  const stats = qTable.stats || {};
  console.log(`    ${chalk.gray('States learned:')} ${qTable.table.size}`);
  console.log(`    ${chalk.gray('Updates:')}        ${stats.updates || 0}`);
  console.log(`    ${chalk.gray('Visits:')}         ${qTable.visits.size}`);

  // Calculate action stats
  const actionCounts = {};
  let maxQ = -Infinity;
  let minQ = Infinity;

  for (const actions of qTable.table.values()) {
    for (const [action, value] of Object.entries(actions)) {
      actionCounts[action] = (actionCounts[action] || 0) + 1;
      if (value > maxQ) maxQ = value;
      if (value < minQ) minQ = value;
    }
  }

  console.log(`    ${chalk.gray('Max Q-value:')}    ${formatQValue(maxQ === -Infinity ? 0 : maxQ)}`);
  console.log(`    ${chalk.gray('Min Q-value:')}    ${formatQValue(minQ === Infinity ? 0 : minQ)}`);

  // Most learned dogs
  const sortedDogs = Object.entries(actionCounts)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);

  console.log(chalk.bold('\n  ── Most Learned Dogs ──'));
  for (const [dog, count] of sortedDogs) {
    const dogColor = DOG_COLORS[dog] || chalk.gray;
    console.log(`    ${dogColor(dog.padEnd(12))} ${count} state-action pairs`);
  }
}

/**
 * Try to fetch Q-table from MCP server
 */
async function fetchQTable() {
  const port = process.env.CYNIC_MCP_PORT || 3000;
  const baseUrl = `http://localhost:${port}`;

  try {
    const response = await fetch(`${baseUrl}/api/qtable`, {
      signal: AbortSignal.timeout(5000)
    });

    if (!response.ok) {
      throw new Error(`Server returned ${response.status}`);
    }

    return await response.json();
  } catch (error) {
    return null;
  }
}

/**
 * Load Q-table from file
 */
async function loadFromFile(filePath) {
  try {
    const content = await readFile(filePath, 'utf8');
    const json = JSON.parse(content);

    // Reconstruct QTable-like structure
    return {
      table: new Map(Object.entries(json.table || {})),
      visits: new Map(Object.entries(json.visits || {})),
      stats: json.stats || {},
    };
  } catch (error) {
    return null;
  }
}

/**
 * Create demo Q-table for testing
 */
function createDemoQTable() {
  const demoTable = new Map();

  // Add some demo entries
  demoTable.set('task:security', {
    GUARDIAN: 0.85,
    ANALYST: 0.3,
    SCOUT: 0.1,
  });
  demoTable.set('task:code_change', {
    ARCHITECT: 0.7,
    GUARDIAN: 0.4,
    ANALYST: 0.2,
  });
  demoTable.set('task:exploration', {
    SCOUT: 0.9,
    CARTOGRAPHER: 0.6,
    ANALYST: 0.3,
  });
  demoTable.set('task:cleanup', {
    JANITOR: 0.95,
    GUARDIAN: 0.2,
  });
  demoTable.set('task:documentation', {
    SCHOLAR: 0.8,
    SAGE: 0.5,
  });

  return {
    table: demoTable,
    visits: new Map(),
    stats: { states: 5, updates: 100 },
  };
}

/**
 * Q-table command handler
 */
export async function qtableCommand(options) {
  const { heatmap = false, top, state, verbose = false, json = false, demo = false } = options;
  const exportPath = options.export;
  const importPath = options.import;

  if (!json) {
    console.log(chalk.yellow('\n╔═════════════════════════════════════════╗'));
    console.log(chalk.yellow('║') + chalk.bold.cyan('  CYNIC Q-Table Visualizer              ') + chalk.yellow('║'));
    console.log(chalk.yellow('╚═════════════════════════════════════════╝\n'));
  }

  let qTable = null;

  // Import from file
  if (importPath) {
    qTable = await loadFromFile(importPath);
    if (!qTable) {
      console.log(chalk.red(`  ✗ Cannot read file: ${importPath}`));
      process.exit(1);
    }
    console.log(chalk.gray(`  Loaded Q-table from ${importPath}\n`));
  }

  // Demo mode
  if (demo) {
    qTable = createDemoQTable();
    console.log(chalk.gray('  Using demo Q-table for visualization\n'));
  }

  // Try server
  if (!qTable) {
    console.log(chalk.gray('  *sniff* Fetching Q-table...\n'));
    const serverData = await fetchQTable();

    if (serverData) {
      qTable = {
        table: new Map(Object.entries(serverData.table || {})),
        visits: new Map(Object.entries(serverData.visits || {})),
        stats: serverData.stats || {},
      };
    }
  }

  // Fallback message
  if (!qTable) {
    console.log(chalk.yellow('  ⚠ Q-table not available'));
    console.log(chalk.gray('  The Q-Learning router builds its table over time.'));
    console.log(chalk.gray('\n  Options:'));
    console.log(chalk.gray('    --demo    Show demo visualization'));
    console.log(chalk.gray('    --import  Load from JSON file\n'));
    return;
  }

  // Export mode
  if (exportPath) {
    const exportData = {
      table: Object.fromEntries(qTable.table),
      visits: Object.fromEntries(qTable.visits),
      stats: qTable.stats,
      exported: new Date().toISOString(),
    };
    try {
      await writeFile(exportPath, JSON.stringify(exportData, null, 2));
      console.log(chalk.green(`  ✓ Exported Q-table to ${exportPath}\n`));
    } catch (error) {
      console.log(chalk.red(`  ✗ Cannot write file: ${error.message}`));
    }
    return;
  }

  // JSON output
  if (json) {
    console.log(JSON.stringify({
      table: Object.fromEntries(qTable.table),
      visits: Object.fromEntries(qTable.visits),
      stats: qTable.stats,
    }, null, 2));
    return;
  }

  // Display modes
  if (heatmap) {
    displayHeatmap(qTable);
  } else if (top) {
    displayTopPairs(qTable, parseInt(top) || 20);
  } else if (state) {
    displayStateValues(qTable, state);
  } else {
    displayOverview(qTable);
    displayTopPairs(qTable, 10);
  }

  // Summary
  console.log(chalk.yellow('\n═════════════════════════════════════════'));
  console.log(chalk.gray('\n  Q-Learning: Q(s,a) = R + γ × max Q(s\',a\')'));
  console.log(chalk.gray('  Hyperparameters: α=0.618 (φ⁻¹), γ=0.382 (φ⁻²)'));
  console.log(chalk.cyan(`\n  φ⁻¹ = ${(PHI_INV * 100).toFixed(1)}% max confidence\n`));
}

export default { qtableCommand };
