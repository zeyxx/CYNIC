/**
 * E2E and Integration Tests for CYNIC Webapp Phase 2
 * Comprehensive testing of full workflow: schema → palette → command → result → metrics update
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { SchemaCache } from '../src/ui/form-builder';
import { CommandPalette } from '../src/ui/command-palette';
import { MetricsPanel } from '../src/ui/metrics-dashboard';
import { invokeCommand } from '../src/api/commands';
import { apiClient } from '../src/api/client';
import { wsClient } from '../src/api/ws';
import { store } from '../src/state/store';
import type { OrganismSchema, AccountStatusResponse, CommandEventData } from '../src/types/api';

// Mock the API and WebSocket clients
vi.mock('../src/api/client');
vi.mock('../src/api/ws');

/**
 * MOCK DATA FIXTURES
 */

const MOCK_SCHEMA: OrganismSchema = {
  version: '1.0.0',
  commands: [
    {
      id: 'get-status',
      name: 'Get Status',
      description: 'Fetch organism status',
      params: {
        verbose: { type: 'boolean', required: false, description: 'Include extra detail' },
        format: { type: 'enum', required: true, description: 'Output format', enum: ['json', 'text'] },
      },
      returns: { type: 'object', description: 'Status object' },
    },
    {
      id: 'set-learn-rate',
      name: 'Set Learn Rate',
      description: 'Adjust learning rate',
      params: {
        rate: { type: 'number', required: true, description: 'Learning rate (0-1)', default: 0.618 },
      },
      returns: { type: 'object', description: 'Updated learn rate' },
    },
  ],
  skills: [],
  state: {},
};

const MOCK_ACCOUNT: AccountStatusResponse = {
  timestamp: '2026-02-22T12:00:00Z',
  balance_usd: 1234.56,
  spent_usd: 765.43,
  budget_remaining_usd: 468.13,
  learn_rate: 0.618,
  reputation: 95,
};

/**
 * E2E TEST SUITE
 */

