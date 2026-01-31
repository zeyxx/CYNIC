/**
 * SkillActivator - Auto-activation of skills via rules.json
 *
 * Loads configurable rules from skill-rules.json and activates skills
 * based on keyword matching, regex patterns, and context.
 *
 * P3.1: External, user-configurable skill activation rules.
 *
 * "φ knows when to invoke" - κυνικός
 *
 * @module @cynic/node/orchestration/skill-activator
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { promises as fs } from 'fs';
import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const log = createLogger('SkillActivator');

/**
 * Default configuration
 */
export const ACTIVATOR_CONFIG = {
  enabled: true,
  maxSuggestions: 3,
  minConfidence: PHI_INV_2,      // 38.2% minimum confidence
  showSuggestions: true,
  autoInvoke: false,
  priorityOrder: ['high', 'medium', 'low'],
  rulesPath: null,               // Auto-detect from project
};

/**
 * Match result from rule evaluation
 */
export class SkillMatch {
  /**
   * @param {Object} options
   * @param {string} options.skill - Skill name (e.g., '/judge')
   * @param {string} options.ruleId - Rule ID that matched
   * @param {string} options.description - Skill description
   * @param {string} options.priority - Priority level
   * @param {number} options.confidence - Match confidence (0-1)
   * @param {string[]} options.matchedKeywords - Keywords that matched
   * @param {string[]} options.matchedPatterns - Patterns that matched
   */
  constructor(options) {
    this.skill = options.skill;
    this.ruleId = options.ruleId;
    this.description = options.description;
    this.priority = options.priority;
    this.confidence = options.confidence;
    this.matchedKeywords = options.matchedKeywords || [];
    this.matchedPatterns = options.matchedPatterns || [];
    this.timestamp = Date.now();
  }

  /**
   * Get match score for sorting
   */
  get score() {
    const priorityScore = { high: 3, medium: 2, low: 1 }[this.priority] || 0;
    return this.confidence * priorityScore;
  }
}

/**
 * SkillActivator - Loads rules and activates skills automatically
 */
