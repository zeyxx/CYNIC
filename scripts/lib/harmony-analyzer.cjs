/**
 * CYNIC Harmony Analyzer
 *
 * Detects gaps between philosophy and implementation.
 * "φ distrusts φ" - even harmony must be questioned.
 *
 * @module @cynic/scripts/harmony-analyzer
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawnSync } = require('child_process');

// φ Constants
const PHI = 1.618033988749895;
const PHI_INV = 1 / PHI;
const PHI_INV_2 = 1 / (PHI * PHI);

// ═══════════════════════════════════════════════════════════════════════════
// FILE TYPE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Test file patterns - these files get relaxed analysis
 * "Tests are documentation, not production code" - κυνικός
 * Patterns match both /test/ in middle and test/ at start
 */
const TEST_PATTERNS = [
  /(?:^|[/\\])test[/\\]/i,           // /test/ directory
  /(?:^|[/\\])tests[/\\]/i,          // /tests/ directory
  /(?:^|[/\\])__tests__[/\\]/i,      // /__tests__/ directory
  /(?:^|[/\\])spec[/\\]/i,           // /spec/ directory
  /\.test\.[jt]sx?$/i,               // *.test.js, *.test.ts, etc.
  /\.spec\.[jt]sx?$/i,               // *.spec.js, *.spec.ts, etc.
  /_test\.[jt]sx?$/i,                // *_test.js, *_test.ts, etc.
  /-test\.[jt]sx?$/i,                // *-test.js, *-test.ts, etc.
  /(?:^|[/\\])fixtures?[/\\]/i,      // /fixture/ or /fixtures/
  /(?:^|[/\\])mocks?[/\\]/i,         // /mock/ or /mocks/
  /(?:^|[/\\])__mocks__[/\\]/i,      // /__mocks__/
  /(?:^|[/\\])e2e[/\\]/i,            // /e2e/ directory
  /(?:^|[/\\])integration[/\\]/i,    // /integration/ directory
];

/**
 * Script/tool patterns - relaxed BURN checks
 * Patterns match both /scripts/ in middle and scripts/ at start
 */
const SCRIPT_PATTERNS = [
  /(?:^|[/\\])scripts?[/\\]/i,       // /script/ or /scripts/ or starts with scripts/
  /(?:^|[/\\])tools?[/\\]/i,         // /tool/ or /tools/
  /(?:^|[/\\])bin[/\\]/i,            // /bin/ directory
  /(?:^|[/\\])cli[/\\]/i,            // /cli/ directory
  /-cli\.[jt]sx?$/i,                  // *-cli.js
];

/**
 * Check if a file is a test file
 * @param {string} filePath - Path to check
 * @returns {boolean}
 */
function isTestFile(filePath) {
  return TEST_PATTERNS.some(pattern => pattern.test(filePath));
}

/**
 * Check if a file is a script/tool file
 * @param {string} filePath - Path to check
 * @returns {boolean}
 */
function isScriptFile(filePath) {
  return SCRIPT_PATTERNS.some(pattern => pattern.test(filePath));
}

/**
 * Get file context for analysis
 * @param {string} filePath - Path to analyze
 * @returns {{isTest: boolean, isScript: boolean, category: string}}
 */
function getFileContext(filePath) {
  const isTest = isTestFile(filePath);
  const isScript = isScriptFile(filePath);

  let category = 'production';
  if (isTest) category = 'test';
  else if (isScript) category = 'script';

  return { isTest, isScript, category };
}

// ═══════════════════════════════════════════════════════════════════════════
// PHILOSOPHY PRINCIPLES
// ═══════════════════════════════════════════════════════════════════════════

