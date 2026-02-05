/**
 * CYNIC Self-Judge Module
 *
 * "Le chien se juge lui-même" — Meta-consciousness for code modifications
 *
 * When CYNIC's own code is modified, this module judges the modification
 * using the same 25-dimension system used for external code.
 *
 * Gap 1 Fix: CYNIC must know itself across all fractal scales.
 *
 * @module @cynic/scripts/lib/self-judge
 */

'use strict';

const path = require('path');

// φ constants for scoring
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// CYNIC component mappings
const CYNIC_COMPONENTS = {
  'collective-singleton': { domain: 'orchestration', criticality: 'critical', scale: 'session' },
  'judge': { domain: 'judgment', criticality: 'critical', scale: 'judgment' },
  'learning': { domain: 'learning', criticality: 'high', scale: 'session' },
  'server': { domain: 'mcp', criticality: 'critical', scale: 'session' },
  'observe': { domain: 'hooks', criticality: 'high', scale: 'response' },
  'awaken': { domain: 'hooks', criticality: 'high', scale: 'session' },
  'orchestrat': { domain: 'orchestration', criticality: 'critical', scale: 'judgment' },
  'shared-memory': { domain: 'memory', criticality: 'critical', scale: 'session' },
  'q-learning': { domain: 'learning', criticality: 'high', scale: 'session' },
  'dimension': { domain: 'judgment', criticality: 'critical', scale: 'dimension' },
  'residual': { domain: 'judgment', criticality: 'high', scale: 'dimension' },
  'persistence': { domain: 'persistence', criticality: 'critical', scale: 'session' },
  'test': { domain: 'testing', criticality: 'medium', scale: 'project' },
};

// Fractal scales (8 levels)
const FRACTAL_SCALES = ['axiom', 'dimension', 'judgment', 'response', 'session', 'project', 'ecosystem', 'cosmos'];

