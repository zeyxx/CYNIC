/**
 * @cynic/node - Organic Profile Signals
 *
 * 100% automatic signal detection from user behavior.
 * No explicit declaration - profile emerges from interaction patterns.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/profile/organic-signals
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * φ-aligned constants for signal detection
 */
export const SIGNAL_CONSTANTS = {
  /** Fibonacci weights for signal combination */
  WEIGHTS: {
    linguistic: 8,    // Fib(6) - Most immediate signal
    behavioral: 5,    // Fib(5) - Strong indicator
    code: 3,          // Fib(4) - When available
    temporal: 2,      // Fib(3) - Long-term refinement
  },

  /** Total weight for normalization */
  TOTAL_WEIGHT: 18,   // 8 + 5 + 3 + 2

  /** Signal thresholds for level mapping */
  LEVEL_THRESHOLDS: {
    1: 0,     // Novice: 0-20
    2: 20,    // Apprentice: 20-40
    3: 40,    // Practitioner: 40-60
    5: 60,    // Expert: 60-80
    8: 80,    // Master: 80-100
  },

  /** Re-evaluation interval (Fib(8) = 21 interactions) */
  REEVALUATION_INTERVAL: 21,

  /** Minimum samples for confidence (Fib(5) = 5) */
  MIN_SAMPLES_FOR_CONFIDENCE: 5,

  /** Max confidence cap (φ⁻¹) */
  MAX_CONFIDENCE: PHI_INV,
};

/**
 * Signal types for categorization
 */
export const SignalType = {
  LINGUISTIC: 'linguistic',
  BEHAVIORAL: 'behavioral',
  CODE: 'code',
  TEMPORAL: 'temporal',
};

// ═══════════════════════════════════════════════════════════════════════════
// LINGUISTIC SIGNALS (from message content)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Analyze vocabulary complexity
 *
 * @param {string} message - User message
 * @returns {number} Score 0-100
 */
export function analyzeVocabulary(message) {
  if (!message || typeof message !== 'string') return 50;

  const words = message.toLowerCase().split(/\s+/).filter(w => w.length > 0);
  if (words.length === 0) return 50;

  // Technical term indicators
  const technicalPatterns = [
    /async|await|promise|callback/i,
    /api|rest|graphql|endpoint/i,
    /docker|kubernetes|k8s|container/i,
    /git|commit|branch|merge|rebase/i,
    /database|sql|query|index/i,
    /algorithm|complexity|optimization/i,
    /architecture|pattern|design/i,
    /component|module|service|layer/i,
    /interface|abstract|polymorphism/i,
    /concurrency|thread|mutex|lock/i,
    /encryption|hash|authentication/i,
    /deployment|ci\/cd|pipeline/i,
  ];

  // Count technical terms
  const technicalCount = technicalPatterns.reduce((count, pattern) => {
    return count + (pattern.test(message) ? 1 : 0);
  }, 0);

  // Average word length (longer words often more technical)
  const avgWordLength = words.reduce((sum, w) => sum + w.length, 0) / words.length;

  // Unique word ratio (vocabulary diversity)
  const uniqueRatio = new Set(words).size / words.length;

  // Calculate score (weighted combination)
  const technicalScore = Math.min(100, technicalCount * 10);
  const lengthScore = Math.min(100, (avgWordLength - 3) * 15);
  const diversityScore = uniqueRatio * 100;

  return Math.round(
    technicalScore * 0.5 +
    lengthScore * 0.25 +
    diversityScore * 0.25
  );
}

/**
 * Classify question depth
 *
 * what → why → how → trade-offs
 *
 * @param {string} message - User message
 * @returns {{ depth: number, type: string }} Depth score and type
 */