const PRINCIPLES = {
  PHI: {
    name: 'φ (Golden Ratio)',
    description: 'All ratios derive from φ',
    checks: [
      'thresholds_use_phi',
      'timeouts_phi_aligned',
      'confidence_max_618',
    ],
  },
  BURN: {
    name: 'BURN',
    description: "Don't extract, burn - simplicity is strength",
    checks: [
      'no_over_engineering',
      'minimal_dependencies',
      'code_simplicity',
    ],
  },
  VERIFY: {
    name: 'VERIFY',
    description: "Don't trust, verify - skepticism is wisdom",
    checks: [
      'input_validation',
      'error_handling',
      'no_blind_trust',
    ],
  },
  CULTURE: {
    name: 'CULTURE',
    description: 'Culture is a moat - patterns define identity',
    checks: [
      'consistent_naming',
      'consistent_style',
      'documentation_present',
    ],
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// GAP TYPES
// ═══════════════════════════════════════════════════════════════════════════

const GAP_TYPES = {
  PHILOSOPHY: {
    name: 'philosophy',
    severity: 'high',
    description: 'Misalignment with core principles',
  },
  HARMONY: {
    name: 'harmony',
    severity: 'medium',
    description: 'Inconsistency across codebase',
  },
  UPDATE: {
    name: 'update',
    severity: 'low',
    description: 'Outdated dependencies or patterns',
  },
  PRECISION: {
    name: 'precision',
    severity: 'medium',
    description: 'Lack of precision in implementation',
  },
  GLOBALITY: {
    name: 'globality',
    severity: 'medium',
    description: 'Missing global perspective',
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// ANALYSIS FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Analyze a file for philosophy alignment
 *
 * Test files and scripts get relaxed analysis:
 * - Tests: Skip BURN checks (long test functions are OK)
 * - Scripts: Relaxed BURN thresholds (100 lines instead of 62)
 *
 * "φ knows context matters" - κυνικός
 */
function analyzeFile(filePath, options = {}) {
  const gaps = [];

  if (!fs.existsSync(filePath)) return gaps;

  const content = fs.readFileSync(filePath, 'utf8');
  const ext = path.extname(filePath);
  const context = getFileContext(filePath);

  // PHI checks - apply to all files (philosophy is universal)
  gaps.push(...checkPhiAlignment(content, filePath));

  // BURN checks - skip for tests, relaxed for scripts
  if (!context.isTest) {
    gaps.push(...checkBurnPrinciple(content, filePath, { relaxed: context.isScript }));
  }

  // VERIFY checks - apply to all (validation matters everywhere)
  gaps.push(...checkVerifyPrinciple(content, filePath));

  // CULTURE checks - apply to all (consistency matters everywhere)
  gaps.push(...checkCultureAlignment(content, filePath));

  // Add context to gaps for filtering later
  for (const gap of gaps) {
    gap.fileContext = context.category;
  }

  return gaps;
}

/**
 * Check φ alignment in code
 */
function checkPhiAlignment(content, filePath) {
  const gaps = [];

  // Check for hardcoded thresholds that should use φ
  const hardcodedThresholds = content.match(/(?:threshold|limit|max|min)\s*[:=]\s*(\d+(?:\.\d+)?)/gi) || [];

  for (const match of hardcodedThresholds) {
    const value = parseFloat(match.match(/(\d+(?:\.\d+)?)/)?.[1] || '0');

    // Check if value is close to φ-derived values
    const phiValues = [PHI, PHI_INV, PHI_INV_2, 0.618, 0.382, 0.236, 61.8, 38.2, 23.6];
    const isPhiAligned = phiValues.some(pv => Math.abs(value - pv) < 0.01 || Math.abs(value - pv * 100) < 1);

    if (!isPhiAligned && value > 0 && value !== 100 && value !== 0) {
      // Check if PHI constant is even defined in file
      if (!content.includes('PHI') && !content.includes('phi') && !content.includes('1.618')) {
        gaps.push({
          type: 'PHILOSOPHY',
          principle: 'PHI',
          file: filePath,
          message: `Hardcoded threshold ${value} - consider using φ-derived value`,
          suggestion: `Use PHI_INV (0.618) or PHI_INV_2 (0.382) for thresholds`,
          severity: 'low',
          line: findLineNumber(content, match),
        });
      }
    }
  }

  // Check for timeouts not φ-aligned
  const timeouts = content.match(/timeout\s*[:=]\s*(\d+)/gi) || [];
  for (const match of timeouts) {
    const value = parseInt(match.match(/(\d+)/)?.[1] || '0', 10);
    // φ-aligned timeouts: 618, 1000, 1618, 2618, 6180, 61800
    const phiTimeouts = [618, 1000, 1618, 2618, 6180, 10000, 61800];
    const isPhiTimeout = phiTimeouts.some(pt => Math.abs(value - pt) < 100);

    if (!isPhiTimeout && value > 500) {
      gaps.push({
        type: 'PHILOSOPHY',
        principle: 'PHI',
        file: filePath,
        message: `Timeout ${value}ms - consider φ-aligned value`,
        suggestion: `Use 618ms, 1618ms, 6180ms, or 61800ms`,
        severity: 'info',
        line: findLineNumber(content, match),
      });
    }
  }

  return gaps;
}

/**
 * Check BURN principle (simplicity)
 *
 * @param {string} content - File content
 * @param {string} filePath - File path
 * @param {Object} [options] - Analysis options
 * @param {boolean} [options.relaxed=false] - Use relaxed thresholds (for scripts)
 */
function checkBurnPrinciple(content, filePath, options = {}) {
  const gaps = [];
  const lines = content.split('\n');
  const { relaxed = false } = options;

  // φ-based thresholds:
  // - Production code: 62 lines (φ⁻¹ × 100)
  // - Scripts: 100 lines (round number, scripts are often procedural)
  const FUNC_LINE_THRESHOLD = relaxed ? 100 : 62;
  const FUNC_LINE_THRESHOLD_DESC = relaxed
    ? '100 lines for scripts'
    : '62 lines (φ⁻¹ × 100)';

  // Check for overly complex functions (too many lines)
  const functionMatches = content.matchAll(/(?:function\s+\w+|(?:const|let|var)\s+\w+\s*=\s*(?:async\s*)?\([^)]*\)\s*=>)/g);

  for (const match of functionMatches) {
    const startLine = findLineNumber(content, match[0]);
    // Count lines until function ends (rough heuristic)
    let braceCount = 0;
    let funcLines = 0;
    let started = false;

    for (let i = startLine - 1; i < lines.length && funcLines < 200; i++) {
      const line = lines[i];
      if (line.includes('{')) {
        braceCount += (line.match(/{/g) || []).length;
        started = true;
      }
      if (line.includes('}')) {
        braceCount -= (line.match(/}/g) || []).length;
      }
      if (started) funcLines++;
      if (started && braceCount <= 0) break;
    }

    // Apply threshold based on file type
    if (funcLines > FUNC_LINE_THRESHOLD) {
      gaps.push({
        type: 'PHILOSOPHY',
        principle: 'BURN',
        file: filePath,
        message: `Function at line ${startLine} has ${funcLines} lines - consider breaking down`,
        suggestion: `Keep functions under ${FUNC_LINE_THRESHOLD_DESC}`,
        severity: relaxed ? 'low' : 'medium',
        line: startLine,
      });
    }
  }

  // Check for too many imports (dependency bloat)
  const imports = content.match(/(?:import|require)\s*\(/g) || [];
  if (imports.length > 20) {
    gaps.push({
      type: 'PHILOSOPHY',
      principle: 'BURN',
      file: filePath,
      message: `${imports.length} imports - possible dependency bloat`,
      suggestion: `Review if all dependencies are necessary`,
      severity: 'low',
    });
  }

  // Check for commented-out code (should be burned)
  const commentedCode = content.match(/\/\/\s*(?:const|let|var|function|if|for|while|return)\s/g) || [];
  if (commentedCode.length > 5) {
    gaps.push({
      type: 'PHILOSOPHY',
      principle: 'BURN',
      file: filePath,
      message: `${commentedCode.length} lines of commented-out code - burn it`,
      suggestion: `Remove dead code, git has history`,
      severity: 'low',
    });
  }

  return gaps;
}

/**
 * Check VERIFY principle (validation)
 */
function checkVerifyPrinciple(content, filePath) {
  const gaps = [];

  // Check for missing input validation in functions
  const functions = content.matchAll(/function\s+(\w+)\s*\(([^)]*)\)/g);

  for (const match of functions) {
    const funcName = match[1];
    const params = match[2];

    if (params.trim()) {
      // Check if function validates inputs
      const funcStart = match.index || 0;
      const funcBody = content.slice(funcStart, funcStart + 500);

      const hasValidation = funcBody.includes('if (!') ||
                           funcBody.includes('typeof') ||
                           funcBody.includes('instanceof') ||
                           funcBody.includes('?.') ||
                           funcBody.includes('|| ') ||
                           funcBody.includes('?? ');

      if (!hasValidation && !funcName.startsWith('_')) {
        gaps.push({
          type: 'PHILOSOPHY',
          principle: 'VERIFY',
          file: filePath,
          message: `Function ${funcName} may lack input validation`,
          suggestion: `Add input validation for parameters: ${params}`,
          severity: 'info',
          line: findLineNumber(content, match[0]),
        });
      }
    }
  }

  // Check for catch blocks that swallow errors silently
  const emptyCatches = content.match(/catch\s*\([^)]*\)\s*{\s*(?:\/\/[^\n]*)?\s*}/g) || [];
  if (emptyCatches.length > 0) {
    gaps.push({
      type: 'PHILOSOPHY',
      principle: 'VERIFY',
      file: filePath,
      message: `${emptyCatches.length} empty catch blocks - errors may be silently swallowed`,
      suggestion: `At minimum, log errors or add explicit ignore comment`,
      severity: 'medium',
    });
  }

  return gaps;
}

/**
 * Check CULTURE alignment (consistency)
 */
function checkCultureAlignment(content, filePath) {
  const gaps = [];

  // Check for inconsistent naming conventions
  const camelCase = (content.match(/\b[a-z]+[A-Z][a-zA-Z]*\b/g) || []).length;
  const snakeCase = (content.match(/\b[a-z]+_[a-z]+\b/g) || []).length;

  if (camelCase > 10 && snakeCase > 10) {
    const ratio = Math.min(camelCase, snakeCase) / Math.max(camelCase, snakeCase);
    if (ratio > 0.3) {
      gaps.push({
        type: 'PHILOSOPHY',
        principle: 'CULTURE',
        file: filePath,
        message: `Mixed naming conventions: ${camelCase} camelCase, ${snakeCase} snake_case`,
        suggestion: `Stick to one naming convention (camelCase for JS/TS)`,
        severity: 'low',
      });
    }
  }

  // Check for missing module documentation
  if (!content.includes('/**') && !content.includes('@module') && content.length > 500) {
    gaps.push({
      type: 'PHILOSOPHY',
      principle: 'CULTURE',
      file: filePath,
      message: `Missing module documentation`,
      suggestion: `Add JSDoc header with @module tag`,
      severity: 'info',
    });
  }

  return gaps;
}

/**
 * Find line number for a match
 */
function findLineNumber(content, match) {
  const index = content.indexOf(match);
  if (index === -1) return 1;
  return content.slice(0, index).split('\n').length;
}

// ═══════════════════════════════════════════════════════════════════════════
// PROJECT-LEVEL ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Analyze entire project for harmony
 */
function analyzeProject(projectPath = '.') {
  const gaps = [];
  const stats = {
    filesAnalyzed: 0,
    totalGaps: 0,
    byPrinciple: {},
    bySeverity: {},
  };

  // Find all JS/TS files
  const files = findCodeFiles(projectPath);

  for (const file of files.slice(0, 50)) { // Limit to 50 files
    const fileGaps = analyzeFile(file);
    gaps.push(...fileGaps);
    stats.filesAnalyzed++;
  }

  // Aggregate stats
  stats.totalGaps = gaps.length;
  for (const gap of gaps) {
    stats.byPrinciple[gap.principle] = (stats.byPrinciple[gap.principle] || 0) + 1;
    stats.bySeverity[gap.severity] = (stats.bySeverity[gap.severity] || 0) + 1;
  }

  // Add project-level gaps
  gaps.push(...analyzeProjectStructure(projectPath));
  gaps.push(...analyzeDependencies(projectPath));

  return { gaps, stats };
}

/**
 * Find code files in project
 */
function findCodeFiles(projectPath) {
  const files = [];
  const extensions = ['.js', '.cjs', '.mjs', '.ts', '.tsx'];

  function scan(dir, depth = 0) {
    if (depth > 5) return; // Max depth
    if (!fs.existsSync(dir)) return;

    try {
      const entries = fs.readdirSync(dir, { withFileTypes: true });

      for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);

        // Skip common non-code directories
        if (entry.isDirectory()) {
          if (!['node_modules', '.git', 'dist', 'build', 'coverage', '.next'].includes(entry.name)) {
            scan(fullPath, depth + 1);
          }
        } else if (extensions.includes(path.extname(entry.name))) {
          files.push(fullPath);
        }
      }
    } catch (e) {
      // Permission denied or other error
    }
  }

  scan(projectPath);
  return files;
}