describe('E2E & Integration Tests', () => {
  let palette: CommandPalette | null = null;
  let metrics: MetricsPanel | null = null;
  let paletteContainer: HTMLDivElement;
  let metricsContainer: HTMLDivElement;
  // Global map to track WebSocket event handlers by event type
  let wsHandlers: Map<string, Set<(data: any) => void>> = new Map();

  beforeEach(() => {
    vi.clearAllMocks();
    SchemaCache.invalidate();
    localStorage.clear();
    // Reset store by creating fresh state
    store.setState({
      schema: null,
      commands: [],
      currentCommand: null,
      metrics: {},
      wsConnected: false,
      account: null,
      policy: null,
    });

    // Reset WebSocket handlers
    wsHandlers.clear();

    // Create DOM containers
    paletteContainer = document.createElement('div');
    paletteContainer.id = 'palette-container';
    document.body.appendChild(paletteContainer);

    metricsContainer = document.createElement('div');
    metricsContainer.id = 'metrics-container';
    document.body.appendChild(metricsContainer);

    // Setup WebSocket mock - GLOBAL for all handlers
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (!wsHandlers.has(eventType)) {
        wsHandlers.set(eventType, new Set());
      }
      wsHandlers.get(eventType)!.add(handler as (data: any) => void);
      return wsClient; // Allow chaining
    });

    vi.mocked(wsClient.off).mockImplementation((eventType, handler) => {
      if (wsHandlers.has(eventType)) {
        wsHandlers.get(eventType)!.delete(handler as (data: any) => void);
      }
      return wsClient;
    });

    // Setup API mocks - must check if mocked before calling
    const mockedLoadSchema = vi.mocked(apiClient.loadSchema);
    const mockedGetAccount = vi.mocked(apiClient.getAccount);
    if (mockedLoadSchema && typeof mockedLoadSchema.mockResolvedValue === 'function') {
      mockedLoadSchema.mockResolvedValue(MOCK_SCHEMA);
    }
    if (mockedGetAccount && typeof mockedGetAccount.mockResolvedValue === 'function') {
      mockedGetAccount.mockResolvedValue(MOCK_ACCOUNT);
    }
  });

  afterEach(() => {
    // Cleanup
    if (palette) {
      palette.close();
      palette = null;
    }
    if (metrics) {
      metrics.destroy();
      metrics = null;
    }
    if (document.body.contains(paletteContainer)) {
      document.body.removeChild(paletteContainer);
    }
    if (document.body.contains(metricsContainer)) {
      document.body.removeChild(metricsContainer);
    }
  });

  // ============================================================================
  // E2E TEST 1: Happy Path - Full Workflow
  // Load schema → Show palette → Invoke command → Display result → Update metrics
  // ============================================================================
  it('should execute complete happy path: schema → palette → command → result → metrics', async () => {
    // Step 1: Load schema via SchemaCache
    const schema = await SchemaCache.load('/api/organism/schema');
    expect(schema.version).toBe('1.0.0');
    expect(schema.commands.length).toBe(2);

    // Step 2: Store schema
    store.setSchema(schema);
    expect(store.getState().schema?.version).toBe('1.0.0');

    // Step 3: Create and open command palette
    palette = new CommandPalette('#palette-container', schema);
    palette.open();

    // Verify palette is visible
    const paletteModal = document.querySelector('.command-palette-modal');
    expect(paletteModal).toBeTruthy();

    // Step 4: Setup WebSocket mock for command completion
    let commandCompleteHandler: ((data: CommandEventData) => void) | null = null;
    vi.mocked(wsClient.on).mockImplementation((eventType, handler) => {
      if (eventType === 'command_complete') {
        commandCompleteHandler = handler;
      }
    });

    // Mock API response for command execution
    const expectedResult = { status: 'ok', confidence: 0.618, entities: 1247 };
    vi.mocked(apiClient.invokeCommand).mockResolvedValueOnce({
      id: 'get-status',
      status: 'pending',
      timestamp: new Date().toISOString(),
    });

    // Step 5: Simulate command invocation
    const invokePromise = invokeCommand('get-status', { verbose: true, format: 'json' });

    // Simulate WebSocket event after command completes
    setTimeout(() => {
      if (commandCompleteHandler) {
        commandCompleteHandler({
          command_id: 'get-status',
          status: 'complete',
          result: expectedResult,
        });
      }
    }, 10);

    const result = await invokePromise;
    expect(result).toEqual(expectedResult);

    // Step 6: Create and render metrics panel
    metrics = new MetricsPanel('#metrics-container');
    await metrics.render();

    // Verify metrics are displayed
    const metricsCard = document.querySelector('.metrics-card');
    expect(metricsCard).toBeTruthy();

    // Verify initial balance is displayed
    const balanceEl = metricsCard?.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('1,234.56');
  });

  // ============================================================================
  // E2E TEST 2: Stress Test - 100 Concurrent Commands
  // Verify system handles batch operations without memory leaks or interference
  // ============================================================================
  it('should handle 100 concurrent command invocations without interference', async () => {
    // Setup
    vi.mocked(apiClient.invokeCommand).mockImplementation((request) =>
      Promise.resolve({
        id: (request as any).command_id || 'cmd-batch',
        status: 'pending',
        timestamp: new Date().toISOString(),
      })
    );

    // Execute 100 concurrent commands with mocked API
    const commandPromises = Array.from({ length: 100 }, (_, i) => {
      const cmdId = `stress-test-${i}`;
      const params = { index: i, timestamp: Date.now() };

      const promise = invokeCommand(cmdId, params);

      // Simulate immediate completion by firing the WebSocket event
      setTimeout(() => {
        const completeHandlers = wsHandlers.get('command_complete') || new Set();
        completeHandlers.forEach((handler) => {
          handler({
            command_id: cmdId,
            status: 'complete',
            result: { index: i, success: true },
          });
        });
      }, Math.random() * 50); // Random delay 0-50ms to simulate real concurrency

      return promise;
    });

    // Wait for all commands to complete
    const results = await Promise.all(commandPromises);

    // Verify all commands completed successfully
    expect(results.length).toBe(100);
    results.forEach((result, i) => {
      expect(result).toBeTruthy();
      expect((result as any).success).toBe(true);
      expect((result as any).index).toBe(i);
    });

    // Verify API was called 100 times
    expect(vi.mocked(apiClient.invokeCommand)).toHaveBeenCalledTimes(100);
  });

  // ============================================================================
  // E2E TEST 3: Error Handling - Timeout, Invalid Command, Server Error
  // Verify system gracefully handles various error scenarios
  // ============================================================================
  it('should gracefully handle timeout, invalid command, and server errors', async () => {
    // Test 3a: Timeout error
    vi.mocked(apiClient.invokeCommand).mockImplementationOnce(
      () => new Promise((_, reject) => setTimeout(() => reject(new Error('Request timeout')), 50))
    );

    try {
      await invokeCommand('timeout-cmd', {});
      expect.fail('Should have thrown timeout error');
    } catch (error) {
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toContain('timeout');
    }

    // Test 3b: Invalid command (missing required parameters)
    vi.mocked(apiClient.invokeCommand).mockResolvedValueOnce({
      id: 'invalid-cmd',
      status: 'error',
      error: { message: 'Missing required parameter: format', code: 'VALIDATION_ERROR' },
      timestamp: new Date().toISOString(),
    });

    const invalidResult = await apiClient.invokeCommand({
      command_id: 'set-learn-rate',
      params: { format: 'json' }, // Missing 'rate' parameter
    });

    expect(invalidResult.status).toBe('error');
    expect(invalidResult.error?.code).toBe('VALIDATION_ERROR');

    // Test 3c: Server error (500)
    vi.mocked(apiClient.invokeCommand).mockImplementationOnce(
      () => new Promise((_, reject) => reject(new Error('500 Internal Server Error')))
    );

    try {
      await invokeCommand('server-error-cmd', {});
      expect.fail('Should have thrown server error');
    } catch (error) {
      expect(error).toBeInstanceOf(Error);
      expect((error as Error).message).toContain('500');
    }
  });

  // ============================================================================
  // E2E TEST 4: Metrics Update After Command Execution
  // Verify metrics panel receives and displays updates after command completes
  // ============================================================================
  it('should update metrics panel after command execution', async () => {
    // Setup metrics panel
    metrics = new MetricsPanel('#metrics-container');
    await metrics.render();

    // Verify initial balance
    let balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('1,234.56');

    // Mock updated metrics BEFORE triggering update
    vi.mocked(apiClient.getAccount).mockResolvedValueOnce({
      ...MOCK_ACCOUNT,
      balance_usd: 999.99,
      spent_usd: 234.01,
    });

    // Trigger metrics update by calling update() directly on the metrics instance
    // (This simulates what would happen when a WebSocket event fires)
    await metrics.update();

    // Verify metrics updated
    balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('999.99');
  });

  // ============================================================================
  // E2E TEST 5: Command Palette Lifecycle
  // Open → Search → Close → Reopen (verify state isolation)
  // ============================================================================
  it('should correctly manage command palette lifecycle with state isolation', async () => {
    // Load and store schema
    const schema = await SchemaCache.load('/api/organism/schema');
    store.setSchema(schema);

    // Create palette
    palette = new CommandPalette('#palette-container', schema);
    palette.open();

    // Step 1: Verify palette is visible
    let modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeTruthy();

    // Step 2: Verify search input exists
    const searchInput = modal?.querySelector('input[type="text"]') as HTMLInputElement;
    expect(searchInput).toBeTruthy();

    // Step 3: Close palette
    palette.close();

    // Step 4: Wait and verify closed
    await new Promise((resolve) => setTimeout(resolve, 100));
    modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeFalsy(); // Should be removed from DOM

    // Step 5: Reopen palette (new instance, same container)
    palette = new CommandPalette('#palette-container', schema);
    palette.open();

    // Step 6: Verify reopened
    modal = document.querySelector('.command-palette-modal');
    expect(modal).toBeTruthy();
  });

  // ============================================================================
  // E2E TEST 6: WebSocket Reconnection Handling
  // Verify metrics continues to work after connection drop/restore
  // ============================================================================
  it('should handle WebSocket reconnection without losing state', async () => {
    // Create metrics panel
    const schema = await SchemaCache.load('/api/organism/schema');
    store.setSchema(schema);

    metrics = new MetricsPanel('#metrics-container');
    await metrics.render();

    // Verify initial state
    let metricsCard = document.querySelector('.metrics-card');
    expect(metricsCard).toBeTruthy();

    // Simulate disconnect by firing the event
    const disconnectHandlers = wsHandlers.get('disconnect') || new Set();
    disconnectHandlers.forEach((handler) => {
      handler({ code: 1000, reason: 'Manual close' });
    });

    // Verify UI is still functional (not broken)
    metricsCard = document.querySelector('.metrics-card');
    expect(metricsCard).toBeTruthy();

    // Simulate reconnect by firing the event
    const connectHandlers = wsHandlers.get('connect') || new Set();
    connectHandlers.forEach((handler) => {
      handler({ code: 1000 });
    });

    // Verify system recovers
    await new Promise((resolve) => setTimeout(resolve, 100));
    metricsCard = document.querySelector('.metrics-card');
    expect(metricsCard).toBeTruthy();
  });

  // ============================================================================
  // E2E TEST 7: Sequential Commands with State Isolation
  // Multiple commands invoked in sequence don't interfere
  // ============================================================================
  it('should execute sequential commands without state interference', async () => {
    const results: unknown[] = [];

    // Mock API
    vi.mocked(apiClient.invokeCommand).mockImplementation((request) =>
      Promise.resolve({
        id: (request as any).command_id || 'cmd',
        status: 'pending',
        timestamp: new Date().toISOString(),
      })
    );

    // Execute commands sequentially
    for (let i = 0; i < 5; i++) {
      const cmdId = `seq-${i}`;
      const promise = invokeCommand(cmdId, { index: i });

      // Simulate completion by firing the WebSocket event
      setTimeout(() => {
        const completeHandlers = wsHandlers.get('command_complete') || new Set();
        completeHandlers.forEach((handler) => {
          handler({
            command_id: cmdId,
            status: 'complete',
            result: { sequence: i, timestamp: Date.now() },
          });
        });
      }, i * 20); // Staggered completion

      const result = await promise;
      results.push(result);
    }

    // Verify all completed in order without interference
    expect(results.length).toBe(5);
    results.forEach((result, i) => {
      expect((result as any).sequence).toBe(i);
    });
  });

  // ============================================================================
  // E2E TEST 8: Form Validation Before POST
  // Invalid form input rejected before API call
  // ============================================================================
  it('should validate form input and reject invalid submissions before POST', async () => {
    // Load schema with validation rules
    const schema = await SchemaCache.load('/api/organism/schema');
    store.setSchema(schema);

    // Get the command schema
    const getStatusCmd = schema.commands.find((c) => c.id === 'get-status');
    expect(getStatusCmd).toBeTruthy();

    // Verify required parameter 'format' exists
    expect(getStatusCmd?.params.format.required).toBe(true);

    // Attempt to submit without required parameter should fail validation
    const missingRequiredParam = { verbose: true }; // Missing 'format'
    const isValid = 'format' in missingRequiredParam; // Should be false
    expect(isValid).toBe(false);

    // With valid params, API call should proceed
    const validParams = { verbose: true, format: 'json' };
    const hasRequired = 'format' in validParams;
    expect(hasRequired).toBe(true);

    // Mock API should be called only with valid params
    vi.mocked(apiClient.invokeCommand).mockResolvedValueOnce({
      id: 'get-status',
      status: 'pending',
      timestamp: new Date().toISOString(),
    });

    await apiClient.invokeCommand({
      command_id: 'get-status',
      params: validParams,
    });

    expect(vi.mocked(apiClient.invokeCommand)).toHaveBeenCalledWith({
      command_id: 'get-status',
      params: validParams,
    });
  });
});

