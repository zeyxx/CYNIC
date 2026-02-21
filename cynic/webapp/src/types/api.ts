/**
 * CYNIC API Types
 * TypeScript interfaces for REST API communication
 */

/**
 * Parameter definition for command parameters
 */
export interface ParamDef {
  type: 'string' | 'number' | 'boolean' | 'array' | 'object';
  required: boolean;
  description: string;
  enum?: string[] | number[];
  default?: string | number | boolean | unknown;
}

/**
 * Command schema definition
 */
export interface CommandSchema {
  id: string;
  name: string;
  description: string;
  params: Record<string, ParamDef>;
  returns: {
    type: string;
    description: string;
  };
}

/**
 * Skill schema definition
 */
export interface SkillSchema {
  id: string;
  name: string;
  description: string;
  template: string;
}

/**
 * Organism schema - complete state snapshot
 */
export interface OrganismSchema {
  version: string;
  commands: CommandSchema[];
  skills: SkillSchema[];
  state: Record<string, unknown>;
}

/**
 * Command execution request
 */
export interface CommandRequest {
  command_id: string;
  params: Record<string, unknown>;
}

/**
 * Command execution response
 */
export interface CommandResponse {
  id: string;
  status: 'success' | 'error' | 'pending';
  result?: unknown;
  error?: {
    message: string;
    code?: string;
  };
  timestamp: string;
}

/**
 * Skill creation/update request
 */
export interface SkillRequest {
  name: string;
  description: string;
  code: string;
}

/**
 * Skill list response
 */
export interface SkillListResponse {
  skills: SkillSchema[];
}

/**
 * API error structure
 */
export interface ApiError {
  error: {
    message: string;
    code?: string;
    details?: Record<string, unknown>;
  };
}

/**
 * Generic API response wrapper
 */
export interface ApiResponse<T> {
  data?: T;
  error?: ApiError['error'];
  timestamp?: string;
}

/**
 * Account status response from GET /api/organism/account
 */
export interface AccountStatusResponse {
  timestamp: string;
  balance_usd: number;
  spent_usd: number;
  budget_remaining_usd: number;
  learn_rate: number;
  reputation: number;
}

/**
 * Policy response from GET /api/organism/policy
 */
export interface PolicyResponse {
  patterns: Array<{
    name: string;
    count: number;
    confidence: number;
  }>;
  coverage: {
    total: number;
    covered: number;
    percentage: number;
  };
}

/**
 * WebSocket event types
 */
export type WebSocketEventType =
  | 'connect'
  | 'disconnect'
  | 'error'
  | 'command_start'
  | 'command_complete'
  | 'state_update'
  | 'learning_update';

/**
 * WebSocket message structure
 */
export interface WebSocketMessage<T = unknown> {
  type: WebSocketEventType;
  timestamp: string;
  data: T;
  sequence?: number;
}

/**
 * Learning update event data
 */
export interface LearningUpdateData {
  learn_rate: number;
  iteration: number;
  quality_score: number;
  confidence: number;
}

/**
 * State update event data
 */
export interface StateUpdateData {
  component: string;
  state: Record<string, unknown>;
  timestamp: string;
}

/**
 * Command event data
 */
export interface CommandEventData {
  command_id: string;
  status: 'start' | 'complete' | 'error';
  result?: unknown;
  error?: {
    message: string;
    code?: string;
  };
}
