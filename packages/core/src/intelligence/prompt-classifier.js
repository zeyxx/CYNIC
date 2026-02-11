/**
 * Prompt Classifier — Intent detection + domain relevance scoring
 *
 * Replaces ~500-token Brain.think() calls with instant regex classification.
 * Maps prompts to the 7×7 matrix domains and scores relevance per section.
 *
 * Usage:
 *   const result = classifyPrompt("fix the solana transaction bug");
 *   // { intent: 'debug', domains: { SOLANA: 0.9, CODE: 0.7 }, complexity: 'medium', tokenBudget: 800 }
 *
 * "Le chien renifle avant de creuser" — κυνικός
 *
 * @module @cynic/core/intelligence/prompt-classifier
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '../axioms/constants.js';

// =============================================================================
// DOMAIN KEYWORDS — one entry per 7×7 matrix row
// =============================================================================

const DOMAIN_KEYWORDS = {
  CODE: {
    strong: ['code', 'function', 'class', 'module', 'import', 'export', 'refactor', 'lint',
      'typescript', 'javascript', 'python', 'test', 'coverage', 'dependency', 'package',
      'compile', 'build', 'webpack', 'eslint', 'prettier', 'debug', 'stack trace',
      'factory', 'singleton', 'pattern', 'abstract', 'interface', 'implement'],
    weak: ['file', 'fix', 'bug', 'error', 'create', 'update', 'delete', 'add', 'remove',
      'write', 'read', 'edit', 'change', 'modify', 'method', 'variable', 'type'],
  },
  SOLANA: {
    strong: ['solana', 'anchor', 'spl', 'token', 'wallet', 'keypair', 'lamport',
      'transaction', 'instruction', 'program', 'devnet', 'mainnet', 'airdrop',
      'mint', 'burn', 'stake', 'validator', 'rpc', 'web3', 'phantom',
      'pump.fun', 'raydium', 'jupiter', 'metaplex', 'nft', 'pda'],
    weak: ['blockchain', 'crypto', 'chain', 'deploy', 'on-chain', 'off-chain',
      'gas', 'fee', 'compute', 'account', 'signature'],
  },
  MARKET: {
    strong: ['price', 'market', 'liquidity', 'volume', 'dex', 'swap', 'amm',
      'orderbook', 'candle', 'chart', 'trading', 'buy', 'sell', 'bid', 'ask',
      'mcap', 'market cap', 'tvl', 'yield', 'apy'],
    weak: ['trend', 'pump', 'dump', 'bull', 'bear', 'sentiment', 'whale',
      'profit', 'loss', 'portfolio'],
  },
  SOCIAL: {
    strong: ['twitter', 'tweet', 'discord', 'telegram', 'community', 'followers',
      'engagement', 'post', 'reply', 'retweet', 'thread', 'viral',
      'influencer', 'kol', 'social media', 'audience'],
    weak: ['share', 'message', 'notification', 'feed', 'timeline',
      'mention', 'hashtag', 'content', 'reach'],
  },
  HUMAN: {
    strong: ['user', 'burnout', 'focus', 'energy', 'psychology', 'cognitive',
      'bias', 'fatigue', 'motivation', 'habit', 'emotion', 'stress',
      'productivity', 'flow state', 'attention'],
    weak: ['experience', 'preference', 'feedback', 'session', 'help',
      'frustration', 'confusion', 'learning curve'],
  },
  CYNIC: {
    strong: ['cynic', 'dog', 'consciousness', 'self', 'meta', 'axiom',
      'phi', 'judge', 'verdict', 'howl', 'wag', 'growl', 'bark',
      'kabbalistic', 'sefir', 'topology', 'emergence', 'collective'],
    weak: ['identity', 'personality', 'memory', 'pattern', 'hook',
      'dimension', 'matrix', 'cycle', 'health'],
  },
  COSMOS: {
    strong: ['ecosystem', 'cross-project', 'repository', 'repo', 'monorepo',
      'workspace', 'package', 'dependency graph', 'coherence',
      'distribution', 'integration', 'synchronize'],
    weak: ['project', 'structure', 'architecture', 'system', 'overview',
      'health', 'status', 'monitor'],
  },
};

// =============================================================================
// INTENT PATTERNS — analysis-layer classification
// =============================================================================

const INTENT_PATTERNS = {
  debug: {
    patterns: [/\b(error|bug|fail|broken|crash|fix|doesn'?t work|not working|exception|traceback|stack trace)/i],
    priority: 'high',
  },
  build: {
    patterns: [/\b(build|compile|install|npm|yarn|bun|pnpm|webpack|vite|rollup|esbuild)/i],
    priority: 'medium',
  },
  test: {
    patterns: [/\b(test|spec|coverage|assert|mock|stub|jest|vitest|node:test)/i],
    priority: 'medium',
  },
  deploy: {
    patterns: [/\b(deploy|render|docker|ci|cd|pipeline|release|publish|production|staging)/i],
    priority: 'medium',
  },
  git: {
    patterns: [/\b(commit|push|pull|merge|branch|rebase|cherry-pick|stash|diff|log|pr\b|pull request)/i],
    priority: 'low',
  },
  architecture: {
    patterns: [/\b(architect|design|structure|refactor|reorganize|pattern|abstract|simplif)/i],
    priority: 'high',
  },
  decision: {
    patterns: [/\b(should|decide|choose|which|better|recommend|option|trade-?off|compare)/i],
    priority: 'high',
  },
  security: {
    patterns: [/\b(security|vulnerab|safe|dangerous|attack|exploit|inject|xss|csrf|auth)/i],
    priority: 'critical',
  },
  explain: {
    patterns: [/\b(explain|what is|why does|how does|understand|teach|learn about)/i],
    priority: 'low',
  },
  danger: {
    patterns: [/\b(delete|remove|drop|force|reset --hard|rm -rf|wipe|destroy|purge)/i],
    priority: 'critical',
  },
};

// =============================================================================
// COMPLEXITY INDICATORS
// =============================================================================

const COMPLEXITY_MARKERS = {
  codeBlock: /```[\s\S]*?```/g,
  multiQuestion: /\?.*\?/s,
  longSentence: /[^.!?]{200,}/,
  enumeration: /\d+\.\s|\-\s.*\n\-\s/,
  technicalDepth: /\b(implement|architect|design|optimize|refactor|migrate|integrate)\b/i,
};

// =============================================================================
// SMART SKIP — minimal prompts that need no injection
// =============================================================================

const SKIP_PATTERNS = [
  /^(ok|yes|no|oui|non|sure|thanks|merci|go|done|next|continue|\+)$/i,
  /^(y|n|k|👍|👎|✅|❌)$/,
  /^\/\w+/,  // Slash commands — handled by skills, not injection
];

// =============================================================================
// TOKEN BUDGET
// =============================================================================

const TOKEN_BUDGETS = {
  skip: 0,
  minimal: 150,
  light: 400,
  moderate: 800,
  heavy: 1500,
  full: 2500,
};

// =============================================================================
// EXPORTS
// =============================================================================

/**
 * Classify a prompt: intent, domain relevance, complexity, token budget.
 *
 * @param {string} prompt - User prompt text
 * @param {Object} [options] - Options
 * @param {number} [options.sessionCount=0] - Current session number (experience curve)
 * @param {string} [options.recentDomain] - Most recent active domain (boost)
 * @returns {Object} Classification result
 */
