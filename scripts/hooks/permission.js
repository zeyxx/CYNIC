#!/usr/bin/env node
/**
 * CYNIC Permission Hook - PermissionRequest/PermissionResponse
 *
 * "Le chien garde la porte" - CYNIC tracks all access decisions
 *
 * This hook runs when permission is requested or granted/denied.
 * It maintains a security audit trail for compliance and learning.
 *
 * @event PermissionRequest, PermissionResponse
 * @behavior non-blocking (logs and learns)
 */

'use strict';

// ESM imports from the lib bridge
import cynic, {
  DC,
  detectUser,
  detectProject,
  loadUserProfile,
  updateUserProfile,
  saveCollectivePattern,
  sendHookToCollectiveSync,
  callBrainTool,
  getConsciousness,
} from '../lib/index.js';

// Phase 22: Session state
import { getSessionState } from './lib/index.js';

// Load optional modules
const consciousness = getConsciousness();

// =============================================================================
// PERMISSION CATEGORIES
// =============================================================================

/** Permission categories for classification */
const PERMISSION_CATEGORIES = {
  // File operations
  'Write': { category: 'file_write', sensitivity: 'medium', auditable: true },
  'Edit': { category: 'file_edit', sensitivity: 'medium', auditable: true },
  'Read': { category: 'file_read', sensitivity: 'low', auditable: false },

  // Command execution
  'Bash': { category: 'command_exec', sensitivity: 'high', auditable: true },

  // Web operations
  'WebFetch': { category: 'web_access', sensitivity: 'medium', auditable: true },
  'WebSearch': { category: 'web_search', sensitivity: 'low', auditable: false },

  // MCP tools
  'mcp__': { category: 'mcp_tool', sensitivity: 'medium', auditable: true },

  // Task/Agent spawning
  'Task': { category: 'agent_spawn', sensitivity: 'medium', auditable: true },
};

/** Sensitive permission patterns */
const SENSITIVE_PATTERNS = [
  { pattern: /\.env|credentials|secret|key|token|password/i, reason: 'Contains sensitive keywords' },
  { pattern: /\/etc\/|\/root\/|C:\\Windows/i, reason: 'System directory access' },
  { pattern: /rm\s+-rf|delete|drop\s+table/i, reason: 'Destructive operation' },
  { pattern: /git\s+push.*force|git\s+reset\s+--hard/i, reason: 'Destructive git operation' },
  { pattern: /npm\s+publish|docker\s+push/i, reason: 'Publishing operation' },
  { pattern: /curl.*-X\s+(POST|PUT|DELETE)/i, reason: 'Modifying HTTP request' },
];

// =============================================================================
// AUDIT TRAIL
// =============================================================================

/** In-memory audit log (persisted to collective) */
const auditLog = [];
const MAX_AUDIT_LOG = 100;

/**
 * Classify a permission request
 */
function classifyPermission(toolName, toolInput) {
  // Find matching category
  let classification = { category: 'unknown', sensitivity: 'medium', auditable: true };

  for (const [prefix, config] of Object.entries(PERMISSION_CATEGORIES)) {
    if (toolName.startsWith(prefix)) {
      classification = { ...config };
      break;
    }
  }

  // Check for sensitive patterns
  const inputStr = JSON.stringify(toolInput);
  for (const { pattern, reason } of SENSITIVE_PATTERNS) {
    if (pattern.test(inputStr)) {
      classification.sensitivity = 'high';
      classification.sensitiveReason = reason;
      classification.auditable = true;
      break;
    }
  }

  return classification;
}

/**
 * Record to audit trail
 */
function recordAudit(entry) {
  auditLog.push({
    ...entry,
    timestamp: Date.now(),
  });

  // Trim if too large
  if (auditLog.length > MAX_AUDIT_LOG) {
    auditLog.shift();
  }
}

/**
 * Get permission summary stats
 */
