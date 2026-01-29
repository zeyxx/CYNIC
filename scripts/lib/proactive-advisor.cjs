/**
 * CYNIC Proactive Advisor
 *
 * Generates intelligent suggestions by combining harmony analysis
 * and context weaving. Surfaces insights at the right moment.
 *
 * "See the gap before it becomes a chasm" - κυνικός
 *
 * @module @cynic/scripts/proactive-advisor
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// φ Constants
const PHI = 1.618033988749895;
const PHI_INV = 1 / PHI;

// Load dependencies
let harmonyAnalyzer = null;
let contextWeaver = null;
let consciousness = null;

try {
  harmonyAnalyzer = require('./harmony-analyzer.cjs');
} catch { /* not available */ }

try {
  contextWeaver = require('./context-weaver.cjs');
} catch { /* not available */ }

try {
  consciousness = require('./consciousness.cjs');
  consciousness.init();
} catch { /* not available */ }

// Storage
const ADVISOR_DIR = path.join(os.homedir(), '.cynic/advisor');
const SUGGESTIONS_FILE = path.join(ADVISOR_DIR, 'suggestions.jsonl');
const STATE_FILE = path.join(ADVISOR_DIR, 'state.json');

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION TYPES
// ═══════════════════════════════════════════════════════════════════════════

