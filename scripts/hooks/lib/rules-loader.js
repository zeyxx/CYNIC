/**
 * Rules Loader - Load skill auto-activation rules from JSON
 *
 * S1: External configuration for skill triggers via rules.json
 *
 * "Le chien apprend de nouvelles règles" - CYNIC learns from config
 *
 * @module scripts/hooks/lib/rules-loader
 */

'use strict';

import fs from 'fs';
import path from 'path';

// Default rules file path
const DEFAULT_RULES_PATH = path.resolve(
  process.cwd(),
  '.claude',
  'skill-rules.json'
);

// Fallback rules path (relative to this file)
const FALLBACK_RULES_PATH = path.resolve(
  import.meta.dirname,
  '../../..',
  '.claude',
  'skill-rules.json'
);

/**
 * Cached rules and compiled patterns
 */
let _cachedRules = null;
let _compiledTriggers = null;
let _cacheTime = 0;
const CACHE_TTL = 60000; // 1 minute cache

/**
 * Load rules from JSON file
 *
 * @param {string} [rulesPath] - Optional custom path to rules file
 * @returns {Object|null} Parsed rules object or null if not found
 */
export function loadRulesFile(rulesPath = null) {
  const paths = [
    rulesPath,
    DEFAULT_RULES_PATH,
    FALLBACK_RULES_PATH,
  ].filter(Boolean);

  for (const p of paths) {
    try {
      if (fs.existsSync(p)) {
        const content = fs.readFileSync(p, 'utf-8');
        return JSON.parse(content);
      }
    } catch (e) {
      // Try next path
    }
  }

  return null;
}

/**
 * Compile pattern strings to RegExp objects
 *
 * @param {string[]} patterns - Array of pattern strings
 * @returns {RegExp[]} Array of compiled RegExp objects
 */
function compilePatterns(patterns) {
  if (!patterns || !Array.isArray(patterns)) return [];

  return patterns.map(p => {
    try {
      return new RegExp(p, 'i');
    } catch (e) {
      // Invalid regex - return a never-matching pattern
      return /(?!)/;
    }
  });
}

/**
 * Convert rules JSON to SKILL_TRIGGERS format for backward compatibility
 *
 * @param {Object} rules - Rules object from JSON
 * @returns {Object} SKILL_TRIGGERS-compatible object
 */
export function convertRulesToTriggers(rules) {
  if (!rules?.rules || !Array.isArray(rules.rules)) {
    return {};
  }

  const triggers = {};

  for (const rule of rules.rules) {
    if (!rule.skill || !rule.triggers) continue;

    triggers[rule.skill] = {
      patterns: compilePatterns(rule.triggers.patterns),
      keywords: rule.triggers.keywords || [],
      description: rule.description || '',
      priority: rule.priority || 'medium',
      context: rule.triggers.context || {},
      conditions: rule.conditions || {},
      id: rule.id,
    };
  }

  return triggers;
}

/**
 * Get skill triggers (with caching)
 *
 * Loads from skill-rules.json and converts to SKILL_TRIGGERS format.
 * Falls back to default triggers if file not found.
 *
 * @param {Object} [options] - Options
 * @param {boolean} [options.forceReload] - Force reload from disk
 * @param {string} [options.rulesPath] - Custom path to rules file
 * @returns {Object} SKILL_TRIGGERS object
 */
export function getSkillTriggers(options = {}) {
  const now = Date.now();

  // Check cache
  if (!options.forceReload && _compiledTriggers && (now - _cacheTime) < CACHE_TTL) {
    return _compiledTriggers;
  }

  // Load and convert
  const rules = loadRulesFile(options.rulesPath);

  if (rules) {
    _cachedRules = rules;
    _compiledTriggers = convertRulesToTriggers(rules);
    _cacheTime = now;
    return _compiledTriggers;
  }

  // Return default triggers if no rules file found
  return getDefaultTriggers();
}

/**
 * Get rules settings
 *
 * @returns {Object} Settings from rules file or defaults
 */