function getPermissionStats() {
  const stats = {
    total: auditLog.length,
    granted: 0,
    denied: 0,
    byCategory: {},
    bySensitivity: { low: 0, medium: 0, high: 0 },
  };

  for (const entry of auditLog) {
    if (entry.granted) stats.granted++;
    else stats.denied++;

    stats.byCategory[entry.category] = (stats.byCategory[entry.category] || 0) + 1;
    stats.bySensitivity[entry.sensitivity] = (stats.bySensitivity[entry.sensitivity] || 0) + 1;
  }

  return stats;
}

// =============================================================================
// SAFE OUTPUT - Handle EPIPE errors gracefully
// =============================================================================

function safeOutput(data) {
  try {
    const str = typeof data === 'string' ? data : JSON.stringify(data);
    process.stdout.write(str + '\n');
  } catch (e) {
    if (e.code === 'EPIPE') process.exit(0);
  }
}

// =============================================================================
// HANDLERS
// =============================================================================

/**
 * Handle PermissionRequest event
 */
async function handlePermissionRequest(hookContext, sessionState, user) {
  const toolName = hookContext.tool_name || hookContext.toolName || '';
  const toolInput = hookContext.tool_input || hookContext.toolInput || {};
  const requestId = hookContext.request_id || hookContext.requestId || `req_${Date.now()}`;

  // Classify the permission
  const classification = classifyPermission(toolName, toolInput);

  // Record in audit trail if auditable
  if (classification.auditable) {
    recordAudit({
      type: 'request',
      requestId,
      tool: toolName,
      category: classification.category,
      sensitivity: classification.sensitivity,
      sensitiveReason: classification.sensitiveReason,
      granted: null, // Not yet decided
    });
  }

  // Record pattern for collective learning
  const pattern = {
    type: 'permission_request',
    signature: `permission_${classification.category}_${classification.sensitivity}`,
    description: `Permission requested for ${toolName}`,
    context: {
      tool: toolName,
      category: classification.category,
      sensitivity: classification.sensitivity,
    },
  };
  saveCollectivePattern(pattern);

  // Record in session state
  if (sessionState.isInitialized()) {
    sessionState.recordPattern(pattern);
  }

  // Log high-sensitivity requests in consciousness
  if (consciousness && classification.sensitivity === 'high') {
    try {
      consciousness.recordInsight({
        type: 'permission_request',
        title: `High-sensitivity permission: ${toolName}`,
        message: classification.sensitiveReason || 'Requires attention',
        data: { tool: toolName, category: classification.category },
        priority: 'medium',
      });
    } catch (e) { /* ignore */ }
  }

  // Send to MCP server
  sendHookToCollectiveSync('PermissionRequest', {
    requestId,
    toolName,
    category: classification.category,
    sensitivity: classification.sensitivity,
    userId: user.userId,
    timestamp: Date.now(),
  });

  return {
    continue: true,
    requestId,
    classification,
    message: classification.sensitivity === 'high'
      ? `*sniff* High-sensitivity permission requested: ${toolName}`
      : null,
  };
}

/**
 * Handle PermissionResponse event
 */
