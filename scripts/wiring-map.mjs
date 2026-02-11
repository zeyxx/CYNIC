#!/usr/bin/env node
/**
 * CYNIC Wiring Truth Map
 *
 * Parses all .js/.mjs files and extracts the complete event wiring graph.
 * Detects: publishers, subscribers, orphan events, ghost listeners, broken chains.
 *
 * "Le chien renifle chaque fil" — κυνικός
 *
 * Usage:
 *   node scripts/wiring-map.mjs                    # Full terminal report
 *   node scripts/wiring-map.mjs --json              # JSON output
 *   node scripts/wiring-map.mjs --orphans           # Orphan events only
 *   node scripts/wiring-map.mjs --ghosts            # Ghost listeners only
 *   node scripts/wiring-map.mjs --chains            # Broken chains only
 *   node scripts/wiring-map.mjs --dead              # All dead code
 *   node scripts/wiring-map.mjs --event code:decision  # Single event detail
 *   node scripts/wiring-map.mjs --file solana-actor # Events in a file
 *   node scripts/wiring-map.mjs --save              # Save JSON to ~/.cynic/wiring-map.json
 *
 * @module scripts/wiring-map
 */

import { readFileSync, writeFileSync, readdirSync, mkdirSync, existsSync } from 'fs';
import { resolve, relative, join } from 'path';
import { homedir } from 'os';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG
// ═══════════════════════════════════════════════════════════════════════════

const ROOT = resolve(__dirname, '..');
const SCAN_DIRS = ['packages', 'scripts/hooks'];
const SKIP_DIRS = ['node_modules', 'dist', 'coverage', '.git', 'test'];
const EXTENSIONS = ['.js', '.mjs', '.cjs'];

// ═══════════════════════════════════════════════════════════════════════════
// FILE DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════

function discoverFiles() {
  const files = [];

  function walk(dir) {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch { return; }

    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!SKIP_DIRS.some(s => entry.name === s)) walk(fullPath);
      } else if (entry.isFile() && EXTENSIONS.some(e => entry.name.endsWith(e))) {
        files.push(fullPath);
      }
    }
  }

  for (const d of SCAN_DIRS) {
    walk(resolve(ROOT, d));
  }
  return files;
}

// ═══════════════════════════════════════════════════════════════════════════
// REGEX PATTERNS — extract event names from source
// ═══════════════════════════════════════════════════════════════════════════

