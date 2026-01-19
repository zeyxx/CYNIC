/**
 * CYNIC Task Continuation Enforcer
 *
 * "Le chien ne lâche pas l'os" - CYNIC doesn't let go of incomplete tasks
 *
 * Tracks todos during a session and blocks premature stopping.
 * Forces agents to complete their tasks before ending.
 *
 * @module cynic/lib/task-enforcer
 */

'use strict';

const fs = require('fs');
const path = require('path');

// =============================================================================
// CONSTANTS
// =============================================================================

const PHI_INV = 0.618033988749895; // φ⁻¹ = 61.8% completion threshold

// =============================================================================
// PATHS
// =============================================================================

function getEnforcerDir() {
  const root = process.env.CLAUDE_PLUGIN_ROOT || process.cwd();
  const dir = path.join(root, '..', '.cynic', 'enforcer');
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  return dir;
}

function getSessionTodoPath(sessionId) {
  return path.join(getEnforcerDir(), `${sessionId || 'default'}-todos.json`);
}

function getEnforcerStatePath(sessionId) {
  return path.join(getEnforcerDir(), `${sessionId || 'default'}-state.json`);
}

// =============================================================================
// TODO TRACKING
// =============================================================================

/**
 * Load todos for a session
 * @param {string} sessionId - Session identifier
 * @returns {Array} Todo items
 */
function loadTodos(sessionId) {
  const todoPath = getSessionTodoPath(sessionId);
  if (fs.existsSync(todoPath)) {
    try {
      return JSON.parse(fs.readFileSync(todoPath, 'utf-8'));
    } catch (e) {
      return [];
    }
  }
  return [];
}

/**
 * Save todos for a session
 * @param {string} sessionId - Session identifier
 * @param {Array} todos - Todo items
 */
function saveTodos(sessionId, todos) {
  const todoPath = getSessionTodoPath(sessionId);
  fs.writeFileSync(todoPath, JSON.stringify(todos, null, 2));
}

/**
 * Update todos from TodoWrite tool output
 * @param {string} sessionId - Session identifier
 * @param {Array} newTodos - New todo items from tool output
 */
function updateTodosFromTool(sessionId, newTodos) {
  if (!Array.isArray(newTodos)) return;

  // Transform to our format
  const todos = newTodos.map((t, idx) => ({
    id: `todo_${idx}`,
    content: t.content,
    status: t.status, // 'pending', 'in_progress', 'completed'
    activeForm: t.activeForm,
    createdAt: new Date().toISOString(),
  }));

  saveTodos(sessionId, todos);
}

/**
 * Get incomplete todos
 * @param {string} sessionId - Session identifier
 * @returns {Array} Incomplete todo items
 */
function getIncompleteTodos(sessionId) {
  const todos = loadTodos(sessionId);
  return todos.filter(t => t.status !== 'completed');
}

/**
 * Calculate completion percentage
 * @param {string} sessionId - Session identifier
 * @returns {number} Completion percentage (0-1)
 */
function getCompletionRate(sessionId) {
  const todos = loadTodos(sessionId);
  if (todos.length === 0) return 1; // No todos = complete

  const completed = todos.filter(t => t.status === 'completed').length;
  return completed / todos.length;
}

// =============================================================================
// ENFORCER STATE
// =============================================================================

/**
 * Load enforcer state
 * @param {string} sessionId - Session identifier
 * @returns {Object} Enforcer state
 */
function loadEnforcerState(sessionId) {
  const statePath = getEnforcerStatePath(sessionId);
  if (fs.existsSync(statePath)) {
    try {
      return JSON.parse(fs.readFileSync(statePath, 'utf-8'));
    } catch (e) {
      return createDefaultState();
    }
  }
  return createDefaultState();
}

/**
 * Save enforcer state
 * @param {string} sessionId - Session identifier
 * @param {Object} state - Enforcer state
 */
function saveEnforcerState(sessionId, state) {
  const statePath = getEnforcerStatePath(sessionId);
  fs.writeFileSync(statePath, JSON.stringify(state, null, 2));
}