export function classifyQuestion(message) {
  if (!message || typeof message !== 'string') {
    return { depth: 50, type: 'unknown' };
  }

  const lower = message.toLowerCase();

  // Level 1: Basic "what" questions
  if (/^(what|qui|quoi|c'est quoi|what's|what is|define|explain what)/i.test(lower)) {
    return { depth: 25, type: 'what' };
  }

  // Level 2: "Why" questions - understanding motivation
  if (/^(why|pourquoi|why does|why is|reason|purpose)/i.test(lower)) {
    return { depth: 50, type: 'why' };
  }

  // Level 3: "How" questions - implementation
  if (/^(how|comment|how do|how to|how can|implement)/i.test(lower)) {
    return { depth: 65, type: 'how' };
  }

  // Level 4: Trade-off questions - expert level
  const tradeoffPatterns = [
    /trade-?off/i,
    /vs\.?|versus/i,
    /better|worse|prefer/i,
    /pros.*cons|cons.*pros/i,
    /when.*should.*use/i,
    /compare|comparison/i,
    /advantages.*disadvantages/i,
    /performance.*versus|versus.*performance/i,
  ];

  if (tradeoffPatterns.some(p => p.test(lower))) {
    return { depth: 85, type: 'tradeoff' };
  }

  // Level 5: Architecture/design questions - master level
  const architecturePatterns = [
    /architect/i,
    /design pattern/i,
    /scalab/i,
    /distributed/i,
    /microservice/i,
    /system design/i,
    /high availability/i,
    /fault toleran/i,
  ];

  if (architecturePatterns.some(p => p.test(lower))) {
    return { depth: 95, type: 'architecture' };
  }

  // Default: statement or unknown question type
  return { depth: 50, type: 'statement' };
}

/**
 * Calculate technical term density
 *
 * @param {string} message - User message
 * @returns {number} Density 0-1
 */
export function calculateTechnicalDensity(message) {
  if (!message || typeof message !== 'string') return 0.5;

  const words = message.split(/\s+/).filter(w => w.length > 0);
  if (words.length === 0) return 0.5;

  // Technical indicators (more comprehensive list)
  const technicalWords = new Set([
    // Programming
    'function', 'class', 'method', 'variable', 'const', 'let', 'var',
    'async', 'await', 'promise', 'callback', 'closure', 'scope',
    'array', 'object', 'string', 'number', 'boolean', 'null', 'undefined',
    'interface', 'type', 'generic', 'enum', 'abstract',
    // Architecture
    'api', 'rest', 'graphql', 'grpc', 'websocket', 'http', 'https',
    'database', 'sql', 'nosql', 'query', 'index', 'schema',
    'cache', 'redis', 'memcached', 'cdn',
    'docker', 'kubernetes', 'container', 'pod', 'service',
    // Operations
    'deploy', 'ci', 'cd', 'pipeline', 'build', 'test',
    'log', 'monitor', 'metric', 'alert', 'trace',
    'git', 'commit', 'branch', 'merge', 'rebase', 'pull', 'push',
    // Security
    'auth', 'authentication', 'authorization', 'token', 'jwt', 'oauth',
    'encrypt', 'decrypt', 'hash', 'salt', 'certificate',
  ]);

  const technicalCount = words.filter(w =>
    technicalWords.has(w.toLowerCase().replace(/[^a-z]/g, ''))
  ).length;

  return technicalCount / words.length;
}

/**
 * Detect self-correction patterns
 * Users who correct themselves show learning/awareness
 *
 * @param {string[]} messageHistory - Recent messages
 * @returns {number} Self-correction rate 0-1
 */
export function detectSelfCorrection(messageHistory) {
  if (!Array.isArray(messageHistory) || messageHistory.length < 2) {
    return 0;
  }

  const correctionPatterns = [
    /actually|wait|sorry|my bad|correction|i meant/i,
    /no,? wait|let me rephrase|i was wrong/i,
    /scratch that|never mind|ignore that/i,
  ];

  let corrections = 0;
  for (const msg of messageHistory) {
    if (correctionPatterns.some(p => p.test(msg))) {
      corrections++;
    }
  }

  return corrections / messageHistory.length;
}

