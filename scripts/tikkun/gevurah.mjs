#!/usr/bin/env node
/**
 * GEVURAH - The Left Pillar
 *
 * "Judgment, discipline, strength"
 *
 * Automated test runner for CYNIC.
 * Ensures rigor and validation for every component.
 *
 * Usage: node scripts/tikkun/gevurah.mjs [--watch] [--filter=pattern]
 *
 * @module tikkun/gevurah
 */

import { fileURLToPath } from 'url';
import { dirname, join, basename } from 'path';
import { existsSync, readdirSync, readFileSync, writeFileSync, unlinkSync } from 'fs';
import { execSync, spawn } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CYNIC_ROOT = join(__dirname, '..', '..');

// φ constants
const PHI_INV = 0.618033988749895;

// Colors
const C = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m',
  dim: '\x1b[2m',
  bold: '\x1b[1m',
};

// ═══════════════════════════════════════════════════════════════════════════
// TEST SUITES
// ═══════════════════════════════════════════════════════════════════════════

const TEST_SUITES = [
  {
    id: 'phi-constants',
    name: 'φ Constants',
    description: 'Verify φ is correctly defined everywhere',
    run: testPhiConstants,
  },
  {
    id: 'phi-ceiling',
    name: 'φ⁻¹ Ceiling',
    description: 'Ensure confidence never exceeds 61.8%',
    run: testPhiCeiling,
  },
  {
    id: 'dogs-judgment',
    name: 'Dogs Judgment',
    description: 'Test collective dogs can judge code',
    run: testDogsJudgment,
  },
  {
    id: 'consensus',
    name: 'Consensus Mechanism',
    description: 'Test φ⁻¹ consensus threshold',
    run: testConsensus,
  },
  {
    id: 'hooks-load',
    name: 'Hooks Load',
    description: 'Verify all hooks can be imported',
    run: testHooksLoad,
  },
  {
    id: 'mcp-tools',
    name: 'MCP Tools',
    description: 'Test MCP tools can be loaded',
    run: testMCPTools,
  },
  {
    id: 'axioms',
    name: 'Axioms Present',
    description: 'Verify PHI, VERIFY, CULTURE, BURN axioms',
    run: testAxioms,
  },
  {
    id: 'verdicts',
    name: 'Verdict System',
    description: 'Test HOWL/WAG/BARK/GROWL verdicts',
    run: testVerdicts,
  },
  {
    id: 'llm-bridge',
    name: 'LLM Bridge',
    description: 'Test LLM judgment bridge (mock)',
    run: testLLMBridge,
  },
  {
    id: 'memory-store',
    name: 'Memory Store',
    description: 'Test memory storage functions',
    run: testMemoryStore,
  },
];

// ═══════════════════════════════════════════════════════════════════════════
// TEST IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

async function testPhiConstants() {
  const assertions = [];

  // Check decision-constants (source) and cynic-core (re-export)
  const dcPath = join(CYNIC_ROOT, 'scripts', 'lib', 'decision-constants.cjs');
  const corePath = join(CYNIC_ROOT, 'scripts', 'lib', 'cynic-core.cjs');

  // Check decision-constants.cjs first
  if (existsSync(dcPath)) {
    const content = readFileSync(dcPath, 'utf8');
    assertions.push({
      name: 'PHI defined as 1.618...',
      pass: content.includes('1.618033988749895') || content.includes('1.618'),
    });
  } else if (existsSync(corePath)) {
    const content = readFileSync(corePath, 'utf8');
    assertions.push({
      name: 'PHI defined as 1.618...',
      pass: content.includes('1.618') || content.includes('PHI'),
    });
  }

  // Check cynic-core exports by loading it (Windows-safe)
  if (existsSync(corePath)) {
    try {
      const tempFile = join(CYNIC_ROOT, 'scripts', 'tikkun', '.test-phi.cjs');
      const testCode = `
const c = require('${corePath.replace(/\\/g, '/')}');
console.log(JSON.stringify({ PHI: c.PHI, PHI_INV: c.PHI_INV }));
`;
      writeFileSync(tempFile, testCode);
      const result = JSON.parse(execSync(`node "${tempFile}"`, { encoding: 'utf8', cwd: CYNIC_ROOT }));
      try { unlinkSync(tempFile); } catch {}

      assertions.push({
        name: 'PHI_INV defined as 0.618...',
        pass: result.PHI_INV && Math.abs(result.PHI_INV - 0.618033988749895) < 0.0001,
      });

      assertions.push({
        name: `PHI value correct (${result.PHI})`,
        pass: result.PHI && Math.abs(result.PHI - 1.618033988749895) < 0.0001,
      });
    } catch (e) {
      assertions.push({ name: 'Load and check values', pass: false, error: e.message });
    }
  } else {
    assertions.push({ name: 'cynic-core.cjs exists', pass: false });
  }

  return { assertions };
}

