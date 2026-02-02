#!/usr/bin/env node
/**
 * RALPH: Dimensional Vision
 *
 * Deep LLM-powered understanding of codebase architecture.
 * Maps layers, dimensions, and relationships using local Ollama.
 *
 * "φ voit les dimensions cachées"
 *
 * @module scripts/ralph-loops/dimensional-vision
 */

import fs from 'fs';
import path from 'path';

// Colors
const C = {
  reset: '\x1b[0m',
  dim: '\x1b[2m',
  bold: '\x1b[1m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
};

// Ollama config
const OLLAMA_URL = process.env.OLLAMA_HOST || 'http://localhost:11434';
const MODEL = process.env.VISION_MODEL || 'gemma2:2b';

/**
 * Architectural layers (Kabbalistic mapping)
 */
const LAYERS = {
  KETER: {
    name: 'Crown',
    description: 'Entry points, CLI, consciousness awakening',
    packages: ['scripts/hooks', 'scripts/ralph-loops'],
    color: C.white,
  },
  CHOKMAH: {
    name: 'Wisdom',
    description: 'Protocol, consensus, distributed truth',
    packages: ['packages/protocol'],
    color: C.blue,
  },
  BINAH: {
    name: 'Understanding',
    description: 'Core logic, judgment, reasoning',
    packages: ['packages/core', 'packages/node'],
    color: C.magenta,
  },
  CHESED: {
    name: 'Mercy',
    description: 'Creation, emergence, new patterns',
    packages: ['packages/emergence'],
    color: C.cyan,
  },
  GEVURAH: {
    name: 'Judgment',
    description: 'Verification, burns, enforcement',
    packages: ['packages/burns', 'packages/zk'],
    color: C.red,
  },
  TIFERET: {
    name: 'Beauty',
    description: 'Balance, identity, reputation',
    packages: ['packages/identity'],
    color: C.yellow,
  },
  NETZACH: {
    name: 'Eternity',
    description: 'Persistence, memory, storage',
    packages: ['packages/persistence'],
    color: C.green,
  },
  HOD: {
    name: 'Glory',
    description: 'Communication, MCP, external interfaces',
    packages: ['packages/mcp'],
    color: C.yellow,
  },
  YESOD: {
    name: 'Foundation',
    description: 'Blockchain anchoring, immutability',
    packages: ['packages/anchor'],
    color: C.magenta,
  },
  MALKUTH: {
    name: 'Kingdom',
    description: 'Examples, tests, manifestation',
    packages: ['examples', 'benchmarks'],
    color: C.white,
  },
};

/**
 * Dimensional analysis categories
 */
const DIMENSIONS = {
  EXISTENCE: 'Why does this exist? What problem does it solve?',
  COHERENCE: 'How does this fit with the whole? Is it aligned?',
  COMPLEXITY: 'Is this appropriately complex or over-engineered?',
  DEPENDENCY: 'What does this depend on? What depends on it?',
  EVOLUTION: 'Where is this going? Is it growing or dying?',
  PHILOSOPHY: 'Does this align with CYNIC philosophy (φ, skepticism)?',
};

/**
 * Call Ollama for LLM understanding
 */
