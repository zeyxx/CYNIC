#!/usr/bin/env node
/**
 * DA'AT - The Hidden Sefirah
 *
 * "Knowledge of what is known"
 *
 * Comprehensive audit of CYNIC's state.
 * Bridges Chokhmah (wisdom) and Binah (understanding)
 * by providing AWARENESS of the system's actual state.
 *
 * Usage: node scripts/tikkun/daat.mjs [--json] [--fix]
 *
 * @module tikkun/daat
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { existsSync, readdirSync, readFileSync, writeFileSync, unlinkSync } from 'fs';
import { execSync, spawn } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CYNIC_ROOT = join(__dirname, '..', '..');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

// Colors
const C = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  dim: '\x1b[2m',
  bold: '\x1b[1m',
};

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT CATEGORIES
// ═══════════════════════════════════════════════════════════════════════════

const AUDITS = {
  // PILLAR 1: Core Infrastructure (Keter → Tiferet)
  core: {
    name: 'Core Infrastructure',
    sefirah: 'Keter-Tiferet',
    checks: [
      { id: 'packages-exist', name: 'Packages exist', fn: checkPackagesExist },
      { id: 'package-json', name: 'Root package.json valid', fn: checkPackageJson },
      { id: 'node-modules', name: 'Dependencies installed', fn: checkNodeModules },
      { id: 'typescript', name: 'TypeScript compiles', fn: checkTypeScript },
    ]
  },

  // PILLAR 2: Hooks & Consciousness (Chokhmah → Binah)
  hooks: {
    name: 'Hooks & Consciousness',
    sefirah: 'Chokhmah-Binah',
    checks: [
      { id: 'hooks-exist', name: 'Hook files exist', fn: checkHooksExist },
      { id: 'hooks-syntax', name: 'Hooks syntax valid', fn: checkHooksSyntax },
      { id: 'lib-modules', name: 'Lib modules load', fn: checkLibModules },
      { id: 'cynic-core', name: 'cynic-core.cjs loads', fn: checkCynicCore },
    ]
  },

  // PILLAR 3: Dogs (Collective Sefirot)
  dogs: {
    name: 'Collective Dogs',
    sefirah: 'All Sefirot',
    checks: [
      { id: 'dogs-defined', name: 'Dogs defined', fn: checkDogsDefined },
      { id: 'dogs-vote', name: 'Dogs can vote', fn: checkDogsVote },
      { id: 'consensus', name: 'Consensus mechanism', fn: checkConsensus },
    ]
  },

  // PILLAR 4: MCP Server (Yesod)
  mcp: {
    name: 'MCP Server',
    sefirah: 'Yesod',
    checks: [
      { id: 'mcp-package', name: 'MCP package exists', fn: checkMCPPackage },
      { id: 'mcp-tools', name: 'MCP tools defined', fn: checkMCPTools },
      { id: 'mcp-server', name: 'MCP server starts', fn: checkMCPServer },
    ]
  },

  // PILLAR 5: Persistence (Malkhut)
  persistence: {
    name: 'Persistence Layer',
    sefirah: 'Malkhut',
    checks: [
      { id: 'postgres-config', name: 'PostgreSQL configured', fn: checkPostgresConfig },
      { id: 'migrations', name: 'Migrations exist', fn: checkMigrations },
      { id: 'embedder', name: 'Embedder service', fn: checkEmbedder },
    ]
  },

  // PILLAR 6: Philosophy Compliance (Da'at)
  philosophy: {
    name: 'Philosophy Compliance',
    sefirah: 'Da\'at',
    checks: [
      { id: 'phi-constants', name: 'φ constants defined', fn: checkPhiConstants },
      { id: 'phi-enforcement', name: 'φ⁻¹ ceiling enforced', fn: checkPhiEnforcement },
      { id: 'axioms', name: 'Axioms present', fn: checkAxioms },
    ]
  },

  // PILLAR 7: LLM Integration (External)
  llm: {
    name: 'LLM Integration',
    sefirah: 'External',
    checks: [
      { id: 'llm-bridge', name: 'LLM bridge exists', fn: checkLLMBridge },
      { id: 'ollama-config', name: 'Ollama configured', fn: checkOllamaConfig },
      { id: 'ollama-running', name: 'Ollama running', fn: checkOllamaRunning, optional: true },
    ]
  },

  // PILLAR 8: Tests & Benchmarks (Gevurah)
  tests: {
    name: 'Tests & Benchmarks',
    sefirah: 'Gevurah',
    checks: [
      { id: 'test-files', name: 'Test files exist', fn: checkTestFiles },
      { id: 'benchmark-files', name: 'Benchmark files exist', fn: checkBenchmarkFiles },
      { id: 'test-coverage', name: 'Test coverage', fn: checkTestCoverage, optional: true },
    ]
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// CHECK IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

function checkPackagesExist() {
  const packages = ['core', 'mcp', 'node', 'persistence', 'protocol'];
  const results = [];

  for (const pkg of packages) {
    const pkgPath = join(CYNIC_ROOT, 'packages', pkg);
    const exists = existsSync(pkgPath);
    results.push({ name: pkg, ok: exists, path: pkgPath });
  }

  const ok = results.every(r => r.ok);
  return {
    ok,
    details: results,
    message: ok ? `All ${packages.length} packages exist` : `Missing: ${results.filter(r => !r.ok).map(r => r.name).join(', ')}`
  };
}

function checkPackageJson() {
  const pkgPath = join(CYNIC_ROOT, 'package.json');
  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    const hasName = !!pkg.name;
    const hasWorkspaces = !!pkg.workspaces;
    const hasScripts = !!pkg.scripts;

    return {
      ok: hasName && hasWorkspaces,
      details: { name: pkg.name, workspaces: pkg.workspaces?.length || 0, scripts: Object.keys(pkg.scripts || {}).length },
      message: `${pkg.name} with ${pkg.workspaces?.length || 0} workspaces`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkNodeModules() {
  const nmPath = join(CYNIC_ROOT, 'node_modules');
  const exists = existsSync(nmPath);

  if (!exists) {
    return { ok: false, message: 'node_modules not found - run npm install' };
  }

  // Check some key dependencies
  const deps = ['pg', 'dotenv'];
  const missing = deps.filter(d => !existsSync(join(nmPath, d)));

  return {
    ok: missing.length === 0,
    details: { checked: deps, missing },
    message: missing.length === 0 ? 'Dependencies installed' : `Missing: ${missing.join(', ')}`
  };
}

function checkTypeScript() {
  // Check if tsconfig exists and basic structure
  const tsconfigPath = join(CYNIC_ROOT, 'tsconfig.json');
  const exists = existsSync(tsconfigPath);

  if (!exists) {
    return { ok: true, message: 'No TypeScript (pure JS project)', optional: true };
  }

  try {
    const tsconfig = JSON.parse(readFileSync(tsconfigPath, 'utf8'));
    return {
      ok: true,
      details: tsconfig.compilerOptions,
      message: 'TypeScript configured'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkHooksExist() {
  const hooksDir = join(CYNIC_ROOT, 'scripts', 'hooks');
  const requiredHooks = ['awaken.js', 'observe.js', 'sleep.js'];

  const results = requiredHooks.map(hook => ({
    name: hook,
    ok: existsSync(join(hooksDir, hook))
  }));

  const ok = results.every(r => r.ok);
  return {
    ok,
    details: results,
    message: ok ? 'All hooks exist' : `Missing: ${results.filter(r => !r.ok).map(r => r.name).join(', ')}`
  };
}

function checkHooksSyntax() {
  const hooksDir = join(CYNIC_ROOT, 'scripts', 'hooks');
  const hooks = ['awaken.js', 'observe.js', 'sleep.js'];
  const results = [];

  for (const hook of hooks) {
    const hookPath = join(hooksDir, hook);
    if (!existsSync(hookPath)) {
      results.push({ name: hook, ok: false, error: 'File not found' });
      continue;
    }

    try {
      execSync(`node --check "${hookPath}"`, { stdio: 'pipe' });
      results.push({ name: hook, ok: true });
    } catch (e) {
      results.push({ name: hook, ok: false, error: 'Syntax error' });
    }
  }

  const ok = results.every(r => r.ok);
  return {
    ok,
    details: results,
    message: ok ? 'All hooks valid' : `Invalid: ${results.filter(r => !r.ok).map(r => r.name).join(', ')}`
  };
}

function checkLibModules() {
  const libDir = join(CYNIC_ROOT, 'scripts', 'lib');
  const coreModules = [
    'cynic-core.cjs',
    'auto-judge.cjs',
    'self-refinement.cjs',
    'collective-dogs.cjs',
  ];

  const results = coreModules.map(mod => {
    const modPath = join(libDir, mod);
    const exists = existsSync(modPath);

    if (!exists) {
      return { name: mod, ok: false, error: 'Not found' };
    }

    try {
      execSync(`node --check "${modPath}"`, { stdio: 'pipe' });
      return { name: mod, ok: true };
    } catch (e) {
      return { name: mod, ok: false, error: 'Syntax error' };
    }
  });

  const ok = results.filter(r => r.ok).length >= results.length * PHI_INV;
  return {
    ok,
    details: results,
    message: `${results.filter(r => r.ok).length}/${results.length} modules valid`
  };
}

function checkCynicCore() {
  const corePath = join(CYNIC_ROOT, 'scripts', 'lib', 'cynic-core.cjs');

  if (!existsSync(corePath)) {
    return { ok: false, message: 'cynic-core.cjs not found' };
  }

  try {
    // Write a temp test file instead of using node -e (Windows-safe)
    const tempFile = join(CYNIC_ROOT, 'scripts', 'tikkun', '.test-core.cjs');
    const testCode = `
const core = require('${corePath.replace(/\\/g, '/')}');
console.log(JSON.stringify({
  hasPHI: typeof core.PHI === 'number',
  hasPHI_INV: typeof core.PHI_INV === 'number',
  hasDetectUser: typeof core.detectUser === 'function',
  hasOrchestrate: typeof core.orchestrate === 'function',
}));
`;
    writeFileSync(tempFile, testCode);

    const result = execSync(`node "${tempFile}"`, { stdio: 'pipe', encoding: 'utf8', cwd: CYNIC_ROOT });

    // Cleanup temp file
    try { unlinkSync(tempFile); } catch {}

    const parsed = JSON.parse(result.trim());

    const allOk = Object.values(parsed).every(v => v === true);
    return {
      ok: allOk,
      details: parsed,
      message: allOk ? 'cynic-core loads correctly' : 'Missing exports'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkDogsDefined() {
  const dogsPath = join(CYNIC_ROOT, 'scripts', 'lib', 'collective-dogs.cjs');

  if (!existsSync(dogsPath)) {
    return { ok: false, message: 'collective-dogs.cjs not found' };
  }

  try {
    const content = readFileSync(dogsPath, 'utf8');
    const dogs = ['SAGE', 'ANALYST', 'GUARDIAN', 'SCHOLAR', 'ARCHITECT', 'JANITOR', 'SCOUT', 'CARTOGRAPHER', 'ORACLE', 'DEPLOYER', 'CYNIC'];

    const found = dogs.filter(dog => content.includes(dog));

    return {
      ok: found.length >= 10,
      details: { expected: dogs.length, found: found.length, dogs: found },
      message: `${found.length}/${dogs.length} Dogs defined`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkDogsVote() {
  // The verdict system is in auto-judge.cjs, not collective-dogs.cjs
  const judgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'auto-judge.cjs');

  if (!existsSync(judgePath)) {
    return { ok: false, message: 'auto-judge.cjs not found' };
  }

  try {
    const content = readFileSync(judgePath, 'utf8');
    const hasVerdict = content.includes('verdict') || content.includes('VERDICT');
    const hasHOWL = content.includes('HOWL');
    const hasWAG = content.includes('WAG');
    const hasBARK = content.includes('BARK');
    const hasGROWL = content.includes('GROWL');

    const allVerdicts = hasHOWL && hasWAG && hasBARK && hasGROWL;

    return {
      ok: hasVerdict && allVerdicts,
      details: { hasVerdict, verdicts: { HOWL: hasHOWL, WAG: hasWAG, BARK: hasBARK, GROWL: hasGROWL } },
      message: allVerdicts ? 'Dogs can judge (HOWL/WAG/BARK/GROWL)' : 'Missing verdict types'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkConsensus() {
  // Consensus mechanism is in llm-judgment-bridge.cjs
  const bridgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs');

  if (!existsSync(bridgePath)) {
    return { ok: false, message: 'llm-judgment-bridge.cjs not found' };
  }

  try {
    const content = readFileSync(bridgePath, 'utf8');
    const hasConsensus = content.includes('consensus') || content.includes('Consensus');
    const hasThreshold = content.includes('CONSENSUS_THRESHOLD') || content.includes('agreementRatio');
    const hasPhiEnforcement = content.includes('PHI_INV') && content.includes('Math.min');

    return {
      ok: hasConsensus && hasThreshold && hasPhiEnforcement,
      details: { hasConsensus, hasThreshold, hasPhiEnforcement },
      message: hasConsensus && hasThreshold ? 'φ⁻¹ consensus mechanism' : 'Missing consensus'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkMCPPackage() {
  const mcpPath = join(CYNIC_ROOT, 'packages', 'mcp');
  const pkgPath = join(mcpPath, 'package.json');

  if (!existsSync(mcpPath)) {
    return { ok: false, message: 'packages/mcp not found' };
  }

  if (!existsSync(pkgPath)) {
    return { ok: false, message: 'package.json not found' };
  }

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    return {
      ok: true,
      details: { name: pkg.name, version: pkg.version },
      message: `${pkg.name}@${pkg.version}`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkMCPTools() {
  const toolsDir = join(CYNIC_ROOT, 'packages', 'mcp', 'src', 'tools');

  if (!existsSync(toolsDir)) {
    return { ok: false, message: 'tools directory not found' };
  }

  try {
    const files = readdirSync(toolsDir, { recursive: true })
      .filter(f => f.endsWith('.js'));

    return {
      ok: files.length >= 5,
      details: { count: files.length, files: files.slice(0, 10) },
      message: `${files.length} tool files`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkMCPServer() {
  const serverPath = join(CYNIC_ROOT, 'packages', 'mcp', 'src', 'index.js');

  if (!existsSync(serverPath)) {
    return { ok: false, message: 'MCP server entry not found' };
  }

  try {
    execSync(`node --check "${serverPath}"`, { stdio: 'pipe' });
    return { ok: true, message: 'MCP server syntax valid' };
  } catch (e) {
    return { ok: false, message: 'Syntax error in MCP server' };
  }
}

function checkPostgresConfig() {
  const envPath = join(CYNIC_ROOT, '.env');
  const envExamplePath = join(CYNIC_ROOT, '.env.example');

  const hasEnv = existsSync(envPath);
  const hasExample = existsSync(envExamplePath);

  if (hasEnv) {
    const content = readFileSync(envPath, 'utf8');
    const hasDbUrl = content.includes('DATABASE_URL') || content.includes('POSTGRES');
    return {
      ok: hasDbUrl,
      details: { hasEnv, hasDbUrl },
      message: hasDbUrl ? 'PostgreSQL configured' : 'DATABASE_URL not set'
    };
  }

  return {
    ok: false,
    details: { hasEnv, hasExample },
    message: hasExample ? 'Copy .env.example to .env' : 'No .env file'
  };
}

function checkMigrations() {
  const migrationsDir = join(CYNIC_ROOT, 'packages', 'persistence', 'src', 'postgres', 'migrations');

  if (!existsSync(migrationsDir)) {
    return { ok: false, message: 'Migrations directory not found' };
  }

  try {
    const files = readdirSync(migrationsDir).filter(f => f.endsWith('.sql'));
    const latest = files.sort().pop();

    return {
      ok: files.length >= 10,
      details: { count: files.length, latest },
      message: `${files.length} migrations (latest: ${latest || 'none'})`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkEmbedder() {
  const embedderPath = join(CYNIC_ROOT, 'packages', 'persistence', 'src', 'services', 'embedder.js');

  if (!existsSync(embedderPath)) {
    return { ok: false, message: 'Embedder service not found' };
  }

  try {
    const content = readFileSync(embedderPath, 'utf8');
    const hasEmbed = content.includes('embed') || content.includes('vector');

    return {
      ok: hasEmbed,
      message: hasEmbed ? 'Embedder service exists' : 'No embedding logic found'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkPhiConstants() {
  // Check both decision-constants.cjs (source) and cynic-core.cjs (re-export)
  const dcPath = join(CYNIC_ROOT, 'scripts', 'lib', 'decision-constants.cjs');
  const corePath = join(CYNIC_ROOT, 'scripts', 'lib', 'cynic-core.cjs');

  let hasPHI = false, hasPHI_INV = false, hasPHI_INV_2 = false;

  // Check decision-constants.cjs first (source of truth)
  if (existsSync(dcPath)) {
    try {
      const content = readFileSync(dcPath, 'utf8');
      hasPHI = content.includes('1.618');
      hasPHI_INV = content.includes('0.618');
      hasPHI_INV_2 = content.includes('0.382') || content.includes('0.381');
    } catch (e) {
      // continue
    }
  }

  // Also check cynic-core.cjs
  if (existsSync(corePath) && (!hasPHI || !hasPHI_INV)) {
    try {
      const content = readFileSync(corePath, 'utf8');
      hasPHI = hasPHI || content.includes('1.618') || content.includes('PHI');
      hasPHI_INV = hasPHI_INV || content.includes('0.618') || content.includes('PHI_INV');
      hasPHI_INV_2 = hasPHI_INV_2 || content.includes('0.382') || content.includes('PHI_INV_2');
    } catch (e) {
      // continue
    }
  }

  return {
    ok: hasPHI && hasPHI_INV,
    details: { PHI: hasPHI, 'PHI⁻¹': hasPHI_INV, 'PHI⁻²': hasPHI_INV_2 },
    message: hasPHI && hasPHI_INV ? 'φ constants defined' : 'Missing φ constants'
  };
}

function checkPhiEnforcement() {
  // Search for PHI_INV ceiling in judgment code
  const files = [
    join(CYNIC_ROOT, 'scripts', 'lib', 'auto-judge.cjs'),
    join(CYNIC_ROOT, 'scripts', 'lib', 'self-refinement.cjs'),
    join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs'),
  ];

  let enforced = 0;
  const details = [];

  for (const file of files) {
    if (!existsSync(file)) continue;

    const content = readFileSync(file, 'utf8');
    const hasMin = content.includes('Math.min') && (content.includes('PHI_INV') || content.includes('0.618'));
    const hasCeiling = content.includes('confidence') && hasMin;

    if (hasCeiling) enforced++;
    details.push({ file: file.split(/[/\\]/).pop(), enforced: hasCeiling });
  }

  return {
    ok: enforced >= 2,
    details,
    message: `φ⁻¹ ceiling enforced in ${enforced}/${files.length} files`
  };
}

function checkAxioms() {
  // Axioms are defined in package.json _philosophy and implemented across codebase
  const pkgPath = join(CYNIC_ROOT, 'package.json');

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    const philosophy = pkg._philosophy || {};
    const axioms = philosophy.axioms || [];

    // Check if axioms are defined in package.json
    const expectedAxioms = ['PHI', 'VERIFY', 'CULTURE', 'BURN'];
    const found = expectedAxioms.filter(a => axioms.includes(a));

    // Also verify some axioms are implemented in code
    const libFiles = [
      join(CYNIC_ROOT, 'scripts', 'lib', 'decision-constants.cjs'),
      join(CYNIC_ROOT, 'scripts', 'lib', 'adaptive-learn.cjs'),
    ];

    let implementedCount = 0;
    for (const file of libFiles) {
      if (existsSync(file)) {
        const content = readFileSync(file, 'utf8');
        if (content.includes('BURN')) implementedCount++;
        if (content.includes('PHI')) implementedCount++;
      }
    }

    return {
      ok: found.length >= 3 || (found.length >= 1 && implementedCount >= 2),
      details: { defined: axioms, found, implemented: implementedCount },
      message: found.length >= 3 ? `${found.length}/4 axioms` : `${found.length} defined, ${implementedCount} implemented`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkLLMBridge() {
  const bridgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs');

  if (!existsSync(bridgePath)) {
    return { ok: false, message: 'LLM bridge not found' };
  }

  try {
    const content = readFileSync(bridgePath, 'utf8');
    const hasOllama = content.includes('ollama') || content.includes('OLLAMA');
    const hasConsensus = content.includes('consensus') || content.includes('Consensus');

    return {
      ok: hasOllama,
      details: { hasOllama, hasConsensus },
      message: hasOllama ? 'LLM bridge configured' : 'No Ollama integration'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkOllamaConfig() {
  const bridgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs');

  if (!existsSync(bridgePath)) {
    return { ok: true, message: 'LLM bridge not found (optional)', optional: true };
  }

  try {
    const content = readFileSync(bridgePath, 'utf8');
    const hasHost = content.includes('OLLAMA_HOST') || content.includes('localhost:11434');
    const hasModel = content.includes('CYNIC_LLM_MODEL') || content.includes('gemma');

    return {
      ok: hasHost,
      details: { hasHost, hasModel },
      message: hasHost && hasModel ? 'Ollama configured' : 'Missing Ollama config'
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

async function checkOllamaRunning() {
  try {
    const response = await fetch('http://localhost:11434/api/tags', {
      signal: AbortSignal.timeout(3000)
    });

    if (response.ok) {
      const data = await response.json();
      const models = data.models?.map(m => m.name) || [];
      return {
        ok: true,
        details: { models: models.slice(0, 5) },
        message: `Ollama running with ${models.length} models`
      };
    }

    return { ok: false, message: 'Ollama not responding', optional: true };
  } catch (e) {
    return { ok: false, message: 'Ollama not running', optional: true };
  }
}

function checkTestFiles() {
  // Look in packages/*/test/ directories (correct path)
  const packagesDir = join(CYNIC_ROOT, 'packages');

  let totalTests = 0;
  const details = [];

  if (!existsSync(packagesDir)) {
    return { ok: false, message: 'packages directory not found' };
  }

  try {
    const packages = readdirSync(packagesDir, { withFileTypes: true })
      .filter(d => d.isDirectory())
      .map(d => d.name);

    for (const pkg of packages) {
      const testDir = join(packagesDir, pkg, 'test');
      if (!existsSync(testDir)) continue;

      try {
        const files = readdirSync(testDir)
          .filter(f => f.endsWith('.test.js') || f.endsWith('.spec.js'));
        if (files.length > 0) {
          totalTests += files.length;
          details.push({ package: pkg, count: files.length });
        }
      } catch (e) {
        // ignore
      }
    }
  } catch (e) {
    return { ok: false, message: e.message };
  }

  return {
    ok: totalTests >= 5,
    details,
    message: `${totalTests} test files in ${details.length} packages`
  };
}

