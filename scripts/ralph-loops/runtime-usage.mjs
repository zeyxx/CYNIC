#!/usr/bin/env node
/**
 * RALPH: Runtime Usage Analysis
 *
 * Distinguishes between:
 * - WIRED: Code that's imported somewhere
 * - INTEGRATED: Code that's called in the flow
 * - USED: Code that actually runs
 * - CRITICAL: Code that's essential (100% uptime)
 *
 * "φ doute même du code importé"
 */

import fs from 'fs';
import path from 'path';

const C = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  magenta: '\x1b[35m',
};

/**
 * Categories of code usage
 */
const Usage = {
  CRITICAL: 'CRITICAL',   // Essential, always runs
  ACTIVE: 'ACTIVE',       // Runs regularly
  CONDITIONAL: 'CONDITIONAL', // Runs sometimes
  DORMANT: 'DORMANT',     // Imported but never runs
  DEAD: 'DEAD',           // Not imported anywhere
};

/**
 * Known entry points that actually run
 */
const KNOWN_ENTRY_POINTS = [
  // Hooks that run on every session
  'scripts/hooks/awaken.js',
  'scripts/hooks/observe.js',

  // MCP server (always running)
  'packages/mcp/src/server.js',
  'packages/mcp/src/tools/index.js',

  // Core node
  'packages/node/src/node.js',
];

/**
 * Known dormant features (imported but not active)
 */
const KNOWN_DORMANT = [
  // Solana - not deployed yet
  'packages/anchor/',

  // Discord/Slack - not configured
  'packages/mcp/src/discord-service.js',
  'packages/mcp/src/slack-service.js',

  // ZK proofs - not active
  'packages/zk/',

  // Graph DB - PostgreSQL used instead
  'packages/persistence/src/graph/',

  // DAG/IPFS - not active
  'packages/persistence/src/dag/',

  // Redis - not configured
  'packages/persistence/src/redis/',
];

/**
 * Trace what code actually runs from entry points
 */
function traceRuntime(entryPoint, depth = 0, visited = new Set()) {
  if (depth > 20 || visited.has(entryPoint)) return [];
  visited.add(entryPoint);

  const fullPath = path.join(process.cwd(), entryPoint);
  if (!fs.existsSync(fullPath)) return [];

  try {
    const content = fs.readFileSync(fullPath, 'utf8');
    const used = [entryPoint];

    // Find imports
    const importRegex = /(?:import|require)\s*\(?['"]([^'"]+)['"]\)?/g;
    let match;

    while ((match = importRegex.exec(content)) !== null) {
      const imp = match[1];

      // Resolve relative imports
      if (imp.startsWith('.')) {
        const dir = path.dirname(entryPoint);
        let resolved = path.join(dir, imp).replace(/\\/g, '/');
        if (!resolved.endsWith('.js')) resolved += '.js';

        if (fs.existsSync(path.join(process.cwd(), resolved))) {
          used.push(...traceRuntime(resolved, depth + 1, visited));
        }
      }
      // Resolve @cynic imports
      else if (imp.startsWith('@cynic/')) {
        const parts = imp.replace('@cynic/', '').split('/');
        const pkg = parts[0];
        const subpath = parts.slice(1).join('/') || 'src/index.js';
        const resolved = `packages/${pkg}/${subpath.includes('.') ? subpath : 'src/' + subpath + '.js'}`;

        // Try index.js for package imports
        const indexPath = `packages/${pkg}/src/index.js`;
        if (fs.existsSync(path.join(process.cwd(), indexPath))) {
          used.push(...traceRuntime(indexPath, depth + 1, visited));
        }
      }
    }

    return used;
  } catch (e) {
    return [entryPoint];
  }
}

/**
 * Check if file is in a dormant path
 */
function isDormant(filePath) {
  return KNOWN_DORMANT.some(d => filePath.includes(d));
}

/**
 * Analyze a file's actual usage
 */
