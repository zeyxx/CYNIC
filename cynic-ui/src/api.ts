import type { JudgeRequest, Verdict, HealthResponse } from './types';

const BASE = 'http://<TAILSCALE_UBUNTU>:3030';

export async function judgeContent(req: JudgeRequest): Promise<Verdict> {
  const res = await fetch(`${BASE}/judge`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(`Judge failed: ${res.status}`);
  return res.json();
}

export async function checkHealth(): Promise<HealthResponse> {
  const res = await fetch(`${BASE}/health`);
  if (!res.ok) throw new Error('Health check failed');
  return res.json();
}
