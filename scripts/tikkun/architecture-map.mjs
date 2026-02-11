#!/usr/bin/env node
/**
 * CYNIC Architecture Map Generator
 *
 * Scans REAL code to produce verified system diagrams.
 * No documentation — only what exists in the codebase.
 *
 * "Le chien cartographie le territoire, pas la carte" - CYNIC
 *
 * Outputs:
 *   - docs/architecture/system-map.md (Mermaid diagrams, auto-generated)
 *   - docs/architecture/system-map.json (machine-readable topology)
 *
 * Usage:
 *   node scripts/tikkun/architecture-map.mjs          # Full map
 *   node scripts/tikkun/architecture-map.mjs --json   # JSON only
 *
 * @module tikkun/architecture-map
 */

import { fileURLToPath } from 'url';
import { dirname, join, relative, basename } from 'path';
import { existsSync, readFileSync, writeFileSync, mkdirSync, readdirSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..', '..');

// Colors
const C = {
  reset: '\x1b[0m', red: '\x1b[31m', green: '\x1b[32m',
  yellow: '\x1b[33m', blue: '\x1b[34m', cyan: '\x1b[36m',
  dim: '\x1b[2m', bold: '\x1b[1m',
};

// ═══════════════════════════════════════════════════════════════════════════════
// 1. PACKAGE TOPOLOGY — what packages exist and their real dependencies
// ═══════════════════════════════════════════════════════════════════════════════

function scanPackages() {
  const pkgDir = join(ROOT, 'packages');
  const packages = [];

  for (const name of readdirSync(pkgDir)) {
    const pkgJsonPath = join(pkgDir, name, 'package.json');
    if (!existsSync(pkgJsonPath)) continue;

    const pkg = JSON.parse(readFileSync(pkgJsonPath, 'utf8'));
    const srcDir = join(pkgDir, name, 'src');
    const testDir = join(pkgDir, name, 'test');

    const srcFiles = existsSync(srcDir) ? countFiles(srcDir, '.js') : 0;
    const testFiles = existsSync(testDir) ? countFiles(testDir, '.test.js') : 0;
    const deps = Object.keys(pkg.dependencies || {}).filter(d => d.startsWith('@cynic/'));
    const optDeps = Object.keys(pkg.optionalDependencies || {});
    const exports = pkg.exports ? Object.keys(pkg.exports) : [];

    packages.push({
      name: pkg.name, shortName: name, version: pkg.version,
      srcFiles, testFiles, internalDeps: deps, optionalDeps: optDeps,
      exports: exports.length,
    });
  }
  return packages;
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. EVENT BUS WIRING — who publishes what, who subscribes to what
// ═══════════════════════════════════════════════════════════════════════════════

function scanEventWiring() {
  const wiring = { publishers: [], subscribers: [], bridges: [] };

  const patterns = [
    { regex: /globalEventBus\.emit\(\s*(?:EventType\.(\w+)|'([^']+)'|"([^"]+)")/g, type: 'publish', bus: 'core' },
    { regex: /globalEventBus\.on\(\s*(?:EventType\.(\w+)|'([^']+)'|"([^"]+)")/g, type: 'subscribe', bus: 'core' },
    { regex: /getEventBus\(\)\.(?:emit|publish)\(\s*(?:'([^']+)'|"([^"]+)")/g, type: 'publish', bus: 'automation' },
    { regex: /getEventBus\(\)\.(?:on|subscribe)\(\s*(?:'([^']+)'|"([^"]+)")/g, type: 'subscribe', bus: 'automation' },
    { regex: /this\.eventBus\.(?:emit|publish)\(\s*(?:'([^']+)'|"([^"]+)")/g, type: 'publish', bus: 'agent' },
    { regex: /this\.eventBus\.(?:on|subscribe)\(\s*(?:'([^']+)'|"([^"]+)")/g, type: 'subscribe', bus: 'agent' },
  ];

  const srcDirs = ['packages/core/src', 'packages/node/src'];

  for (const dir of srcDirs) {
    const absDir = join(ROOT, dir);
    if (!existsSync(absDir)) continue;

    walkFiles(absDir, '.js', (filePath) => {
      const content = readFileSync(filePath, 'utf8');
      const relPath = relative(ROOT, filePath).replace(/\\/g, '/');

      for (const { regex, type, bus } of patterns) {
        regex.lastIndex = 0;
        let match;
        while ((match = regex.exec(content)) !== null) {
          const event = match[1] || match[2] || match[3];
          if (!event) continue;
          const entry = { event, file: relPath, bus, type };
          if (type === 'publish') wiring.publishers.push(entry);
          else wiring.subscribers.push(entry);
        }
      }

      if (content.includes('EventBusBridge') || content.includes('eventBusBridge')) {
        wiring.bridges.push({ file: relPath });
      }
    });
  }
  return wiring;
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. DAEMON TOPOLOGY — endpoints, handlers, hook delegation
// ═══════════════════════════════════════════════════════════════════════════════

function scanDaemon() {
  const daemon = { endpoints: [], hookHandlers: [], thinHooks: [], standaloneHooks: [] };

  // Express routes
  const indexPath = join(ROOT, 'packages/node/src/daemon/index.js');
  if (existsSync(indexPath)) {
    const content = readFileSync(indexPath, 'utf8');
    const routeRegex = /this\.app\.(get|post|put|delete)\(\s*'([^']+)'/g;
    let match;
    while ((match = routeRegex.exec(content)) !== null) {
      daemon.endpoints.push({ method: match[1].toUpperCase(), path: match[2] });
    }
  }

  // Hook handler switch cases
  const handlersPath = join(ROOT, 'packages/node/src/daemon/hook-handlers.js');
  if (existsSync(handlersPath)) {
    const content = readFileSync(handlersPath, 'utf8');
    const caseRegex = /case '(\w+)':/g;
    let match;
    while ((match = caseRegex.exec(content)) !== null) {
      daemon.hookHandlers.push(match[1]);
    }
  }

  // hooks-thin.json
  const thinPath = join(ROOT, '.claude/hooks/hooks-thin.json');
  if (existsSync(thinPath)) {
    const thinConfig = JSON.parse(readFileSync(thinPath, 'utf8'));
    for (const [event, matchers] of Object.entries(thinConfig.hooks || {})) {
      for (const m of matchers) {
        for (const hook of (m.hooks || [])) {
          const cmd = hook.command || '';
          if (cmd.includes('thin/')) daemon.thinHooks.push({ event, command: cmd });
          else daemon.standaloneHooks.push({ event, command: cmd });
        }
      }
    }
  }
  return daemon;
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. CYCLE MAPPING — trace the ONE cycle through real code
// ═══════════════════════════════════════════════════════════════════════════════

function scanCycleImplementations() {
  const cycle = {
    PERCEIVE: [], JUDGE: [], DECIDE: [], ACT: [], LEARN: [], ACCOUNT: [], EMERGE: [],
  };

  const nodeDir = join(ROOT, 'packages/node/src');
  const domains = ['code', 'solana', 'social', 'cosmos', 'cynic', 'market', 'symbiosis'];

  for (const domain of domains) {
    const domainDir = join(nodeDir, domain);
    if (!existsSync(domainDir)) continue;

    for (const file of readdirSync(domainDir)) {
      if (!file.endsWith('.js')) continue;
      const name = file.toLowerCase();
      const entry = { domain, file: `${domain}/${file}` };

      if (name.includes('perceive') || name.includes('perception') || name.includes('watcher')) cycle.PERCEIVE.push(entry);
      else if (name.includes('judge')) cycle.JUDGE.push(entry);
      else if (name.includes('decider') || name.includes('decide') || name.includes('governance')) cycle.DECIDE.push(entry);
      else if (name.includes('actor') || name.includes('act')) cycle.ACT.push(entry);
      else if (name.includes('learn')) cycle.LEARN.push(entry);
      else if (name.includes('account')) cycle.ACCOUNT.push(entry);
      else if (name.includes('emerge')) cycle.EMERGE.push(entry);
    }
  }

  // Cross-domain dirs
  const crossMap = {
    judge: 'JUDGE', orchestration: 'DECIDE', learning: 'LEARN',
    accounting: 'ACCOUNT', emergence: 'EMERGE', routing: 'DECIDE',
  };
  for (const [dir, defaultPhase] of Object.entries(crossMap)) {
    const fullDir = join(nodeDir, dir);
    if (!existsSync(fullDir)) continue;

    for (const file of readdirSync(fullDir)) {
      if (!file.endsWith('.js')) continue;
      const name = file.toLowerCase();
      let phase = defaultPhase;

      if (name.includes('judge') || name.includes('dimension') || name.includes('residual')) phase = 'JUDGE';
      else if (name.includes('learn') || name.includes('sona') || name.includes('thompson') || name.includes('behavior')) phase = 'LEARN';
      else if (name.includes('account') || name.includes('cost') || name.includes('ledger')) phase = 'ACCOUNT';
      else if (name.includes('emerge') || name.includes('detect')) phase = 'EMERGE';

      cycle[phase].push({ domain: 'cross', file: `${dir}/${file}` });
    }
  }

  // Factories
  const cycleDir = join(nodeDir, 'cycle');
  if (existsSync(cycleDir)) {
    for (const file of readdirSync(cycleDir)) {
      if (!file.endsWith('.js') || !file.startsWith('create-')) continue;
      const phase = file.replace('create-', '').replace('.js', '').toUpperCase();
      if (cycle[phase]) cycle[phase].push({ domain: 'factory', file: `cycle/${file}` });
    }
  }

  return cycle;
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. 7x7 MATRIX REALITY CHECK — count real files per cell
// ═══════════════════════════════════════════════════════════════════════════════

function scan7x7Matrix() {
  const realities = ['CODE', 'SOLANA', 'MARKET', 'SOCIAL', 'HUMAN', 'CYNIC', 'COSMOS'];
  const analyses = ['PERCEIVE', 'JUDGE', 'DECIDE', 'ACT', 'LEARN', 'ACCOUNT', 'EMERGE'];
  const nodeDir = join(ROOT, 'packages/node/src');

  const matrix = {};
  for (const r of realities) {
    matrix[r] = {};
    for (const a of analyses) {
      matrix[r][a] = { files: 0, lines: 0 };
    }
  }

  const domainMap = {
    CODE: 'code', SOLANA: 'solana', MARKET: 'market',
    SOCIAL: 'social', HUMAN: 'symbiosis', CYNIC: 'cynic', COSMOS: 'cosmos',
  };

  const phaseKeywords = {
    PERCEIVE: ['perceive', 'perception', 'watcher', 'monitor', 'sensor'],
    JUDGE: ['judge', 'dimension', 'score', 'evaluate', 'residual'],
    DECIDE: ['decider', 'decide', 'governance', 'router', 'pipeline'],
    ACT: ['actor', 'act', 'execute', 'action', 'deploy'],
    LEARN: ['learn', 'sona', 'thompson', 'meta-cognition', 'behavior', 'q-learning'],
    ACCOUNT: ['account', 'cost', 'ledger', 'budget', 'economic'],
    EMERGE: ['emerge', 'emergence', 'detect', 'pattern', 'transcend'],
  };

  for (const [reality, dirName] of Object.entries(domainMap)) {
    const domainDir = join(nodeDir, dirName);
    if (!existsSync(domainDir)) continue;

    for (const file of readdirSync(domainDir)) {
      if (!file.endsWith('.js')) continue;
      const lines = readFileSync(join(domainDir, file), 'utf8').split('\n').length;
      const name = file.toLowerCase();

      for (const [phase, keywords] of Object.entries(phaseKeywords)) {
        if (keywords.some(kw => name.includes(kw))) {
          matrix[reality][phase].files++;
          matrix[reality][phase].lines += lines;
        }
      }
    }
  }

  return { matrix, realities, analyses };
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. SINGLETON MAP
// ═══════════════════════════════════════════════════════════════════════════════

function scanSingletons() {
  const singletons = [];
  const collectivePath = join(ROOT, 'packages/node/src/collective-singleton.js');
  if (!existsSync(collectivePath)) return singletons;

  const content = readFileSync(collectivePath, 'utf8');
  const getterRegex = /export\s+(?:function|const)\s+(get\w+)/g;
  let match;
  while ((match = getterRegex.exec(content)) !== null) {
    singletons.push(match[1]);
  }
  return singletons;
}

// ═══════════════════════════════════════════════════════════════════════════════
// MERMAID GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

function generateMermaid(packages, wiring, daemon, cycle, matrixData, singletons) {
  const lines = [];

  // ── System Topology ──
  lines.push('## System Topology');
  lines.push('');
  lines.push('```mermaid');
  lines.push('graph TD');
  lines.push('  subgraph "Claude Code Host"');
  lines.push('    CC[Claude Code CLI]');
  lines.push('    HOOKS["Thin Hooks<br/>HTTP delegates"]');
  lines.push('    STANDALONE["Standalone Hooks<br/>platform-compat, auto-wire"]');
  lines.push('  end');
  lines.push('');
  lines.push('  subgraph "CYNIC Daemon :6180"');
  lines.push('    DAEMON[DaemonServer]');
  lines.push('    HANDLERS[Hook Handlers]');
  lines.push('    SINGLETONS["Warm Singletons<br/>' + singletons.slice(0, 5).join(', ') + '"]');
  lines.push('    WATCHDOG[Watchdog]');
  lines.push('    LLM_EP["LLM Endpoints<br/>/llm/ask, /models, /feedback"]');
  lines.push('  end');
  lines.push('');
  lines.push('  subgraph "Three Nervous Systems"');
  lines.push('    GEB["globalEventBus<br/>core"]');
  lines.push('    AEB["getEventBus<br/>automation"]');
  lines.push('    AGEB["AgentEventBus<br/>dogs"]');
  lines.push('    BRIDGE["EventBusBridge"]');
  lines.push('  end');
  lines.push('');
  lines.push('  subgraph "Remote"');
  lines.push('    MCP["CYNIC MCP<br/>Render"]');
  lines.push('    ANTHROPIC["Anthropic API"]');
  lines.push('  end');
  lines.push('');
  lines.push('  CC -->|hook events| HOOKS');
  lines.push('  CC -->|hook events| STANDALONE');
  lines.push('  HOOKS -->|HTTP POST| DAEMON');
  lines.push('  DAEMON --> HANDLERS');
  lines.push('  HANDLERS --> SINGLETONS');
  lines.push('  HANDLERS -->|emit| GEB');
  lines.push('  DAEMON --> LLM_EP');
  lines.push('  LLM_EP --> ANTHROPIC');
  lines.push('  GEB <--> BRIDGE');
  lines.push('  AEB <--> BRIDGE');
  lines.push('  AGEB <--> BRIDGE');
  lines.push('  WATCHDOG -->|monitors| DAEMON');
  lines.push('```');
  lines.push('');

  // ── Package Dependencies ──
  lines.push('## Package Dependencies');
  lines.push('');
  lines.push('```mermaid');
  lines.push('graph LR');
  for (const pkg of packages) {
    lines.push(`  ${pkg.shortName}["${pkg.shortName}<br/>${pkg.srcFiles} src, ${pkg.testFiles} tests"]`);
  }
  for (const pkg of packages) {
    for (const dep of pkg.internalDeps) {
      lines.push(`  ${pkg.shortName} --> ${dep.replace('@cynic/', '')}`);
    }
  }
  lines.push('```');
  lines.push('');

  // ── Hook Delegation ──
  lines.push('## Hook Delegation');
  lines.push('');
  lines.push('| Event | Type | Target |');
  lines.push('|-------|------|--------|');
  for (const h of daemon.thinHooks) {
    const script = basename(h.command.split(' ')[1] || '');
    lines.push(`| ${h.event} | thin | \`${script}\` -> daemon |`);
  }
  for (const h of daemon.standaloneHooks) {
    const script = basename((h.command.split(' ')[1] || '').replace(' || true', ''));
    lines.push(`| ${h.event} | standalone | \`${script}\` |`);
  }
  lines.push('');

  // ── ONE Cycle ──
  lines.push('## The ONE Cycle');
  lines.push('');
  lines.push('```mermaid');
  lines.push('graph LR');
  const phases = ['PERCEIVE', 'JUDGE', 'DECIDE', 'ACT', 'LEARN', 'ACCOUNT', 'EMERGE'];
  for (let i = 0; i < phases.length; i++) {
    const p = phases[i];
    const count = cycle[p]?.length || 0;
    lines.push(`  ${p}["${p}<br/>${count} files"]`);
    if (i > 0) lines.push(`  ${phases[i-1]} --> ${p}`);
  }
  lines.push('  EMERGE -.->|residual| PERCEIVE');
  lines.push('```');
  lines.push('');

  // Cycle detail table
  lines.push('### Cycle Implementations');
  lines.push('');
  for (const p of phases) {
    if (!cycle[p] || cycle[p].length === 0) continue;
    lines.push(`**${p}** (${cycle[p].length}):`);
    for (const impl of cycle[p]) {
      lines.push(`- \`${impl.file}\` (${impl.domain})`);
    }
    lines.push('');
  }

  // ── Event Wiring Health ──
  const uniqueEvents = new Set();
  for (const p of wiring.publishers) uniqueEvents.add(p.event);
  for (const s of wiring.subscribers) uniqueEvents.add(s.event);

  const orphanPubs = [];
  const orphanSubs = [];
  const healthy = [];

  for (const event of uniqueEvents) {
    const pubs = wiring.publishers.filter(p => p.event === event);
    const subs = wiring.subscribers.filter(s => s.event === event);
    if (pubs.length > 0 && subs.length > 0) healthy.push({ event, pubs: pubs.length, subs: subs.length });
    else if (pubs.length > 0) orphanPubs.push({ event, pubs: pubs.length });
    else orphanSubs.push({ event, subs: subs.length });
  }

  lines.push('## Event Wiring Health');
  lines.push('');
  lines.push(`| Metric | Count |`);
  lines.push(`|--------|------:|`);
  lines.push(`| Healthy (pub+sub) | ${healthy.length} |`);
  lines.push(`| Orphan publishers | ${orphanPubs.length} |`);
  lines.push(`| Orphan subscribers | ${orphanSubs.length} |`);
  lines.push(`| Total unique events | ${uniqueEvents.size} |`);
  lines.push(`| Bridge files | ${wiring.bridges.length} |`);
  const healthPct = uniqueEvents.size > 0 ? Math.round((healthy.length / uniqueEvents.size) * 100) : 0;
  lines.push(`| **Wiring health** | **${healthPct}%** |`);
  lines.push('');

  if (orphanPubs.length > 0) {
    lines.push('<details><summary>Orphan Publishers (no subscriber)</summary>');
    lines.push('');
    for (const o of orphanPubs) lines.push(`- \`${o.event}\` (${o.pubs}x)`);
    lines.push('</details>');
    lines.push('');
  }

  if (orphanSubs.length > 0) {
    lines.push('<details><summary>Orphan Subscribers (no publisher)</summary>');
    lines.push('');
    for (const o of orphanSubs) lines.push(`- \`${o.event}\` (${o.subs}x)`);
    lines.push('</details>');
    lines.push('');
  }

  // ── 7x7 Matrix ──
  lines.push('## 7x7 Matrix (verified file count)');
  lines.push('');
  lines.push('| | ' + matrixData.analyses.join(' | ') + ' | Total |');
  lines.push('|---|' + matrixData.analyses.map(() => '---:').join('|') + '|---:|');

  for (const r of matrixData.realities) {
    let total = 0;
    const cells = matrixData.analyses.map(a => {
      const cell = matrixData.matrix[r][a];
      total += cell.files;
      if (cell.files === 0) return '-';
      return `${cell.files}f/${cell.lines}L`;
    });
    lines.push(`| **${r}** | ${cells.join(' | ')} | ${total} |`);
  }
  lines.push('');

  // ── Daemon Endpoints ──
  lines.push('## Daemon Endpoints');
  lines.push('');
  lines.push('| Method | Path |');
  lines.push('|--------|------|');
  for (const ep of daemon.endpoints) {
    lines.push(`| ${ep.method} | \`${ep.path}\` |`);
  }
  lines.push('');

  return lines.join('\n');
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

function countFiles(dir, ext) {
  let count = 0;
  try {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) count += countFiles(join(dir, entry.name), ext);
      else if (entry.name.endsWith(ext)) count++;
    }
  } catch { /* skip */ }
  return count;
}

function walkFiles(dir, ext, callback) {
  try {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) walkFiles(full, ext, callback);
      else if (entry.name.endsWith(ext)) callback(full);
    }
  } catch { /* skip */ }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════════

function main() {
  const args = process.argv.slice(2);
  const jsonOnly = args.includes('--json');
  const start = Date.now();

  console.log(`${C.cyan}${C.bold}`);
  console.log('  CYNIC Architecture Map Generator');
  console.log('  "Le chien cartographie le territoire"');
  console.log(`${C.reset}`);

  console.log(`${C.dim}Scanning packages...${C.reset}`);
  const packages = scanPackages();

  console.log(`${C.dim}Scanning event wiring...${C.reset}`);
  const wiring = scanEventWiring();

  console.log(`${C.dim}Scanning daemon topology...${C.reset}`);
  const daemon = scanDaemon();

  console.log(`${C.dim}Scanning cycle implementations...${C.reset}`);
  const cycle = scanCycleImplementations();

  console.log(`${C.dim}Scanning 7x7 matrix...${C.reset}`);
  const matrixData = scan7x7Matrix();

  console.log(`${C.dim}Scanning singletons...${C.reset}`);
  const singletons = scanSingletons();

  // Build topology JSON
  const uniqueEvents = new Set([...wiring.publishers.map(p => p.event), ...wiring.subscribers.map(s => s.event)]);
  const topology = {
    generated: new Date().toISOString(),
    packages: packages.map(p => ({ name: p.name, src: p.srcFiles, tests: p.testFiles, deps: p.internalDeps })),
    wiring: { publishers: wiring.publishers.length, subscribers: wiring.subscribers.length, bridges: wiring.bridges.length, uniqueEvents: uniqueEvents.size },
    daemon: { endpoints: daemon.endpoints.length, handlers: daemon.hookHandlers, thinHooks: daemon.thinHooks.length, standalone: daemon.standaloneHooks.length },
    cycle: Object.fromEntries(Object.entries(cycle).map(([k, v]) => [k, v.length])),
    singletons,
  };

  // Output
  const outDir = join(ROOT, 'docs/architecture');
  mkdirSync(outDir, { recursive: true });

  const jsonPath = join(outDir, 'system-map.json');
  writeFileSync(jsonPath, JSON.stringify(topology, null, 2));
  console.log(`${C.green}  JSON -> ${relative(ROOT, jsonPath)}${C.reset}`);

  if (!jsonOnly) {
    const markdown = generateMermaid(packages, wiring, daemon, cycle, matrixData, singletons);
    const header = [
      '<!-- AUTO-GENERATED by scripts/tikkun/architecture-map.mjs -->',
      `<!-- Generated: ${new Date().toISOString()} -->`,
      '<!-- DO NOT EDIT MANUALLY -- run the generator to update -->',
      '', '# CYNIC System Architecture Map', '',
      `> Auto-generated from code reality. ${packages.reduce((s, p) => s + p.srcFiles, 0)} source files scanned.`, '',
    ].join('\n');

    const mdPath = join(outDir, 'system-map.md');
    writeFileSync(mdPath, header + markdown);
    console.log(`${C.green}  Mermaid -> ${relative(ROOT, mdPath)}${C.reset}`);
  }

  // Summary
  const elapsed = Date.now() - start;
  console.log('');
  console.log(`${C.bold}Summary:${C.reset}`);
  console.log(`  Packages: ${packages.length} (${packages.reduce((s, p) => s + p.srcFiles, 0)} src files)`);
  console.log(`  Wiring: ${wiring.publishers.length} pubs, ${wiring.subscribers.length} subs, ${uniqueEvents.size} events`);
  console.log(`  Daemon: ${daemon.endpoints.length} endpoints, ${daemon.hookHandlers.length} handlers`);
  console.log(`  Cycle: ${Object.values(cycle).reduce((s, v) => s + v.length, 0)} implementations`);
  console.log(`  Singletons: ${singletons.length}`);
  console.log(`  ${C.dim}Done in ${elapsed}ms${C.reset}`);
  console.log('');
  console.log(`${C.cyan}*sniff* Map updated. Territory verified.${C.reset}`);
}

main();
