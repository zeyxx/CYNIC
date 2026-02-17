#!/usr/bin/env node
/**
 * CYNIC Thin Error Hook — Error
 *
 * Delegates to daemon via HTTP. Error history persists in daemon RAM.
 * Also notifies Python kernel (fire-and-forget) — immune system learning.
 *
 * "Le chien apprend de ses erreurs" - CYNIC
 *
 * @event Error
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('Error', input);

const result = await callDaemon('Error', input, { timeout: 5000 });
safeOutput(result);
