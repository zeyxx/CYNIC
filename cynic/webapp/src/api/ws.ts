/**
 * CYNIC WebSocket Client
 * Real-time event streaming with auto-reconnect and exponential backoff
 */

import type {
  WebSocketEventType,
  WebSocketMessage,
  LearningUpdateData,
  StateUpdateData,
  CommandEventData,
} from '../types/api';

/**
 * Type for event handler function
 */
type EventHandler<T = unknown> = (data: T) => void;

/**
 * WebSocket client with auto-reconnect and event handling
 * Manages real-time communication with CYNIC backend
 */
export class CynicWebSocketClient {
  private url: string;
  private ws: WebSocket | null = null;
  private listeners: Map<WebSocketEventType, Set<EventHandler>> = new Map();
  private reconnectDelay: number = 1000; // Start at 1 second
  private reconnectAttempts: number = 0;
  private maxReconnectAttempts: number = 5;
  private maxReconnectDelay: number = 30000; // Cap at 30 seconds
  private isIntentionallyClosed: boolean = false;
  private messageSequence: number = 0;

  constructor(url: string) {
    this.url = url;
    this.initializeListeners();
  }

  /**
   * Initialize event listener map with all supported event types
   */
  private initializeListeners(): void {
    const eventTypes: WebSocketEventType[] = [
      'connect',
      'disconnect',
      'error',
      'command_start',
      'command_complete',
      'state_update',
      'learning_update',
    ];

    eventTypes.forEach((eventType) => {
      this.listeners.set(eventType, new Set());
    });
  }

  /**
   * Connect to WebSocket server
   * @returns Promise that resolves when connection is established
   */
  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        resolve();
        return;
      }

      this.isIntentionallyClosed = false;

      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log('*tail wag* WebSocket connected');
          this.reconnectDelay = 1000; // Reset delay on successful connection
          this.reconnectAttempts = 0;
          this.emit('connect', {});
          resolve();
        };

        this.ws.onmessage = (event) => {
          this.handleMessage(event.data);
        };

        this.ws.onerror = (error) => {
          console.error('*GROWL* WebSocket error:', error);
          this.emit('error', { message: 'WebSocket error occurred' });
          reject(new Error('WebSocket connection failed'));
        };

        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          this.emit('disconnect', {});

          if (!this.isIntentionallyClosed) {
            this.attemptReconnect();
          }
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        reject(new Error(`Failed to create WebSocket: ${message}`));
      }
    });
  }

  /**
   * Attempt to reconnect with exponential backoff
   * Max 5 attempts, capped at 30 second delay
   */
  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('*GROWL* Max reconnection attempts reached');
      this.emit('error', {
        message: 'Failed to reconnect after maximum attempts',
      });
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(
      this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1),
      this.maxReconnectDelay
    );

    console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts})`);

    setTimeout(async () => {
      try {
        await this.connect();
      } catch (error) {
        // Connection failed, will retry via onclose handler
      }
    }, delay);
  }

  /**
   * Handle incoming WebSocket message
   * @param data Raw message data from WebSocket
   */
  private handleMessage(data: unknown): void {
    try {
      const message = JSON.parse(data as string) as WebSocketMessage;
      this.messageSequence++;

      // Verify message structure
      if (!message.type || !message.timestamp) {
        console.warn('Invalid message format, missing required fields');
        return;
      }

      // Route message to appropriate listeners
      const handlers = this.listeners.get(message.type);
      if (handlers && handlers.size > 0) {
        handlers.forEach((handler) => {
          try {
            handler(message.data);
          } catch (error) {
            console.error(
              `Error in event handler for ${message.type}:`,
              error instanceof Error ? error.message : String(error)
            );
          }
        });
      }
    } catch (error) {
      console.error(
        'Failed to parse WebSocket message:',
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Register event listener
   * @param eventType Type of event to listen for
   * @param handler Function to call when event occurs
   */
  on<T = unknown>(eventType: WebSocketEventType, handler: EventHandler<T>): void {
    const listeners = this.listeners.get(eventType);
    if (listeners) {
      listeners.add(handler as EventHandler);
    }
  }

  /**
   * Unregister event listener
   * @param eventType Type of event
   * @param handler Handler function to remove
   */
  off(eventType: WebSocketEventType, handler: EventHandler): void {
    const listeners = this.listeners.get(eventType);
    if (listeners) {
      listeners.delete(handler);
    }
  }

  /**
   * Close WebSocket connection
   * Prevents automatic reconnection
   */
  close(): void {
    this.isIntentionallyClosed = true;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.emit('disconnect', {});
  }

  /**
   * Emit event to all registered listeners
   * @param eventType Type of event to emit
   * @param data Data to pass to listeners
   */
  private emit(eventType: WebSocketEventType, data: unknown): void {
    const handlers = this.listeners.get(eventType);
    if (handlers) {
      handlers.forEach((handler) => {
        try {
          handler(data);
        } catch (error) {
          console.error(
            `Error in event handler for ${eventType}:`,
            error instanceof Error ? error.message : String(error)
          );
        }
      });
    }
  }

  /**
   * Check if WebSocket is currently connected
   */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Get current connection state
   */
  getState(): 'connecting' | 'connected' | 'disconnected' {
    if (!this.ws) return 'disconnected';
    if (this.ws.readyState === WebSocket.OPEN) return 'connected';
    if (this.ws.readyState === WebSocket.CONNECTING) return 'connecting';
    return 'disconnected';
  }
}

/**
 * Create WebSocket URL from current location
 * Converts http/https to ws/wss and appends /ws path
 */
function createWebSocketUrl(): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const host = window.location.host;
  return `${protocol}//${host}/ws`;
}

// Export singleton instance
export const wsClient = new CynicWebSocketClient(createWebSocketUrl());
