/**
 * CYNIC Metrics Dashboard
 * Real-time display of organism metrics: balance, learn_rate, reputation
 */

import { apiClient } from '../api/client';
import { wsClient } from '../api/ws';
import type { AccountStatusResponse, StateUpdateData, CommandEventData } from '../types/api';
import type { CynicWebSocketClient } from '../api/ws';

/**
 * MetricsPanel displays real-time metrics from GET /api/organism/account
 * Auto-refreshes on WebSocket state_update and command_complete events
 */
export class MetricsPanel {
  private containerId: string;
  private wsClient: CynicWebSocketClient;
  private currentMetrics: AccountStatusResponse | null = null;
  private stateUpdateHandler: ((data: StateUpdateData) => void) | null = null;
  private commandCompleteHandler: ((data: CommandEventData) => void) | null = null;
  private container: Element | null = null;

  constructor(containerId: string, wsClient?: CynicWebSocketClient) {
    this.containerId = containerId;
    this.wsClient = wsClient || (globalThis as any).wsClient || (window as any).CYNIC?.ws;
    this.container = document.querySelector(containerId);
  }

  /**
   * Render metrics panel and start listening to WebSocket events
   */
  async render(): Promise<void> {
    try {
      // Fetch metrics from API
      this.currentMetrics = await apiClient.getAccount();

      // Render to DOM
      this.renderMetrics();

      // Register WebSocket listeners for auto-refresh
      this.registerListeners();
    } catch (error) {
      this.renderError(error);
    }
  }

  /**
   * Render metrics to DOM using safe DOM manipulation
   */
  private renderMetrics(): void {
    if (!this.currentMetrics || !this.container) return;

    // Clear container
    this.container.innerHTML = '';

    // Create metrics card
    const card = document.createElement('div');
    card.className = 'metrics-card';

    // Create header
    const header = document.createElement('div');
    header.className = 'metrics-header';

    const title = document.createElement('h2');
    title.textContent = 'Metrics Dashboard';
    header.appendChild(title);

    const timestamp = document.createElement('div');
    timestamp.className = 'metrics-timestamp';
    timestamp.textContent = this.formatTimestamp(this.currentMetrics.timestamp);
    header.appendChild(timestamp);

    card.appendChild(header);

    // Create body
    const body = document.createElement('div');
    body.className = 'metrics-body';

    // Balance item
    body.appendChild(
      this.createMetricItem(
        'Balance',
        this.formatCurrency(this.currentMetrics.balance_usd),
        'balance'
      )
    );

    // Spent item
    body.appendChild(
      this.createMetricItem(
        'Spent',
        this.formatCurrency(this.currentMetrics.spent_usd),
        'spent'
      )
    );

    // Budget remaining item
    body.appendChild(
      this.createMetricItem(
        'Budget Remaining',
        this.formatCurrency(this.currentMetrics.budget_remaining_usd),
        'budget_remaining'
      )
    );

    // Learn rate with progress bar
    body.appendChild(
      this.createMetricWithBar(
        'Learn Rate',
        this.formatPercentage(this.currentMetrics.learn_rate),
        'learn_rate',
        Math.min(this.currentMetrics.learn_rate * 100, 61.8)
      )
    );

    // Reputation with progress bar
    body.appendChild(
      this.createMetricWithBar(
        'Reputation',
        this.formatReputation(this.currentMetrics.reputation),
        'reputation',
        this.currentMetrics.reputation
      )
    );

    card.appendChild(body);
    this.container.appendChild(card);
  }

  /**
   * Create a simple metric item element
   */
  private createMetricItem(label: string, value: string, dataMetric: string): HTMLElement {
    const item = document.createElement('div');
    item.className = 'metric-item';

    const labelEl = document.createElement('div');
    labelEl.className = 'metric-label';
    labelEl.textContent = label;

    const valueEl = document.createElement('div');
    valueEl.className = 'metric-value';
    valueEl.setAttribute('data-metric', dataMetric);
    valueEl.textContent = value;

    item.appendChild(labelEl);
    item.appendChild(valueEl);

    return item;
  }

