import type { HealthResponse, JudgeRequest, Verdict, Crystal, AsyncJudgeResponse, AsyncJudgeStatus, Observation, PublicObservation, AgentTask, AuthInputResponse, AuthVerifyResponse, StateHistoryResponse } from './types';
import { getAuthSessionToken } from './auth';

function base(): string {
  return import.meta.env.DEV ? '/api' : (import.meta.env.VITE_API_BASE ?? '');
}

/**
 * Enhanced fetch wrapper that forwards the current wallet session when available.
 */
async function cynicFetch(path: string, options: RequestInit = {}): Promise<Response> {
  const headers = new Headers(options.headers);
  const sessionId = getAuthSessionToken();
  if (sessionId) {
    headers.set('X-Cynic-Session', sessionId);
  }

  const url = path.startsWith('http') ? path : `${base()}${path}`;
  return fetch(url, { ...options, headers });
}

export async function checkHealth(): Promise<HealthResponse> {
  const res = await cynicFetch('/health');
  if (!res.ok && res.status !== 503) throw new Error(`Health check failed: ${res.status}`);
  return res.json();
}

export async function judge(req: JudgeRequest): Promise<Verdict> {
  const res = await cynicFetch('/judge', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Judge failed: ${res.status} ${text}`);
  }
  return res.json();
}

export async function getVerdicts(): Promise<Verdict[]> {
  const res = await cynicFetch('/verdicts');
  if (!res.ok) throw new Error('Failed to fetch verdicts');
  return res.json();
}

export async function getVerdict(id: string): Promise<Verdict> {
  const res = await cynicFetch(`/verdict/${id}`);
  if (!res.ok) throw new Error('Failed to fetch verdict');
  return res.json();
}

export async function getAvailableDogs(): Promise<string[]> {
  const res = await cynicFetch('/dogs');
  if (!res.ok) throw new Error('Failed to fetch dogs');
  const data = await res.json();
  if (!Array.isArray(data) || data.length === 0) return [];
  if (typeof data[0] === 'string') return data;
  return data.map((d: { id: string }) => d.id);
}

export async function getCrystals(): Promise<Crystal[]> {
  const res = await cynicFetch('/crystals');
  if (!res.ok) throw new Error('Failed to fetch crystals');
  return res.json();
}

export async function judgeAsync(req: JudgeRequest): Promise<AsyncJudgeResponse> {
  const res = await cynicFetch('/judge/async', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Async judge failed: ${res.status} ${text}`);
  }
  return res.json();
}

export async function getJudgeStatus(requestId: string): Promise<AsyncJudgeStatus> {
  const res = await cynicFetch(`/judge/status/${requestId}`);
  if (!res.ok) throw new Error(`Status poll failed: ${res.status}`);
  return res.json();
}

export async function getObservations(domain?: string, limit = 100): Promise<Observation[]> {
  let path = `/observations?limit=${limit}`;
  if (domain) path += `&domain=${domain}`;
  const res = await cynicFetch(path);
  if (!res.ok) throw new Error('Failed to fetch observations');
  const data = await res.json();
  return data.observations ?? [];
}

export async function getPublicObservations(domain?: string, limit = 100): Promise<PublicObservation[]> {
  let path = `/observations/public?limit=${limit}`;
  if (domain) path += `&domain=${domain}`;
  const res = await cynicFetch(path);
  if (!res.ok) throw new Error('Failed to fetch public observations');
  return res.json();
}

export async function getAgentTasks(kind = 'hermes', limit = 10): Promise<{ tasks: AgentTask[] }> {
  const res = await cynicFetch(`/agent-tasks?kind=${kind}&limit=${limit}`);
  if (!res.ok) throw new Error('Failed to fetch agent tasks');
  return res.json();
}

export async function updateAgentTaskResult(id: string, result?: string, error?: string): Promise<void> {
  const res = await cynicFetch(`/agent-tasks/${id}/result`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ result, error }),
  });
  if (!res.ok) throw new Error(`Failed to update agent task: ${res.status}`);
}

export async function dispatchAgentTask(req: {
  kind: string;
  domain: string;
  content: string;
  agent_id?: string;
}): Promise<{ task_id: string; status: string }> {
  const res = await cynicFetch('/agent-tasks', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(`Failed to dispatch agent task: ${res.status}`);
  return res.json();
}

export async function getStateHistory(limit = 100): Promise<StateHistoryResponse> {
  const res = await cynicFetch(`/state-history?limit=${limit}`);
  if (!res.ok) throw new Error('Failed to fetch state history');
  const data = await res.json();
  if (Array.isArray(data)) {
    return { blocks: data, count: data.length, chain_valid: true, blocks_valid: true };
  }
  if (Array.isArray(data?.blocks)) {
    return {
      blocks: data.blocks,
      count: typeof data.count === 'number' ? data.count : data.blocks.length,
      chain_valid: typeof data.chain_valid === 'boolean' ? data.chain_valid : true,
      blocks_valid: typeof data.blocks_valid === 'boolean' ? data.blocks_valid : true,
    };
  }
  return { blocks: [], count: 0, chain_valid: false, blocks_valid: false };
}

export async function getAuthInput(): Promise<AuthInputResponse> {
  const res = await fetch(`${base()}/auth/input`);
  if (!res.ok) throw new Error('Failed to get auth input');
  return res.json();
}

export async function verifyAuth(address: string, signature: string, nonce: string, timestamp: number): Promise<AuthVerifyResponse> {
  const res = await fetch(`${base()}/auth/verify`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ address, signature, nonce, timestamp }),
  });
  if (!res.ok) throw new Error('Auth verification failed');
  return res.json();
}
