import type { HealthResponse, JudgeRequest, Verdict, Crystal } from './types';

import { getKernelUrl } from './utils';

function base(): string {
  return getKernelUrl().replace(/\/$/, '');
}

export async function checkHealth(): Promise<HealthResponse> {
  const res = await fetch(`${base()}/health`);
  if (!res.ok) throw new Error('Health check failed');
  return res.json();
}

export async function judge(req: JudgeRequest): Promise<Verdict> {
  const res = await fetch(`${base()}/judge`, {
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
  const res = await fetch(`${base()}/verdicts`);
  if (!res.ok) throw new Error('Failed to fetch verdicts');
  return res.json();
}

export async function getVerdict(id: string): Promise<Verdict> {
  const res = await fetch(`${base()}/verdict/${id}`);
  if (!res.ok) throw new Error('Failed to fetch verdict');
  return res.json();
}

export async function getAvailableDogs(): Promise<string[]> {
  const res = await fetch(`${base()}/dogs`);
  if (!res.ok) throw new Error('Failed to fetch dogs');
  return res.json();
}

export async function getCrystals(): Promise<Crystal[]> {
  const res = await fetch(`${base()}/crystals`);
  if (!res.ok) throw new Error('Failed to fetch crystals');
  return res.json();
}