function analyzeUsage(filePath, runtimeFiles, allFiles) {
  // Check if in known dormant paths
  if (isDormant(filePath)) {
    return { status: Usage.DORMANT, reason: 'In known dormant feature path' };
  }

  // Check if reached from runtime
  if (runtimeFiles.has(filePath)) {
    // Check if it's in a critical path
    const isCritical = KNOWN_ENTRY_POINTS.some(e => filePath.includes(e.split('/')[0]));
    if (isCritical) {
      return { status: Usage.CRITICAL, reason: 'In critical runtime path' };
    }
    return { status: Usage.ACTIVE, reason: 'Reached from runtime entry points' };
  }

  // Check if imported anywhere
  const importedBy = [];
  for (const [path, content] of allFiles) {
    if (content.includes(filePath.split('/').pop().replace('.js', ''))) {
      importedBy.push(path);
    }
  }

  if (importedBy.length > 0) {
    return { status: Usage.CONDITIONAL, reason: `Imported by ${importedBy.length} files but not in runtime path` };
  }

  return { status: Usage.DEAD, reason: 'Not imported anywhere' };
}

// === MAIN ===
async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`${C.magenta}🔍 RALPH: RUNTIME USAGE ANALYSIS${C.reset}`);
  console.log(`${C.dim}   "Câblé ≠ Utilisé réellement"${C.reset}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');

  // Trace runtime from entry points
  console.log(`${C.cyan}── Tracing Runtime ────────────────────────────────────────────${C.reset}`);

  const runtimeFiles = new Set();

  for (const entry of KNOWN_ENTRY_POINTS) {
    process.stdout.write(`   ${entry}: `);
    const files = traceRuntime(entry);
    files.forEach(f => runtimeFiles.add(f));
    console.log(`${C.green}${files.length} files${C.reset}`);
  }

  console.log('');
  console.log(`   Total runtime files: ${C.green}${runtimeFiles.size}${C.reset}`);
  console.log('');

  // Collect all files
  const allFiles = new Map();
  const walk = (dir) => {
    try {
      for (const item of fs.readdirSync(dir)) {
        if (item.startsWith('.') || item === 'node_modules' || item === 'dist') continue;
        const full = path.join(dir, item);
        const stat = fs.statSync(full);
        if (stat.isDirectory()) {
          walk(full);
        } else if (item.endsWith('.js') || item.endsWith('.mjs')) {
          const rel = path.relative(process.cwd(), full).replace(/\\/g, '/');
          try {
            allFiles.set(rel, fs.readFileSync(full, 'utf8'));
          } catch (e) {}
        }
      }
    } catch (e) {}
  };

  walk(path.join(process.cwd(), 'packages'));
  walk(path.join(process.cwd(), 'scripts'));

  console.log(`${C.cyan}── Analyzing Usage ────────────────────────────────────────────${C.reset}`);
  console.log('');

  // Categorize all files
  const categories = {
    [Usage.CRITICAL]: [],
    [Usage.ACTIVE]: [],
    [Usage.CONDITIONAL]: [],
    [Usage.DORMANT]: [],
    [Usage.DEAD]: [],
  };

  for (const [filePath] of allFiles) {
    const usage = analyzeUsage(filePath, runtimeFiles, allFiles);
    categories[usage.status].push({ path: filePath, reason: usage.reason });
  }

  // Display results
  const colors = {
    [Usage.CRITICAL]: C.green,
    [Usage.ACTIVE]: C.cyan,
    [Usage.CONDITIONAL]: C.yellow,
    [Usage.DORMANT]: C.magenta,
    [Usage.DEAD]: C.red,
  };

  for (const [status, files] of Object.entries(categories)) {
    const color = colors[status];
    console.log(`   ${color}${status}${C.reset}: ${files.length} files`);
  }

  console.log('');

  // Show dormant in detail
  console.log(`${C.cyan}── DORMANT Features (câblé mais pas utilisé) ─────────────────${C.reset}`);
  console.log('');

  const dormantByPath = {};
  for (const file of categories[Usage.DORMANT]) {
    const dir = file.path.split('/').slice(0, 3).join('/');
    if (!dormantByPath[dir]) dormantByPath[dir] = [];
    dormantByPath[dir].push(file.path);
  }

  for (const [dir, files] of Object.entries(dormantByPath).sort((a, b) => b[1].length - a[1].length)) {
    console.log(`   ${C.magenta}${dir}${C.reset}: ${files.length} files`);
  }

  console.log('');

  // Show dead files
  if (categories[Usage.DEAD].length > 0) {
    console.log(`${C.cyan}── DEAD Code (vraiment inutilisé) ─────────────────────────────${C.reset}`);
    console.log('');
    categories[Usage.DEAD].slice(0, 20).forEach(f => {
      console.log(`   ${C.red}✗${C.reset} ${f.path}`);
    });
    if (categories[Usage.DEAD].length > 20) {
      console.log(`   ${C.dim}... +${categories[Usage.DEAD].length - 20} more${C.reset}`);
    }
    console.log('');
  }

  // Summary
  const totalLines = {
    [Usage.CRITICAL]: 0,
    [Usage.ACTIVE]: 0,
    [Usage.CONDITIONAL]: 0,
    [Usage.DORMANT]: 0,
    [Usage.DEAD]: 0,
  };

  for (const [status, files] of Object.entries(categories)) {
    for (const file of files) {
      const content = allFiles.get(file.path) || '';
      totalLines[status] += content.split('\n').length;
    }
  }

  console.log(`${C.cyan}── SUMMARY ────────────────────────────────────────────────────${C.reset}`);
  console.log('');
  console.log(`   ${C.green}CRITICAL${C.reset}:    ${categories[Usage.CRITICAL].length} files (${totalLines[Usage.CRITICAL].toLocaleString()} lines) - Essential, always runs`);
  console.log(`   ${C.cyan}ACTIVE${C.reset}:      ${categories[Usage.ACTIVE].length} files (${totalLines[Usage.ACTIVE].toLocaleString()} lines) - Runs regularly`);
  console.log(`   ${C.yellow}CONDITIONAL${C.reset}: ${categories[Usage.CONDITIONAL].length} files (${totalLines[Usage.CONDITIONAL].toLocaleString()} lines) - Runs sometimes`);
  console.log(`   ${C.magenta}DORMANT${C.reset}:     ${categories[Usage.DORMANT].length} files (${totalLines[Usage.DORMANT].toLocaleString()} lines) - Câblé mais pas actif`);
  console.log(`   ${C.red}DEAD${C.reset}:        ${categories[Usage.DEAD].length} files (${totalLines[Usage.DEAD].toLocaleString()} lines) - Vraiment inutilisé`);
  console.log('');

  // Save report
  const report = {
    timestamp: new Date().toISOString(),
    summary: {
      critical: { files: categories[Usage.CRITICAL].length, lines: totalLines[Usage.CRITICAL] },
      active: { files: categories[Usage.ACTIVE].length, lines: totalLines[Usage.ACTIVE] },
      conditional: { files: categories[Usage.CONDITIONAL].length, lines: totalLines[Usage.CONDITIONAL] },
      dormant: { files: categories[Usage.DORMANT].length, lines: totalLines[Usage.DORMANT] },
      dead: { files: categories[Usage.DEAD].length, lines: totalLines[Usage.DEAD] },
    },
    dormantPaths: dormantByPath,
    deadFiles: categories[Usage.DEAD].map(f => f.path),
  };

  fs.writeFileSync('./ralph-runtime-usage.json', JSON.stringify(report, null, 2));

  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`   ${C.dim}Report saved: ralph-runtime-usage.json${C.reset}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
}

main().catch(console.error);