export class SkillActivator {
  /**
   * @param {Object} options
   * @param {string} [options.rulesPath] - Path to skill-rules.json
   * @param {Object} [options.config] - Override config
   * @param {Function} [options.onActivate] - Callback when skill is activated
   */
  constructor(options = {}) {
    this.config = { ...ACTIVATOR_CONFIG, ...options.config };
    this.rulesPath = options.rulesPath || this.config.rulesPath;
    this.onActivate = options.onActivate || null;

    // Loaded rules
    this.rules = [];
    this.settings = {};
    this.version = null;

    // Compiled regex patterns (for performance)
    this._compiledPatterns = new Map();

    // Statistics
    this.stats = {
      rulesLoaded: 0,
      evaluations: 0,
      matches: 0,
      activations: 0,
      bySkill: {},
    };

    // Auto-load rules if path provided
    if (this.rulesPath) {
      this.loadRules(this.rulesPath).catch(err => {
        log.warn('Failed to auto-load rules', { error: err.message });
      });
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rule Loading
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Load rules from JSON file
   *
   * @param {string} filePath - Path to skill-rules.json
   * @returns {Promise<boolean>} Success
   */
  async loadRules(filePath) {
    try {
      const resolvedPath = resolve(filePath);

      if (!existsSync(resolvedPath)) {
        log.warn('Rules file not found', { path: resolvedPath });
        return false;
      }

      const content = await fs.readFile(resolvedPath, 'utf-8');
      const data = JSON.parse(content);

      // Validate structure
      if (!data.rules || !Array.isArray(data.rules)) {
        throw new Error('Invalid rules file: missing "rules" array');
      }

      // Store rules and settings
      this.rules = data.rules;
      this.settings = data.settings || {};
      this.version = data.version || '1.0.0';

      // Merge settings into config
      Object.assign(this.config, this.settings);

      // Compile regex patterns for performance
      this._compilePatterns();

      this.stats.rulesLoaded = this.rules.length;
      log.info('Rules loaded', { count: this.rules.length, version: this.version });

      return true;
    } catch (err) {
      log.error('Failed to load rules', { path: filePath, error: err.message });
      return false;
    }
  }

  /**
   * Compile regex patterns from string definitions
   * @private
   */
  _compilePatterns() {
    this._compiledPatterns.clear();

    for (const rule of this.rules) {
      if (!rule.triggers?.patterns) continue;

      const compiled = [];
      for (const pattern of rule.triggers.patterns) {
        try {
          compiled.push(new RegExp(pattern, 'i'));
        } catch (err) {
          log.warn('Invalid regex pattern', { ruleId: rule.id, pattern, error: err.message });
        }
      }

      this._compiledPatterns.set(rule.id, compiled);
    }
  }

  /**
   * Add a rule dynamically
   *
   * @param {Object} rule - Rule definition
   */
  addRule(rule) {
    if (!rule.id || !rule.skill) {
      throw new Error('Rule must have id and skill');
    }

    // Check for duplicate
    const existing = this.rules.findIndex(r => r.id === rule.id);
    if (existing >= 0) {
      this.rules[existing] = rule;
    } else {
      this.rules.push(rule);
    }

    // Recompile patterns
    this._compilePatterns();
    this.stats.rulesLoaded = this.rules.length;
  }

  /**
   * Remove a rule
   *
   * @param {string} ruleId - Rule ID to remove
   * @returns {boolean} Whether rule was removed
   */
  removeRule(ruleId) {
    const index = this.rules.findIndex(r => r.id === ruleId);
    if (index >= 0) {
      this.rules.splice(index, 1);
      this._compiledPatterns.delete(ruleId);
      this.stats.rulesLoaded = this.rules.length;
      return true;
    }
    return false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rule Evaluation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Evaluate prompt against all rules and return matches
   *
   * @param {string} prompt - User prompt to evaluate
   * @param {Object} [context] - Additional context
   * @returns {SkillMatch[]} Sorted list of matching skills
   */
  evaluate(prompt, context = {}) {
    if (!this.config.enabled || this.rules.length === 0) {
      return [];
    }

    this.stats.evaluations++;
    const promptLower = prompt.toLowerCase();
    const matches = [];

    for (const rule of this.rules) {
      const match = this._evaluateRule(rule, prompt, promptLower, context);
      if (match && match.confidence >= this.config.minConfidence) {
        matches.push(match);
        this.stats.matches++;
        this.stats.bySkill[rule.skill] = (this.stats.bySkill[rule.skill] || 0) + 1;
      }
    }

    // Sort by score (confidence * priority)
    matches.sort((a, b) => b.score - a.score);

    // Limit results
    return matches.slice(0, this.config.maxSuggestions);
  }

  /**
   * Evaluate a single rule against prompt
   * @private
   */
  _evaluateRule(rule, prompt, promptLower, context) {
    const triggers = rule.triggers;
    if (!triggers) return null;

    let confidence = 0;
    const matchedKeywords = [];
    const matchedPatterns = [];

    // Check keywords
    if (triggers.keywords) {
      for (const keyword of triggers.keywords) {
        if (promptLower.includes(keyword.toLowerCase())) {
          matchedKeywords.push(keyword);
          confidence += 0.3; // Each keyword adds 30%
        }
      }
    }

    // Check patterns
    const patterns = this._compiledPatterns.get(rule.id) || [];
    for (const pattern of patterns) {
      if (pattern.test(prompt)) {
        matchedPatterns.push(pattern.source);
        confidence += 0.5; // Each pattern adds 50%
      }
    }

    // Check context conditions
    if (triggers.context && context) {
      if (triggers.context.itemTypes && context.itemType) {
        if (triggers.context.itemTypes.includes(context.itemType)) {
          confidence += 0.2;
        }
      }
    }

    // Check rule-specific conditions
    if (rule.conditions) {
      if (rule.conditions.minConfidence && confidence < rule.conditions.minConfidence) {
        return null;
      }
    }

    // Cap confidence at φ⁻¹ (never exceed 61.8%)
    confidence = Math.min(confidence, PHI_INV);

    // No match
    if (matchedKeywords.length === 0 && matchedPatterns.length === 0) {
      return null;
    }

    return new SkillMatch({
      skill: rule.skill,
      ruleId: rule.id,
      description: rule.description,
      priority: rule.priority || 'medium',
      confidence,
      matchedKeywords,
      matchedPatterns,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Skill Activation
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Activate skills based on prompt
   *
   * @param {string} prompt - User prompt
   * @param {Object} [context] - Additional context
   * @returns {Object} Activation result
   */
  async activate(prompt, context = {}) {
    const matches = this.evaluate(prompt, context);

    if (matches.length === 0) {
      return {
        activated: false,
        suggestions: [],
        reason: 'No matching rules',
      };
    }

    // If auto-invoke is enabled, activate the top match
    if (this.config.autoInvoke && matches[0].confidence >= PHI_INV) {
      const topMatch = matches[0];
      this.stats.activations++;

      if (this.onActivate) {
        await this.onActivate(topMatch);
      }

      return {
        activated: true,
        skill: topMatch.skill,
        confidence: topMatch.confidence,
        suggestions: matches,
      };
    }

    // Otherwise, just return suggestions
    return {
      activated: false,
      suggestions: matches,
      reason: this.config.autoInvoke
        ? `Top match confidence ${(matches[0].confidence * 100).toFixed(1)}% < 61.8%`
        : 'Auto-invoke disabled',
    };
  }

  /**
   * Format suggestions for display
   *
   * @param {SkillMatch[]} matches - Matches to format
   * @returns {string} Formatted string
   */
  formatSuggestions(matches) {
    if (matches.length === 0) return '';

    const lines = ['*sniff* Skill suggestions:'];

    for (const match of matches) {
      const conf = (match.confidence * 100).toFixed(0);
      const icon = match.priority === 'high' ? '!' : match.priority === 'medium' ? '~' : '.';
      lines.push(`   ${icon} ${match.skill} (${conf}%) - ${match.description}`);
    }

    return lines.join('\n');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Utilities
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get all registered skills
   *
   * @returns {string[]} Skill names
   */
  getSkills() {
    return [...new Set(this.rules.map(r => r.skill))];
  }

  /**
   * Get rules for a specific skill
   *
   * @param {string} skill - Skill name
   * @returns {Object[]} Rules for this skill
   */
  getRulesForSkill(skill) {
    return this.rules.filter(r => r.skill === skill);
  }

  /**
   * Get statistics
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      enabled: this.config.enabled,
      version: this.version,
    };
  }

  /**
   * Export rules for persistence
   *
   * @returns {Object} Exportable rules data
   */
  exportRules() {
    return {
      version: this.version,
      rules: this.rules,
      settings: this.settings,
    };
  }
}

/**
 * Create SkillActivator instance
 *
 * @param {Object} options - Options
 * @returns {SkillActivator}
 */
export function createSkillActivator(options = {}) {
  return new SkillActivator(options);
}

// Singleton for global access
let _activatorInstance = null;

/**
 * Get or create global SkillActivator
 *
 * @param {Object} [options] - Options for creation
 * @returns {SkillActivator}
 */
export function getSkillActivator(options = {}) {
  if (!_activatorInstance) {
    _activatorInstance = createSkillActivator(options);
  }
  return _activatorInstance;
}

/**
 * Find skill-rules.json in project
 *
 * @param {string} [startDir] - Directory to start search
 * @returns {string|null} Path to rules file or null
 */
export function findRulesFile(startDir = process.cwd()) {
  const candidates = [
    resolve(startDir, '.claude', 'skill-rules.json'),
    resolve(startDir, 'skill-rules.json'),
    resolve(startDir, '.cynic', 'skill-rules.json'),
  ];

  for (const path of candidates) {
    if (existsSync(path)) {
      return path;
    }
  }

  return null;
}

export default SkillActivator;