function createDefaultState() {
  return {
    active: true, // Enforcer is active by default
    blockCount: 0, // How many times we've blocked
    maxBlocks: 3, // Max blocks before giving up (φ-derived: ~61.8% persistence)
    lastBlockReason: null,
    createdAt: new Date().toISOString(),
  };
}

/**
 * Set enforcer active state
 * @param {string} sessionId - Session identifier
 * @param {boolean} active - Whether enforcer is active
 */
function setEnforcerActive(sessionId, active) {
  const state = loadEnforcerState(sessionId);
  state.active = active;
  saveEnforcerState(sessionId, state);
}

/**
 * Check if enforcer is active
 * @param {string} sessionId - Session identifier
 * @returns {boolean} Whether enforcer is active
 */
function isEnforcerActive(sessionId) {
  const state = loadEnforcerState(sessionId);
  return state.active;
}

// =============================================================================
// CONTINUATION ENFORCEMENT
// =============================================================================

/**
 * Check if agent should be blocked from stopping
 * @param {string} sessionId - Session identifier
 * @returns {Object} Block decision with reason and inject prompt
 */
function shouldBlockStop(sessionId) {
  const state = loadEnforcerState(sessionId);

  // Enforcer disabled
  if (!state.active) {
    return { block: false };
  }

  // Max blocks reached - give up gracefully
  if (state.blockCount >= state.maxBlocks) {
    return {
      block: false,
      reason: `*yawn* Gave up after ${state.blockCount} attempts. Some tasks remain incomplete.`
    };
  }

  const incompleteTodos = getIncompleteTodos(sessionId);
  const completionRate = getCompletionRate(sessionId);

  // No incomplete todos - allow stop
  if (incompleteTodos.length === 0) {
    return { block: false };
  }

  // Completion below φ⁻¹ threshold - block
  if (completionRate < PHI_INV) {
    state.blockCount++;
    state.lastBlockReason = `${incompleteTodos.length} todos incomplete`;
    saveEnforcerState(sessionId, state);

    const todoList = incompleteTodos
      .map(t => `   • [${t.status}] ${t.content}`)
      .join('\n');

    return {
      block: true,
      reason: `*GROWL* Cannot stop! ${incompleteTodos.length} tasks remain incomplete (${Math.round(completionRate * 100)}% done, need ${Math.round(PHI_INV * 100)}% minimum).`,
      injectPrompt: `TASK CONTINUATION REQUIRED

*growl* You have incomplete tasks. You MUST continue working.

REMAINING TASKS:
${todoList}

DO NOT attempt to stop again until these are completed or explicitly cancelled by the user.
Mark tasks as 'completed' when done, or ask the user if you're blocked.

Continue now.`,
    };
  }

  // Above threshold but still incomplete - warn but allow
  return {
    block: false,
    reason: `*sniff* ${incompleteTodos.length} tasks still pending, but progress is sufficient (${Math.round(completionRate * 100)}%).`,
  };
}

/**
 * Clean up enforcer data for a session
 * @param {string} sessionId - Session identifier
 */
function cleanupSession(sessionId) {
  const todoPath = getSessionTodoPath(sessionId);
  const statePath = getEnforcerStatePath(sessionId);

  try {
    if (fs.existsSync(todoPath)) fs.unlinkSync(todoPath);
    if (fs.existsSync(statePath)) fs.unlinkSync(statePath);
  } catch (e) {
    // Ignore cleanup errors
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  PHI_INV,

  // Paths
  getEnforcerDir,
  getSessionTodoPath,

  // Todo tracking
  loadTodos,
  saveTodos,
  updateTodosFromTool,
  getIncompleteTodos,
  getCompletionRate,

  // State management
  loadEnforcerState,
  saveEnforcerState,
  setEnforcerActive,
  isEnforcerActive,

  // Enforcement
  shouldBlockStop,
  cleanupSession,
};
