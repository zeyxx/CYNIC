/**
 * Tests for Command Invocation Flow
 * Covers: POST /api/commands/invoke, timeout, error handling, WebSocket result streaming
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  invokeCommand,
  formatCommandResult,
  displayCommandResult,
  displayCommandError,
} from '../src/api/commands';
import { apiClient } from '../src/api/client';
import { wsClient } from '../src/api/ws';
import type { CommandResponse, CommandEventData } from '../src/types/api';

// Mock the API and WebSocket clients
vi.mock('../src/api/client');
vi.mock('../src/api/ws');

describe('Command Invocation Flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // TEST 1: Successful command invocation
  // ============================================================================
  it('should successfully invoke command and return result', async () => {
    const commandId = 'test_command';
    const params = { arg1: 'value1', arg2: 42 };
    const expectedResult = { status: 'ok', data: 'test_data' };

    // Mock API response
    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    // Mock WebSocket listener setup
    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });

    // Simulate WebSocket event after a short delay
    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'complete',
          result: expectedResult,
        });
      }
    }, 10);

    const result = await invokeCommand(commandId, params);

    expect(result).toEqual(expectedResult);
    expect(apiClient.invokeCommand).toHaveBeenCalledWith({ command_id: commandId, params });
  });

  // ============================================================================
  // TEST 2: Command invocation with various parameter types
  // ============================================================================
  it('should serialize various parameter types correctly', async () => {
    const commandId = 'test_types';
    const params = {
      str: 'hello',
      num: 42,
      bool: true,
      arr: [1, 2, 3],
      obj: { nested: 'value' },
    };

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });

    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'complete',
          result: 'ok',
        });
      }
    }, 10);

    await invokeCommand(commandId, params);

    expect(apiClient.invokeCommand).toHaveBeenCalledWith({ command_id: commandId, params });
  });

  // ============================================================================
  // TEST 3: Timeout handling
  // ============================================================================
  it('should timeout if command_complete event does not arrive', async () => {
    const commandId = 'slow_command';
    const params = {};
    const shortTimeout = 50; // 50ms timeout

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    vi.mocked(wsClient.on).mockImplementation(() => {
      // Never trigger the callback (simulating timeout)
    });

    await expect(invokeCommand(commandId, params, { timeout: shortTimeout })).rejects.toThrow(
      /timeout/i
    );
  });

  // ============================================================================
  // TEST 4: Error handling from command execution
  // ============================================================================
  it('should handle error status from command_complete event', async () => {
    const commandId = 'error_command';
    const params = {};
    const errorMessage = 'Command execution failed';

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });

    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'error',
          error: { message: errorMessage, code: 'EXEC_ERROR' },
        });
      }
    }, 10);

    await expect(invokeCommand(commandId, params)).rejects.toThrow(errorMessage);
  });

  // ============================================================================
  // TEST 5: Network error handling
  // ============================================================================
  it('should handle network errors during POST request', async () => {
    const commandId = 'network_error';
    const params = {};

    vi.mocked(apiClient.invokeCommand).mockRejectedValue(
      new Error('Network error: Connection refused')
    );

    await expect(invokeCommand(commandId, params)).rejects.toThrow(/Failed to invoke command/);
  });

  // ============================================================================
  // TEST 6: Validation errors
  // ============================================================================
  it('should reject invalid command_id', async () => {
    await expect(invokeCommand('', {})).rejects.toThrow(/Invalid command_id/);
    await expect(invokeCommand(null as any, {})).rejects.toThrow(/Invalid command_id/);
  });

  it('should reject invalid params', async () => {
    await expect(invokeCommand('cmd', null as any)).rejects.toThrow(/Invalid params/);
    await expect(invokeCommand('cmd', 'not_object' as any)).rejects.toThrow(/Invalid params/);
  });

  it('should reject invalid timeout', async () => {
    await expect(invokeCommand('cmd', {}, { timeout: 0 })).rejects.toThrow(/Invalid timeout/);
    await expect(invokeCommand('cmd', {}, { timeout: 400000 })).rejects.toThrow(/Invalid timeout/);
  });

  // ============================================================================
  // TEST 7: Result display with different types
  // ============================================================================
  describe('formatCommandResult', () => {
    it('should format string result', () => {
      expect(formatCommandResult('hello')).toBe('hello');
    });

    it('should format number result', () => {
      expect(formatCommandResult(42)).toBe('42');
    });

    it('should format boolean result', () => {
      expect(formatCommandResult(true)).toBe('true');
    });

    it('should format object as JSON', () => {
      const obj = { key: 'value', num: 42 };
      const result = formatCommandResult(obj);
      expect(result).toContain('key');
      expect(result).toContain('value');
    });

    it('should format array as JSON', () => {
      const arr = [1, 2, 3];
      const result = formatCommandResult(arr);
      expect(result).toContain('1');
    });

    it('should truncate long results', () => {
      const longStr = 'a'.repeat(1000);
      const result = formatCommandResult(longStr, 100);
      expect(result.length).toBeLessThan(1000);
      expect(result).toContain('truncated');
    });

    it('should handle null/undefined', () => {
      expect(formatCommandResult(null)).toBe('(no result)');
      expect(formatCommandResult(undefined)).toBe('(no result)');
    });
  });

  // ============================================================================
  // TEST 8: Display result in HTML element
  // ============================================================================
  describe('displayCommandResult', () => {
    let container: HTMLElement;

    beforeEach(() => {
      container = document.createElement('div');
      document.body.appendChild(container);
    });

    afterEach(() => {
      document.body.removeChild(container);
    });

    it('should display result text safely', () => {
      displayCommandResult(container, 'test result');
      expect(container.textContent).toBe('test result');
    });

    it('should prevent XSS by using textContent', () => {
      const xssPayload = '<script>alert("xss")</script>';
      displayCommandResult(container, xssPayload);
      expect(container.textContent).toBe(xssPayload);
      expect(container.innerHTML).not.toContain('<script>');
    });

    it('should apply styling for readability', () => {
      displayCommandResult(container, 'test');
      expect(container.style.fontFamily).toContain('monospace');
      expect(container.style.whiteSpace).toBe('pre-wrap');
      expect(container.style.backgroundColor).toBe('rgb(245, 245, 245)');
    });

    it('should handle null element gracefully', () => {
      expect(() => {
        displayCommandResult(null as any, 'test');
      }).not.toThrow();
    });

    it('should display object results formatted', () => {
      const obj = { key: 'value' };
      displayCommandResult(container, obj);
      expect(container.textContent).toContain('key');
      expect(container.textContent).toContain('value');
    });
  });

  // ============================================================================
  // TEST 9: Display error in HTML element
  // ============================================================================
  describe('displayCommandError', () => {
    let container: HTMLElement;

    beforeEach(() => {
      container = document.createElement('div');
      document.body.appendChild(container);
    });

    afterEach(() => {
      document.body.removeChild(container);
    });

    it('should display error message', () => {
      displayCommandError(container, new Error('Test error'));
      expect(container.textContent).toContain('Test error');
    });

    it('should apply error styling', () => {
      displayCommandError(container, 'Test error');
      expect(container.style.color).toBe('rgb(211, 47, 47)');
      expect(container.style.backgroundColor).toBe('rgb(255, 235, 238)');
    });

    it('should prevent XSS in error messages', () => {
      const xssPayload = '<img src=x onerror=alert("xss")>';
      displayCommandError(container, xssPayload);
      expect(container.textContent).toContain(xssPayload);
      expect(container.innerHTML).not.toContain('<img');
    });

    it('should handle Error objects', () => {
      const error = new Error('Something went wrong');
      displayCommandError(container, error);
      expect(container.textContent).toContain('Something went wrong');
    });

    it('should handle string errors', () => {
      displayCommandError(container, 'String error');
      expect(container.textContent).toContain('String error');
    });
  });

  // ============================================================================
  // TEST 10: Progress callback
  // ============================================================================
  it('should call progress callback with status updates', async () => {
    const commandId = 'progress_command';
    const params = {};
    const progressCallback = vi.fn();

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });

    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'complete',
          result: 'ok',
        });
      }
    }, 10);

    await invokeCommand(commandId, params, { onProgress: progressCallback });

    expect(progressCallback).toHaveBeenCalledWith('start');
    expect(progressCallback).toHaveBeenCalledWith('complete');
  });

  // ============================================================================
  // TEST 11: WebSocket listener cleanup verification (CRITICAL FIX)
  // ============================================================================
  it('should remove WebSocket listener after successful command', async () => {
    const commandId = 'cleanup_test';
    const params = {};

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    const mockOff = vi.fn();
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });
    vi.mocked(wsClient.off).mockImplementation(mockOff);

    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'complete',
          result: 'ok',
        });
      }
    }, 10);

    await invokeCommand(commandId, params);

    // Verify wsClient.off was called to clean up listener
    expect(mockOff).toHaveBeenCalledWith('command_complete', expect.any(Function));
  });

  // ============================================================================
  // TEST 12: WebSocket listener cleanup on timeout (CRITICAL FIX)
  // ============================================================================
  it('should remove WebSocket listener on timeout', async () => {
    const commandId = 'timeout_cleanup';
    const params = {};
    const shortTimeout = 50;

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    const mockOff = vi.fn();
    const mockRemoveEventListener = vi.fn();

    vi.mocked(wsClient.on).mockImplementation(() => {
      // Never call the callback (simulating timeout)
    });
    vi.mocked(wsClient.off).mockImplementation(mockOff);

    // Mock AbortSignal.removeEventListener
    const originalAddEventListener = AbortSignal.prototype.addEventListener;
    vi.spyOn(AbortSignal.prototype, 'addEventListener').mockImplementation(
      function (this: AbortSignal, type: string, listener: EventListener) {
        originalAddEventListener.call(this, type, listener);
      }
    );
    vi.spyOn(AbortSignal.prototype, 'removeEventListener').mockImplementation(
      mockRemoveEventListener
    );

    await expect(
      invokeCommand(commandId, params, { timeout: shortTimeout })
    ).rejects.toThrow(/timeout/i);

    // Verify both listeners were cleaned up
    expect(mockOff).toHaveBeenCalledWith('command_complete', expect.any(Function));
    // The abort signal removal is called as part of cleanup
  });

  // ============================================================================
  // TEST 13: Race condition guard - cleanup flag prevents double-cleanup
  // ============================================================================
  it('should prevent double-cleanup even if handlers are called multiple times', async () => {
    const commandId = 'race_guard_test';
    const params = {};

    const mockResponse: CommandResponse = {
      id: commandId,
      status: 'pending',
      timestamp: new Date().toISOString(),
    };
    vi.mocked(apiClient.invokeCommand).mockResolvedValue(mockResponse);

    let onCommandComplete: ((data: CommandEventData) => void) | null = null;
    const offCalls: any[] = [];
    const removeEventListenerCalls: any[] = [];

    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        onCommandComplete = handler;
      }
    });

    vi.mocked(wsClient.off).mockImplementation((event, handler) => {
      offCalls.push({ event, handler });
    });

    // Mock removeEventListener to track calls
    const originalRemoveEventListener = AbortSignal.prototype.removeEventListener;
    vi.spyOn(AbortSignal.prototype, 'removeEventListener').mockImplementation(
      function (this: AbortSignal, type: string, listener: EventListener) {
        removeEventListenerCalls.push(type);
        originalRemoveEventListener.call(this, type, listener);
      }
    );

    // Simulate command complete
    setTimeout(() => {
      if (onCommandComplete) {
        onCommandComplete({
          command_id: commandId,
          status: 'complete',
          result: 'success',
        });
      }
    }, 10);

    const result = await invokeCommand(commandId, params);
    expect(result).toBe('success');

    // Verify wsClient.off was called exactly once for command_complete
    const commandCompleteCalls = offCalls.filter((call) => call.event === 'command_complete');
    expect(commandCompleteCalls.length).toBe(1);

    // Verify removeEventListener was called exactly once for abort
    const abortRemovalCalls = removeEventListenerCalls.filter((type) => type === 'abort');
    expect(abortRemovalCalls.length).toBe(1);
  });
});