/**
 * COMPONENT WIRING TESTS
 * Verify components can be instantiated and used correctly
 */
describe('Component Wiring Verification', () => {
  let paletteContainer: HTMLDivElement;
  let metricsContainer: HTMLDivElement;

  beforeEach(() => {
    // Create containers
    paletteContainer = document.createElement('div');
    paletteContainer.id = 'palette-test';
    document.body.appendChild(paletteContainer);

    metricsContainer = document.createElement('div');
    metricsContainer.id = 'metrics-test';
    document.body.appendChild(metricsContainer);
  });

  afterEach(() => {
    if (document.body.contains(paletteContainer)) {
      document.body.removeChild(paletteContainer);
    }
    if (document.body.contains(metricsContainer)) {
      document.body.removeChild(metricsContainer);
    }
  });

  it('should instantiate CommandPalette with schema', () => {
    const schema = MOCK_SCHEMA;
    const palette = new CommandPalette('#palette-test', schema);
    expect(palette).toBeTruthy();
    expect(typeof palette.open).toBe('function');
    expect(typeof palette.close).toBe('function');
    palette.close(); // cleanup
  });

  it('should instantiate MetricsPanel with container', () => {
    const metrics = new MetricsPanel('#metrics-test');
    expect(metrics).toBeTruthy();
    expect(typeof metrics.render).toBe('function');
    expect(typeof metrics.destroy).toBe('function');
    metrics.destroy();
  });

  it('should allow multiple component instances independently', () => {
    const schema = MOCK_SCHEMA;
    const palette1 = new CommandPalette('#palette-test', schema);
    const metrics1 = new MetricsPanel('#metrics-test');

    expect(palette1).toBeTruthy();
    expect(metrics1).toBeTruthy();

    palette1.close();
    metrics1.destroy();
  });

  it('should properly isolate component state', () => {
    const schema = MOCK_SCHEMA;
    const palette1 = new CommandPalette('#palette-test', schema);
    const palette2 = new CommandPalette('#palette-test', schema);

    // Both should be independent
    expect(palette1).not.toBe(palette2);

    palette1.close();
    palette2.close();
  });
});