/**
 * Combined linguistic signal
 *
 * @param {string} message - Current message
 * @param {string[]} [history] - Message history
 * @returns {{ score: number, breakdown: object }}
 */
export function calculateLinguisticSignal(message, history = []) {
  const vocabularyScore = analyzeVocabulary(message);
  const { depth: questionDepth } = classifyQuestion(message);
  const technicalDensity = calculateTechnicalDensity(message);
  const selfCorrection = detectSelfCorrection(history);

  // Weighted combination
  const score = Math.round(
    vocabularyScore * 0.3 +
    questionDepth * 0.4 +
    technicalDensity * 100 * 0.2 +
    selfCorrection * 100 * 0.1
  );

  return {
    score: Math.min(100, Math.max(0, score)),
    breakdown: {
      vocabulary: vocabularyScore,
      questionDepth,
      technicalDensity: Math.round(technicalDensity * 100),
      selfCorrection: Math.round(selfCorrection * 100),
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// BEHAVIORAL SIGNALS (from tool usage)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tool complexity weights
 */
export const TOOL_COMPLEXITY = {
  // Basic tools (Novice-friendly)
  read_file: 1,
  list_dir: 1,
  search: 2,

  // Intermediate tools
  grep: 2,
  glob: 2,
  web_fetch: 2,
  web_search: 2,

  // Applied tools
  edit: 3,
  write: 3,
  bash: 3,

  // Expert tools
  architect: 5,
  refactor: 5,
  review: 5,
  analyze: 5,

  // Default for unknown tools
  default: 2,
};

/**
 * Calculate tool complexity score
 *
 * @param {Array<{ tool: string, success: boolean }>} toolUsage - Recent tool calls
 * @returns {{ score: number, avgComplexity: number }}
 */
export function calculateToolComplexity(toolUsage) {
  if (!Array.isArray(toolUsage) || toolUsage.length === 0) {
    return { score: 50, avgComplexity: 2 };
  }

  let totalComplexity = 0;
  let successfulComplexity = 0;
  let successCount = 0;

  for (const { tool, success } of toolUsage) {
    const complexity = TOOL_COMPLEXITY[tool] || TOOL_COMPLEXITY.default;
    totalComplexity += complexity;

    if (success) {
      successfulComplexity += complexity;
      successCount++;
    }
  }

  const avgComplexity = totalComplexity / toolUsage.length;
  const successRate = successCount / toolUsage.length;

  // Score: complexity × success rate
  const score = Math.round((avgComplexity / 5) * 100 * (0.5 + successRate * 0.5));

  return {
    score: Math.min(100, Math.max(0, score)),
    avgComplexity: Math.round(avgComplexity * 10) / 10,
    successRate: Math.round(successRate * 100),
    totalCalls: toolUsage.length,
  };
}

/**
 * Calculate error recovery rate
 * How quickly user recovers from errors indicates experience
 *
 * @param {Array<{ timestamp: number, isError: boolean }>} events - Event timeline
 * @returns {{ score: number, avgRecoveryTime: number }}
 */
export function calculateErrorRecovery(events) {
  if (!Array.isArray(events) || events.length < 2) {
    return { score: 50, avgRecoveryTime: null };
  }

  const errorStarts = [];
  const recoveryTimes = [];

  for (let i = 0; i < events.length; i++) {
    const event = events[i];

    if (event.isError) {
      errorStarts.push(event.timestamp);
    } else if (errorStarts.length > 0) {
      // Success after error = recovery
      const recoveryTime = event.timestamp - errorStarts[errorStarts.length - 1];
      recoveryTimes.push(recoveryTime);
      errorStarts.pop();
    }
  }

  if (recoveryTimes.length === 0) {
    return { score: 75, avgRecoveryTime: null }; // No errors = good
  }

  const avgRecoveryTime = recoveryTimes.reduce((a, b) => a + b, 0) / recoveryTimes.length;

  // Fast recovery (< 30s) = expert, slow (> 5min) = novice
  // Using exponential decay
  const score = Math.round(100 * Math.exp(-avgRecoveryTime / 180000)); // 3 min half-life

  return {
    score: Math.min(100, Math.max(0, score)),
    avgRecoveryTime: Math.round(avgRecoveryTime / 1000), // seconds
    totalErrors: recoveryTimes.length + errorStarts.length,
  };
}

/**
 * Calculate iteration depth
 * More iterations before satisfaction = potentially less experienced
 *
 * @param {number} iterationCount - Number of iterations/attempts
 * @param {boolean} satisfied - Whether final result was satisfactory
 * @returns {number} Score 0-100
 */
export function calculateIterationDepth(iterationCount, satisfied) {
  if (!satisfied) {
    return 30; // Unsatisfied = lower score
  }

  // Fewer iterations with success = higher score
  // 1 iteration = expert (95), 10+ iterations = novice (35)
  const score = Math.max(35, 100 - (iterationCount - 1) * 7);

  return Math.round(score);
}

/**
 * Combined behavioral signal
 *
 * @param {Array<{ tool: string, success: boolean }>} toolUsage
 * @param {Array<{ timestamp: number, isError: boolean }>} [events]
 * @param {{ iterations: number, satisfied: boolean }} [iterationInfo]
 * @returns {{ score: number, breakdown: object }}
 */
export function calculateBehavioralSignal(toolUsage, events = [], iterationInfo = null) {
  const toolResult = calculateToolComplexity(toolUsage);
  const errorResult = calculateErrorRecovery(events);
  const iterationScore = iterationInfo
    ? calculateIterationDepth(iterationInfo.iterations, iterationInfo.satisfied)
    : 50;

  const score = Math.round(
    toolResult.score * 0.4 +
    errorResult.score * 0.35 +
    iterationScore * 0.25
  );

  return {
    score: Math.min(100, Math.max(0, score)),
    breakdown: {
      toolComplexity: toolResult.score,
      avgComplexity: toolResult.avgComplexity,
      errorRecovery: errorResult.score,
      iterationDepth: iterationScore,
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// CODE SIGNALS (from submitted code)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Analyze code abstraction level
 *
 * @param {string} code - Code snippet
 * @returns {{ score: number, level: string }}
 */
export function analyzeAbstractionLevel(code) {
  if (!code || typeof code !== 'string') {
    return { score: 50, level: 'unknown' };
  }

  const patterns = {
    // Metaprogramming (highest) - Pattern detection for analysis, not execution
    metaprogramming: [
      /Proxy\s*\(/,
      /Reflect\./,
      /Object\.defineProperty/,
      /decorator/i,
      /@\w+\s*[(\n]/,
    ],
    // Generics/types (high)
    generics: [
      /<T>|<T,|<T\s+extends/,
      /type\s+\w+<\w+>/,
      /interface\s+\w+<\w+>/,
      /generic|Generic/,
    ],
    // Classes/OOP (medium-high)
    classes: [
      /class\s+\w+/,
      /extends\s+\w+/,
      /implements\s+\w+/,
      /abstract\s+class/,
      /private\s+\w+|public\s+\w+|protected\s+\w+/,
    ],
    // Functions/modules (medium)
    functions: [
      /function\s+\w+/,
      /const\s+\w+\s*=\s*\(/,
      /=>\s*\{/,
      /export\s+(default\s+)?/,
      /import\s+/,
    ],
    // Basic (low)
    basic: [
      /console\.log/,
      /var\s+\w+/,
      /if\s*\(.*\)\s*\{/,
      /for\s*\(.*\)\s*\{/,
    ],
  };

  // Count matches per level
  const counts = {};
  let totalMatches = 0;

  for (const [level, pats] of Object.entries(patterns)) {
    counts[level] = pats.filter(p => p.test(code)).length;
    totalMatches += counts[level];
  }

  // Determine dominant level
  if (counts.metaprogramming > 0) {
    return { score: 95, level: 'metaprogramming' };
  }
  if (counts.generics > 0) {
    return { score: 80, level: 'generics' };
  }
  if (counts.classes >= 2) {
    return { score: 65, level: 'oop' };
  }
  if (counts.functions >= 2) {
    return { score: 50, level: 'functions' };
  }
  if (counts.basic >= 2) {
    return { score: 30, level: 'basic' };
  }

  return { score: 50, level: 'mixed' };
}

/**
 * Score error handling maturity
 *
 * @param {string} code - Code snippet
 * @returns {{ score: number, patterns: string[] }}
 */
export function scoreErrorHandling(code) {
  if (!code || typeof code !== 'string') {
    return { score: 50, patterns: [] };
  }

  const patterns = [];
  let score = 30; // Base score

  // Basic try-catch
  if (/try\s*\{/.test(code) && /catch\s*\(/.test(code)) {
    score += 15;
    patterns.push('try-catch');
  }

  // Specific error types
  if (/catch\s*\(\s*\w+:\s*\w+Error/.test(code) || /instanceof\s+\w+Error/.test(code)) {
    score += 15;
    patterns.push('typed-errors');
  }

  // Error logging
  if (/console\.error|logger\.error|log\.error/.test(code)) {
    score += 10;
    patterns.push('error-logging');
  }

  // Error propagation
  if (/throw\s+new\s+\w+Error/.test(code)) {
    score += 10;
    patterns.push('error-propagation');
  }

  // Result/Option pattern (advanced)
  if (/Result<|Option<|Either<|\.ok\(|\.err\(|isErr\(\)|isOk\(\)/.test(code)) {
    score += 15;
    patterns.push('result-pattern');
  }

  // Async error handling
  if (/\.catch\s*\(|async.*try|await.*catch/.test(code)) {
    score += 10;
    patterns.push('async-errors');
  }

  // Finally clause
  if (/finally\s*\{/.test(code)) {
    score += 5;
    patterns.push('finally');
  }

  return {
    score: Math.min(100, score),
    patterns,
  };
}

/**
 * Check for testing awareness
 *
 * @param {string} code - Code snippet
 * @returns {{ hasTests: boolean, score: number, framework: string | null }}
 */
export function analyzeTestingAwareness(code) {
  if (!code || typeof code !== 'string') {
    return { hasTests: false, score: 30, framework: null };
  }

  const frameworks = {
    jest: /describe\s*\(|it\s*\(|test\s*\(|expect\s*\(/,
    mocha: /describe\s*\(|it\s*\(|beforeEach|afterEach/,
    vitest: /describe\s*\(|it\s*\(|vi\.|expect\s*\(/,
    nodeTest: /import.*node:test|from\s+'node:test'/,
    pytest: /def\s+test_|@pytest/,
    unittest: /class\s+\w+.*TestCase|self\.assert/,
  };

  let detectedFramework = null;
  let score = 30;

  for (const [name, pattern] of Object.entries(frameworks)) {
    if (pattern.test(code)) {
      detectedFramework = name;
      score = 70;
      break;
    }
  }

  if (detectedFramework) {
    // Additional test quality indicators
    if (/beforeEach|afterEach|setUp|tearDown/.test(code)) {
      score += 10;
    }
    if (/mock|Mock|stub|spy|jest\.fn/.test(code)) {
      score += 10;
    }
    if (/assert\w+|expect\s*\(.*\)\.(to|not)/.test(code)) {
      score += 10;
    }
  }

  return {
    hasTests: detectedFramework !== null,
    score: Math.min(100, score),
    framework: detectedFramework,
  };
}

/**
 * Count recognized architectural patterns
 *
 * @param {string} code - Code snippet
 * @returns {{ score: number, patterns: string[] }}
 */
export function countArchitecturalPatterns(code) {
  if (!code || typeof code !== 'string') {
    return { score: 50, patterns: [] };
  }

  const patternDefinitions = {
    singleton: /static\s+instance|getInstance\s*\(|private\s+constructor/,
    factory: /Factory|create\w+\s*\(|build\w+\s*\(/,
    observer: /subscribe|unsubscribe|notify|emit|on\s*\(|off\s*\(/,
    strategy: /Strategy|setStrategy|executeStrategy/,
    decorator: /Decorator|wrap|@\w+/,
    repository: /Repository|findBy|getAll|save\s*\(/,
    service: /Service|@Injectable|@Service/,
    middleware: /middleware|next\s*\(\)|use\s*\(/,
    mvc: /Controller|Model|View|@Controller|@Get|@Post/,
    dependency_injection: /inject|@Inject|Container|resolve\s*\(/,
  };

  const detected = [];
  for (const [name, pattern] of Object.entries(patternDefinitions)) {
    if (pattern.test(code)) {
      detected.push(name);
    }
  }

  // Score based on pattern count
  const score = Math.min(100, 40 + detected.length * 15);

  return {
    score,
    patterns: detected,
  };
}

/**
 * Combined code signal
 *
 * @param {string} code - Code snippet
 * @returns {{ score: number, breakdown: object }}
 */
export function calculateCodeSignal(code) {
  const abstraction = analyzeAbstractionLevel(code);
  const errorHandling = scoreErrorHandling(code);
  const testing = analyzeTestingAwareness(code);
  const architectural = countArchitecturalPatterns(code);

  const score = Math.round(
    abstraction.score * 0.3 +
    errorHandling.score * 0.25 +
    testing.score * 0.2 +
    architectural.score * 0.25
  );

  return {
    score: Math.min(100, Math.max(0, score)),
    breakdown: {
      abstractionLevel: abstraction.score,
      abstractionType: abstraction.level,
      errorHandling: errorHandling.score,
      errorPatterns: errorHandling.patterns,
      testingAwareness: testing.score,
      testFramework: testing.framework,
      architecturalPatterns: architectural.patterns,
      architecturalScore: architectural.score,
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TEMPORAL SIGNALS (from session history)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate learning rate
 * How fast signals improve over time
 *
 * @param {Array<{ value: number, timestamp: number }>} signalHistory
 * @returns {{ score: number, slope: number }}
 */
export function calculateLearningRate(signalHistory) {
  if (!Array.isArray(signalHistory) || signalHistory.length < 3) {
    return { score: 50, slope: 0 };
  }

  // Simple linear regression
  const n = signalHistory.length;
  let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0;

  for (let i = 0; i < n; i++) {
    sumX += i;
    sumY += signalHistory[i].value;
    sumXY += i * signalHistory[i].value;
    sumX2 += i * i;
  }

  const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

  // Positive slope = improving, map to score
  // slope of 5 per interaction = excellent (100)
  // slope of 0 = neutral (50)
  // slope of -5 = declining (0)
  const score = Math.round(50 + slope * 10);

  return {
    score: Math.min(100, Math.max(0, score)),
    slope: Math.round(slope * 100) / 100,
  };
}

/**
 * Calculate signal consistency
 * Stable signals indicate established skill level
 *
 * @param {number[]} signals - Recent signal values
 * @returns {{ score: number, variance: number }}
 */
export function calculateConsistency(signals) {
  if (!Array.isArray(signals) || signals.length < 2) {
    return { score: 50, variance: 0 };
  }

  const mean = signals.reduce((a, b) => a + b, 0) / signals.length;
  const variance = signals.reduce((acc, s) => acc + Math.pow(s - mean, 2), 0) / signals.length;
  const stdDev = Math.sqrt(variance);

  // Lower variance = higher consistency
  // stdDev of 0 = perfect (100)
  // stdDev of 30 = very inconsistent (10)
  const score = Math.round(100 * Math.exp(-stdDev / 20));

  return {
    score: Math.min(100, Math.max(0, score)),
    variance: Math.round(variance * 10) / 10,
    stdDev: Math.round(stdDev * 10) / 10,
  };
}

/**
 * Calculate engagement depth
 * Longer, focused sessions indicate higher engagement
 *
 * @param {Array<{ durationMs: number }>} sessions - Session data
 * @returns {{ score: number, avgDuration: number }}
 */
export function calculateEngagementDepth(sessions) {
  if (!Array.isArray(sessions) || sessions.length === 0) {
    return { score: 50, avgDuration: 0 };
  }

  const avgDuration = sessions.reduce((acc, s) => acc + (s.durationMs || 0), 0) / sessions.length;

  // Map duration to score
  // < 5 min = low engagement (30)
  // 30 min = moderate (60)
  // 60+ min = high engagement (90)
  const minutes = avgDuration / 60000;
  const score = Math.round(30 + Math.min(60, minutes));

  return {
    score: Math.min(100, Math.max(0, score)),
    avgDuration: Math.round(avgDuration / 1000), // seconds
    avgMinutes: Math.round(minutes * 10) / 10,
  };
}

/**
 * Combined temporal signal
 *
 * @param {Array<{ value: number, timestamp: number }>} signalHistory
 * @param {Array<{ durationMs: number }>} [sessions]
 * @returns {{ score: number, breakdown: object }}
 */
export function calculateTemporalSignal(signalHistory, sessions = []) {
  const learningRate = calculateLearningRate(signalHistory);
  const consistency = calculateConsistency(signalHistory.map(h => h.value));
  const engagement = calculateEngagementDepth(sessions);

  const score = Math.round(
    learningRate.score * 0.35 +
    consistency.score * 0.35 +
    engagement.score * 0.3
  );

  return {
    score: Math.min(100, Math.max(0, score)),
    breakdown: {
      learningRate: learningRate.score,
      slope: learningRate.slope,
      consistency: consistency.score,
      variance: consistency.variance,
      engagement: engagement.score,
      avgSessionMinutes: engagement.avgMinutes,
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// SIGNAL COLLECTOR (Combines All Sources)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Organic signal data structure
 */
export class OrganicSignals {
  constructor() {
    this.linguistic = null;
    this.behavioral = null;
    this.code = null;
    this.temporal = null;
    this.collectedAt = null;
    this.sampleCounts = {
      linguistic: 0,
      behavioral: 0,
      code: 0,
      temporal: 0,
    };
  }

  /**
   * Collect linguistic signal (calculate and return without storing)
   */
  collectLinguistic(message, history = []) {
    return calculateLinguisticSignal(message, history);
  }

  /**
   * Collect behavioral signal (calculate and return without storing)
   */
  collectBehavioral(toolUsage, events = [], iterationInfo = null) {
    return calculateBehavioralSignal(toolUsage, events, iterationInfo);
  }

  /**
   * Collect code signal (calculate and return without storing)
   */
  collectCode(code) {
    return calculateCodeSignal(code);
  }

  /**
   * Collect temporal signal (calculate and return without storing)
   */
  collectTemporal(signalHistory, sessions = []) {
    return calculateTemporalSignal(signalHistory, sessions);
  }

  /**
   * Update linguistic signal (calculate, store, and return)
   */
  updateLinguistic(message, history = []) {
    this.linguistic = calculateLinguisticSignal(message, history);
    this.sampleCounts.linguistic++;
    this.collectedAt = new Date();
    return this.linguistic;
  }

  /**
   * Update behavioral signal (calculate, store, and return)
   */
  updateBehavioral(toolUsage, events = [], iterationInfo = null) {
    this.behavioral = calculateBehavioralSignal(toolUsage, events, iterationInfo);
    this.sampleCounts.behavioral++;
    this.collectedAt = new Date();
    return this.behavioral;
  }

  /**
   * Update code signal (calculate, store, and return)
   */
  updateCode(code) {
    this.code = calculateCodeSignal(code);
    this.sampleCounts.code++;
    this.collectedAt = new Date();
    return this.code;
  }

  /**
   * Update temporal signal (calculate, store, and return)
   */
  updateTemporal(signalHistory, sessions = []) {
    this.temporal = calculateTemporalSignal(signalHistory, sessions);
    this.sampleCounts.temporal++;
    this.collectedAt = new Date();
    return this.temporal;
  }

  /**
   * Get combined score with Fibonacci weights
   *
   * @returns {{ score: number, level: number, confidence: number }}
   */
  getCombinedScore() {
    const weights = SIGNAL_CONSTANTS.WEIGHTS;
    let weightedSum = 0;
    let usedWeight = 0;

    if (this.linguistic) {
      weightedSum += this.linguistic.score * weights.linguistic;
      usedWeight += weights.linguistic;
    }
    if (this.behavioral) {
      weightedSum += this.behavioral.score * weights.behavioral;
      usedWeight += weights.behavioral;
    }
    if (this.code) {
      weightedSum += this.code.score * weights.code;
      usedWeight += weights.code;
    }
    if (this.temporal) {
      weightedSum += this.temporal.score * weights.temporal;
      usedWeight += weights.temporal;
    }

    if (usedWeight === 0) {
      return { score: 50, level: 3, confidence: 0 };
    }

    const score = weightedSum / usedWeight;

    // Calculate confidence based on sample counts
    const totalSamples = Object.values(this.sampleCounts).reduce((a, b) => a + b, 0);
    const confidence = Math.min(
      SIGNAL_CONSTANTS.MAX_CONFIDENCE,
      totalSamples / (SIGNAL_CONSTANTS.MIN_SAMPLES_FOR_CONFIDENCE * 4)
    );

    // Map score to Fibonacci level
    const level = this._scoreToLevel(score);

    return {
      score: Math.round(score),
      level,
      confidence: Math.round(confidence * 1000) / 1000,
    };
  }

  /**
   * Map score to Fibonacci level
   * @private
   */
  _scoreToLevel(score) {
    const thresholds = SIGNAL_CONSTANTS.LEVEL_THRESHOLDS;

    if (score >= thresholds[8]) return 8;
    if (score >= thresholds[5]) return 5;
    if (score >= thresholds[3]) return 3;
    if (score >= thresholds[2]) return 2;
    return 1;
  }

  /**
   * Get full breakdown
   */
  getBreakdown() {
    return {
      linguistic: this.linguistic,
      behavioral: this.behavioral,
      code: this.code,
      temporal: this.temporal,
      sampleCounts: this.sampleCounts,
      collectedAt: this.collectedAt?.toISOString(),
      combined: this.getCombinedScore(),
    };
  }
}

export default {
  SIGNAL_CONSTANTS,
  SignalType,
  // Linguistic
  analyzeVocabulary,
  classifyQuestion,
  calculateTechnicalDensity,
  detectSelfCorrection,
  calculateLinguisticSignal,
  // Behavioral
  TOOL_COMPLEXITY,
  calculateToolComplexity,
  calculateErrorRecovery,
  calculateIterationDepth,
  calculateBehavioralSignal,
  // Code
  analyzeAbstractionLevel,
  scoreErrorHandling,
  analyzeTestingAwareness,
  countArchitecturalPatterns,
  calculateCodeSignal,
  // Temporal
  calculateLearningRate,
  calculateConsistency,
  calculateEngagementDepth,
  calculateTemporalSignal,
  // Collector
  OrganicSignals,
};