export function classifyPrompt(prompt, options = {}) {
  const { sessionCount = 0, recentDomain = null } = options;

  // Smart skip
  const trimmed = prompt.trim();
  if (shouldSkip(trimmed)) {
    return {
      intent: 'skip',
      intents: [],
      domains: {},
      topDomains: [],
      complexity: 'trivial',
      tokenBudget: TOKEN_BUDGETS.skip,
      skip: true,
    };
  }

  // Intent detection
  const intents = detectIntents(trimmed);
  const primaryIntent = intents.length > 0 ? intents[0].intent : 'general';

  // Domain relevance scoring
  const domains = scoreDomains(trimmed, recentDomain);
  const topDomains = Object.entries(domains)
    .filter(([, score]) => score > 0.1)
    .sort((a, b) => b[1] - a[1])
    .map(([domain, score]) => ({ domain, score }));

  // Complexity assessment
  const complexity = assessComplexity(trimmed, intents);

  // Token budget
  const tokenBudget = calculateTokenBudget(complexity, sessionCount, topDomains.length);

  return {
    intent: primaryIntent,
    intents,
    domains,
    topDomains,
    complexity,
    tokenBudget,
    skip: false,
  };
}

/**
 * Score how relevant each context section is to this prompt.
 * Returns a map of section → relevance (0-1).
 *
 * @param {Object} classification - From classifyPrompt()
 * @param {string[]} availableSections - Section names available for injection
 * @returns {Object} Map of sectionName → relevance score (0-1)
 */
