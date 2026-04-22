export interface JudgeRequest {
  content: string;
  context?: string;
  domain?: string;
  dogs?: string[];
}

export interface QScore {
  total: number;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
}

export interface Reasoning {
  fidelity: string;
  phi: string;
  verify: string;
  culture: string;
  burn: string;
  sovereignty: string;
}

export interface DogScore {
  dog_id: string;
  latency_ms: number;
  fidelity: number;
  phi: number;
  verify: number;
  culture: number;
  burn: number;
  sovereignty: number;
  reasoning: Reasoning;
}

export type VerdictKind = 'Howl' | 'Wag' | 'Growl' | 'Bark';

export interface Verdict {
  verdict_id: string;
  verdict: VerdictKind;
  q_score: QScore;
  reasoning: Reasoning;
  dogs_used: string;
  phi_max: number;
  dog_scores: DogScore[];
  anomaly_detected: boolean;
  max_disagreement: number;
  anomaly_axiom: string | null;
}

export interface HealthResponse {
  status: string;
  version: string;
  phi_max: number;
  axioms: string[];
  dogs: { id: string; kind: string }[];
}

export interface Crystal {
  id: string;
  content: string;
  domain: string;
  confidence: number;
  observations: number;
  state: string;
  created_at: string;
  updated_at: string;
}

export const VERDICT_COLORS: Record<VerdictKind, string> = {
  Howl: '#FFD700',
  Wag: '#2196F3',
  Growl: '#FF9800',
  Bark: '#F44336',
};

export const VERDICT_LABELS: Record<VerdictKind, string> = {
  Howl: 'HOWL — Exceptional',
  Wag: 'WAG — Good',
  Growl: 'GROWL — Questionable',
  Bark: 'BARK — Rejected',
};

export const AXIOM_COLORS: Record<string, string> = {
  fidelity: '#2196F3',
  phi: '#C9A84C',
  verify: '#4CAF50',
  culture: '#9C27B0',
  burn: '#FF9800',
  sovereignty: '#FFD700',
};

export const AXIOM_ICONS: Record<string, string> = {
  fidelity: '🛡',
  phi: '🌀',
  verify: '🔍',
  culture: '🏛',
  burn: '🔥',
  sovereignty: '👑',
};

// Async judge types
export interface AsyncJudgeResponse {
  request_id: string;
  status: 'pending';
  dogs_total: number;
}

export interface DogArrival {
  dog_id: string;
  arrived_at_ms: number;
  success: boolean;
  score?: DogScore;
  error?: string;
}

export type AsyncStatus = 'pending' | 'evaluating' | 'complete' | 'failed';

export interface AsyncJudgeStatus {
  request_id: string;
  status: AsyncStatus;
  dogs_total: number;
  dogs_arrived: DogArrival[];
  verdict: Verdict | null;
  error: string | null;
}

export const DEFAULT_API_BASE = import.meta.env.VITE_API_BASE ?? 'http://localhost:3030';