const PATTERNS = {
  // globalEventBus.publish('event:name') or globalEventBus.publish(EventType.FOO)
  globalPublish: /globalEventBus\.publish\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,
  globalSubscribe: /globalEventBus\.subscribe\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,
  globalOn: /globalEventBus\.on\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,
  globalEmit: /globalEventBus\.emit\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,

  // Local EventEmitter: this.emit('event') / this.on('event')
  localEmit: /this\.emit\(\s*['"]([^'"]+)['"]/g,
  localOn: /this\.on\(\s*['"]([^'"]+)['"]/g,

  // AgentEventBus
  agentPublish: /(?:agentEventBus|AgentEventBus)\.(?:publish|emit)\(\s*(?:AgentEvents\.(\w+)|['"]([^'"]+)['"])/g,
  agentSubscribe: /(?:agentEventBus|AgentEventBus)\.(?:subscribe|on)\(\s*(?:AgentEvents\.(\w+)|['"]([^'"]+)['"])/g,

  // Automation bus: getEventBus()
  autoPublish: /getEventBus\(\)\.publish\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,
  autoSubscribe: /getEventBus\(\)\.(?:subscribe|on)\(\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g,

  // Instance bus: this.eventBus (may be globalEventBus, getEventBus(), or AgentEventBus)
  // Group 1 captures the FULL constant ref (e.g. SolanaEventType.SLOT_CHANGE)
  instancePublish: /this\.eventBus\.(?:publish|emit)\(\s*(?:(\w+EventType\.\w+|EventType\.\w+)|['"]([^'"]+)['"])/g,
  instanceSubscribe: /this\.eventBus\.(?:subscribe|on)\(\s*(?:(\w+EventType\.\w+|EventType\.\w+)|['"]([^'"]+)['"])/g,

  // EventType enum definition
  eventTypeDef: /(\w+)\s*:\s*['"]([^'"]+)['"]/g,
};

// ═══════════════════════════════════════════════════════════════════════════
// PARSER
// ═══════════════════════════════════════════════════════════════════════════

function parseFile(filePath) {
  let content;
  try {
    content = readFileSync(filePath, 'utf8');
  } catch { return null; }

  const rel = relative(ROOT, filePath).replace(/\\/g, '/');
  const result = { file: rel, publishes: [], subscribes: [] };

  function lineOf(match) {
    const idx = content.indexOf(match);
    if (idx === -1) return 0;
    return content.substring(0, idx).split('\n').length;
  }

  function extract(patternKey, bus, direction) {
    const regex = PATTERNS[patternKey];
    const re = new RegExp(regex.source, regex.flags);
    let m;
    while ((m = re.exec(content)) !== null) {
      const constName = m[1];
      const literal = m[2];
      let event;
      if (patternKey.startsWith('local')) {
        event = m[1];
      } else if (patternKey.startsWith('instance') && constName) {
        // Instance patterns: group 1 is full ref like SolanaEventType.SLOT_CHANGE
        event = constName;
      } else {
        event = literal || `EventType.${constName}`;
      }
      const line = lineOf(m[0]);
      const entry = { event, bus, line, file: rel };

      if (direction === 'pub') result.publishes.push(entry);
      else result.subscribes.push(entry);
    }
  }

  // Global bus
  extract('globalPublish', 'global', 'pub');
  extract('globalEmit', 'global', 'pub');
  extract('globalSubscribe', 'global', 'sub');
  extract('globalOn', 'global', 'sub');

  // Automation bus
  extract('autoPublish', 'automation', 'pub');
  extract('autoSubscribe', 'automation', 'sub');

  // Agent bus
  extract('agentPublish', 'agent', 'pub');
  extract('agentSubscribe', 'agent', 'sub');

  // Local emitters
  extract('localEmit', 'local', 'pub');
  extract('localOn', 'local', 'sub');

  // Instance bus (this.eventBus) — only count if file uses globalEventBus
  const usesGlobalBus = /import\s+.*globalEventBus/.test(content) || /this\.eventBus\s*=.*globalEventBus/.test(content);
  if (usesGlobalBus) {
    extract('instancePublish', 'global', 'pub');
    extract('instanceSubscribe', 'global', 'sub');
  }

  // Parameter bus: bus.emit('EVENT') / bus.publish('event') where bus is a function param
  // e.g. circuit-breaker.js: wireEvents(breaker, bus, serviceType) { bus.emit('CIRCUIT_OPENED', ...) }
  // Only count if the function signature takes 'bus' as parameter
  const hasBusParam = /function\s+\w+\([^)]*\bbus\b[^)]*\)/.test(content);
  if (hasBusParam) {
    const paramBusRe = /\bbus\.(emit|publish)\(\s*['"]([^'"]+)['"]/g;
    let m;
    while ((m = paramBusRe.exec(content)) !== null) {
      const event = m[2];
      const line = lineOf(m[0]);
      result.publishes.push({ event, bus: 'global', line, file: rel });
    }
  }

  // Detect dynamic eventType from XxxEventType enum lookup patterns:
  // const eventType = FilesystemEventType[...] || `perception:fs:${action}`;
  // this.eventBus.publish(eventType, ...)
  // If a file defines an EventType enum AND uses this.eventBus.publish(eventType),
  // all enum values are potential publications.
  if (usesGlobalBus) {
    const hasDynamicPublish = /this\.eventBus\.(publish|emit)\(\s*eventType\b/.test(content);
    if (hasDynamicPublish) {
      // Find which EventType enum is used in the eventType assignment
      const enumLookupMatch = content.match(/=\s*(\w+EventType)\[/);
      if (enumLookupMatch) {
        const enumName = enumLookupMatch[1];
        const enumBlockRe = new RegExp('export const ' + enumName + '\\s*=\\s*\\{([\\s\\S]*?)\\};');
        const enumBlockMatch = content.match(enumBlockRe);
        if (enumBlockMatch) {
          const defRe = /(\w+)\s*:\s*['"]([^'"]+)['"]/g;
          let dm;
          while ((dm = defRe.exec(enumBlockMatch[1])) !== null) {
            const dynamicLine = lineOf('this.eventBus.publish(eventType') || lineOf('this.eventBus.emit(eventType');
            result.publishes.push({ event: dm[2], bus: 'global', line: dynamicLine, file: rel });
          }
        }
      }
    }
  }

  return result;
}

// ═══════════════════════════════════════════════════════════════════════════
// EVENT TYPE RESOLUTION — map EventType.FOO → 'foo:bar'
// ═══════════════════════════════════════════════════════════════════════════

function buildEventTypeMap(filePath) {
  let content;
  try {
    content = readFileSync(filePath, 'utf8');
  } catch { return {}; }

  const map = {};
  const blockMatch = content.match(/export const EventType\s*=\s*\{([\s\S]*?)\};/);
  if (!blockMatch) return map;

  const re = new RegExp(PATTERNS.eventTypeDef.source, PATTERNS.eventTypeDef.flags);
  let m;
  while ((m = re.exec(blockMatch[1])) !== null) {
    map[`EventType.${m[1]}`] = m[2];
  }
  return map;
}

/**
 * Build maps from ALL `export const XxxEventType = { ... }` blocks across all files.
 * Resolves SolanaEventType.SLOT_CHANGE, FilesystemEventType.CHANGE, CoreEventType.X, etc.
 */
function buildAllConstantMaps(files) {
  const maps = {};
  const enumPattern = /export const (\w+EventType)\s*=\s*\{([\s\S]*?)\};/g;
  const defRe = /(\w+)\s*:\s*['"]([^'"]+)['"]/g;

  for (const filePath of files) {
    let content;
    try { content = readFileSync(filePath, 'utf8'); } catch { continue; }

    let blockMatch;
    const re = new RegExp(enumPattern.source, enumPattern.flags);
    while ((blockMatch = re.exec(content)) !== null) {
      const enumName = blockMatch[1]; // e.g. SolanaEventType
      const body = blockMatch[2];
      let m;
      const defReLocal = new RegExp(defRe.source, defRe.flags);
      while ((m = defReLocal.exec(body)) !== null) {
        maps[`${enumName}.${m[1]}`] = m[2];
      }
    }
  }

  // Also alias CoreEventType → EventType (they import from the same source)
  for (const [key, val] of Object.entries(maps)) {
    if (key.startsWith('CoreEventType.')) {
      maps[key.replace('CoreEventType.', 'EventType.')] = val;
    }
  }

  return maps;
}

// ═══════════════════════════════════════════════════════════════════════════
// GRAPH BUILDER
// ═══════════════════════════════════════════════════════════════════════════

function buildGraph(allResults, eventTypeMap) {
  const graph = {
    events: {},
    buses: { global: { pubs: 0, subs: 0 }, automation: { pubs: 0, subs: 0 }, agent: { pubs: 0, subs: 0 }, instance: { pubs: 0, subs: 0 }, local: { pubs: 0, subs: 0 } },
    files: {},
  };

  function res(name) { return eventTypeMap[name] || name; }

  for (const result of allResults) {
    if (!result) continue;

    graph.files[result.file] = {
      publishes: result.publishes.map(p => res(p.event)),
      subscribes: result.subscribes.map(s => res(s.event)),
    };

    for (const pub of result.publishes) {
      const event = res(pub.event);
      if (!graph.events[event]) graph.events[event] = { publishers: [], subscribers: [], bus: new Set() };
      graph.events[event].publishers.push({ file: pub.file, line: pub.line, bus: pub.bus });
      graph.events[event].bus.add(pub.bus);
      if (graph.buses[pub.bus]) graph.buses[pub.bus].pubs++;
    }

    for (const sub of result.subscribes) {
      const event = res(sub.event);
      if (!graph.events[event]) graph.events[event] = { publishers: [], subscribers: [], bus: new Set() };
      graph.events[event].subscribers.push({ file: sub.file, line: sub.line, bus: sub.bus });
      graph.events[event].bus.add(sub.bus);
      if (graph.buses[sub.bus]) graph.buses[sub.bus].subs++;
    }
  }

  // Convert bus Sets to arrays for JSON
  for (const data of Object.values(graph.events)) {
    data.bus = [...data.bus];
  }

  return graph;
}

// ═══════════════════════════════════════════════════════════════════════════
// ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════

function analyze(graph) {
  const orphans = [];
  const ghosts = [];
  const healthy = [];
  const localOnly = [];

  for (const [event, data] of Object.entries(graph.events)) {
    // Skip pure local EventEmitter events
    if (data.bus.length === 1 && data.bus[0] === 'local') {
      localOnly.push({ event, pubs: data.publishers.length, subs: data.subscribers.length });
      continue;
    }

    // Filter to non-local entries for analysis
    const pubCount = data.publishers.filter(p => p.bus !== 'local').length;
    const subCount = data.subscribers.filter(s => s.bus !== 'local').length;

    if (pubCount > 0 && subCount === 0) {
      orphans.push({ event, bus: data.bus, publishers: data.publishers.filter(p => p.bus !== 'local') });
    } else if (pubCount === 0 && subCount > 0) {
      ghosts.push({ event, bus: data.bus, subscribers: data.subscribers.filter(s => s.bus !== 'local') });
    } else if (pubCount > 0 && subCount > 0) {
      healthy.push({ event, bus: data.bus, pubs: pubCount, subs: subCount });
    }
  }

  const chains = detectBrokenChains(graph);

  return { orphans, ghosts, healthy, localOnly, chains };
}

function detectBrokenChains(graph) {
  const EXPECTED = [
    // CODE pipeline: PERCEIVE → JUDGE → DECIDE → ACT → LEARN
    { from: 'judgment:created', to: 'code:decision', via: 'CodeDecider' },
    { from: 'code:decision', to: 'code:action', via: 'CodeActor' },
    { from: 'code:action', to: 'accounting:update', via: 'CodeAccountant' },

    // COSMOS pipeline
    { from: 'pattern:detected', to: 'cosmos:judgment', via: 'CosmosJudge' },
    { from: 'cosmos:judgment', to: 'cosmos:decision', via: 'CosmosDecider' },
    { from: 'cosmos:decision', to: 'cosmos:action', via: 'CosmosActor' },

    // SOLANA pipeline
    { from: 'perception:solana:slot', to: 'solana:judgment', via: 'SolanaJudge' },
    { from: 'solana:judgment', to: 'solana:decision', via: 'SolanaDecider' },
    { from: 'solana:decision', to: 'solana:action', via: 'SolanaActor' },

    // SOCIAL pipeline
    { from: 'social:capture', to: 'social:judgment', via: 'SocialJudge' },

    // CYNIC self pipeline
    { from: 'cynic:state', to: 'cynic:judgment', via: 'CynicJudge' },
    { from: 'cynic:decision', to: 'cynic:action', via: 'CynicActor' },

    // HUMAN pipeline
    { from: 'user:feedback', to: 'pattern:learned', via: 'LearningManager' },
    { from: 'human:action', to: 'accounting:update', via: 'HumanAccountant' },

    // Cross-cutting
    { from: 'calibration:drift:detected', to: 'pattern:detected', via: 'CalibrationTracker' },
  ];

  const broken = [];
  const connected = [];

  for (const chain of EXPECTED) {
    const fromEvent = graph.events[chain.from];
    const toEvent = graph.events[chain.to];

    const fromPublished = fromEvent?.publishers.length > 0;

    // Check: does any subscriber of 'from' also publish 'to' (= bridge)?
    const bridgeExists = fromEvent?.subscribers.some(sub => {
      const fileData = graph.files[sub.file];
      return fileData?.publishes.includes(chain.to);
    });

    if (!fromPublished) {
      broken.push({ ...chain, reason: `'${chain.from}' has no publishers` });
    } else if (bridgeExists) {
      connected.push(chain);
    } else {
      broken.push({ ...chain, reason: `No bridge: '${chain.from}' subscribers don't publish '${chain.to}'` });
    }
  }

  return { broken, connected, total: EXPECTED.length };
}

// ═══════════════════════════════════════════════════════════════════════════
// DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

function displayFull(graph, analysis) {
  const { orphans, ghosts, healthy, localOnly, chains } = analysis;
  const total = Object.keys(graph.events).length;
  const nonLocal = healthy.length + orphans.length + ghosts.length;

  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('  CYNIC WIRING TRUTH MAP');
  console.log('  "Le chien renifle chaque fil" — κυνικός');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
  console.log(`  Events discovered: ${total} (${nonLocal} bus + ${localOnly.length} local)`);
  console.log(`  ├── Global bus:     ${graph.buses.global.pubs} pub / ${graph.buses.global.subs} sub`);
  console.log(`  ├── Automation bus: ${graph.buses.automation.pubs} pub / ${graph.buses.automation.subs} sub`);
  console.log(`  ├── Agent bus:      ${graph.buses.agent.pubs} pub / ${graph.buses.agent.subs} sub`);
  console.log(`  └── Local emitters: ${graph.buses.local.pubs} pub / ${graph.buses.local.subs} sub`);
  console.log('');
  console.log(`  ✅ Healthy:       ${healthy.length}`);
  console.log(`  ❌ Orphans:       ${orphans.length}  (published, nobody listens)`);
  console.log(`  👻 Ghosts:        ${ghosts.length}  (subscribed, nobody publishes)`);
  console.log(`  ⛓️  Chains:        ${chains.connected.length}/${chains.total} connected, ${chains.broken.length} broken`);

  if (orphans.length > 0) {
    console.log('');
    console.log('── ORPHAN EVENTS (published → void) ──────────────────────────');
    for (const o of orphans.sort((a, b) => a.event.localeCompare(b.event))) {
      console.log(`  ❌ ${o.event}  [${o.bus.join(',')}]`);
      for (const p of o.publishers) {
        console.log(`     └── ${p.file}:${p.line}`);
      }
    }
  }

  if (ghosts.length > 0) {
    console.log('');
    console.log('── GHOST LISTENERS (listening → silence) ─────────────────────');
    for (const g of ghosts.sort((a, b) => a.event.localeCompare(b.event))) {
      console.log(`  👻 ${g.event}  [${g.bus.join(',')}]`);
      for (const s of g.subscribers) {
        console.log(`     └── ${s.file}:${s.line}`);
      }
    }
  }

  if (chains.broken.length > 0) {
    console.log('');
    console.log('── BROKEN CHAINS ─────────────────────────────────────────────');
    for (const c of chains.broken) {
      console.log(`  ⛓️‍💥 ${c.from} → ${c.to}  (via ${c.via})`);
      console.log(`     ${c.reason}`);
    }
  }

  if (chains.connected.length > 0) {
    console.log('');
    console.log('── CONNECTED CHAINS ──────────────────────────────────────────');
    for (const c of chains.connected) {
      console.log(`  ✅ ${c.from} → ${c.to}  (via ${c.via})`);
    }
  }

  if (healthy.length > 0) {
    console.log('');
    console.log('── HEALTHY WIRING ────────────────────────────────────────────');
    for (const h of healthy.sort((a, b) => a.event.localeCompare(b.event))) {
      console.log(`  ✅ ${h.event}  [${h.bus.join(',')}]  ${h.pubs}→${h.subs}`);
    }
  }

  const healthScore = nonLocal > 0
    ? Math.round((healthy.length / nonLocal) * 100)
    : 0;
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`  Wiring Health: ${healthScore}%  (φ⁻¹ target: 62%)`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
}

function displayOrphans(analysis) {
  if (analysis.orphans.length === 0) { console.log('No orphan events.'); return; }
  console.log(`\n${analysis.orphans.length} orphan events:\n`);
  for (const o of analysis.orphans) {
    console.log(`  ❌ ${o.event}`);
    for (const p of o.publishers) console.log(`     └── ${p.file}:${p.line}`);
  }
}

function displayGhosts(analysis) {
  if (analysis.ghosts.length === 0) { console.log('No ghost listeners.'); return; }
  console.log(`\n${analysis.ghosts.length} ghost listeners:\n`);
  for (const g of analysis.ghosts) {
    console.log(`  👻 ${g.event}`);
    for (const s of g.subscribers) console.log(`     └── ${s.file}:${s.line}`);
  }
}

function displayChains(analysis) {
  const { chains } = analysis;
  console.log(`\nChains: ${chains.connected.length}/${chains.total} connected\n`);
  for (const c of chains.connected) console.log(`  ✅ ${c.from} → ${c.to}`);
  if (chains.broken.length > 0) {
    console.log('');
    for (const c of chains.broken) console.log(`  ⛓️‍💥 ${c.from} → ${c.to}  (${c.reason})`);
  }
}

function displayDead(analysis) {
  const { orphans, ghosts, chains } = analysis;
  const total = orphans.length + ghosts.length + chains.broken.length;
  console.log(`\n${total} dead wiring issues:\n`);
  if (orphans.length) {
    console.log(`  Orphans (${orphans.length}):`);
    for (const o of orphans) console.log(`    ❌ ${o.event}`);
  }
  if (ghosts.length) {
    console.log(`  Ghosts (${ghosts.length}):`);
    for (const g of ghosts) console.log(`    👻 ${g.event}`);
  }
  if (chains.broken.length) {
    console.log(`  Broken chains (${chains.broken.length}):`);
    for (const c of chains.broken) console.log(`    ⛓️‍💥 ${c.from} → ${c.to}`);
  }
}

function displayEventDetail(graph, eventName, eventTypeMap) {
  const resolved = eventTypeMap[eventName] || eventName;
  const data = graph.events[resolved] || graph.events[eventName];

  if (!data) {
    const matches = Object.keys(graph.events).filter(e => e.includes(eventName));
    if (matches.length === 0) { console.log(`Event '${eventName}' not found.`); return; }
    console.log(`Partial matches for '${eventName}':\n`);
    for (const m of matches) {
      const d = graph.events[m];
      console.log(`  ${m}  [pub:${d.publishers.length} sub:${d.subscribers.length}]`);
    }
    return;
  }

  console.log(`\n  Event: ${resolved}`);
  console.log(`  Bus:   ${data.bus.join(', ')}`);
  console.log('');
  if (data.publishers.length === 0) {
    console.log('  Publishers: NONE');
  } else {
    console.log(`  Publishers (${data.publishers.length}):`);
    for (const p of data.publishers) console.log(`    <- ${p.file}:${p.line}`);
  }
  console.log('');
  if (data.subscribers.length === 0) {
    console.log('  Subscribers: NONE');
  } else {
    console.log(`  Subscribers (${data.subscribers.length}):`);
    for (const s of data.subscribers) console.log(`    -> ${s.file}:${s.line}`);
  }
}

function displayFileDetail(graph, fileName) {
  const matches = Object.entries(graph.files).filter(([f]) => f.includes(fileName));
  if (matches.length === 0) { console.log(`No files matching '${fileName}'.`); return; }

  for (const [file, data] of matches) {
    console.log(`\n  ${file}`);
    const pubs = [...new Set(data.publishes)].sort();
    const subs = [...new Set(data.subscribes)].sort();
    if (pubs.length) { console.log(`  Publishes (${pubs.length}):`); for (const e of pubs) console.log(`    -> ${e}`); }
    if (subs.length) { console.log(`  Subscribes (${subs.length}):`); for (const e of subs) console.log(`    <- ${e}`); }
    if (!pubs.length && !subs.length) console.log('  (no events)');
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════

const args = process.argv.slice(2);
const flags = new Set(args.filter(a => a.startsWith('--')));
const positional = args.filter(a => !a.startsWith('--'));

// 1. Discover
const files = discoverFiles();

// 2. EventType maps
const coreMap = buildEventTypeMap(resolve(ROOT, 'packages/core/src/bus/event-bus.js'));
const autoMap = buildEventTypeMap(resolve(ROOT, 'packages/node/src/services/event-bus.js'));
const customMaps = buildAllConstantMaps(files);
const eventTypeMap = { ...coreMap, ...autoMap, ...customMaps };

// 3. Parse
const results = files.map(f => parseFile(f)).filter(Boolean);

// 4. Build graph
const graph = buildGraph(results, eventTypeMap);

// 4b. Inject EventBusBridge synthetic publishers
// The EventBusBridge forwards agent bus events → core bus events.
// These are real publishers but invisible to static analysis.
// Read the AGENT_TO_CORE mapping and inject synthetic "publisher" entries.
try {
  const bridgePath = resolve(ROOT, 'packages/node/src/services/event-bus-bridge.js');
  const bridgeContent = readFileSync(bridgePath, 'utf8');
  const bridgeRel = 'packages/node/src/services/event-bus-bridge.js';

  // Extract AGENT_TO_CORE entries: [AgentEvent.FOO]: CoreEventType.BAR or 'literal'
  const atcBlock = bridgeContent.match(/const AGENT_TO_CORE\s*=\s*\{([\s\S]*?)\};/);
  if (atcBlock) {
    const entryRe = /\]:\s*(?:CoreEventType\.(\w+)|['"]([^'"]+)['"])/g;
    let m;
    while ((m = entryRe.exec(atcBlock[1])) !== null) {
      const coreConstant = m[1] ? `EventType.${m[1]}` : null;
      const coreEvent = coreConstant ? (eventTypeMap[coreConstant] || coreConstant) : m[2];

      if (!graph.events[coreEvent]) {
        graph.events[coreEvent] = { publishers: [], subscribers: [], bus: ['global'] };
      }
      // Only add if not already a publisher (avoid double counting)
      const alreadyPublished = graph.events[coreEvent].publishers.some(p => p.file === bridgeRel);
      if (!alreadyPublished) {
        graph.events[coreEvent].publishers.push({ file: bridgeRel, line: 0, bus: 'global' });
        if (!graph.events[coreEvent].bus.includes('global')) {
          graph.events[coreEvent].bus.push('global');
        }
        graph.buses.global.pubs++;
      }
    }
  }
} catch { /* bridge file not found — skip */ }

// 4c. Inject inter-process bridge (hooks → HTTP → CollectivePack → globalEventBus)
// CollectivePack.receiveHookEvent() maps hookType → EventType and publishes to globalEventBus.
// These are real publications invisible to static analysis because they cross process boundaries.
try {
  const collectivePath = resolve(ROOT, 'packages/node/src/agents/collective/index.js');
  const collectiveContent = readFileSync(collectivePath, 'utf8');
  const collectiveRel = 'packages/node/src/agents/collective/index.js';

  // Detect hookType → EventType mapping pattern:
  // hookType === 'SessionStart' ? EventType.SESSION_STARTED : ...
  const hookMappingRe = /hookType\s*===?\s*['"](\w+)['"]\s*\?\s*(?:EventType\.(\w+)|['"]([^'"]+)['"])/g;
  let m;
  const injected = new Set();
  while ((m = hookMappingRe.exec(collectiveContent)) !== null) {
    const coreConstant = m[2] ? `EventType.${m[2]}` : null;
    const coreEvent = coreConstant ? (eventTypeMap[coreConstant] || coreConstant) : m[3];
    if (!coreEvent || injected.has(coreEvent)) continue;
    injected.add(coreEvent);

    if (!graph.events[coreEvent]) {
      graph.events[coreEvent] = { publishers: [], subscribers: [], bus: ['global'] };
    }
    const already = graph.events[coreEvent].publishers.some(p => p.file === collectiveRel);
    if (!already) {
      graph.events[coreEvent].publishers.push({ file: collectiveRel, line: 0, bus: 'global' });
      if (!graph.events[coreEvent].bus.includes('global')) {
        graph.events[coreEvent].bus.push('global');
      }
      graph.buses.global.pubs++;
    }
  }

  // Also detect automation bus publications in receiveHookEvent:
  // getEventBus().publish('HOOK_PRE_TOOL', ...) or EventTypes.HOOK_PRE_TOOL
  const autoHookRe = /getEventBus\(\)\.publish\(\s*(?:EventTypes\.(\w+)|['"]([^'"]+)['"])/g;
  while ((m = autoHookRe.exec(collectiveContent)) !== null) {
    const autoConstant = m[1] ? `EventTypes.${m[1]}` : null;
    const autoEvent = autoConstant ? (eventTypeMap[autoConstant] || autoConstant) : m[2];
    if (!autoEvent || injected.has(autoEvent)) continue;
    injected.add(autoEvent);

    if (!graph.events[autoEvent]) {
      graph.events[autoEvent] = { publishers: [], subscribers: [], bus: ['automation'] };
    }
    const already = graph.events[autoEvent].publishers.some(p => p.file === collectiveRel);
    if (!already) {
      graph.events[autoEvent].publishers.push({ file: collectiveRel, line: 0, bus: 'automation' });
      if (!graph.events[autoEvent].bus.includes('automation')) {
        graph.events[autoEvent].bus.push('automation');
      }
    }
  }
} catch { /* collective not found — skip */ }

// 5. Analyze
const analysis = analyze(graph);

// 6. Output
if (flags.has('--json')) {
  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    summary: {
      totalEvents: Object.keys(graph.events).length,
      healthy: analysis.healthy.length,
      orphans: analysis.orphans.length,
      ghosts: analysis.ghosts.length,
      brokenChains: analysis.chains.broken.length,
      connectedChains: analysis.chains.connected.length,
      buses: graph.buses,
    },
    orphans: analysis.orphans,
    ghosts: analysis.ghosts,
    healthy: analysis.healthy,
    chains: analysis.chains,
  }, null, 2));
} else if (flags.has('--orphans')) {
  displayOrphans(analysis);
} else if (flags.has('--ghosts')) {
  displayGhosts(analysis);
} else if (flags.has('--chains')) {
  displayChains(analysis);
} else if (flags.has('--dead')) {
  displayDead(analysis);
} else if (flags.has('--event') && positional[0]) {
  displayEventDetail(graph, positional[0], eventTypeMap);
} else if (flags.has('--file') && positional[0]) {
  displayFileDetail(graph, positional[0]);
} else {
  displayFull(graph, analysis);
}

// Save
if (flags.has('--save')) {
  const cynicDir = join(homedir(), '.cynic');
  if (!existsSync(cynicDir)) mkdirSync(cynicDir, { recursive: true });
  const outPath = join(cynicDir, 'wiring-map.json');
  writeFileSync(outPath, JSON.stringify({
    timestamp: new Date().toISOString(),
    summary: {
      totalEvents: Object.keys(graph.events).length,
      healthy: analysis.healthy.length,
      orphans: analysis.orphans.length,
      ghosts: analysis.ghosts.length,
      brokenChains: analysis.chains.broken.length,
      connectedChains: analysis.chains.connected.length,
    },
    orphans: analysis.orphans.map(o => ({ event: o.event, bus: o.bus, publishers: o.publishers.map(p => `${p.file}:${p.line}`) })),
    ghosts: analysis.ghosts.map(g => ({ event: g.event, bus: g.bus, subscribers: g.subscribers.map(s => `${s.file}:${s.line}`) })),
    chains: analysis.chains,
    healthy: analysis.healthy,
  }, null, 2));
  console.log(`\nSaved to: ${outPath}`);
}
