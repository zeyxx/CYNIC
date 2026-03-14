export interface JudgeRequest {
  content: string;
  context?: string;
  domain?: string;
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
}

export const VERDICT_COLORS: Record<VerdictKind, string> = {
  Howl:  '#FFD700',
  Wag:   '#2196F3',
  Growl: '#FF9800',
  Bark:  '#F44336',
};

export const VERDICT_BG: Record<VerdictKind, string> = {
  Howl:  'rgba(255, 215, 0, 0.08)',
  Wag:   'rgba(33, 150, 243, 0.08)',
  Growl: 'rgba(255, 152, 0, 0.08)',
  Bark:  'rgba(244, 67, 54, 0.08)',
};

export const PHI_MAX = 0.618033988749895;

export const AXIOMS: Array<{ key: keyof Omit<QScore, 'total'>; label: string; icon: string }> = [
  { key: 'fidelity',    label: 'Fidelity',    icon: '⚔' },
  { key: 'phi',         label: 'Phi',         icon: 'φ' },
  { key: 'verify',      label: 'Verify',      icon: '⊕' },
  { key: 'culture',     label: 'Culture',     icon: '⟁' },
  { key: 'burn',        label: 'Burn',        icon: '◈' },
  { key: 'sovereignty', label: 'Sovereignty', icon: '◉' },
];