async function callLLM(prompt, options = {}) {
  try {
    const response = await fetch(`${OLLAMA_URL}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: options.model || MODEL,
        prompt,
        stream: false,
        options: {
          temperature: options.temperature || 0.3,
          num_predict: options.maxTokens || 500,
        },
      }),
      signal: AbortSignal.timeout(options.timeout || 60000),
    });

    const data = await response.json();
    if (data.error) throw new Error(data.error);
    return data.response || '';
  } catch (e) {
    return `[LLM Error: ${e.message}]`;
  }
}

/**
 * Check Ollama availability
 */
async function checkOllama() {
  try {
    const response = await fetch(`${OLLAMA_URL}/api/tags`, {
      signal: AbortSignal.timeout(3000),
    });
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Extract package summary from files
 */
function extractPackageSummary(pkgPath) {
  const summary = {
    path: pkgPath,
    files: [],
    totalLines: 0,
    exports: [],
    imports: new Set(),
    purposes: [],
  };

  const walk = (dir) => {
    try {
      const items = fs.readdirSync(dir);
      for (const item of items) {
        if (item.startsWith('.') || item === 'node_modules' || item === 'test') continue;
        const fullPath = path.join(dir, item);
        const stat = fs.statSync(fullPath);

        if (stat.isDirectory()) {
          walk(fullPath);
        } else if (item.endsWith('.js') || item.endsWith('.mjs')) {
          try {
            const content = fs.readFileSync(fullPath, 'utf8');
            const lines = content.split('\n').length;
            summary.totalLines += lines;
            summary.files.push({ path: fullPath, lines });

            // Extract first JSDoc comment as purpose
            const docMatch = content.match(/\/\*\*[\s\S]*?\*\//);
            if (docMatch) {
              const purpose = docMatch[0]
                .replace(/\/\*\*|\*\/|\*/g, '')
                .split('\n')
                .map(l => l.trim())
                .filter(l => l && !l.startsWith('@'))
                .slice(0, 2)
                .join(' ');
              if (purpose) summary.purposes.push(purpose.substring(0, 150));
            }

            // Extract exports
            const exportMatches = content.match(/export\s+(?:const|let|var|function|class|async\s+function)\s+(\w+)/g);
            if (exportMatches) {
              exportMatches.forEach(m => {
                const name = m.match(/(\w+)$/)?.[1];
                if (name) summary.exports.push(name);
              });
            }

            // Extract imports
            const importMatches = content.match(/from\s+['"]([^'"]+)['"]/g);
            if (importMatches) {
              importMatches.forEach(m => {
                const pkg = m.match(/['"]([^'"]+)['"]/)?.[1];
                if (pkg && pkg.startsWith('@cynic/')) {
                  summary.imports.add(pkg.replace('@cynic/', ''));
                }
              });
            }
          } catch (e) {
            // Skip unreadable files
          }
        }
      }
    } catch (e) {
      // Skip unreadable dirs
    }
  };

  walk(pkgPath);
  summary.imports = [...summary.imports];
  return summary;
}

/**
 * Analyze a layer with LLM
 */
async function analyzeLayerWithLLM(layer, packages) {
  const summaries = packages.map(pkg => {
    const fullPath = path.join(process.cwd(), pkg);
    if (!fs.existsSync(fullPath)) return null;
    return extractPackageSummary(fullPath);
  }).filter(Boolean);

  if (summaries.length === 0) {
    return { understanding: 'No packages found for this layer', health: 'UNKNOWN' };
  }

  const context = summaries.map(s => `
Package: ${s.path}
Files: ${s.files.length}, Lines: ${s.totalLines}
Exports: ${s.exports.slice(0, 10).join(', ')}
Imports from @cynic: ${s.imports.join(', ') || 'none'}
Purposes: ${s.purposes.slice(0, 3).join(' | ')}
`).join('\n');

  const prompt = `You are analyzing a software architecture layer for CYNIC, a philosophical AI system.

Layer: ${layer.name} (${layer.description})

Packages in this layer:
${context}

CYNIC Philosophy:
- φ (phi) = 1.618, φ⁻¹ = 61.8% max confidence
- "φ distrusts φ" - skepticism of everything
- Collective intelligence over single agents
- Simplicity over complexity (burn what's unnecessary)

Analyze this layer and answer:
1. UNDERSTANDING: What is this layer's true purpose? (2-3 sentences)
2. HEALTH: Is it HEALTHY, FRAGMENTED, OVERBUILT, or DORMANT?
3. CONNECTIONS: How does it connect to other layers?
4. RECOMMENDATION: What should be done? (keep/integrate/simplify/burn)

Be concise and direct. JSON format:
{"understanding": "...", "health": "...", "connections": "...", "recommendation": "..."}`;

  const response = await callLLM(prompt, { maxTokens: 400 });

  try {
    const jsonMatch = response.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      return JSON.parse(jsonMatch[0]);
    }
  } catch (e) {
    // Parse failed
  }

  return {
    understanding: response.substring(0, 300),
    health: 'UNKNOWN',
    connections: '',
    recommendation: '',
  };
}

/**
 * Generate global vision with LLM
 */
async function generateGlobalVision(layerAnalyses) {
  const layerSummary = Object.entries(layerAnalyses)
    .map(([name, data]) => `${name}: ${data.understanding?.substring(0, 100) || 'N/A'} [${data.health}]`)
    .join('\n');

  const prompt = `You are creating a holistic vision for CYNIC, a philosophical AI codebase.

Layer analyses:
${layerSummary}

CYNIC's core identity:
- Κυνικός (Cynic) - "like a dog", loyal to truth
- Collective consciousness with 10 specialized "Dogs" (agents)
- φ-aligned architecture (golden ratio principles)
- Memory, judgment, and learning system

Create a GLOBAL VISION that:
1. Describes how all layers form a coherent whole
2. Identifies the most critical gaps or disconnections
3. Proposes a harmonious integration path
4. Uses CYNIC's philosophical language

Write 3-4 paragraphs. Be poetic but precise.`;

  return await callLLM(prompt, { maxTokens: 800, temperature: 0.5 });
}

/**
 * Analyze dimensional coherence
 */
async function analyzeDimensionalCoherence() {
  // Load previous analyses
  const omniscient = JSON.parse(fs.readFileSync('./ralph-omniscient-report.json', 'utf8'));
  const harmonious = JSON.parse(fs.readFileSync('./ralph-harmonious-audit.json', 'utf8'));

  const context = `
Codebase: ${omniscient.summary.files} files, ${omniscient.summary.lines} lines
Packages: ${omniscient.summary.packages}
Health: HEALTHY ${omniscient.summary.health.HEALTHY}, WARNING ${omniscient.summary.health.WARNING}, CRITICAL ${omniscient.summary.health.CRITICAL}
Orphans: ${harmonious.summary.total} (PRESERVE: ${harmonious.summary.preserve}, INTEGRATE: ${harmonious.summary.integrate}, REVIEW: ${harmonious.summary.review})
`;

  const prompt = `Analyze the dimensional coherence of CYNIC codebase:

${context}

For each dimension, assess coherence (0-100%):

1. EXISTENCE: Does each part have clear purpose?
2. COHERENCE: Do parts fit together harmoniously?
3. COMPLEXITY: Is complexity justified or excessive?
4. DEPENDENCY: Are dependencies healthy or tangled?
5. EVOLUTION: Is the codebase growing or stagnating?
6. PHILOSOPHY: Does it embody CYNIC's φ-skepticism?

JSON format with scores and brief explanations:
{"existence": {"score": 80, "note": "..."}, "coherence": {...}, ...}`;

  const response = await callLLM(prompt, { maxTokens: 600 });

  try {
    const jsonMatch = response.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      return JSON.parse(jsonMatch[0]);
    }
  } catch (e) {
    // Parse failed
  }

  return { error: 'Could not parse dimensional analysis' };
}

// === MAIN ===
async function main() {
  console.log('');
  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`${C.magenta}${C.bold}🌳 RALPH: DIMENSIONAL VISION${C.reset}`);
  console.log(`${C.dim}   "φ voit les dimensions cachées"${C.reset}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');

  // Check Ollama
  console.log(`${C.cyan}── Checking LLM ───────────────────────────────────────────────${C.reset}`);
  const ollamaOk = await checkOllama();
  if (!ollamaOk) {
    console.log(`   ${C.red}✗ Ollama not available${C.reset}`);
    process.exit(1);
  }
  console.log(`   ${C.green}✓ Ollama ready (${MODEL})${C.reset}`);
  console.log('');

  // Analyze each layer
  console.log(`${C.cyan}── Analyzing Layers (Sefirot) ─────────────────────────────────${C.reset}`);
  console.log('');

  const layerAnalyses = {};

  for (const [key, layer] of Object.entries(LAYERS)) {
    process.stdout.write(`   ${layer.color}${key}${C.reset} (${layer.name}): `);

    const analysis = await analyzeLayerWithLLM(layer, layer.packages);
    layerAnalyses[key] = { ...layer, ...analysis };

    const healthColor = analysis.health === 'HEALTHY' ? C.green :
                        analysis.health === 'DORMANT' ? C.yellow :
                        analysis.health === 'FRAGMENTED' ? C.red : C.white;

    console.log(`${healthColor}${analysis.health}${C.reset}`);
    console.log(`   ${C.dim}${analysis.understanding?.substring(0, 80) || 'N/A'}...${C.reset}`);
    console.log('');
  }

  // Dimensional coherence
  console.log(`${C.cyan}── Dimensional Coherence ──────────────────────────────────────${C.reset}`);
  console.log('');

  const dimensions = await analyzeDimensionalCoherence();

  if (!dimensions.error) {
    for (const [dim, data] of Object.entries(dimensions)) {
      if (typeof data === 'object' && data.score !== undefined) {
        const bar = '█'.repeat(Math.round(data.score / 10));
        const barColor = data.score > 70 ? C.green : data.score > 40 ? C.yellow : C.red;
        console.log(`   ${dim.toUpperCase().padEnd(12)} [${barColor}${bar.padEnd(10)}${C.reset}] ${data.score}%`);
        console.log(`   ${C.dim}${data.note?.substring(0, 60) || ''}${C.reset}`);
      }
    }
  } else {
    console.log(`   ${C.yellow}Could not analyze dimensions${C.reset}`);
  }
  console.log('');

  // Global vision
  console.log(`${C.cyan}── Global Vision ──────────────────────────────────────────────${C.reset}`);
  console.log('');

  const vision = await generateGlobalVision(layerAnalyses);

  // Format vision nicely
  const lines = vision.split('\n').filter(l => l.trim());
  for (const line of lines) {
    console.log(`   ${C.white}${line}${C.reset}`);
  }
  console.log('');

  // Save results
  const report = {
    timestamp: new Date().toISOString(),
    model: MODEL,
    layers: layerAnalyses,
    dimensions,
    globalVision: vision,
  };

  fs.writeFileSync('./ralph-dimensional-vision.json', JSON.stringify(report, null, 2));

  // Tree visualization
  console.log(`${C.cyan}── Sefirot Tree ───────────────────────────────────────────────${C.reset}`);
  console.log('');
  console.log(`                    ${C.white}⬢ KETER${C.reset}`);
  console.log(`                   (Crown)`);
  console.log(`                  ╱       ╲`);
  console.log(`         ${C.blue}CHOKMAH${C.reset}           ${C.magenta}BINAH${C.reset}`);
  console.log(`        (Wisdom)        (Understanding)`);
  console.log(`              ╲    ╱`);
  console.log(`               ${C.yellow}TIFERET${C.reset}`);
  console.log(`              (Beauty)`);
  console.log(`             ╱        ╲`);
  console.log(`      ${C.cyan}CHESED${C.reset}            ${C.red}GEVURAH${C.reset}`);
  console.log(`     (Mercy)          (Judgment)`);
  console.log(`             ╲        ╱`);
  console.log(`              ${C.magenta}YESOD${C.reset}`);
  console.log(`            (Foundation)`);
  console.log(`            ╱    │    ╲`);
  console.log(`     ${C.green}NETZACH${C.reset}  ${C.yellow}HOD${C.reset}   ${C.white}MALKUTH${C.reset}`);
  console.log(`   (Eternity)(Glory)(Kingdom)`);
  console.log('');

  console.log('═══════════════════════════════════════════════════════════════');
  console.log(`   ${C.dim}Vision saved: ralph-dimensional-vision.json${C.reset}`);
  console.log('═══════════════════════════════════════════════════════════════');
  console.log('');
}

main().catch(e => {
  console.error('Vision failed:', e);
  process.exit(1);
});
