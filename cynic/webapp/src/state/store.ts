/**
 * CYNIC Application State Store
 * Simple pub/sub state management without external dependencies
 *
 * Manages app state across components using observer pattern.
 * Emits notifications to all subscribers on state changes.
 */

import type {
  OrganismSchema,
  CommandResponse,
  AccountStatusResponse,
  PolicyResponse,
} from '../types/api';

/**
 * Complete application state shape
 */
export interface AppState {
  /** Organism schema with commands and skills */
  schema: OrganismSchema | null;

  /** History of executed commands */
  commands: CommandResponse[];

  /** Most recently executed command */
  currentCommand: CommandResponse | null;

  /** Application metrics (varies based on data received) */
  metrics: Record<string, unknown>;

  /** WebSocket connection status */
  wsConnected: boolean;

  /** Account status data */
  account: AccountStatusResponse | null;

  /** Policy patterns and coverage */
  policy: PolicyResponse | null;
}

/**
 * Internal listener type
 */
type StateListener = () => void;

/**
 * Application state store with pub/sub pattern
 *
 * Provides centralized state management and observer notifications.
 * All state updates trigger listener notifications for reactivity.
 */
class Store {
  /**
   * Current application state
   */
  private state: AppState = {
    schema: null,
    commands: [],
    currentCommand: null,
    metrics: {},
    wsConnected: false,
    account: null,
    policy: null,
  };

  /**
   * Set of listener functions to notify on state changes
   */
  private listeners: Set<StateListener> = new Set();

  /**
   * Retrieve a copy of the current application state
   *
   * Returns a shallow copy to prevent external mutations
   * of the internal state object.
   *
   * @returns Copy of current AppState
   */
  getState(): AppState {
    return { ...this.state };
  }

  /**
   * Update state with partial changes and notify all listeners
   *
   * Merges provided updates with current state and triggers
   * all registered listeners. Performs shallow merge only.
   *
   * @param updates Partial AppState object with changes
   */
  setState(updates: Partial<AppState>): void {
    this.state = { ...this.state, ...updates };
    this.notifyListeners();
  }

  /**
   * Register a listener function to be called on state changes
   *
   * Returns an unsubscribe function that removes this listener
   * when called.
   *
   * @param listener Function to call on state changes
   * @returns Unsubscribe function
   */
  subscribe(listener: StateListener): () => void {
    this.listeners.add(listener);

    // Return unsubscribe function
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * Add a command to history and set as current command
   *
   * Appends the command to the commands array and sets it
   * as the currentCommand. Triggers state update notification.
   *
   * @param command CommandResponse to add
   */
  addCommand(command: CommandResponse): void {
    this.setState({
      commands: [...this.state.commands, command],
      currentCommand: command,
    });
  }

  /**
   * Set the organism schema
   *
   * @param schema OrganismSchema from API
   */
  setSchema(schema: OrganismSchema): void {
    this.setState({ schema });
  }

  /**
   * Set application metrics
   *
   * @param metrics Record of metric key-value pairs
   */
  setMetrics(metrics: Record<string, unknown>): void {
    this.setState({ metrics });
  }

  /**
   * Set WebSocket connection status
   *
   * @param connected Boolean indicating connection state
   */
  setWsConnected(connected: boolean): void {
    this.setState({ wsConnected: connected });
  }

  /**
   * Set account status
   *
   * @param account AccountStatusResponse from API
   */
  setAccount(account: AccountStatusResponse): void {
    this.setState({ account });
  }

  /**
   * Set policy patterns and coverage
   *
   * @param policy PolicyResponse from API
   */
  setPolicy(policy: PolicyResponse): void {
    this.setState({ policy });
  }

  /**
   * Notify all registered listeners of state change
   *
   * Internal method called after every setState() call
   */
  private notifyListeners(): void {
    this.listeners.forEach((listener) => {
      try {
        listener();
      } catch (error) {
        console.error(
          'Error in state listener:',
          error instanceof Error ? error.message : String(error)
        );
      }
    });
  }
}

/**
 * Global singleton store instance
 * Use this throughout the application for centralized state management
 */
export const store = new Store();
