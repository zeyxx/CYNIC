#!/usr/bin/env node
/**
 * CYNIC Thin Perceive Hook — UserPromptSubmit
 *
 * Delegates to daemon via HTTP. Auto-starts daemon if needed.
 * Also notifies Python kernel (fire-and-forget) for learning.
 * "Le chien délègue" - CYNIC
 *
 * @event UserPromptSubmit
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel, readKernelGuidance } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('UserPromptSubmit', input);

const result = await callDaemon('UserPromptSubmit', input, { timeout: 8000 });

// Inject kernel guidance from last judgment — this is the feedback loop closing.
// guidance.json was written by the Python kernel after judging the PREVIOUS interaction.
// CYNIC sees its own past judgment as context for the current response.
const guidance = readKernelGuidance();
if (guidance) {
  result.kernelGuidance = guidance;
}

safeOutput(result);
