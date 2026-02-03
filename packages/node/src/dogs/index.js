/**
 * Dog Heuristics Loader
 *
 * "Le chien charge ses instincts" - CYNIC
 *
 * This module loads L1 heuristics for all Dogs at boot time.
 * Each Dog has:
 * - patterns.json: Static patterns for instant matching
 * - rules.js: Local checks without LLM
 *
 * @module @cynic/node/dogs
 */

'use strict';

import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * List of all Dogs with their heuristics status
 */
export const DOGS = [
  { id: 'guardian', name: 'Guardian', sefira: 'Gevurah', hasL1: true },
  { id: 'scout', name: 'Scout', sefira: 'Netzach', hasL1: true },
  { id: 'analyst', name: 'Analyst', sefira: 'Binah', hasL1: true },
  { id: 'janitor', name: 'Janitor', sefira: 'Yesod', hasL1: true },
  { id: 'architect', name: 'Architect', sefira: 'Chesed', hasL1: true },
  { id: 'scholar', name: 'Scholar', sefira: 'Daat', hasL1: true },
  { id: 'sage', name: 'Sage', sefira: 'Chochmah', hasL1: false },
  { id: 'oracle', name: 'Oracle', sefira: 'Tiferet', hasL1: false },
  { id: 'deployer', name: 'Deployer', sefira: 'Hod', hasL1: false },
  { id: 'cartographer', name: 'Cartographer', sefira: 'Malkhut', hasL1: false },
  { id: 'cynic', name: 'CYNIC', sefira: 'Keter', hasL1: false },
];

/**
 * Cache for loaded patterns
 */
const patternsCache = new Map();

/**
 * Load patterns for a specific Dog
 * @param {string} dogId - Dog identifier
 * @returns {object | null} Patterns object or null if not found
 */
export function loadDogPatterns(dogId) {
  if (patternsCache.has(dogId)) {
    return patternsCache.get(dogId);
  }

  const patternsPath = join(__dirname, dogId, 'patterns.json');

  if (!existsSync(patternsPath)) {
    return null;
  }

  try {
    const patterns = JSON.parse(readFileSync(patternsPath, 'utf8'));
    patternsCache.set(dogId, patterns);
    return patterns;
  } catch (e) {
    console.error(`Failed to load patterns for ${dogId}:`, e.message);
    return null;
  }
}

/**
 * Load rules module for a specific Dog
 * @param {string} dogId - Dog identifier
 * @returns {Promise<object | null>} Rules module or null if not found
 */
export async function loadDogRules(dogId) {
  const rulesPath = join(__dirname, dogId, 'rules.js');

  if (!existsSync(rulesPath)) {
    return null;
  }

  try {
    const rules = await import(`file://${rulesPath.replace(/\\/g, '/')}`);
    return rules.default || rules;
  } catch (e) {
    console.error(`Failed to load rules for ${dogId}:`, e.message);
    return null;
  }
}

/**
 * Load all Dog heuristics at boot time
 * @returns {Promise<object>} Object with loaded heuristics per Dog
 */
export async function loadAllDogHeuristics() {
  const loaded = {
    patterns: {},
    rules: {},
    stats: {
      totalDogs: DOGS.length,
      l1Loaded: 0,
      l1Failed: 0,
      rulesLoaded: 0,
    },
  };

  for (const dog of DOGS) {
    if (!dog.hasL1) continue;

    // Load patterns
    const patterns = loadDogPatterns(dog.id);
    if (patterns) {
      loaded.patterns[dog.id] = patterns;
      loaded.stats.l1Loaded++;
    } else {
      loaded.stats.l1Failed++;
    }

    // Load rules
    try {
      const rules = await loadDogRules(dog.id);
      if (rules) {
        loaded.rules[dog.id] = rules;
        loaded.stats.rulesLoaded++;
      }
    } catch (e) {
      // Rules are optional
    }
  }

  return loaded;
}

/**
 * Get L1 check function for a Dog
 * @param {string} dogId - Dog identifier
 * @returns {Function | null} L1 check function or null
 */
export async function getL1Check(dogId) {
  const rules = await loadDogRules(dogId);
  return rules?.l1Check || null;
}

/**
 * Add learned pattern to a Dog (L3 feedback)
 * @param {string} dogId - Dog identifier
 * @param {object} pattern - Pattern to add
 */
export function addLearnedPattern(dogId, pattern) {
  const patterns = loadDogPatterns(dogId);
  if (!patterns) return false;

  if (!patterns.learnedPatterns) {
    patterns.learnedPatterns = [];
  }

  patterns.learnedPatterns.push({
    ...pattern,
    learnedAt: new Date().toISOString(),
  });

  // Update cache
  patternsCache.set(dogId, patterns);

  // Persist to file (async, don't wait)
  const patternsPath = join(__dirname, dogId, 'patterns.json');
  import('fs/promises').then(fs => {
    fs.writeFile(patternsPath, JSON.stringify(patterns, null, 2)).catch(() => {});
  });

  return true;
}

// Re-export learning service
export { DogLearningService, createDogLearningService } from './learning-service.js';

export default {
  DOGS,
  loadDogPatterns,
  loadDogRules,
  loadAllDogHeuristics,
  getL1Check,
  addLearnedPattern,
};
