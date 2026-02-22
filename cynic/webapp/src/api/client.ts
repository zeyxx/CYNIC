/**
 * CYNIC REST API Client
 * Type-safe fetch wrapper for CYNIC backend API
 */

import type {
  OrganismSchema,
  CommandRequest,
  CommandResponse,
  SkillRequest,
  SkillSchema,
  SkillListResponse,
  AccountStatusResponse,
  PolicyResponse,
  ApiError,
} from '../types/api';
import { ErrorDisplay, type ErrorResponse } from '../ui/error-display';

/**
 * REST API client for CYNIC backend
 * Provides type-safe methods for API communication
 */
export class CynicApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = '/api') {
    this.baseUrl = baseUrl;
  }

  /**
   * Make a typed HTTP request
   * @param method HTTP method (GET, POST, PUT, DELETE)
   * @param path API path (relative to baseUrl)
   * @param body Request body (optional, for POST/PUT)
   * @returns Promise with typed response
   * @throws Error with descriptive message on failure
   */
  private async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE',
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = new URL(path, this.baseUrl).toString();

    const options: RequestInit = {
      method,
      headers: {
        'Content-Type': 'application/json',
      },
    };

    if (body !== undefined) {
      options.body = JSON.stringify(body);
    }

    let response: Response;
    try {
      response = await fetch(url, options);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);

      // Display network error to user
      ErrorDisplay.show({
        error: `Network error: ${errorMsg}\n\nℹ️ Check your internet connection and try again.\n\nError code: #NET`,
        code: '#NET',
        type: 'NetworkError',
      });

      throw new Error(`Network error: ${errorMsg}`);
    }

    // Handle non-2xx responses
    if (!response.ok) {
      let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
      let errorCode = `#${response.status}`;
      let errorType = 'HTTPError';

      try {
        const errorData = (await response.json()) as unknown;

        // New error format (from error_handler.py)
        if (
          errorData &&
          typeof errorData === 'object' &&
          'error' in errorData &&
          'code' in errorData
        ) {
          const newError = errorData as ErrorResponse;
          errorMessage = newError.error;
          errorCode = newError.code;
          errorType = newError.type;
        }
        // Old error format fallback
        else if (
          errorData &&
          typeof errorData === 'object' &&
          'error' in errorData
        ) {
          const oldError = errorData as ApiError;
          if (oldError.error?.message) {
            errorMessage = oldError.error.message;
          }
        }
      } catch {
        // Could not parse error response, use default message
      }

      // Display error to user
      ErrorDisplay.show({
        error: errorMessage,
        code: errorCode,
        type: errorType,
      });

      throw new Error(errorMessage);
    }

    // Parse successful response
    try {
      return (await response.json()) as T;
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);

      // Display parse error to user
      ErrorDisplay.show({
        error: `An unexpected error occurred while processing the response.\n\nℹ️ Try again or refresh the page.\n\nError code: #PARSE`,
        code: '#PARSE',
        type: 'ResponseParseError',
      });

      throw new Error(`Failed to parse response: ${errorMsg}`);
    }
  }

  /**
   * Get complete organism schema
   * @returns Organism schema with version, commands, skills, state
   */
  async getSchema(): Promise<OrganismSchema> {
    return this.request<OrganismSchema>('GET', '/organism/schema');
  }

  /**
   * Get current organism state
   * @returns Current state snapshot of all components
   */
  async getState(): Promise<Record<string, unknown>> {
    return this.request<Record<string, unknown>>('GET', '/organism/state');
  }

  /**
   * Get account status and financial metrics
   * @returns Account balance, spending, budget, learn rate, reputation
   */
  async getAccount(): Promise<AccountStatusResponse> {
    return this.request<AccountStatusResponse>('GET', '/organism/account');
  }

  /**
   * Get policy patterns and Q-table coverage
   * @returns Patterns with counts and coverage statistics
   */
  async getPolicy(): Promise<PolicyResponse> {
    return this.request<PolicyResponse>('GET', '/organism/policy');
  }

  /**
   * Invoke a command by ID
   * @param request Command ID and parameters
   * @returns Command execution response with result or error
   */
  async invokeCommand(request: CommandRequest): Promise<CommandResponse> {
    return this.request<CommandResponse>('POST', '/commands/invoke', request);
  }

  /**
   * Get command execution history
   * @param limit Maximum number of historical commands to return
   * @returns Array of past command responses
   */
  async getCommandHistory(limit: number = 50): Promise<CommandResponse[]> {
    const url = `/commands/history?limit=${Math.max(1, Math.min(limit, 1000))}`;
    return this.request<CommandResponse[]>('GET', url);
  }

  /**
   * List all available skills
   * @returns Array of skill schemas
   */
  async listSkills(): Promise<SkillListResponse> {
    return this.request<SkillListResponse>('GET', '/skills');
  }

  /**
   * Create a new skill
   * @param request Skill name, description, and code
   * @returns Created skill schema
   */
  async createSkill(request: SkillRequest): Promise<SkillSchema> {
    return this.request<SkillSchema>('POST', '/skills', request);
  }

  /**
   * Get a specific skill by ID
   * @param id Skill identifier
   * @returns Skill schema
   */
  async getSkill(id: string): Promise<SkillSchema> {
    return this.request<SkillSchema>('GET', `/skills/${encodeURIComponent(id)}`);
  }

  /**
   * Update a skill
   * @param id Skill identifier
   * @param request Updated skill data
   * @returns Updated skill schema
   */
  async updateSkill(id: string, request: Partial<SkillRequest>): Promise<SkillSchema> {
    return this.request<SkillSchema>('PUT', `/skills/${encodeURIComponent(id)}`, request);
  }

  /**
   * Delete a skill
   * @param id Skill identifier
   * @returns Success status
   */
  async deleteSkill(id: string): Promise<{ success: boolean }> {
    return this.request<{ success: boolean }>('DELETE', `/skills/${encodeURIComponent(id)}`);
  }
}

// Export singleton instance
export const apiClient = new CynicApiClient();
