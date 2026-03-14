import { API_BASE } from './types';
import type { HealthResponse, JudgeRequest, Verdict } from './types';

export async function checkHealth(): Promise<HealthResponse> {
  const res = await fetch(`${API_BASE}/health`);
  if (!res.ok) throw new Error('Health check failed');
  return res.json();
}

export async function judge(req: JudgeRequest): Promise<Verdict> {
  const res = await fetch(`${API_BASE}/judge`, {
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
  const res = await fetch(`${API_BASE}/verdicts`);
  if (!res.ok) throw new Error('Failed to fetch verdicts');
  return res.json();
}

export async function getVerdict(id: string): Promise<Verdict> {
  const res = await fetch(`${API_BASE}/verdict/${id}`);
  if (!res.ok) throw new Error('Failed to fetch verdict');
  return res.json();
}
