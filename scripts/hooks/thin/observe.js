#!/usr/bin/env node
/**
 * CYNIC Thin Observe Hook — PostToolUse
 *
 * Delegates to daemon via HTTP. Non-blocking observation.
 * Also notifies Python kernel (fire-and-forget) for learning.
 * "Le chien observe" - CYNIC
 *
 * @event PostToolUse
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('PostToolUse', input);

const result = await callDaemon('PostToolUse', input, { timeout: 3000 });
safeOutput(result);