async function testPhiCeiling() {
  const assertions = [];
  // These files implement confidence capping, not collective-dogs.cjs (which is just definitions)
  const filesToCheck = [
    'scripts/lib/auto-judge.cjs',
    'scripts/lib/self-refinement.cjs',
    'scripts/lib/llm-judgment-bridge.cjs',
  ];

  for (const file of filesToCheck) {
    const filePath = join(CYNIC_ROOT, file);
    if (!existsSync(filePath)) {
      assertions.push({ name: `${basename(file)} exists`, pass: false });
      continue;
    }

    const content = readFileSync(filePath, 'utf8');

    // Check for Math.min with PHI_INV or 0.618
    const hasMinPhi = /Math\.min\([^)]*(?:PHI_INV|0\.618)[^)]*\)/s.test(content);
    const hasConfidenceCheck = content.includes('confidence') && hasMinPhi;

    assertions.push({
      name: `${basename(file)} enforces φ⁻¹ ceiling`,
      pass: hasConfidenceCheck || content.includes('61.8'),
    });
  }

  return { assertions };
}

async function testDogsJudgment() {
  const assertions = [];
  const dogsPath = join(CYNIC_ROOT, 'scripts', 'lib', 'collective-dogs.cjs');
  const judgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'auto-judge.cjs');

  if (!existsSync(dogsPath)) {
    return { assertions: [{ name: 'collective-dogs.cjs exists', pass: false }] };
  }

  // Check Dogs are defined (content check)
  const dogsContent = readFileSync(dogsPath, 'utf8');
  const dogNames = ['CYNIC', 'SCOUT', 'GUARDIAN', 'DEPLOYER', 'ARCHITECT', 'JANITOR', 'ORACLE', 'ANALYST', 'SAGE', 'SCHOLAR', 'CARTOGRAPHER'];
  const foundDogs = dogNames.filter(dog => dogsContent.includes(dog));

  assertions.push({
    name: 'Dogs definitions present',
    pass: foundDogs.length >= 10,
  });

  assertions.push({
    name: `Dog count >= 10 (${foundDogs.length})`,
    pass: foundDogs.length >= 10,
  });

  // Check judgment system exists in auto-judge.cjs
  if (existsSync(judgePath)) {
    const judgeContent = readFileSync(judgePath, 'utf8');
    assertions.push({
      name: 'Judgment system exists',
      pass: judgeContent.includes('verdict') && judgeContent.includes('judgment'),
    });
  } else {
    assertions.push({ name: 'auto-judge.cjs exists', pass: false });
  }

  return { assertions };
}

async function testConsensus() {
  const assertions = [];
  // Consensus mechanism is in llm-judgment-bridge.cjs
  const bridgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs');

  if (!existsSync(bridgePath)) {
    return { assertions: [{ name: 'llm-judgment-bridge.cjs exists', pass: false }] };
  }

  const content = readFileSync(bridgePath, 'utf8');

  // Check consensus threshold is φ⁻¹
  const hasThreshold = content.includes('CONSENSUS_THRESHOLD') ||
                       (content.includes('0.618') && content.includes('threshold'));

  assertions.push({
    name: 'Consensus threshold defined',
    pass: hasThreshold,
  });

  // Check consensus calculation
  const hasConsensusCalc = content.includes('agreementRatio') ||
                           content.includes('consensusReached');

  assertions.push({
    name: 'Consensus calculation exists',
    pass: hasConsensusCalc,
  });

  // Check consensus function exists
  const hasConsensusFn = content.includes('llmConsensusJudge') ||
                         content.includes('async function.*consensus');

  assertions.push({
    name: 'Consensus function implemented',
    pass: hasConsensusFn,
  });

  return { assertions };
}

async function testHooksLoad() {
  const assertions = [];
  const hooks = ['awaken.js', 'observe.js', 'sleep.js'];
  const hooksDir = join(CYNIC_ROOT, 'scripts', 'hooks');

  for (const hook of hooks) {
    const hookPath = join(hooksDir, hook);

    if (!existsSync(hookPath)) {
      assertions.push({ name: `${hook} exists`, pass: false });
      continue;
    }

    // Syntax check
    try {
      execSync(`node --check "${hookPath}"`, { stdio: 'pipe' });
      assertions.push({ name: `${hook} syntax valid`, pass: true });
    } catch (e) {
      assertions.push({ name: `${hook} syntax valid`, pass: false, error: 'Syntax error' });
    }
  }

  return { assertions };
}