  /**
   * Create a metric item with progress bar
   */
  private createMetricWithBar(
    label: string,
    value: string,
    dataBar: string,
    width: number
  ): HTMLElement {
    const item = document.createElement('div');
    item.className = 'metric-item metric-with-bar';

    const labelEl = document.createElement('div');
    labelEl.className = 'metric-label';
    labelEl.textContent = label;
    item.appendChild(labelEl);

    const barContainer = document.createElement('div');
    barContainer.className = 'progress-bar-container';

    const barFill = document.createElement('div');
    barFill.className = 'progress-bar-fill';
    barFill.setAttribute('data-bar', dataBar);
    barFill.style.width = `${width}%`;

    const barText = document.createElement('span');
    barText.className = 'progress-text';
    barText.textContent = value;

    barFill.appendChild(barText);
    barContainer.appendChild(barFill);
    item.appendChild(barContainer);

    return item;
  }

  /**
   * Render error state
   */
  private renderError(error: unknown): void {
    if (!this.container) return;

    this.container.innerHTML = '';

    const errorDiv = document.createElement('div');
    errorDiv.className = 'metrics-error';

    const icon = document.createElement('div');
    icon.className = 'error-icon';
    icon.textContent = '⚠️';

    const message = document.createElement('div');
    message.className = 'error-message';
    const errorMessage =
      error instanceof Error ? error.message : 'Failed to load metrics';
    message.textContent = `Error loading metrics: ${errorMessage}`;

    errorDiv.appendChild(icon);
    errorDiv.appendChild(message);
    this.container.appendChild(errorDiv);
  }

  /**
   * Register WebSocket listeners for auto-refresh
   */
  private registerListeners(): void {
    if (!this.wsClient) return;

    // Create handlers with bound context
    this.stateUpdateHandler = () => this.update();
    this.commandCompleteHandler = () => this.update();

    // Register listeners
    this.wsClient.on('state_update', this.stateUpdateHandler);
    this.wsClient.on('command_complete', this.commandCompleteHandler);
  }

  /**
   * Update metrics from API
   */
  async update(): Promise<void> {
    try {
      this.currentMetrics = await apiClient.getAccount();
      this.renderMetrics();
    } catch (error) {
      console.error('Failed to update metrics:', error);
    }
  }

  /**
   * Clean up resources and remove listeners
   */
  destroy(): void {
    if (!this.wsClient) return;

    if (this.stateUpdateHandler) {
      this.wsClient.off('state_update', this.stateUpdateHandler);
    }
    if (this.commandCompleteHandler) {
      this.wsClient.off('command_complete', this.commandCompleteHandler);
    }

    // Clear handlers
    this.stateUpdateHandler = null;
    this.commandCompleteHandler = null;
  }

  /**
   * Format number as USD currency
   * @param value Number to format
   * @returns Formatted string with dollar sign, commas, and 2 decimals
   */
  private formatCurrency(value: number): string {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  }

  /**
   * Format number as percentage
   * @param value Number between 0 and 1
   * @returns Formatted string (e.g., "61.8%")
   */
  private formatPercentage(value: number): string {
    return new Intl.NumberFormat('en-US', {
      style: 'percent',
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(value);
  }

  /**
   * Format reputation as ratio
   * @param value Reputation score (0-100)
   * @returns Formatted string (e.g., "95/100")
   */
  private formatReputation(value: number): string {
    return `${Math.round(value)}/100`;
  }

  /**
   * Format ISO timestamp as readable time
   * @param timestamp ISO 8601 timestamp
   * @returns Formatted time string
   */
  private formatTimestamp(timestamp: string): string {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString('en-US', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      });
    } catch {
      return 'N/A';
    }
  }
}