export function scoreContextRelevance(classification, availableSections) {
  if (classification.skip) {
    return Object.fromEntries(availableSections.map(s => [s, 0]));
  }

  const scores = {};

  for (const section of availableSections) {
    let score = 0.1; // Base relevance

    // Boost sections matching top domains
    for (const { domain, score: domainScore } of classification.topDomains) {
      if (sectionMatchesDomain(section, domain)) {
        score = Math.max(score, domainScore);
      }
    }

    // Boost sections matching intent
    if (sectionMatchesIntent(section, classification.intent)) {
      score = Math.min(1, score + 0.2);
    }

    scores[section] = Math.min(PHI_INV, score);
  }

  return scores;
}

/**
 * Filter sections to inject based on relevance and token budget.
 *
 * @param {Object} relevanceScores - From scoreContextRelevance()
 * @param {number} tokenBudget - Available token budget
 * @param {Object} sectionSizes - Map of section → estimated token count
 * @returns {string[]} Ordered list of sections to inject
 */
export function selectSections(relevanceScores, tokenBudget, sectionSizes) {
  // Sort by relevance (descending)
  const candidates = Object.entries(relevanceScores)
    .filter(([, score]) => score > 0.1)
    .sort((a, b) => b[1] - a[1]);

  const selected = [];
  let remaining = tokenBudget;

  for (const [section, score] of candidates) {
    const size = sectionSizes[section] || 100;
    if (size <= remaining) {
      selected.push(section);
      remaining -= size;
    }
  }

  return selected;
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

function shouldSkip(prompt) {
  return SKIP_PATTERNS.some(p => p.test(prompt));
}

function detectIntents(prompt) {
  const lower = prompt.toLowerCase();
  const found = [];

  for (const [intent, config] of Object.entries(INTENT_PATTERNS)) {
    for (const pattern of config.patterns) {
      if (pattern.test(lower)) {
        found.push({ intent, priority: config.priority });
        break;
      }
    }
  }

  // Sort by priority: critical > high > medium > low
  const priorityOrder = { critical: 0, high: 1, medium: 2, low: 3 };
  found.sort((a, b) => (priorityOrder[a.priority] || 4) - (priorityOrder[b.priority] || 4));

  return found;
}

function scoreDomains(prompt, recentDomain) {
  const lower = prompt.toLowerCase();
  const scores = {};

  for (const [domain, keywords] of Object.entries(DOMAIN_KEYWORDS)) {
    let score = 0;

    // Strong matches
    for (const kw of keywords.strong) {
      if (lower.includes(kw.toLowerCase())) {
        score += 0.3;
      }
    }

    // Weak matches
    for (const kw of keywords.weak) {
      if (lower.includes(kw.toLowerCase())) {
        score += 0.1;
      }
    }

    // Recent domain boost (momentum)
    if (domain === recentDomain) {
      score += 0.15;
    }

    scores[domain] = Math.min(1, score);
  }

  return scores;
}

function assessComplexity(prompt, intents) {
  let score = 0;

  // Length
  if (prompt.length > 500) score += 0.2;
  if (prompt.length > 1000) score += 0.2;
  if (prompt.length > 2000) score += 0.2;

  // Code blocks
  if (COMPLEXITY_MARKERS.codeBlock.test(prompt)) score += 0.3;
  // Reset lastIndex after test
  COMPLEXITY_MARKERS.codeBlock.lastIndex = 0;

  // Multiple questions
  if (COMPLEXITY_MARKERS.multiQuestion.test(prompt)) score += 0.15;

  // Enumeration (numbered/bulleted list)
  if (COMPLEXITY_MARKERS.enumeration.test(prompt)) score += 0.1;

  // Technical depth keywords
  if (COMPLEXITY_MARKERS.technicalDepth.test(prompt)) score += 0.15;

  // High-priority intents add complexity
  const hasCritical = intents.some(i => i.priority === 'critical');
  const hasHigh = intents.some(i => i.priority === 'high');
  if (hasCritical) score += 0.2;
  else if (hasHigh) score += 0.1;

  // Classify
  if (score < 0.1) return 'trivial';
  if (score < 0.3) return 'simple';
  if (score < 0.5) return 'moderate';
  if (score < 0.7) return 'complex';
  return 'deep';
}

function calculateTokenBudget(complexity, sessionCount, domainCount) {
  const base = TOKEN_BUDGETS[complexity] || TOKEN_BUDGETS.moderate;

  // Experience curve: reduce budget as sessions increase
  let experienceMultiplier = 1.0;
  if (sessionCount > 50) experienceMultiplier = PHI_INV_2; // 38.2% of original
  else if (sessionCount > 10) experienceMultiplier = PHI_INV; // 61.8% of original

  // More domains = slightly more budget (cross-domain prompts need more context)
  const domainMultiplier = 1 + Math.min(0.3, domainCount * 0.1);

  return Math.round(base * experienceMultiplier * domainMultiplier);
}

function sectionMatchesDomain(section, domain) {
  const sectionLower = section.toLowerCase();
  const domainLower = domain.toLowerCase();

  // Direct match
  if (sectionLower.includes(domainLower)) return true;

  // Alias matches
  const aliases = {
    CODE: ['code', 'programming', 'development', 'implementation'],
    SOLANA: ['solana', 'blockchain', 'chain', 'token', 'web3'],
    MARKET: ['market', 'price', 'trading', 'liquidity', 'dex'],
    SOCIAL: ['social', 'twitter', 'discord', 'community'],
    HUMAN: ['human', 'user', 'psychology', 'burnout', 'energy'],
    CYNIC: ['cynic', 'self', 'consciousness', 'identity', 'dog'],
    COSMOS: ['cosmos', 'ecosystem', 'cross-project', 'repository'],
  };

  return (aliases[domain] || []).some(a => sectionLower.includes(a));
}

function sectionMatchesIntent(section, intent) {
  const sectionLower = section.toLowerCase();

  const intentSections = {
    debug: ['error', 'debug', 'trace', 'log'],
    build: ['build', 'compile', 'install', 'dependency'],
    test: ['test', 'coverage', 'spec'],
    deploy: ['deploy', 'render', 'docker', 'ci'],
    git: ['git', 'commit', 'branch', 'pr'],
    architecture: ['architecture', 'design', 'structure', 'pattern'],
    decision: ['decision', 'trade-off', 'option', 'compare'],
    security: ['security', 'guard', 'safe', 'vulnerability'],
    explain: ['doc', 'explain', 'guide', 'overview'],
    danger: ['guard', 'warning', 'danger', 'safe'],
  };

  return (intentSections[intent] || []).some(s => sectionLower.includes(s));
}

export { DOMAIN_KEYWORDS, INTENT_PATTERNS, TOKEN_BUDGETS, SKIP_PATTERNS };