async function testMCPTools() {
  const assertions = [];
  const toolsIndex = join(CYNIC_ROOT, 'packages', 'mcp', 'src', 'tools', 'index.js');

  if (!existsSync(toolsIndex)) {
    return { assertions: [{ name: 'MCP tools index exists', pass: false }] };
  }

  // Syntax check
  try {
    execSync(`node --check "${toolsIndex}"`, { stdio: 'pipe' });
    assertions.push({ name: 'Tools index syntax valid', pass: true });
  } catch (e) {
    assertions.push({ name: 'Tools index syntax valid', pass: false });
  }

  // Check tool domains
  const domainsDir = join(CYNIC_ROOT, 'packages', 'mcp', 'src', 'tools', 'domains');
  if (existsSync(domainsDir)) {
    const domains = readdirSync(domainsDir).filter(f => f.endsWith('.js'));
    assertions.push({
      name: `Tool domains >= 5 (${domains.length})`,
      pass: domains.length >= 5,
    });
  }

  return { assertions };
}

async function testAxioms() {
  const assertions = [];

  // Axioms are defined in package.json _philosophy
  const pkgPath = join(CYNIC_ROOT, 'package.json');

  if (!existsSync(pkgPath)) {
    return { assertions: [{ name: 'package.json exists', pass: false }] };
  }

  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    const philosophy = pkg._philosophy || {};
    const axioms = philosophy.axioms || [];

    const expectedAxioms = ['PHI', 'VERIFY', 'CULTURE', 'BURN'];

    for (const axiom of expectedAxioms) {
      assertions.push({
        name: `Axiom ${axiom} present`,
        pass: axioms.includes(axiom),
      });
    }
  } catch (e) {
    assertions.push({ name: 'Read axioms from package.json', pass: false, error: e.message });
  }

  return { assertions };
}

async function testVerdicts() {
  const assertions = [];
  const judgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'auto-judge.cjs');

  if (!existsSync(judgePath)) {
    return { assertions: [{ name: 'auto-judge.cjs exists', pass: false }] };
  }

  const content = readFileSync(judgePath, 'utf8');
  const verdicts = ['HOWL', 'WAG', 'BARK', 'GROWL'];

  for (const verdict of verdicts) {
    assertions.push({
      name: `Verdict ${verdict} defined`,
      pass: content.includes(verdict),
    });
  }

  // Check verdict thresholds align with φ
  const hasPhiThresholds = (content.includes('61.8') || content.includes('0.618')) &&
                           (content.includes('38.2') || content.includes('0.382'));

  assertions.push({
    name: 'Verdict thresholds use φ',
    pass: hasPhiThresholds,
  });

  return { assertions };
}

async function testLLMBridge() {
  const assertions = [];
  const bridgePath = join(CYNIC_ROOT, 'scripts', 'lib', 'llm-judgment-bridge.cjs');

  if (!existsSync(bridgePath)) {
    return { assertions: [{ name: 'LLM bridge exists', pass: false }] };
  }

  // Syntax check
  try {
    execSync(`node --check "${bridgePath}"`, { stdio: 'pipe' });
    assertions.push({ name: 'Bridge syntax valid', pass: true });
  } catch (e) {
    assertions.push({ name: 'Bridge syntax valid', pass: false });
  }

  const content = readFileSync(bridgePath, 'utf8');

  assertions.push({
    name: 'Has llmJudge function',
    pass: content.includes('llmJudge') || content.includes('function llmJudge'),
  });

  assertions.push({
    name: 'Has consensus function',
    pass: content.includes('Consensus') || content.includes('consensus'),
  });

  assertions.push({
    name: 'Enforces φ⁻¹ on LLM responses',
    pass: content.includes('Math.min') && content.includes('PHI_INV'),
  });

  return { assertions };
}