/**
 * Analyze project structure
 */
function analyzeProjectStructure(projectPath) {
  const gaps = [];

  // Check for CLAUDE.md
  if (!fs.existsSync(path.join(projectPath, 'CLAUDE.md'))) {
    gaps.push({
      type: 'GLOBALITY',
      principle: 'CULTURE',
      file: projectPath,
      message: 'Missing CLAUDE.md - CYNIC identity not established',
      suggestion: 'Create CLAUDE.md with project identity and principles',
      severity: 'medium',
    });
  }

  // Check for .claude directory
  if (!fs.existsSync(path.join(projectPath, '.claude'))) {
    gaps.push({
      type: 'GLOBALITY',
      principle: 'CULTURE',
      file: projectPath,
      message: 'Missing .claude/ directory - hooks and skills not configured',
      suggestion: 'Create .claude/ with plugin.json, hooks, and skills',
      severity: 'medium',
    });
  }

  return gaps;
}

/**
 * Analyze dependencies for updates
 */
function analyzeDependencies(projectPath) {
  const gaps = [];
  const packageJsonPath = path.join(projectPath, 'package.json');

  if (!fs.existsSync(packageJsonPath)) return gaps;

  try {
    const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    const deps = { ...pkg.dependencies, ...pkg.devDependencies };

    // Check for outdated patterns
    for (const [name, version] of Object.entries(deps)) {
      // Check for very old major versions of common packages
      if (name === 'typescript' && version.match(/^[~^]?[0-3]\./)) {
        gaps.push({
          type: 'UPDATE',
          principle: 'VERIFY',
          file: packageJsonPath,
          message: `TypeScript ${version} is outdated`,
          suggestion: 'Update to TypeScript 5.x for latest features',
          severity: 'medium',
        });
      }

      // Check for deprecated packages
      const deprecated = ['request', 'tslint', 'moment'];
      if (deprecated.includes(name)) {
        gaps.push({
          type: 'UPDATE',
          principle: 'BURN',
          file: packageJsonPath,
          message: `${name} is deprecated`,
          suggestion: `Replace with modern alternative`,
          severity: 'medium',
        });
      }
    }

    // Check for too many dependencies (BURN principle)
    const totalDeps = Object.keys(deps).length;
    if (totalDeps > 50) {
      gaps.push({
        type: 'PHILOSOPHY',
        principle: 'BURN',
        file: packageJsonPath,
        message: `${totalDeps} dependencies - consider pruning`,
        suggestion: 'Review and remove unused dependencies',
        severity: 'low',
      });
    }
  } catch (e) {
    // Invalid package.json
  }

  return gaps;
}