const SUGGESTION_TYPES = {
  HARMONY: {
    name: 'harmony',
    emoji: '🎵',
    description: 'Philosophy/code alignment',
    priority: 'high',
  },
  UPDATE: {
    name: 'update',
    emoji: '📦',
    description: 'Dependency or tool update',
    priority: 'medium',
  },
  SYNC: {
    name: 'sync',
    emoji: '🔄',
    description: 'Ecosystem synchronization',
    priority: 'medium',
  },
  WORKFLOW: {
    name: 'workflow',
    emoji: '⚡',
    description: 'Development workflow improvement',
    priority: 'low',
  },
  LEARNING: {
    name: 'learning',
    emoji: '📚',
    description: 'Learning opportunity',
    priority: 'low',
  },
  CAPABILITY: {
    name: 'capability',
    emoji: '🔧',
    description: 'Underutilized capability',
    priority: 'info',
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// INITIALIZATION
// ═══════════════════════════════════════════════════════════════════════════

function init() {
  if (!fs.existsSync(ADVISOR_DIR)) {
    fs.mkdirSync(ADVISOR_DIR, { recursive: true });
  }
}

function loadState() {
  if (!fs.existsSync(STATE_FILE)) {
    return {
      lastAnalysis: 0,
      suggestionsShown: [],
      dismissedSuggestions: [],
    };
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return { lastAnalysis: 0, suggestionsShown: [], dismissedSuggestions: [] };
  }
}

function saveState(state) {
  fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
}

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION GENERATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generate all suggestions
 */
function generateSuggestions(projectPath = '.', workspacePath = '/workspaces') {
  init();
  const suggestions = [];
  const state = loadState();

  // Generate from harmony analysis
  if (harmonyAnalyzer) {
    suggestions.push(...generateHarmonySuggestions(projectPath));
  }

  // Generate from context weaving
  if (contextWeaver) {
    suggestions.push(...generateContextSuggestions(projectPath, workspacePath));
  }

  // Generate from consciousness
  if (consciousness) {
    suggestions.push(...generateConsciousnessSuggestions());
  }

  // Generate workflow suggestions
  suggestions.push(...generateWorkflowSuggestions(projectPath));

  // Filter out dismissed suggestions
  const filtered = suggestions.filter(s => {
    const key = `${s.type}-${s.id || s.message}`;
    return !state.dismissedSuggestions.includes(key);
  });

  // Sort by priority
  const priorityOrder = { high: 0, medium: 1, low: 2, info: 3 };
  filtered.sort((a, b) => {
    return (priorityOrder[a.priority] || 3) - (priorityOrder[b.priority] || 3);
  });

  // Update state
  state.lastAnalysis = Date.now();
  saveState(state);

  return filtered;
}

/**
 * Generate suggestions from harmony analysis
 */
function generateHarmonySuggestions(projectPath) {
  const suggestions = [];

  try {
    const analysis = harmonyAnalyzer.analyzeProject(projectPath);
    const score = harmonyAnalyzer.calculateHarmonyScore(analysis);

    // Overall harmony suggestion
    if (score < PHI_INV * 100) { // < 61.8%
      suggestions.push({
        id: 'harmony-score',
        type: 'HARMONY',
        priority: score < PHI_INV * PHI_INV * 100 ? 'high' : 'medium',
        title: `Harmony Score: ${score.toFixed(0)}/100`,
        message: `Code-philosophy alignment needs attention`,
        action: 'Run /judge on key files to identify specific issues',
        data: { score },
      });
    }

    // Top gaps as suggestions
    const topGaps = analysis.gaps
      .filter(g => g.severity === 'high' || g.severity === 'medium')
      .slice(0, 3);

    for (const gap of topGaps) {
      suggestions.push({
        id: `gap-${gap.principle}-${gap.line || 0}`,
        type: 'HARMONY',
        priority: gap.severity === 'high' ? 'high' : 'medium',
        title: `[${gap.principle}] ${gap.message}`,
        message: gap.suggestion || 'Review and align with principles',
        action: gap.file ? `Check ${path.basename(gap.file)}:${gap.line || 1}` : null,
        data: gap,
      });
    }
  } catch { /* ignore */ }

  return suggestions;
}

/**
 * Generate suggestions from context weaving
 */
function generateContextSuggestions(projectPath, workspacePath) {
  const suggestions = [];

  try {
    const woven = contextWeaver.weaveContext(projectPath, workspacePath);

    // Convert alerts to suggestions
    for (const alert of woven.alerts || []) {
      suggestions.push({
        id: `context-alert-${alert.message.slice(0, 20)}`,
        type: alert.level === 'error' ? 'SYNC' : 'UPDATE',
        priority: alert.level === 'error' ? 'high' : 'medium',
        title: alert.message,
        message: 'Detected from context analysis',
        data: alert,
      });
    }

    // Convert context suggestions
    for (const suggestion of woven.suggestions || []) {
      suggestions.push({
        id: `context-${suggestion.type}-${suggestion.message.slice(0, 20)}`,
        type: suggestion.type === 'sync' ? 'SYNC' : 'WORKFLOW',
        priority: 'low',
        title: suggestion.message,
        message: suggestion.projects ? `Projects: ${suggestion.projects.join(', ')}` : '',
        data: suggestion,
      });
    }

    // Outdated dependencies
    if (woven.layers?.dependencies?.outdated?.length > 0) {
      const outdated = woven.layers.dependencies.outdated;
      suggestions.push({
        id: 'deps-outdated',
        type: 'UPDATE',
        priority: outdated.length > 5 ? 'medium' : 'low',
        title: `${outdated.length} outdated dependencies`,
        message: `Top: ${outdated.slice(0, 3).map(d => d.name).join(', ')}`,
        action: 'Run npm update or review package.json',
        data: { outdated: outdated.slice(0, 5) },
      });
    }

    // Security vulnerabilities
    if (woven.layers?.dependencies?.security?.vulnerabilities) {
      const vulns = woven.layers.dependencies.security.vulnerabilities;
      const total = (vulns.high || 0) + (vulns.critical || 0);
      if (total > 0) {
        suggestions.push({
          id: 'deps-security',
          type: 'UPDATE',
          priority: 'high',
          title: `${total} security vulnerabilities`,
          message: `Critical: ${vulns.critical || 0}, High: ${vulns.high || 0}`,
          action: 'Run npm audit fix',
          data: vulns,
        });
      }
    }
  } catch { /* ignore */ }

  return suggestions;
}

/**
 * Generate suggestions from consciousness
 */
function generateConsciousnessSuggestions() {
  const suggestions = [];

  try {
    const capabilities = consciousness.getCapabilityMap();

    // Underutilized capabilities
    for (const cap of (capabilities.underutilized || []).slice(0, 3)) {
      suggestions.push({
        id: `capability-${cap.name}`,
        type: 'CAPABILITY',
        priority: 'info',
        title: `Underutilized: ${cap.name}`,
        message: `${cap.type} used ${cap.uses} times - might help with current work`,
        data: cap,
      });
    }

    // Skill-based suggestions
    const growth = consciousness.getHumanGrowth();
    const skills = Object.entries(growth.skills || {})
      .sort((a, b) => b[1].level - a[1].level)
      .slice(0, 3);

    if (skills.length > 0) {
      const topSkill = skills[0];
      if (topSkill[1].level > 0.7) {
        suggestions.push({
          id: `skill-${topSkill[0]}`,
          type: 'LEARNING',
          priority: 'info',
          title: `Strong skill: ${topSkill[0]}`,
          message: `Consider exploring advanced patterns or teaching others`,
          data: { skill: topSkill[0], level: topSkill[1].level },
        });
      }
    }
  } catch { /* ignore */ }

  return suggestions;
}

/**
 * Generate workflow suggestions
 */
function generateWorkflowSuggestions(projectPath) {
  const suggestions = [];

  // Check for uncommitted changes
  try {
    const { spawnSync } = require('child_process');
    const result = spawnSync('git', ['status', '--porcelain'], {
      cwd: projectPath,
      encoding: 'utf8',
    });

    if (result.status === 0) {
      const lines = result.stdout.trim().split('\n').filter(l => l);
      if (lines.length > 20) {
        suggestions.push({
          id: 'git-large-uncommitted',
          type: 'WORKFLOW',
          priority: 'medium',
          title: `${lines.length} uncommitted changes`,
          message: 'Consider breaking into smaller commits',
          action: 'Review changes and commit logically grouped files',
        });
      }
    }

    // Check branch
    const branchResult = spawnSync('git', ['branch', '--show-current'], {
      cwd: projectPath,
      encoding: 'utf8',
    });

    if (branchResult.status === 0) {
      const branch = branchResult.stdout.trim();
      if (branch !== 'main' && branch !== 'master') {
        // Check how old the branch is
        const logResult = spawnSync('git', ['log', 'main..HEAD', '--oneline'], {
          cwd: projectPath,
          encoding: 'utf8',
        });

        if (logResult.status === 0) {
          const commits = logResult.stdout.trim().split('\n').filter(l => l).length;
          if (commits > 10) {
            suggestions.push({
              id: 'git-long-branch',
              type: 'WORKFLOW',
              priority: 'low',
              title: `Branch ${branch} has ${commits} commits`,
              message: 'Consider merging or rebasing',
            });
          }
        }
      }
    }
  } catch { /* ignore */ }

  return suggestions;
}

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Dismiss a suggestion
 */
function dismissSuggestion(suggestionId) {
  const state = loadState();
  const key = suggestionId;
  if (!state.dismissedSuggestions.includes(key)) {
    state.dismissedSuggestions.push(key);
    // Keep only last 100 dismissed
    state.dismissedSuggestions = state.dismissedSuggestions.slice(-100);
    saveState(state);
  }
}

/**
 * Mark suggestion as shown
 */
function markShown(suggestionId) {
  const state = loadState();
  state.suggestionsShown.push({
    id: suggestionId,
    timestamp: Date.now(),
  });
  // Keep only last 50 shown
  state.suggestionsShown = state.suggestionsShown.slice(-50);
  saveState(state);
}

/**
 * Get suggestions to show now (filter by not recently shown)
 */
function getSuggestionsToShow(limit = 3) {
  const state = loadState();
  const suggestions = generateSuggestions();

  // Filter out recently shown (within last hour)
  const hourAgo = Date.now() - (60 * 60 * 1000);
  const recentlyShown = state.suggestionsShown
    .filter(s => s.timestamp > hourAgo)
    .map(s => s.id);

  const toShow = suggestions.filter(s => !recentlyShown.includes(s.id));

  // Mark as shown
  for (const s of toShow.slice(0, limit)) {
    markShown(s.id);
  }

  return toShow.slice(0, limit);
}

/**
 * Get suggestions relevant to a specific task/topic
 */
function getSuggestionsForTask(taskDescription) {
  const suggestions = generateSuggestions();
  const taskLower = taskDescription.toLowerCase();

  // Score suggestions by relevance
  const scored = suggestions.map(s => {
    let score = 0;
    const content = `${s.title} ${s.message}`.toLowerCase();

    // Keyword matching
    const keywords = taskLower.split(/\s+/);
    for (const keyword of keywords) {
      if (keyword.length > 3 && content.includes(keyword)) {
        score += 10;
      }
    }

    // Type matching
    if (taskLower.includes('update') && s.type === 'UPDATE') score += 20;
    if (taskLower.includes('sync') && s.type === 'SYNC') score += 20;
    if (taskLower.includes('harmony') && s.type === 'HARMONY') score += 20;
    if (taskLower.includes('workflow') && s.type === 'WORKFLOW') score += 20;

    return { ...s, relevanceScore: score };
  });

  return scored
    .filter(s => s.relevanceScore > 0)
    .sort((a, b) => b.relevanceScore - a.relevanceScore)
    .slice(0, 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// PROACTIVE INJECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Generate proactive injection for session start
 * Now generates ACTIONABLE suggestions with executable commands
 * "Kelim for the Ohr" - Vessels for the Light
 */
function generateSessionInjection() {
  const suggestions = getSuggestionsToShow(3);

  if (suggestions.length === 0) {
    return null;
  }

  const lines = ['', '── 🎯 SUGGESTED ACTIONS ────────────────────────────────────'];

  for (let i = 0; i < suggestions.length; i++) {
    const s = suggestions[i];
    const typeInfo = SUGGESTION_TYPES[s.type] || { emoji: '💡' };
    const priorityIcon = s.priority === 'high' ? '🔴' :
                        s.priority === 'medium' ? '🟡' : '🟢';

    // Generate actionable command based on suggestion type
    const command = generateActionCommand(s);

    lines.push(`   [${i + 1}] ${priorityIcon} ${s.title}`);
    if (command) {
      lines.push(`       └─ ${command}`);
    } else if (s.action) {
      lines.push(`       └─ ${s.action}`);
    }
  }

  return lines.join('\n');
}

/**
 * Generate an actionable command from a suggestion
 * Maps suggestions to Task tool invocations with appropriate agents
 * "Seder Hishtalshelut" - Lightning Flash from Keter to Malkhut
 */
function generateActionCommand(suggestion) {
  if (!suggestion) return null;

  const { type, data, title, message, action } = suggestion;

  // Extract file info from title if present (e.g., "Function at line 87 has 68 lines")
  const fileMatch = action?.match(/Check (\S+):(\d+)/);
  const fileName = fileMatch ? fileMatch[1] : null;
  const lineNumber = fileMatch ? fileMatch[2] : null;

  // Extract principle from title if present (e.g., "[BURN]", "[VERIFY]")
  const principleMatch = title?.match(/\[(BURN|VERIFY|CULTURE|PHI)\]/);
  const principle = principleMatch ? principleMatch[1] : null;

  // Map suggestion types to agent commands
  switch (type) {
    case 'HARMONY':
      if (principle === 'BURN' && fileName) {
        return `Task simplifier "simplify ${fileName}${lineNumber ? ':' + lineNumber : ''}"`;
      }
      if (principle === 'VERIFY') {
        if (fileName) {
          return `Task reviewer "check error handling in ${fileName}"`;
        }
        return `Task reviewer "check error handling"`;
      }
      if (principle === 'CULTURE') {
        return `Task doc "update documentation"`;
      }
      if (title?.includes('Harmony Score')) {
        return `Task oracle "show harmony dashboard"`;
      }
      return `/judge on the affected files`;

    case 'UPDATE':
      if (message?.includes('security') || message?.includes('vulnerabilit')) {
        return `Task guardian "run security audit"`;
      }
      if (message?.includes('outdated')) {
        return `npm outdated && npm update`;
      }
      return `npm audit fix`;

    case 'SYNC':
      return `Task integrator "sync ecosystem"`;

    case 'WORKFLOW':
      if (message?.includes('uncommitted') || title?.includes('uncommitted')) {
        return `/commit`;
      }
      if (message?.includes('branch') || title?.includes('branch')) {
        return `git rebase main`;
      }
      return null;

    case 'LEARNING':
      if (data?.skill) {
        return `Task archivist "search patterns for ${data.skill}"`;
      }
      return `Task archivist "search memory"`;

    case 'CAPABILITY':
      if (data?.name) {
        return `Try using ${data.name}`;
      }
      return `/help`;

    default:
      return null;
  }
}

/**
 * Check if we should inject suggestions now
 */
function shouldInjectNow() {
  const state = loadState();
  const hoursSinceLastAnalysis = (Date.now() - state.lastAnalysis) / (1000 * 60 * 60);

  // Inject if more than 1.618 hours since last analysis (φ-aligned)
  return hoursSinceLastAnalysis > PHI;
}

// ═══════════════════════════════════════════════════════════════════════════
// DISPLAY
// ═══════════════════════════════════════════════════════════════════════════

function printAdvisorReport() {
  const suggestions = generateSuggestions();

  console.log('═══════════════════════════════════════════════════════════════════════════════');
  console.log('🎯 CYNIC PROACTIVE ADVISOR');
  console.log('═══════════════════════════════════════════════════════════════════════════════\n');

  if (suggestions.length === 0) {
    console.log('   *tail wag* No suggestions right now. All is harmonious.');
  } else {
    console.log(`SUGGESTIONS (${suggestions.length}):`);
    console.log('────────────────────────────────────────────────────────────────────────────────');

    for (const s of suggestions.slice(0, 10)) {
      const typeInfo = SUGGESTION_TYPES[s.type] || { emoji: '💡', name: s.type };
      const priorityIcon = s.priority === 'high' ? '🔴' :
                          s.priority === 'medium' ? '🟡' :
                          s.priority === 'low' ? '🟢' : 'ℹ️';

      console.log(`\n   ${priorityIcon} ${typeInfo.emoji} [${typeInfo.name}] ${s.title}`);
      if (s.message) {
        console.log(`      ${s.message}`);
      }
      if (s.action) {
        console.log(`      └─ Action: ${s.action}`);
      }
    }

    if (suggestions.length > 10) {
      console.log(`\n   ... and ${suggestions.length - 10} more suggestions`);
    }
  }

  console.log('\n═══════════════════════════════════════════════════════════════════════════════');
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Initialization
  init,

  // Generation
  generateSuggestions,
  getSuggestionsToShow,
  getSuggestionsForTask,

  // Management
  dismissSuggestion,
  markShown,

  // Proactive injection
  generateSessionInjection,
  shouldInjectNow,

  // Display
  printAdvisorReport,

  // Constants
  SUGGESTION_TYPES,
  PHI,
};

// CLI execution
if (require.main === module) {
  init();
  printAdvisorReport();
}
