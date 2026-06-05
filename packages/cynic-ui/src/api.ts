import * as ed from '@noble/ed25519';
import type { HealthResponse, JudgeRequest, Verdict, Crystal, AsyncJudgeResponse, AsyncJudgeStatus, Observation, AgentTask } from './types';

// import { getKernelUrl } from './utils';

function base(): string {
  // En développement, on utilise le proxy Vite (/api)
  // En production, on utilise VITE_API_BASE
  return import.meta.env.DEV ? '/api' : (import.meta.env.VITE_API_BASE ?? '');
}

/**
 * Enhanced fetch wrapper that supports:
 * 1. Ed25519 Cryptographic Signatures (Zero-Trust)
 * 2. Bearer Tokens (Legacy)
 */
async function cynicFetch(path: string, options: RequestInit = {}): Promise<Response> {
  const method = options.method ?? 'GET';
  const body = options.body ? String(options.body) : '';
  const key = localStorage.getItem('cynic_api_key') ?? import.meta.env.VITE_API_KEY ?? '';
  
  const headers = new Headers(options.headers);

  // 1. Try to treat key as Ed25519 Private Key (64-char hex)
  if (key.length === 64 && /^[0-9a-fA-F]+$/.test(key)) {
    try {
      const timestamp = Math.floor(Date.now() / 1000).toString();
      // Payload format: METHOD|PATH|BODY
      const payload = `${method}|${path}|${body}`;
      // Signed payload: TIMESTAMP + PAYLOAD
      const message = new TextEncoder().encode(timestamp + payload);
      
      const pubKey = await ed.getPublicKey(key);
      const signature = await ed.sign(message, key);
      
      headers.set('X-Cynic-Signature', ed.etc.bytesToHex(signature));
      headers.set('X-Cynic-Public-Key', ed.etc.bytesToHex(pubKey));
      headers.set('X-Cynic-Timestamp', timestamp);
    } catch (e) {
      console.warn('Failed to sign request, falling back to Bearer', e);
      headers.set('Authorization', `Bearer ${key}`);
    }
  } else if (key) {
    // 2. Fallback to Bearer Token
    headers.set('Authorization', `Bearer ${key}`);
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

export async function getStateHistory(limit = 100): Promise<any[]> {
  const res = await cynicFetch(`/state-history?limit=${limit}`);
  if (!res.ok) throw new Error('Failed to fetch state history');
  return res.json();
}

export async function getAuthInput(): Promise<{ nonce: string, statement: string, domain: string }> {
  const res = await fetch(`${base()}/auth/input`);
  if (!res.ok) throw new Error('Failed to get auth input');
  return res.json();
}

export async function verifyAuth(address: string, signature: string, nonce: string): Promise<{ role: string, expires_at: number }> {
  const res = await fetch(`${base()}/auth/verify`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ address, signature, nonce }),
  });
  if (!res.ok) throw new Error('Auth verification failed');
  return res.json();
}