export function getRulesSettings() {
  if (_cachedRules?.settings) {
    return _cachedRules.settings;
  }

  // Load fresh if not cached
  const rules = loadRulesFile();
  if (rules?.settings) {
    _cachedRules = rules;
    return rules.settings;
  }

  // Default settings
  return {
    enabled: true,
    maxSuggestions: 3,
    minConfidence: 0.382,
    showSuggestions: true,
    autoInvoke: false,
    priorityOrder: ['high', 'medium', 'low'],
  };
}

/**
 * Detect skill triggers in prompt using loaded rules
 *
 * @param {string} prompt - User prompt to analyze
 * @param {Object} [options] - Detection options
 * @returns {Array<{skill: string, description: string, priority: string, matchedPattern?: string}>}
 */
export function detectSkillTriggersFromRules(prompt, options = {}) {
  const triggers = getSkillTriggers(options);
  const settings = getRulesSettings();
  const matches = [];

  if (!settings.enabled) {
    return matches;
  }

  const promptLower = prompt.toLowerCase();

  for (const [skill, config] of Object.entries(triggers)) {
    let matched = false;
    let matchedPattern = null;

    // Check patterns first (more specific)
    if (config.patterns && config.patterns.length > 0) {
      for (const pattern of config.patterns) {
        if (pattern.test(prompt)) {
          matched = true;
          matchedPattern = pattern.toString();
          break;
        }
      }
    }

    // If no pattern match, check keywords
    if (!matched && config.keywords && config.keywords.length > 0) {
      for (const keyword of config.keywords) {
        if (promptLower.includes(keyword.toLowerCase())) {
          matched = true;
          matchedPattern = `keyword:${keyword}`;
          break;
        }
      }
    }

    if (matched) {
      matches.push({
        skill,
        description: config.description,
        priority: config.priority,
        matchedPattern,
        id: config.id,
      });
    }
  }

  // Sort by priority
  const priorityOrder = settings.priorityOrder || ['high', 'medium', 'low'];
  matches.sort((a, b) => {
    const aIdx = priorityOrder.indexOf(a.priority);
    const bIdx = priorityOrder.indexOf(b.priority);
    return aIdx - bIdx;
  });

  // Limit suggestions
  const maxSuggestions = options.maxSuggestions ?? settings.maxSuggestions ?? 3;
  return matches.slice(0, maxSuggestions);
}

/**
 * Clear the rules cache
 */
export function clearRulesCache() {
  _cachedRules = null;
  _compiledTriggers = null;
  _cacheTime = 0;
}

/**
 * Default triggers (fallback when no rules file exists)
 */
function getDefaultTriggers() {
  return {
    '/judge': {
      patterns: [
        /\b(judge|evaluate|assess|rate|score)\b.*\b(this|code|token|decision)/i,
        /\bwhat.*think.*about\b/i,
        /\bq-?score\b/i,
        /\bgive.*verdict\b/i,
      ],
      description: 'Evaluate using 25-dimension judgment system',
      priority: 'high',
    },
    '/search': {
      patterns: [
        /\b(search|find|look ?up|query|recall)\b.*\b(memory|past|previous|history)/i,
        /\bremember.*when\b/i,
        /\bhave we.*before\b/i,
      ],
      description: 'Search collective memory',
      priority: 'medium',
    },
    '/patterns': {
      patterns: [
        /\b(show|view|list)\b.*\b(patterns?|anomal)/i,
        /\bwhat.*patterns?\b/i,
      ],
      description: 'View detected patterns and anomalies',
      priority: 'medium',
    },
    '/health': {
      patterns: [
        /\b(system|cynic)\b.*\b(health|status|running|ok)\b/i,
        /\bdiagnostic/i,
      ],
      description: 'Check CYNIC system health',
      priority: 'low',
    },
  };
}

export default {
  loadRulesFile,
  convertRulesToTriggers,
  getSkillTriggers,
  getRulesSettings,
  detectSkillTriggersFromRules,
  clearRulesCache,
};
