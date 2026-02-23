/**
 * Command Invocation Flow
 * Handles POST /api/commands/invoke + WebSocket listening for results
 */

import { apiClient } from './client';
import { wsClient } from './ws';
import type { CommandRequest, CommandResponse, CommandEventData } from '../types/api';

/**
 * Options for invokeCommand()
 */
export interface InvokeCommandOptions {
  timeout?: number; // milliseconds, default 30000
  onProgress?: (status: 'start' | 'complete' | 'error') => void;
}

/**
 * Invoke a command by ID with timeout and WebSocket result handling
 *
 * Flow:
 * 1. POST to /api/commands/invoke (returns immediately with command_id)
 * 2. Start listening for command_complete event via WebSocket
 * 3. Wait for result (timeout after 30s by default)
 * 4. Return result or throw error
 *
 * @param command_id - ID of the command to invoke
 * @param params - Command parameters
 * @param options - Configuration (timeout, progress callback)
 * @returns Promise resolving to the command result
 * @throws Error on network failure, validation error, timeout, or server error
 */
export async function invokeCommand(
  command_id: string,
  params: Record<string, unknown>,
  options: InvokeCommandOptions = {}
): Promise<unknown> {
  const { timeout = 30000, onProgress } = options;

  // Validate inputs
  if (!command_id || typeof command_id !== 'string') {
    throw new Error('Invalid command_id: must be a non-empty string');
  }
  if (!params || typeof params !== 'object') {
    throw new Error('Invalid params: must be an object');
  }
  if (timeout <= 0 || timeout > 300000) {
    throw new Error('Invalid timeout: must be between 1ms and 5 minutes');
  }

  // Build request
  const request: CommandRequest = { command_id, params };

  // Setup abort controller for timeout
  const abortController = new AbortController();
  const timeoutId = setTimeout(() => {
    abortController.abort();
  }, timeout);

  try {
    // Step 1: POST to /api/commands/invoke
    let invokeResponse: CommandResponse;
    try {
      invokeResponse = await apiClient.invokeCommand(request);
    } catch (error) {
      throw new Error(
        `Failed to invoke command: ${error instanceof Error ? error.message : String(error)}`
      );
    }

    // Verify response has required fields
    if (!invokeResponse.id) {
      throw new Error('Invalid response: missing command id');
    }

    // Step 2: Notify listeners that command started
    onProgress?.('start');

    // Step 3: Listen for command_complete event via WebSocket
    // Use a promise that resolves when we get the event or timeout occurs
    const result = await listenForCommandComplete(invokeResponse.id, abortController);

    // Step 4: Return result
    onProgress?.('complete');
    return result;
  } catch (error) {
    onProgress?.('error');

    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new Error(`Command timeout: no response within ${timeout}ms`);
    }

    throw error;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Listen for command_complete event via WebSocket
 * Returns a promise that resolves when the event arrives or rejects on timeout/error
 *
 * CRITICAL FIX (2026-02-22):
 * - Added cleanedUp flag to prevent double-cleanup and orphaned listeners
 * - Removed unused cleanup return statement (was never called)
 * - All cleanup paths now use the flag to ensure cleanup happens exactly once
 */
function listenForCommandComplete(
  command_id: string,
  abortController: AbortController
): Promise<unknown> {
  return new Promise((resolve, reject) => {
    // Flag to prevent double-cleanup if both handlers fire
    let cleanedUp = false;

    // Perform cleanup (remove all listeners and abort handler)
    const cleanup = () => {
      if (cleanedUp) return; // Guard: only cleanup once
      cleanedUp = true;

      wsClient.off('command_complete', handleCommandComplete);
      abortController.signal.removeEventListener('abort', handleAbort);
    };

    // Handler called when command_complete event arrives
    const handleCommandComplete = (data: CommandEventData) => {
      // Ignore events for other commands
      if (data.command_id !== command_id) {
        return;
      }

      // Cleanup before resolving/rejecting (guard prevents double-cleanup)
      cleanup();

      // Handle result based on status
      if (data.status === 'error') {
        const errorMsg = data.error?.message || 'Unknown error';
        reject(new Error(`Command error: ${errorMsg}`));
      } else if (data.status === 'complete') {
        resolve(data.result);
      } else {
        reject(new Error(`Unexpected command status: ${data.status}`));
      }
    };

    // Register listener
    wsClient.on('command_complete', handleCommandComplete);

    // Setup abort signal handler
    const handleAbort = () => {
      // Cleanup before rejecting (guard prevents double-cleanup)
      cleanup();
      reject(new DOMException('Command timeout', 'AbortError'));
    };

    abortController.signal.addEventListener('abort', handleAbort);
  });
}

/**
 * Format result for display in UI
 * Handles different types (string, number, object, array)
 *
 * @param result - Raw result from command
 * @param maxLength - Max characters before truncation
 * @returns Formatted string suitable for display
 */
export function formatCommandResult(result: unknown, maxLength: number = 500): string {
  if (result === null || result === undefined) {
    return '(no result)';
  }

  let formatted: string;

  if (typeof result === 'string') {
    formatted = result;
  } else if (typeof result === 'number' || typeof result === 'boolean') {
    formatted = String(result);
  } else if (Array.isArray(result) || typeof result === 'object') {
    try {
      formatted = JSON.stringify(result, null, 2);
    } catch {
      formatted = String(result);
    }
  } else {
    formatted = String(result);
  }

  if (formatted.length > maxLength) {
    return `${formatted.substring(0, maxLength)}...\n(truncated, see logs)`;
  }

  return formatted;
}

/**
 * Safely display result in HTML element
 * Uses textContent to prevent XSS attacks
 *
 * @param element - HTML element to display in
 * @param result - Command result
 * @param maxLength - Max characters before truncation
 */
export function displayCommandResult(
  element: HTMLElement,
  result: unknown,
  maxLength: number = 500
): void {
  if (!element) {
    return;
  }

  const formatted = formatCommandResult(result, maxLength);

  // Use textContent for safety (prevents XSS)
  element.textContent = formatted;

  // Add styling for better readability
  element.style.fontFamily = 'monospace';
  element.style.whiteSpace = 'pre-wrap';
  element.style.wordBreak = 'break-word';
  element.style.backgroundColor = '#f5f5f5';
  element.style.padding = '10px';
  element.style.borderRadius = '4px';
  element.style.fontSize = '12px';
  element.style.lineHeight = '1.5';
}

/**
 * Safely display error in HTML element
 * Uses textContent to prevent XSS attacks
 *
 * @param element - HTML element to display error in
 * @param error - Error message or Error object
 */
export function displayCommandError(element: HTMLElement, error: unknown): void {
  if (!element) {
    return;
  }

  const message = error instanceof Error ? error.message : String(error);
  element.textContent = `Error: ${message}`;

  // Add styling for error state
  element.style.fontFamily = 'monospace';
  element.style.whiteSpace = 'pre-wrap';
  element.style.wordBreak = 'break-word';
  element.style.color = '#d32f2f';
  element.style.backgroundColor = '#ffebee';
  element.style.padding = '10px';
  element.style.borderRadius = '4px';
  element.style.fontSize = '12px';
  element.style.lineHeight = '1.5';
}
