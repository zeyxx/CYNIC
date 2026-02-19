#!/usr/bin/env node
/**
 * CYNIC Thin Observe Hook — PostToolUse
 *
 * Python-kernel-only. No JS daemon dependency.
 * Notifies kernel (fire-and-forget) for learning. Non-blocking observation.
 * "Le chien observe" - CYNIC
 *
 * @event PostToolUse
 */
'use strict';

import { readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';
import fs from 'fs';
import os from 'os';
import path from 'path';

const _ERR_LOG = path.join(os.homedir(), '.cynic', 'hook-error.log');

try {
  const input = await readHookInput();

  // Notify Python kernel (fire-and-forget — never blocks)
  notifyKernel('PostToolUse', input);

  // PostToolUse is pure observation — no output needed.
  safeOutput({});
} catch (err) {
  try {
    fs.appendFileSync(_ERR_LOG, `[${new Date().toISOString()}] observe.js: ${err.stack || err.message}\n`);
  } catch { /* ignore */ }
  process.stdout.write(JSON.stringify({}) + '\n');
}
