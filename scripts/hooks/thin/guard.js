#!/usr/bin/env node
/**
 * CYNIC Thin Guard Hook — PreToolUse
 *
 * Python-kernel-only. No JS daemon dependency.
 * Notifies kernel (fire-and-forget) and reads last guidance to decide on blocking.
 * "Le chien protège" - CYNIC
 *
 * Blocking logic (conservative by design):
 *   - NEVER block without an explicit Python kernel BARK verdict.
 *   - Only block if guidance.verdict === 'BARK' AND tool is in DANGEROUS_TOOLS list.
 *   - If Python kernel is down (no guidance) → allow by default (degrade gracefully).
 *
 * @event PreToolUse
 * @behavior blocking only on BARK + dangerous tool combination
 */
'use strict';

import { readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel, readKernelGuidance } from './kernel-client.js';
import fs from 'fs';
import os from 'os';
import path from 'path';

const _ERR_LOG = path.join(os.homedir(), '.cynic', 'hook-error.log');

// Tools that can cause irreversible harm — only these are candidates for blocking.
const DANGEROUS_TOOLS = new Set([
  'Bash',   // can run rm -rf, format, wipe
  'Edit',   // can overwrite critical files
  'Write',  // can overwrite critical files
]);

try {
  const input = await readHookInput();

  // Notify Python kernel (fire-and-forget — never blocks)
  notifyKernel('PreToolUse', input);

  // Read last kernel guidance (written by Python after previous judgment).
  const guidance = readKernelGuidance();

  // Default: allow (degrade gracefully when kernel is down)
  const hookOutput = {};

  if (guidance && guidance.verdict === 'BARK' && DANGEROUS_TOOLS.has(input.tool_name)) {
    hookOutput.hookSpecificOutput = {
      permissionDecision: 'deny',
      decisionReason: `CYNIC BARK: Q=${guidance.q_score != null ? guidance.q_score.toFixed(1) : '?'} — kernel flagged this operation`,
    };
  }

  safeOutput(hookOutput);
} catch (err) {
  try {
    fs.appendFileSync(_ERR_LOG, `[${new Date().toISOString()}] guard.js: ${err.stack || err.message}\n`);
  } catch { /* ignore */ }
  process.stdout.write(JSON.stringify({}) + '\n');
}