// Critical patterns that could break the fractal
const FRACTAL_RISK_PATTERNS = [
  {
    pattern: /getCollectivePack\s*\(\s*\)/g,
    risk: 'lifecycle',
    severity: 'high',
    description: 'Sync singleton call — may not load persisted state',
    fix: 'Use getCollectivePackAsync() and await the result',
  },
  {
    // getSharedMemory() is often called internally, only warn in non-singleton contexts
    // This is a softer warning since getSharedMemory is designed to be sync
    pattern: /getSharedMemory\s*\(\s*\)/g,
    risk: 'lifecycle',
    severity: 'low',  // Lowered: sync access is often intentional
    description: 'Sync SharedMemory — ensure initialize() is called if persistence needed',
    fix: 'Call sharedMemory.initialize() after if persistence is required',
  },
  // REMOVED: .initialize() pattern had too many false positives
  // The lookbehind couldn't detect await before the variable name
  // e.g., "await obj.initialize()" was incorrectly flagged
  // Async correctness is better checked via the CYNIC dimension scorers
  // REMOVED: saveCollectiveState pattern - same issue as initialize()
  // Better to rely on CYNIC dimension scorers for async correctness
  {
    pattern: /0\.618(?!033)|0\.382(?!966)|0\.236(?!067)/g,
    risk: 'axiom',
    severity: 'low',
    description: 'Imprecise φ constant — should use full precision',
    fix: 'Use PHI_INV (0.618033988749895), PHI_INV_2, or PHI_INV_3 constants',
  },
  {
    pattern: /confidence\s*[>]=?\s*0\.[789]|confidence\s*[>]=?\s*1/g,
    risk: 'axiom',
    severity: 'high',
    description: 'Confidence may exceed φ⁻¹ (61.8%) limit',
    fix: 'Cap confidence at PHI_INV (0.618)',
  },
  // Removed: export pattern was too broad and created false positives
  // API surface changes should be tracked via git diff, not pattern matching
  // REMOVED: CYNICJudge and LearningService are NOT singletons
  // Only CollectivePack, SharedMemory, and QLearningRouter have singleton getters
  // Direct instantiation of Judge and LearningService is intentional in server.js
  {
    pattern: /this\._(?![\w]+\s*=)/g,
    risk: 'encapsulation',
    severity: 'low',
    description: 'Accessing private members directly',
    fix: 'Consider using getter/setter methods',
  },
  {
    pattern: /catch\s*\(\s*\)\s*\{/g,
    risk: 'error_handling',
    severity: 'medium',
    description: 'Empty catch — errors silently swallowed',
    fix: 'Log error or handle appropriately',
  },
];

/**
 * Detect which CYNIC component a file belongs to
 *
 * @param {string} filePath - Path to the modified file
 * @returns {Object} Component info with domain, criticality, scale
 */
function detectComponent(filePath) {
  const normalized = filePath.replace(/\\/g, '/').toLowerCase();
  const basename = path.basename(filePath).toLowerCase();

  for (const [key, info] of Object.entries(CYNIC_COMPONENTS)) {
    if (normalized.includes(key) || basename.includes(key)) {
      return { name: key, ...info };
    }
  }

  // Default for unknown components
  return { name: 'unknown', domain: 'general', criticality: 'low', scale: 'project' };
}

/**
 * Strip comments and string literals from code to avoid false positives
 * This ensures patterns only match actual code, not documentation or strings
 *
 * @param {string} content - Code content
 * @returns {string} Code with comments and strings replaced by spaces
 */
function stripCommentsAndStrings(content) {
  let result = content;

  // Remove multi-line comments /* ... */
  result = result.replace(/\/\*[\s\S]*?\*\//g, match => ' '.repeat(match.length));

  // Remove single-line comments // ...
  result = result.replace(/\/\/[^\n]*/g, match => ' '.repeat(match.length));

  // Remove template literals `...` (preserve length for position accuracy)
  result = result.replace(/`(?:[^`\\]|\\.)*`/g, match => ' '.repeat(match.length));

  // Remove double-quoted strings "..."
  result = result.replace(/"(?:[^"\\]|\\.)*"/g, match => ' '.repeat(match.length));

  // Remove single-quoted strings '...'
  result = result.replace(/'(?:[^'\\]|\\.)*'/g, match => ' '.repeat(match.length));

  // Remove regex literals /.../ (basic — may have edge cases)
  result = result.replace(/\/(?:[^\/\\]|\\.)+\/[gimsuvy]*/g, match => ' '.repeat(match.length));

  return result;
}

/**
 * Analyze code content for fractal risks
 * Strips comments and strings to avoid false positives
 *
 * @param {string} content - Code content to analyze
 * @returns {Object[]} Array of detected risks
 */
function analyzeRisks(content) {
  const risks = [];

  // Strip comments and strings to avoid false positives
  // (patterns should only match actual code, not documentation)
  const codeOnly = stripCommentsAndStrings(content);

  for (const { pattern, risk, severity, description, fix } of FRACTAL_RISK_PATTERNS) {
    // Reset regex state
    pattern.lastIndex = 0;

    let match;
    while ((match = pattern.exec(codeOnly)) !== null) {
      risks.push({
        risk,
        severity,
        description,
        fix,
        match: match[0],
        position: match.index,
      });
    }
  }

  return risks;
}

/**
 * Calculate dimension scores for self-modification
 * Uses CYNIC-specific scoring for code changes
 *
 * @param {Object} item - Self-modification item
 * @param {Object} context - Judgment context
 * @returns {Object} Dimension scores
 */
function scoreSelfModification(item, context = {}) {
  const scores = {};
  const content = item.content || '';
  const risks = item.risks || [];
  const component = item.component || {};

  // ═══════════════════════════════════════════════════════════════════════════
  // PHI AXIOM (φ): Appropriate uncertainty, self-skepticism
  // ═══════════════════════════════════════════════════════════════════════════

  // skepticism: Does the code doubt itself?
  const hasAssertions = /assert|expect|should|must/i.test(content);
  const hasValidation = /if\s*\(!|if\s*\(\s*!\w|typeof\s+\w+\s*[!=]==?\s*['"]undefined/g.test(content);
  scores.skepticism = hasAssertions || hasValidation ? 70 : 40;

  // humility: Does the code avoid overconfidence?
  const phiRisks = risks.filter(r => r.risk === 'axiom');
  scores.humility = phiRisks.length === 0 ? 80 : Math.max(20, 80 - phiRisks.length * 20);

  // uncertainty: Does the code handle unknowns?
  const handlesErrors = /try\s*\{|catch\s*\(|\.catch\s*\(/g.test(content);
  const hasDefaults = /\|\||default|\?\?/g.test(content);
  scores.uncertainty = (handlesErrors ? 40 : 0) + (hasDefaults ? 40 : 0);

  // ═══════════════════════════════════════════════════════════════════════════
  // VERIFY AXIOM: Don't trust, verify
  // ═══════════════════════════════════════════════════════════════════════════

  // verifiability: Can we test this code?
  const isTestable = !/private|#\w+/g.test(content) || /export/g.test(content);
  scores.verifiability = isTestable ? 70 : 40;

  // transparency: Is the code self-documenting?
  const hasComments = /\/\/|\/\*|\*\//g.test(content);
  const hasDescriptiveNames = /[a-z]{3,}[A-Z][a-z]+/g.test(content); // camelCase
  scores.transparency = (hasComments ? 40 : 0) + (hasDescriptiveNames ? 40 : 0);

  // auditability: Can we trace the code's behavior?
  const hasLogging = /log\.|console\./g.test(content);
  const hasMetrics = /metric|telemetry|record/gi.test(content);
  scores.auditability = (hasLogging ? 40 : 0) + (hasMetrics ? 40 : 0);

  // ═══════════════════════════════════════════════════════════════════════════
  // CULTURE AXIOM: Memory makes identity
  // ═══════════════════════════════════════════════════════════════════════════

  // consistency: Does it follow CYNIC patterns?
  const lifecycleRisks = risks.filter(r => r.risk === 'lifecycle');
  const singletonRisks = risks.filter(r => r.risk === 'singleton');
  scores.consistency = Math.max(20, 80 - (lifecycleRisks.length + singletonRisks.length) * 15);

  // coherence: Does it fit with the surrounding code?
  const apiRisks = risks.filter(r => r.risk === 'api');
  scores.coherence = apiRisks.length === 0 ? 75 : Math.max(30, 75 - apiRisks.length * 15);

  // identity: Does it maintain CYNIC's character?
  const hasDogVoice = /sniff|tail|ears|growl|wag|bark/gi.test(content);
  const hasPhiReference = /phi|φ|0\.618|golden/gi.test(content);
  scores.identity = (hasDogVoice ? 30 : 0) + (hasPhiReference ? 30 : 0) + 30;

  // ═══════════════════════════════════════════════════════════════════════════
  // BURN AXIOM: Don't extract, simplify
  // ═══════════════════════════════════════════════════════════════════════════

  // simplicity: Is the code simple?
  const lineCount = content.split('\n').length;
  const avgLineLength = content.length / Math.max(1, lineCount);
  const isSimple = lineCount < 50 && avgLineLength < 80;
  scores.simplicity = isSimple ? 75 : Math.max(30, 75 - Math.floor(lineCount / 20) * 10);

  // directness: Does it do one thing well?
  const functionCount = (content.match(/function\s+\w+|=>\s*\{|\w+\s*\([^)]*\)\s*\{/g) || []).length;
  scores.directness = functionCount <= 2 ? 80 : Math.max(30, 80 - (functionCount - 2) * 10);

  // efficiency: Is it computationally efficient?
  const hasNestedLoops = /for.*for|while.*while|\.forEach.*\.forEach/g.test(content);
  scores.efficiency = hasNestedLoops ? 40 : 70;

  // ═══════════════════════════════════════════════════════════════════════════
  // META DIMENSION: Overall fractal integrity
  // ═══════════════════════════════════════════════════════════════════════════

  // fractal_integrity: Does change maintain fractal coherence?
  const criticalRisks = risks.filter(r => r.severity === 'high');
  const persistenceRisks = risks.filter(r => r.risk === 'persistence');
  scores.fractal_integrity = Math.max(0, 100 - criticalRisks.length * 25 - persistenceRisks.length * 15);

  // ═══════════════════════════════════════════════════════════════════════════
  // CYNIC AXIOM: Self-awareness of own codebase
  // ═══════════════════════════════════════════════════════════════════════════

  // lifecycle_integrity: Does the change maintain proper init/shutdown?
  // (reusing lifecycleRisks from CULTURE section)
  const hasInit = /init|initialize|start|boot|awaken/gi.test(content);
  const hasShutdown = /shutdown|close|cleanup|dispose|destroy/gi.test(content);
  const lifecycleBalanced = (hasInit && hasShutdown) || (!hasInit && !hasShutdown);
  scores.lifecycle_integrity = lifecycleRisks.length === 0
    ? (lifecycleBalanced ? 80 : 60)
    : Math.max(20, 70 - lifecycleRisks.length * 20);

  // persistence_coherence: Are save/load cycles complete?
  const hasSave = /save|persist|store|write.*state/gi.test(content);
  const hasLoad = /load|restore|read.*state|init.*from/gi.test(content);
  const persistenceBalanced = (hasSave && hasLoad) || (!hasSave && !hasLoad);
  scores.persistence_coherence = persistenceRisks.length === 0
    ? (persistenceBalanced ? 80 : 50)
    : Math.max(20, 70 - persistenceRisks.length * 25);

  // fractal_consistency: Does it work at all 8 scales?
  // (axiom → dimension → judgment → response → session → project → ecosystem → cosmos)
  const hasMultiScaleAwareness = /scale|level|layer|scope|context/gi.test(content);
  const hasRecursivePattern = /recursive|fractal|self|meta/gi.test(content);
  scores.fractal_consistency =
    (hasMultiScaleAwareness ? 40 : 20) +
    (hasRecursivePattern ? 40 : 20) -
    (criticalRisks.length * 10);
  scores.fractal_consistency = Math.max(0, Math.min(100, scores.fractal_consistency));

  // singleton_safety: Are singletons properly handled?
  // (reusing singletonRisks from CULTURE section)
  const hasSingletonPattern = /singleton|getInstance|shared|collective/gi.test(content);
  const hasLazyInit = /lazy|once|memoize/gi.test(content);
  scores.singleton_safety = singletonRisks.length === 0
    ? (hasSingletonPattern ? (hasLazyInit ? 85 : 70) : 75)
    : Math.max(20, 75 - singletonRisks.length * 25);

  // async_correctness: Are async operations awaited?
  const hasAsync = /async\s+function|async\s*\(/gi.test(content);
  const hasAwait = /await\s+/gi.test(content);
  const hasPromise = /Promise\.|\.then\(|\.catch\(/gi.test(content);
  const asyncCount = (content.match(/async/gi) || []).length;
  const awaitCount = (content.match(/await/gi) || []).length;
  // If async code exists, awaits should be present
  const asyncBalanced = !hasAsync || (hasAwait && awaitCount >= asyncCount - 1);
  scores.async_correctness = hasAsync
    ? (asyncBalanced ? 80 : 40) + (hasPromise ? 10 : 0)
    : 75; // No async = neutral score

  return scores;
}

/**
 * Calculate axiom scores from dimension scores
 *
 * @param {Object} scores - Dimension scores
 * @returns {Object} Axiom scores
 */
function calculateAxiomScores(scores) {
  return {
    PHI: Math.round((scores.skepticism + scores.humility + scores.uncertainty) / 3),
    VERIFY: Math.round((scores.verifiability + scores.transparency + scores.auditability) / 3),
    CULTURE: Math.round((scores.consistency + scores.coherence + scores.identity) / 3),
    BURN: Math.round((scores.simplicity + scores.directness + scores.efficiency) / 3),
    META: scores.fractal_integrity,
    // CYNIC-specific axiom for self-modification judgment
    CYNIC: Math.round((
      scores.lifecycle_integrity +
      scores.persistence_coherence +
      scores.fractal_consistency +
      scores.singleton_safety +
      scores.async_correctness
    ) / 5),
  };
}

/**
 * Calculate Q-Score (geometric mean of axiom scores)
 *
 * @param {Object} axiomScores - Scores per axiom
 * @returns {Object} Q-Score with verdict
 */
function calculateQScore(axiomScores) {
  const values = Object.values(axiomScores);
  const product = values.reduce((p, v) => p * v, 1);
  const Q = Math.round(Math.pow(product, 1 / values.length));

  // Determine verdict based on Q-Score
  let verdict;
  if (Q >= 80) verdict = { verdict: 'HOWL', emoji: '🐺', description: 'Excellent' };
  else if (Q >= 60) verdict = { verdict: 'WAG', emoji: '🐕', description: 'Good' };
  else if (Q >= 40) verdict = { verdict: 'GROWL', emoji: '😠', description: 'Caution' };
  else verdict = { verdict: 'BARK', emoji: '🚨', description: 'Warning' };

  // Find weakest axiom
  const weakest = Object.entries(axiomScores)
    .sort((a, b) => a[1] - b[1])[0];

  return {
    Q,
    verdict,
    weakest: { axiom: weakest[0], score: weakest[1] },
    confidence: Math.min(PHI_INV * 100, Q * 0.8), // φ-bounded
  };
}

/**
 * Create a self-judgment item from a code modification
 *
 * @param {Object} options - Modification details
 * @param {string} options.filePath - Path to modified file
 * @param {string} options.content - New/modified content
 * @param {string} options.toolName - Edit or Write
 * @param {string} [options.oldContent] - Previous content (for Edit)
 * @returns {Object} Self-judgment item
 */
function createSelfJudgmentItem(options) {
  const { filePath, content, toolName, oldContent } = options;

  const component = detectComponent(filePath);
  const risks = analyzeRisks(content);
  const linesChanged = content.split('\n').length;

  return {
    type: 'self_modification',
    itemType: 'self_modification',
    id: `self_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,

    // File info
    filePath,
    fileName: path.basename(filePath),

    // Component info
    component,

    // Change info
    toolName,
    content,
    oldContent,
    linesChanged,

    // Risk analysis
    risks,
    riskCount: risks.length,
    criticalRiskCount: risks.filter(r => r.severity === 'high').length,

    // Timestamps
    timestamp: Date.now(),

    // Tags for learning
    tags: [
      `component:${component.name}`,
      `domain:${component.domain}`,
      `scale:${component.scale}`,
      `criticality:${component.criticality}`,
      ...risks.map(r => `risk:${r.risk}`),
    ],
  };
}