async function handlePermissionResponse(hookContext, sessionState, user) {
  const requestId = hookContext.request_id || hookContext.requestId || '';
  const toolName = hookContext.tool_name || hookContext.toolName || '';
  const granted = hookContext.granted !== false && hookContext.allowed !== false;
  const reason = hookContext.reason || hookContext.message || '';

  // Classify the permission
  const classification = classifyPermission(toolName, hookContext.tool_input || {});

  // Update audit trail
  if (classification.auditable) {
    // Find and update the request entry
    const requestEntry = auditLog.find(e => e.requestId === requestId && e.type === 'request');
    if (requestEntry) {
      requestEntry.granted = granted;
      requestEntry.responseTime = Date.now() - requestEntry.timestamp;
    }

    // Also record the response
    recordAudit({
      type: 'response',
      requestId,
      tool: toolName,
      category: classification.category,
      sensitivity: classification.sensitivity,
      granted,
      reason,
    });
  }

  // Record pattern for collective learning
  const pattern = {
    type: 'permission_response',
    signature: `permission_${granted ? 'granted' : 'denied'}_${classification.category}`,
    description: `Permission ${granted ? 'granted' : 'denied'} for ${toolName}`,
    context: {
      tool: toolName,
      category: classification.category,
      granted,
      reason,
    },
  };
  saveCollectivePattern(pattern);

  // Record in session state
  if (sessionState.isInitialized()) {
    sessionState.recordPattern(pattern);
  }

  // Log denials in consciousness
  if (consciousness && !granted) {
    try {
      consciousness.recordInsight({
        type: 'permission_denied',
        title: `Permission denied: ${toolName}`,
        message: reason || 'User denied permission',
        data: { tool: toolName, category: classification.category },
        priority: 'low',
      });
    } catch (e) { /* ignore */ }
  }

  // Update user profile stats
  const profile = loadUserProfile(user.userId);
  updateUserProfile(profile, {
    stats: {
      permissionsRequested: (profile.stats?.permissionsRequested || 0) + 1,
      permissionsGranted: (profile.stats?.permissionsGranted || 0) + (granted ? 1 : 0),
      permissionsDenied: (profile.stats?.permissionsDenied || 0) + (granted ? 0 : 1),
    },
  });

  // Send to MCP server
  sendHookToCollectiveSync('PermissionResponse', {
    requestId,
    toolName,
    category: classification.category,
    granted,
    reason,
    userId: user.userId,
    timestamp: Date.now(),
  });

  // Provide feedback to learning system
  if (!granted) {
    callBrainTool('brain_cynic_feedback', {
      feedback: 'incorrect',
      context: `Permission denied for ${toolName}: ${reason}`,
    }).catch(() => { /* ignore */ });
  }

  return {
    continue: true,
    requestId,
    granted,
    classification,
    stats: getPermissionStats(),
    message: !granted
      ? `*sniff* Permission denied for ${toolName}`
      : null,
  };
}

// =============================================================================
// MAIN HANDLER
// =============================================================================

async function main() {
  const output = {
    type: 'Permission',
    timestamp: new Date().toISOString(),
    continue: true,
    event: null,
    requestId: null,
    classification: null,
    granted: null,
    stats: null,
    message: null,
  };

  try {
    // Read stdin
    const fs = await import('fs');
    let input = '';

    try {
      input = fs.readFileSync(0, 'utf8');
    } catch (syncErr) {
      input = await new Promise((resolve) => {
        let data = '';
        process.stdin.setEncoding('utf8');
        process.stdin.on('data', chunk => { data += chunk; });
        process.stdin.on('end', () => resolve(data));
        process.stdin.on('error', () => resolve(''));
        process.stdin.resume();
        setTimeout(() => resolve(data), 3000);
      });
    }

    if (!input || input.trim().length === 0) {
      safeOutput(output);
      return;
    }

    const hookContext = JSON.parse(input);
    const eventType = hookContext.event_type || hookContext.eventType || '';
    output.event = eventType;

    // Get user and session state
    const user = detectUser();
    const sessionState = getSessionState();

    // Route to appropriate handler
    if (eventType === 'PermissionRequest' || eventType === 'permission_request') {
      const result = await handlePermissionRequest(hookContext, sessionState, user);
      output.requestId = result.requestId;
      output.classification = result.classification;
      output.message = result.message;
    } else if (eventType === 'PermissionResponse' || eventType === 'permission_response') {
      const result = await handlePermissionResponse(hookContext, sessionState, user);
      output.requestId = result.requestId;
      output.granted = result.granted;
      output.classification = result.classification;
      output.stats = result.stats;
      output.message = result.message;
    } else {
      output.message = `Unknown permission event: ${eventType}`;
    }

    safeOutput(output);

  } catch (error) {
    output.error = error.message;
    safeOutput(output);
  }
}

main();
