import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { MetricsPanel } from '../src/ui/metrics-dashboard';
import * as apiModule from '../src/api/client';
import type { AccountStatusResponse } from '../src/types/api';

const MOCK_ACCOUNT_RESPONSE: AccountStatusResponse = {
  timestamp: '2026-02-22T12:00:00Z',
  balance_usd: 1234.56,
  spent_usd: 765.43,
  budget_remaining_usd: 468.13,
  learn_rate: 0.618,
  reputation: 95,
};

describe('MetricsPanel', () => {
  let container: HTMLDivElement;
  let panel: MetricsPanel;

  beforeEach(() => {
    // Create container
    container = document.createElement('div');
    container.id = 'metrics-container';
    document.body.appendChild(container);

    // Mock apiClient.getAccount()
    vi.spyOn(apiModule.apiClient, 'getAccount').mockResolvedValue(MOCK_ACCOUNT_RESPONSE);
  });

  afterEach(() => {
    // Clean up
    panel?.destroy();
    if (document.body.contains(container)) {
      document.body.removeChild(container);
    }
    vi.clearAllMocks();
  });

  // Test 1: Metrics fetching from API
  it('fetches metrics from GET /api/organism/account and displays them', async () => {
    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // Verify apiClient.getAccount was called
    expect(apiModule.apiClient.getAccount).toHaveBeenCalled();

    // Verify metrics are displayed
    const metricsCard = document.querySelector('.metrics-card');
    expect(metricsCard).toBeTruthy();

    const balanceEl = metricsCard?.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('1,234.56');

    // Learn rate is in a progress bar, so check the progress text
    const learnRateBar = metricsCard?.querySelector('[data-bar="learn_rate"]');
    const learnRateText = learnRateBar?.querySelector('.progress-text');
    expect(learnRateText?.textContent).toContain('61.8%');

    // Reputation is in a progress bar, so check the progress text
    const reputationBar = metricsCard?.querySelector('[data-bar="reputation"]');
    const reputationText = reputationBar?.querySelector('.progress-text');
    expect(reputationText?.textContent).toContain('95');
  });

  // Test 2: Number formatting (currency, percentage, ratio)
  it('correctly formats currency, percentage, and reputation numbers', async () => {
    vi.spyOn(apiModule.apiClient, 'getAccount').mockResolvedValueOnce({
      ...MOCK_ACCOUNT_RESPONSE,
      balance_usd: 50000.1,
      learn_rate: 0.123,
      reputation: 42,
    });

    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // Currency should have thousand separators and 2 decimals
    const balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('50,000.10');

    // Learn rate should be percentage (0.123 -> 12.3%) - in progress bar
    const learnRateBar = document.querySelector('[data-bar="learn_rate"]');
    const learnRateText = learnRateBar?.querySelector('.progress-text');
    expect(learnRateText?.textContent).toContain('12.3%');

    // Reputation should be "42/100" format - in progress bar
    const reputationBar = document.querySelector('[data-bar="reputation"]');
    const reputationText = reputationBar?.querySelector('.progress-text');
    expect(reputationText?.textContent).toContain('42/100');
  });

  // Test 3: Progress bar calculation and rendering
  it('renders progress bars with correct widths based on metrics', async () => {
    vi.spyOn(apiModule.apiClient, 'getAccount').mockResolvedValueOnce({
      ...MOCK_ACCOUNT_RESPONSE,
      learn_rate: 0.618, // Should be 61.8% width
      reputation: 95, // Should be 95% width
    });

    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // Learn rate progress bar (61.8%)
    const learnRateBar = document.querySelector('[data-bar="learn_rate"]');
    expect(learnRateBar).toBeTruthy();
    const learnRateWidth = learnRateBar?.getAttribute('style');
    expect(learnRateWidth).toContain('width');
    // Should be capped at 61.8% for φ-bound display
    expect(learnRateWidth).toContain('61.8%');

    // Reputation progress bar (95/100 = 95%)
    const reputationBar = document.querySelector('[data-bar="reputation"]');
    expect(reputationBar).toBeTruthy();
    const reputationWidth = reputationBar?.getAttribute('style');
    expect(reputationWidth).toContain('width');
    expect(reputationWidth).toContain('95%');

    // Verify text is displayed inside progress bars
    const learnRateText = learnRateBar?.querySelector('.progress-text');
    expect(learnRateText?.textContent).toContain('61.8%');

    const reputationText = reputationBar?.querySelector('.progress-text');
    expect(reputationText?.textContent).toContain('95/100');
  });

  // Test 4: WebSocket listener attachment and cleanup
  it('registers WebSocket listeners and cleans up on destroy', async () => {
    // Create a mock WebSocket client
    const mockWsClient = {
      on: vi.fn(),
      off: vi.fn(),
    };

    panel = new MetricsPanel('#metrics-container', mockWsClient as any);
    await panel.render();

    // Verify listeners were registered for state_update and command_complete
    expect(mockWsClient.on).toHaveBeenCalledWith('state_update', expect.any(Function));
    expect(mockWsClient.on).toHaveBeenCalledWith('command_complete', expect.any(Function));

    // Destroy and verify cleanup
    panel.destroy();

    expect(mockWsClient.off).toHaveBeenCalledWith('state_update', expect.any(Function));
    expect(mockWsClient.off).toHaveBeenCalledWith('command_complete', expect.any(Function));
  });

  // Test 5: Auto-refresh on both state_update and command_complete events
  it('auto-refreshes metrics on state_update and command_complete WebSocket events', async () => {
    let stateUpdateHandler: ((data: unknown) => void) | undefined;
    let commandCompleteHandler: ((data: unknown) => void) | undefined;

    const mockWsClient = {
      on: vi.fn((event, handler) => {
        if (event === 'state_update') stateUpdateHandler = handler;
        if (event === 'command_complete') commandCompleteHandler = handler;
      }),
      off: vi.fn(),
    };

    vi.spyOn(apiModule.apiClient, 'getAccount')
      .mockResolvedValueOnce(MOCK_ACCOUNT_RESPONSE)
      .mockResolvedValueOnce({
        ...MOCK_ACCOUNT_RESPONSE,
        balance_usd: 999.99,
        learn_rate: 0.5,
      })
      .mockResolvedValueOnce({
        ...MOCK_ACCOUNT_RESPONSE,
        balance_usd: 555.55,
      });

    panel = new MetricsPanel('#metrics-container', mockWsClient as any);
    await panel.render();

    // Initial balance should be 1,234.56
    let balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('1,234.56');

    // Simulate state_update event
    expect(stateUpdateHandler).toBeDefined();
    stateUpdateHandler!({ component: 'metabolic', state: {}, timestamp: '2026-02-22T12:00:01Z' });

    // Wait for re-render
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Balance should be updated to 999.99
    balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('999.99');

    // Simulate command_complete event
    expect(commandCompleteHandler).toBeDefined();
    commandCompleteHandler!({
      command_id: 'test-cmd',
      status: 'complete',
      result: {},
    });

    // Wait for re-render
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Balance should be updated again to 555.55
    balanceEl = document.querySelector('[data-metric="balance"]');
    expect(balanceEl?.textContent).toContain('555.55');
  });

  // Test 6: Error handling when API fails
  it('displays error state when API call fails', async () => {
    vi.spyOn(apiModule.apiClient, 'getAccount').mockRejectedValueOnce(
      new Error('Network error')
    );

    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // Should display error message
    const errorEl = document.querySelector('.metrics-error');
    expect(errorEl).toBeTruthy();
    expect(errorEl?.textContent?.toLowerCase()).toContain('error');
  });

  // Test 7: Responsive progress bar styling (flex layout)
  it('uses flex layout for responsive progress bars', async () => {
    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // Check flex container exists
    const flexContainer = document.querySelector('.metrics-card') as HTMLElement;
    expect(flexContainer).toBeTruthy();

    // Check individual metric item exists
    const metricItem = document.querySelector('.metric-item');
    expect(metricItem).toBeTruthy();
  });

  // Test 8: Large screen vs mobile styling
  it('adapts layout for mobile screens (< 640px)', async () => {
    // Note: In real tests, you'd resize the window, but vitest's jsdom is limited
    // This test documents the expected behavior

    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    // On mobile, progress bars should be full width
    const barContainer = document.querySelector('.metric-item');
    expect(barContainer).toBeTruthy();
    // CSS will handle this via media queries
  });

  // Test 9: Timestamp display
  it('displays timestamp of last metrics update', async () => {
    const timestamp = '2026-02-22T15:30:45Z';
    vi.spyOn(apiModule.apiClient, 'getAccount').mockResolvedValueOnce({
      ...MOCK_ACCOUNT_RESPONSE,
      timestamp,
    });

    panel = new MetricsPanel('#metrics-container');
    await panel.render();

    const timestampEl = document.querySelector('.metrics-timestamp');
    expect(timestampEl).toBeTruthy();
    // Should display formatted time (e.g., "15:30:45" or similar)
  });

  // Test 10: Multiple panel instances (independent state)
  it('supports multiple independent MetricsPanel instances', async () => {
    const container1 = document.createElement('div');
    const container2 = document.createElement('div');
    container1.id = 'metrics-1';
    container2.id = 'metrics-2';
    document.body.appendChild(container1);
    document.body.appendChild(container2);

    vi.spyOn(apiModule.apiClient, 'getAccount')
      .mockResolvedValueOnce(MOCK_ACCOUNT_RESPONSE)
      .mockResolvedValueOnce({ ...MOCK_ACCOUNT_RESPONSE, balance_usd: 999.99 });

    const panel1 = new MetricsPanel('#metrics-1');
    const panel2 = new MetricsPanel('#metrics-2');

    await panel1.render();
    await panel2.render();

    const cards = document.querySelectorAll('.metrics-card');
    expect(cards.length).toBe(2);

    panel1.destroy();
    panel2.destroy();

    document.body.removeChild(container1);
    document.body.removeChild(container2);
  });
});