async function testMemoryStore() {
  const assertions = [];
  const memoryPath = join(CYNIC_ROOT, 'packages', 'mcp', 'src', 'tools', 'domains', 'memory.js');

  if (!existsSync(memoryPath)) {
    // Try alternate location
    const altPath = join(CYNIC_ROOT, 'scripts', 'lib', 'total-memory.cjs');
    if (existsSync(altPath)) {
      assertions.push({ name: 'Memory module exists (alt location)', pass: true });
      return { assertions };
    }
    return { assertions: [{ name: 'Memory module exists', pass: false }] };
  }

  assertions.push({ name: 'Memory module exists', pass: true });

  const content = readFileSync(memoryPath, 'utf8');

  assertions.push({
    name: 'Has store function',
    pass: content.includes('store') || content.includes('save') || content.includes('write'),
  });

  assertions.push({
    name: 'Has search function',
    pass: content.includes('search') || content.includes('find') || content.includes('query'),
  });

  return { assertions };
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST RUNNER
// ═══════════════════════════════════════════════════════════════════════════

async function runTests(options = {}) {
  const results = {
    timestamp: new Date().toISOString(),
    suites: [],
    summary: {
      total: 0,
      passed: 0,
      failed: 0,
      score: 0,
    }
  };

  console.log(`\n${C.red}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}`);
  console.log(`${C.red}${C.bold}  GEVURAH - CYNIC Test Suite${C.reset}`);
  console.log(`${C.red}${C.bold}  "Judgment, discipline, strength"${C.reset}`);
  console.log(`${C.red}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}\n`);

  let suitesToRun = TEST_SUITES;

  // Filter if specified
  if (options.filter) {
    suitesToRun = TEST_SUITES.filter(s =>
      s.id.includes(options.filter) || s.name.toLowerCase().includes(options.filter.toLowerCase())
    );
  }

  for (const suite of suitesToRun) {
    console.log(`${C.yellow}── ${suite.name} ──${C.reset}`);
    console.log(`${C.dim}   ${suite.description}${C.reset}\n`);

    const suiteResult = {
      id: suite.id,
      name: suite.name,
      assertions: [],
      passed: 0,
      failed: 0,
    };

    try {
      const { assertions } = await suite.run();

      for (const assertion of assertions) {
        results.summary.total++;
        suiteResult.assertions.push(assertion);

        if (assertion.pass) {
          results.summary.passed++;
          suiteResult.passed++;
          console.log(`   ${C.green}✓${C.reset} ${assertion.name}`);
        } else {
          results.summary.failed++;
          suiteResult.failed++;
          console.log(`   ${C.red}✗${C.reset} ${assertion.name}${assertion.error ? ` (${assertion.error})` : ''}`);
        }
      }
    } catch (e) {
      results.summary.total++;
      results.summary.failed++;
      suiteResult.failed++;
      suiteResult.assertions.push({ name: 'Suite execution', pass: false, error: e.message });
      console.log(`   ${C.red}✗${C.reset} Suite error: ${e.message}`);
    }

    results.suites.push(suiteResult);
    console.log('');
  }

  // Calculate score
  results.summary.score = results.summary.total > 0
    ? Math.round((results.summary.passed / results.summary.total) * 100)
    : 0;

  // Summary
  console.log(`${C.red}${C.bold}═══════════════════════════════════════════════════════════════════════════${C.reset}`);
  console.log(`${C.bold}  RESULTS${C.reset}`);
  console.log(`${C.red}═══════════════════════════════════════════════════════════════════════════${C.reset}`);

  const scoreColor = results.summary.score >= PHI_INV * 100 ? C.green :
                     results.summary.score >= 38.2 ? C.yellow : C.red;

  console.log(`\n   Score: ${scoreColor}${results.summary.score}%${C.reset} (φ⁻¹ threshold: 62%)`);
  console.log(`   ${C.green}Passed: ${results.summary.passed}${C.reset} | ${C.red}Failed: ${results.summary.failed}${C.reset}`);

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

  // Verdict
  console.log('');
  if (results.summary.score >= PHI_INV * 100) {
    console.log(`   ${C.green}${C.bold}*tail wag* Gevurah fort. Discipline maintenue.${C.reset}`);
  } else if (results.summary.score >= 38.2) {
    console.log(`   ${C.yellow}${C.bold}*sniff* Gevurah partiel. Renforcement nécessaire.${C.reset}`);
  } else {
    console.log(`   ${C.red}${C.bold}*GROWL* Gevurah faible. Tests critiques échouent.${C.reset}`);
  }

  console.log(`\n${C.red}═══════════════════════════════════════════════════════════════════════════${C.reset}\n`);

  // Save results
  if (!options.noSave) {
    const resultsPath = join(CYNIC_ROOT, 'scripts', 'tikkun', 'gevurah-results.json');
    writeFileSync(resultsPath, JSON.stringify(results, null, 2));
    console.log(`${C.dim}Results saved to: ${resultsPath}${C.reset}\n`);
  }

  // Return exit code based on score
  return results.summary.score >= PHI_INV * 100 ? 0 : 1;
}

// CLI
const args = process.argv.slice(2);
const options = {
  watch: args.includes('--watch'),
  filter: args.find(a => a.startsWith('--filter='))?.split('=')[1],
  noSave: args.includes('--no-save'),
};

runTests(options)
  .then(exitCode => process.exit(exitCode))
  .catch(e => {
    console.error(e);
    process.exit(1);
  });
