/**
 * CYNIC Python Kernel Client
 *
 * Fire-and-forget bridge from JS thin hooks to the Python kernel API.
 * Every hook event is posted to /perceive so the Python kernel can:
 *   1. Judge it (REFLEX cycle, <10ms)
 *   2. Learn from it (QTable TD(0) update)
 *   3. Accumulate event history in PostgreSQL
 *
 * Design contract (NON-NEGOTIABLE):
 *   - NEVER blocks the hook execution (fire-and-forget)
 *   - NEVER throws to the caller
 *   - NEVER affects Claude Code behavior (no output, no exit codes)
 *   - Degrades silently if Python kernel is down
 *
 * Reality mapping (hook event → CYNIC reality dimension):
 *   PostToolUse      → CODE   (CYNIC observing code operations)
 *   UserPromptSubmit → HUMAN  (user interaction)
 *   Error            → CYNIC  (self-monitoring, immune system)
 *   Notification     → CYNIC  (system notification)
 *   SubagentStart    → CYNIC  (dog spawned)
 *   SubagentStop     → CYNIC  (dog stopped)
 *   SessionStart     → HUMAN  (new session begins)
 *   SessionEnd       → HUMAN  (session ends)
 *
 * @module scripts/hooks/thin/kernel-client
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';

const KERNEL_PORT = parseInt(process.env.CYNIC_KERNEL_PORT || '8765', 10);
const KERNEL_URL = `http://127.0.0.1:${KERNEL_PORT}`;
const NOTIFY_TIMEOUT_MS = 800;  // Must not slow down hooks

// Reality dimension per hook event
const REALITY_MAP = {
  PostToolUse:      'CODE',
  PreToolUse:       'CODE',
  UserPromptSubmit: 'HUMAN',
  Stop:             'HUMAN',
  SessionStart:     'HUMAN',
  SessionEnd:       'HUMAN',
  Error:            'CYNIC',
  Notification:     'CYNIC',
  SubagentStart:    'CYNIC',
  SubagentStop:     'CYNIC',
};

// Simple in-process cache: kernel alive status (TTL: 10s)
let _kernelAlive = null;
let _kernelCheckedAt = 0;
const ALIVE_TTL_MS = 10_000;

const _GUIDANCE_FILE = path.join(os.homedir(), '.cynic', 'guidance.json');
const _GUIDANCE_STALE_MS = 5 * 60 * 1000; // 5 minutes

/**
 * Read the kernel's last guidance (written by Python after each /perceive judgment).
 *
 * This is the feedback loop: kernel judges → writes guidance.json →
 * next hook reads it → Claude Code sees kernel recommendation.
 *
 * @returns {{ state_key, verdict, q_score, confidence, reality, dog_votes, timestamp }|null}
 */
export function readKernelGuidance() {
  try {
    if (!fs.existsSync(_GUIDANCE_FILE)) return null;
    const data = JSON.parse(fs.readFileSync(_GUIDANCE_FILE, 'utf8'));
    // Staleness: ignore guidance older than 5 minutes
    if (Date.now() - (data.timestamp * 1000) > _GUIDANCE_STALE_MS) return null;
    return data;
  } catch {
    return null;
  }
}

/**
 * Notify the Python kernel about a hook event.
 *
 * Fire-and-forget: this function returns immediately.
 * The POST to /perceive happens asynchronously in the background.
 *
 * @param {string} hookEvent  — e.g. 'PostToolUse', 'UserPromptSubmit'
 * @param {Object} hookInput  — raw hook stdin JSON
 */
export function notifyKernel(hookEvent, hookInput) {
  // Launch async work — DO NOT await
  _sendToKernel(hookEvent, hookInput).catch(() => {
    // Silent degradation — kernel down is not an error
  });
}

/**
 * Internal: async send (not exported, never awaited by callers)
 */
async function _sendToKernel(hookEvent, hookInput) {
  // Skip if kernel was recently confirmed dead
  const now = Date.now();
  if (_kernelAlive === false && (now - _kernelCheckedAt) < ALIVE_TTL_MS) {
    return;
  }

  const reality = REALITY_MAP[hookEvent] ?? 'CYNIC';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), NOTIFY_TIMEOUT_MS);

  try {
    const resp = await fetch(`${KERNEL_URL}/perceive`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        source: `hook:${hookEvent}`,
        reality,
        data: _sanitize(hookInput),
        context: _context(hookEvent, hookInput),
        run_judgment: true,
        level: 'REFLEX',
      }),
      signal: controller.signal,
    });

    _kernelAlive = resp.ok;
    _kernelCheckedAt = Date.now();
  } catch {
    // Timeout, ECONNREFUSED, etc. — kernel is down
    _kernelAlive = false;
    _kernelCheckedAt = Date.now();
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Sanitize hook input: strip huge fields, keep what matters for learning.
 * The Python kernel doesn't need full file contents — it needs signal shape.
 */
function _sanitize(input) {
  if (!input || typeof input !== 'object') return input;

  const out = { ...input };

  // Truncate large string fields (code diffs, file contents)
  for (const key of ['content', 'diff', 'output', 'file_content', 'prompt']) {
    if (typeof out[key] === 'string' && out[key].length > 500) {
      out[key] = out[key].slice(0, 500) + `…[${out[key].length - 500} chars truncated]`;
    }
  }

  return out;
}

/**
 * Build a human-readable context string for the Python kernel.
 */
function _context(hookEvent, input) {
  switch (hookEvent) {
    case 'PostToolUse':
      return `Tool used: ${input.tool_name ?? 'unknown'} on ${input.tool_input?.file_path ?? '?'}`;
    case 'PreToolUse':
      return `Tool about to run: ${input.tool_name ?? 'unknown'}`;
    case 'UserPromptSubmit':
      return `User prompt (${String(input.prompt ?? '').length} chars)`;
    case 'Error':
      return `Error in ${input.hook_name ?? 'hook'}: ${String(input.error ?? '').slice(0, 100)}`;
    case 'Notification':
      return `Notification: ${input.title ?? ''} — ${String(input.message ?? '').slice(0, 100)}`;
    case 'SubagentStart':
      return `Subagent spawned: ${input.subagent_type ?? 'unknown'}`;
    case 'SubagentStop':
      return `Subagent stopped: ${input.subagent_type ?? 'unknown'}`;
    case 'SessionStart':
      return `Session started`;
    case 'SessionEnd':
      return `Session ended`;
    default:
      return `Hook event: ${hookEvent}`;
  }
}