// ═══════════════════════════════════════════════════════════════════════════
// HARMONY SCORE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate harmony score (0-100)
 */
function calculateHarmonyScore(analysis) {
  const { gaps, stats } = analysis;

  // Base score
  let score = 100;

  // Deduct based on severity
  const severityPenalty = {
    high: 10,
    medium: 5,
    low: 2,
    info: 1,
  };

  for (const gap of gaps) {
    score -= severityPenalty[gap.severity] || 1;
  }

  // Bonus for having few gaps relative to files analyzed
  if (stats.filesAnalyzed > 0) {
    const gapsPerFile = gaps.length / stats.filesAnalyzed;
    if (gapsPerFile < 0.5) score += 10; // Very clean
  }

  // Clamp to 0-100
  return Math.max(0, Math.min(100, score));
}

/**
 * Get harmony status
 */
function getHarmonyStatus(score) {
  if (score >= PHI_INV * 100) return { status: 'harmonious', emoji: '🎵' }; // >= 61.8%
  if (score >= PHI_INV_2 * 100) return { status: 'acceptable', emoji: '🎶' }; // >= 38.2%
  return { status: 'dissonant', emoji: '🔇' };
}

// ═══════════════════════════════════════════════════════════════════════════
// DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

function printHarmonyReport(analysis) {
  const score = calculateHarmonyScore(analysis);
  const status = getHarmonyStatus(score);

  console.log('═══════════════════════════════════════════════════════════════════════════════');
  console.log(`${status.emoji} CYNIC HARMONY ANALYSIS`);
  console.log('═══════════════════════════════════════════════════════════════════════════════\n');

  console.log(`HARMONY SCORE: ${score.toFixed(1)}/100 (${status.status})`);
  console.log(`Files analyzed: ${analysis.stats.filesAnalyzed}`);
  console.log(`Gaps detected: ${analysis.gaps.length}\n`);

  // By principle
  console.log('BY PRINCIPLE:');
  for (const [principle, count] of Object.entries(analysis.stats.byPrinciple).sort((a, b) => b[1] - a[1])) {
    console.log(`   ${principle.padEnd(10)} ${count} gaps`);
  }

  // Top gaps
  if (analysis.gaps.length > 0) {
    console.log('\nTOP GAPS:');
    console.log('────────────────────────────────────────────────────────────────────────────────');
    const topGaps = analysis.gaps
      .sort((a, b) => {
        const severityOrder = { high: 0, medium: 1, low: 2, info: 3 };
        return (severityOrder[a.severity] || 3) - (severityOrder[b.severity] || 3);
      })
      .slice(0, 10);

    for (const gap of topGaps) {
      const icon = gap.severity === 'high' ? '🔴' :
                   gap.severity === 'medium' ? '🟡' :
                   gap.severity === 'low' ? '🟢' : 'ℹ️';
      console.log(`   ${icon} [${gap.principle}] ${gap.message}`);
      if (gap.suggestion) {
        console.log(`      └─ 💡 ${gap.suggestion}`);
      }
    }
  }

  console.log('\n═══════════════════════════════════════════════════════════════════════════════');

  return { score, status, gaps: analysis.gaps };
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Analysis
  analyzeFile,
  analyzeProject,
  analyzeDependencies,
  analyzeProjectStructure,

  // File context detection
  isTestFile,
  isScriptFile,
  getFileContext,

  // Scoring
  calculateHarmonyScore,
  getHarmonyStatus,

  // Display
  printHarmonyReport,

  // Constants
  PRINCIPLES,
  GAP_TYPES,
  TEST_PATTERNS,
  SCRIPT_PATTERNS,
  PHI,
  PHI_INV,
  PHI_INV_2,
};

// CLI execution
if (require.main === module) {
  const args = process.argv.slice(2);
  const projectPath = args[0] || '.';

  console.log(`Analyzing ${projectPath}...\n`);
  const analysis = analyzeProject(projectPath);
  printHarmonyReport(analysis);
}
