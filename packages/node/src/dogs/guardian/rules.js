/**
 * Guardian L1 Rules - Local checks without LLM
 *
 * "Je mords avant que tu ne regrettes" - Guardian
 *
 * These rules execute in <10ms for instant protection.
 * L1 catches ~80% of dangers, L2 (LLM) handles the rest.
 *
 * @module @cynic/node/dogs/guardian/rules
 */

'use strict';

import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Load patterns from JSON
 */
let patterns = null;
function loadPatterns() {
  if (!patterns) {
    try {
      const patternsPath = join(__dirname, 'patterns.json');
      patterns = JSON.parse(readFileSync(patternsPath, 'utf8'));
    } catch (e) {
      patterns = { patterns: {}, sensitivePaths: { paths: [] }, systemPaths: {} };
    }
  }
  return patterns;
}

/**
 * Check if a command matches critical patterns
 * @param {string} command - The command to check
 * @returns {{ blocked: boolean, level: string, reason: string } | null}
 */
export function checkCommand(command) {
  const p = loadPatterns();

  // Check critical patterns first (always block)
  for (const cmd of p.patterns.critical?.commands || []) {
    const match = cmd.regex
      ? new RegExp(cmd.pattern, 'i').test(command)
      : command.toLowerCase().includes(cmd.pattern.toLowerCase());

    if (match) {
      return { blocked: true, level: 'critical', reason: cmd.reason };
    }
  }

  // Check high patterns (block with override)
  for (const cmd of p.patterns.high?.commands || []) {
    const match = cmd.regex
      ? new RegExp(cmd.pattern, 'i').test(command)
      : command.toLowerCase().includes(cmd.pattern.toLowerCase());

    if (match) {
      return { blocked: true, level: 'high', reason: cmd.reason, allowOverride: true };
    }
  }

  // Check medium patterns (warn)
  for (const cmd of p.patterns.medium?.commands || []) {
    const match = cmd.regex
      ? new RegExp(cmd.pattern, 'i').test(command)
      : command.toLowerCase().includes(cmd.pattern.toLowerCase());

    if (match) {
      return { blocked: false, level: 'medium', reason: cmd.reason, warn: true };
    }
  }

  // Check learned patterns
  for (const learned of p.learnedPatterns || []) {
    const match = learned.regex
      ? new RegExp(learned.pattern, 'i').test(command)
      : command.toLowerCase().includes(learned.pattern.toLowerCase());

    if (match) {
      return {
        blocked: learned.level === 'critical' || learned.level === 'high',
        level: learned.level,
        reason: learned.reason,
        learned: true,
      };
    }
  }

  return null; // No match - pass to L2 if uncertain
}

/**
 * Check if a path is sensitive
 * @param {string} filePath - The path to check
 * @returns {{ sensitive: boolean, reason: string } | null}
 */
export function checkPath(filePath) {
  const p = loadPatterns();
  const normalizedPath = filePath.replace(/\\/g, '/').toLowerCase();

  // Check sensitive paths
  for (const sensitivePath of p.sensitivePaths?.paths || []) {
    if (normalizedPath.includes(sensitivePath.toLowerCase())) {
      return { sensitive: true, reason: `Path contains sensitive pattern: ${sensitivePath}` };
    }
  }

  // Check system paths (Unix)
  for (const sysPath of p.systemPaths?.unix || []) {
    if (normalizedPath.startsWith(sysPath.toLowerCase())) {
      return { sensitive: true, reason: `System path: ${sysPath}`, systemPath: true };
    }
  }

  // Check system paths (Windows)
  for (const sysPath of p.systemPaths?.windows || []) {
    const normalizedSys = sysPath.replace(/\\/g, '/').toLowerCase();
    if (normalizedPath.startsWith(normalizedSys)) {
      return { sensitive: true, reason: `System path: ${sysPath}`, systemPath: true };
    }
  }

  return null;
}

/**
 * Full L1 check for Guardian
 * @param {object} context - Tool use context
 * @returns {{ decision: 'allow' | 'block' | 'warn' | 'escalate', details: object }}
 */
export function l1Check(context) {
  const { tool, params } = context;

  // For Bash commands
  if (tool === 'Bash' && params?.command) {
    const commandCheck = checkCommand(params.command);
    if (commandCheck) {
      return {
        decision: commandCheck.blocked ? 'block' : 'warn',
        details: commandCheck,
      };
    }
  }

  // For Write/Edit operations
  if ((tool === 'Write' || tool === 'Edit') && params?.file_path) {
    const pathCheck = checkPath(params.file_path);
    if (pathCheck) {
      return {
        decision: pathCheck.systemPath ? 'block' : 'warn',
        details: pathCheck,
      };
    }
  }

  // No L1 match - might need L2 escalation
  return { decision: 'allow', details: null };
}

/**
 * Add a learned pattern (from L3 feedback)
 * @param {object} pattern - Pattern to add
 */
export function addLearnedPattern(pattern) {
  const p = loadPatterns();
  if (!p.learnedPatterns) p.learnedPatterns = [];

  // Check for duplicates
  const exists = p.learnedPatterns.some(lp => lp.pattern === pattern.pattern);
  if (!exists) {
    p.learnedPatterns.push({
      ...pattern,
      learnedAt: new Date().toISOString(),
    });

    // Persist to file
    try {
      const patternsPath = join(__dirname, 'patterns.json');
      writeFileSync(patternsPath, JSON.stringify(p, null, 2));
    } catch {
      // Could not persist - will be lost on restart
    }
  }
}

export default {
  checkCommand,
  checkPath,
  l1Check,
  addLearnedPattern,
};