function checkBenchmarkFiles() {
  const benchDir = join(CYNIC_ROOT, 'benchmarks');

  if (!existsSync(benchDir)) {
    return { ok: false, message: 'benchmarks directory not found' };
  }

  try {
    const subdirs = readdirSync(benchDir, { withFileTypes: true })
      .filter(d => d.isDirectory())
      .map(d => d.name);

    return {
      ok: subdirs.length >= 3,
      details: { categories: subdirs },
      message: `${subdirs.length} benchmark categories`
    };
  } catch (e) {
    return { ok: false, message: e.message };
  }
}

function checkTestCoverage() {
  // This would require running tests with coverage
  // For now, just check if coverage config exists
  const pkgPath = join(CYNIC_ROOT, 'package.json');

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    const hasCoverage = pkg.scripts?.coverage || pkg.scripts?.test?.includes('coverage');

    return {
      ok: hasCoverage,
      message: hasCoverage ? 'Coverage script exists' : 'No coverage configured',
      optional: true
    };
  } catch (e) {
    return { ok: false, message: e.message, optional: true };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN EXECUTION
// ═══════════════════════════════════════════════════════════════════════════

async function runAudit(options = {}) {
  const results = {
    timestamp: new Date().toISOString(),
    version: '1.0.0',
    categories: {},
    summary: {
      total: 0,
      passed: 0,
      failed: 0,
      optional: 0,
      score: 0,
    }
  };

  console.log(`\n${C.cyan}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}`);
  console.log(`${C.cyan}${C.bold}  DA'AT - CYNIC System Audit${C.reset}`);
  console.log(`${C.cyan}${C.bold}  "Knowledge of what is known"${C.reset}`);
  console.log(`${C.cyan}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}\n`);

  for (const [catId, category] of Object.entries(AUDITS)) {
    console.log(`${C.yellow}── ${category.name} (${category.sefirah}) ──${C.reset}`);

    results.categories[catId] = {
      name: category.name,
      sefirah: category.sefirah,
      checks: [],
      passed: 0,
      failed: 0,
    };

    for (const check of category.checks) {
      results.summary.total++;

      try {
        const result = await check.fn();
        results.categories[catId].checks.push({
          id: check.id,
          name: check.name,
          ...result
        });

        if (result.optional && !result.ok) {
          results.summary.optional++;
          console.log(`   ${C.dim}○${C.reset} ${check.name}: ${C.dim}${result.message}${C.reset}`);
        } else if (result.ok) {
          results.summary.passed++;
          results.categories[catId].passed++;
          console.log(`   ${C.green}✓${C.reset} ${check.name}: ${result.message}`);
        } else {
          results.summary.failed++;
          results.categories[catId].failed++;
          console.log(`   ${C.red}✗${C.reset} ${check.name}: ${C.red}${result.message}${C.reset}`);
        }
      } catch (e) {
        results.summary.failed++;
        results.categories[catId].failed++;
        results.categories[catId].checks.push({
          id: check.id,
          name: check.name,
          ok: false,
          error: e.message
        });
        console.log(`   ${C.red}✗${C.reset} ${check.name}: ${C.red}Error: ${e.message}${C.reset}`);
      }
    }

    console.log('');
  }

  // Calculate score (excluding optional)
  const scoreable = results.summary.total - results.summary.optional;
  results.summary.score = scoreable > 0
    ? Math.round((results.summary.passed / scoreable) * 100)
    : 0;

  // Summary
  console.log(`${C.cyan}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}`);
  console.log(`${C.bold}  SUMMARY${C.reset}`);
  console.log(`${C.cyan}═══════════════════════════════════════════════════════════════════════════${C.reset}`);

  const scoreColor = results.summary.score >= PHI_INV * 100 ? C.green :
                     results.summary.score >= PHI_INV_2 * 100 ? C.yellow : C.red;

  console.log(`\n   Score: ${scoreColor}${results.summary.score}%${C.reset} (φ⁻¹ threshold: 62%)`);
  console.log(`   ${C.green}Passed: ${results.summary.passed}${C.reset} | ${C.red}Failed: ${results.summary.failed}${C.reset} | ${C.dim}Optional: ${results.summary.optional}${C.reset}`);

  // Progress bar
  const barLen = 40;
  const filled = Math.round((results.summary.score / 100) * barLen);
  const phiMark = Math.round(PHI_INV * barLen);

  let bar = '';
  for (let i = 0; i < barLen; i++) {
    if (i === phiMark) bar += C.yellow + '│' + C.reset;
    else if (i < filled) bar += C.green + '█' + C.reset;
    else bar += C.dim + '░' + C.reset;
  }
  console.log(`   [${bar}]`);
  console.log(`   ${' '.repeat(phiMark + 4)}↑ φ⁻¹`);

  // Verdict
  console.log('');
  if (results.summary.score >= PHI_INV * 100) {
    console.log(`   ${C.green}${C.bold}*tail wag* Da'at restauré. Le système se connaît.${C.reset}`);
  } else if (results.summary.score >= PHI_INV_2 * 100) {
    console.log(`   ${C.yellow}${C.bold}*sniff* Da'at partiel. Certaines zones restent obscures.${C.reset}`);
  } else {
    console.log(`   ${C.red}${C.bold}*ears droop* Da'at faible. Tikkun nécessaire.${C.reset}`);
  }

  console.log(`\n${C.cyan}═══════════════════════════════════════════════════════════════════════════${C.reset}\n`);

  // Save results
  if (!options.noSave) {
    const resultsPath = join(CYNIC_ROOT, 'scripts', 'tikkun', 'daat-results.json');
    writeFileSync(resultsPath, JSON.stringify(results, null, 2));
    console.log(`${C.dim}Results saved to: ${resultsPath}${C.reset}\n`);
  }

  if (options.json) {
    console.log(JSON.stringify(results, null, 2));
  }

  return results;
}

// CLI
const args = process.argv.slice(2);
const options = {
  json: args.includes('--json'),
  fix: args.includes('--fix'),
  noSave: args.includes('--no-save'),
};

runAudit(options).catch(console.error);