/**
 * Judge a self-modification
 *
 * @param {Object} item - Self-judgment item
 * @param {Object} [context] - Additional context
 * @returns {Object} Judgment result
 */
function judgeSelfModification(item, context = {}) {
  const t0 = Date.now();

  // Score dimensions
  const dimensionScores = scoreSelfModification(item, context);

  // Calculate axiom scores
  const axiomScores = calculateAxiomScores(dimensionScores);

  // Calculate Q-Score
  const qResult = calculateQScore(axiomScores);

  // Build judgment
  const judgment = {
    id: `sjdg_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    item,

    // Scores
    dimensionScores,
    axiomScores,
    qScore: qResult.Q,
    verdict: qResult.verdict,
    confidence: qResult.confidence,

    // Analysis
    weakest: qResult.weakest,
    risks: item.risks,

    // Metadata
    judgedAt: t0,
    duration_ms: Date.now() - t0,
  };

  return judgment;
}

/**
 * Generate specific fix suggestions based on weakest axiom/dimensions
 *
 * @param {Object} judgment - Self-judgment result
 * @returns {Object} Fix suggestions with priority
 */
function generateFixSuggestions(judgment) {
  const suggestions = [];
  const { dimensionScores, axiomScores, weakest, item } = judgment;

  // Suggest fixes for weakest axiom
  const axiomSuggestions = {
    PHI: [
      { dimension: 'skepticism', threshold: 50, fix: 'Add assertions or validation (assert, if(!x), typeof checks)' },
      { dimension: 'humility', threshold: 50, fix: 'Avoid overconfident constants (100%, always, never). Use φ-aligned limits.' },
      { dimension: 'uncertainty', threshold: 50, fix: 'Add error handling (try/catch, .catch(), || defaults, ??)' },
    ],
    VERIFY: [
      { dimension: 'verifiability', threshold: 50, fix: 'Make code testable - export functions, avoid private state (#)' },
      { dimension: 'transparency', threshold: 50, fix: 'Add descriptive names (camelCase) and comments for complex logic' },
      { dimension: 'auditability', threshold: 50, fix: 'Add logging (console.log) or metrics (recordMetric, telemetry)' },
    ],
    CULTURE: [
      { dimension: 'consistency', threshold: 60, fix: 'Follow CYNIC patterns - check lifecycle (init/shutdown), singleton usage' },
      { dimension: 'coherence', threshold: 60, fix: 'Match existing API patterns in the same domain' },
      { dimension: 'identity', threshold: 50, fix: 'Include CYNIC voice (*sniff*, φ, golden) in logs/comments' },
    ],
    BURN: [
      { dimension: 'simplicity', threshold: 50, fix: 'Reduce file size (<50 lines ideal), split large functions' },
      { dimension: 'directness', threshold: 50, fix: 'Each function should do one thing well (≤2 functions ideal)' },
      { dimension: 'efficiency', threshold: 50, fix: 'Avoid nested loops (O(n²)). Consider Set/Map for lookups.' },
    ],
    META: [
      { dimension: 'fractal_integrity', threshold: 70, fix: 'Ensure change works at all scales. Check if it affects other components.' },
    ],
    CYNIC: [
      { dimension: 'lifecycle_integrity', threshold: 60, fix: 'Balance init with shutdown. Every awaken() needs a dispose().' },
      { dimension: 'persistence_coherence', threshold: 60, fix: 'Balance save with load. Every _persist() needs a load().' },
      { dimension: 'fractal_consistency', threshold: 50, fix: 'Add scale/context awareness. Consider how change affects other fractal levels.' },
      { dimension: 'singleton_safety', threshold: 60, fix: 'Use lazy initialization. Check for race conditions in singleton access.' },
      { dimension: 'async_correctness', threshold: 60, fix: 'Ensure all async functions are awaited. Check for missing await keywords.' },
    ],
  };

  // Check weakest axiom first
  const weakAxiom = weakest.axiom;
  if (axiomSuggestions[weakAxiom]) {
    for (const s of axiomSuggestions[weakAxiom]) {
      const score = dimensionScores[s.dimension];
      if (score !== undefined && score < s.threshold) {
        suggestions.push({
          axiom: weakAxiom,
          dimension: s.dimension,
          score,
          threshold: s.threshold,
          fix: s.fix,
          priority: s.threshold - score, // Higher priority for bigger gaps
        });
      }
    }
  }

  // Also check CYNIC axiom (self-awareness) if it exists
  if (axiomScores.CYNIC && axiomScores.CYNIC < 60) {
    for (const s of axiomSuggestions.CYNIC) {
      const score = dimensionScores[s.dimension];
      if (score !== undefined && score < s.threshold) {
        suggestions.push({
          axiom: 'CYNIC',
          dimension: s.dimension,
          score,
          threshold: s.threshold,
          fix: s.fix,
          priority: s.threshold - score,
        });
      }
    }
  }

  // Sort by priority (highest gap first)
  suggestions.sort((a, b) => b.priority - a.priority);

  return {
    suggestions: suggestions.slice(0, 5), // Top 5 suggestions
    totalIssues: suggestions.length,
    primaryFix: suggestions[0] || null,
  };
}

/**
 * Format judgment for console output
 *
 * @param {Object} judgment - Self-judgment result
 * @returns {string[]} Lines to output
 */
function formatJudgmentOutput(judgment) {
  const lines = [];
  const { item, qScore, verdict, axiomScores, weakest, risks } = judgment;

  // Header
  lines.push(`\n🪞 SELF-JUDGMENT: ${item.component.name} (${item.fileName})`);
  lines.push(`   Q-Score: ${qScore} ${verdict.emoji} ${verdict.verdict}`);

  // Axiom breakdown (include CYNIC if present)
  let axiomLine = `   Axioms: PHI:${axiomScores.PHI} VERIFY:${axiomScores.VERIFY} CULTURE:${axiomScores.CULTURE} BURN:${axiomScores.BURN} META:${axiomScores.META}`;
  if (axiomScores.CYNIC !== undefined) {
    axiomLine += ` CYNIC:${axiomScores.CYNIC}`;
  }
  lines.push(axiomLine);

  // Weakest axiom
  if (weakest.score < 60) {
    lines.push(`   ⚠️ Weakest: ${weakest.axiom} (${weakest.score})`);
  }

  // Risks
  if (risks.length > 0) {
    lines.push(`   Risks detected: ${risks.length}`);
    const criticalRisks = risks.filter(r => r.severity === 'high');
    for (const risk of criticalRisks.slice(0, 3)) {
      lines.push(`      🔴 ${risk.risk}: ${risk.description}`);
      lines.push(`         Fix: ${risk.fix}`);
    }

    const mediumRisks = risks.filter(r => r.severity === 'medium');
    for (const risk of mediumRisks.slice(0, 2)) {
      lines.push(`      🟡 ${risk.risk}: ${risk.description}`);
    }
  }

  // Fix suggestions for low scores
  if (qScore < 70) {
    const fixResult = generateFixSuggestions(judgment);
    if (fixResult.primaryFix) {
      lines.push(`   💡 Primary fix (${fixResult.primaryFix.dimension}):`);
      lines.push(`      ${fixResult.primaryFix.fix}`);
    }
    if (fixResult.suggestions.length > 1) {
      lines.push(`   📋 ${fixResult.totalIssues - 1} more suggestions available`);
    }
  }

  // Confidence
  lines.push(`   Confidence: ${Math.round(judgment.confidence)}% (φ-bounded)`);

  return lines;
}

// Exports
module.exports = {
  // Core functions
  createSelfJudgmentItem,
  judgeSelfModification,
  formatJudgmentOutput,
  generateFixSuggestions,

  // Analysis functions
  detectComponent,
  analyzeRisks,
  scoreSelfModification,
  calculateAxiomScores,
  calculateQScore,

  // Constants
  CYNIC_COMPONENTS,
  FRACTAL_SCALES,
  FRACTAL_RISK_PATTERNS,
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
};
