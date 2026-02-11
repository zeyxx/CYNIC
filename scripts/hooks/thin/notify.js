#!/usr/bin/env node
/**
 * CYNIC Thin Notify Hook — Notification
 *
 * Delegates to daemon via HTTP. Burst detection persists in daemon RAM.
 *
 * "Le chien alerte" - CYNIC
 *
 * @event Notification
 */
'use strict';

import { callDaemon, readHookInput, safeOutput } from './daemon-client.js';

const input = await readHookInput();
const result = await callDaemon('Notification', input, { timeout: 5000 });
safeOutput(result);
