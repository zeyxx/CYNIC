#!/usr/bin/env node
/**
 * CYNIC Thin Notify Hook — Notification
 *
 * Delegates to daemon via HTTP. Burst detection persists in daemon RAM.
 * Also notifies Python kernel (fire-and-forget) for pattern learning.
 *
 * "Le chien alerte" - CYNIC
 *
 * @event Notification
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';
import { notifyKernel } from './kernel-client.js';

const input = await readHookInput();

// Notify Python kernel (fire-and-forget — never blocks)
notifyKernel('Notification', input);

const result = await callDaemon('Notification', input, { timeout: 5000 });
safeOutput(result);
