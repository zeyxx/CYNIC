import { AXIOM_ICONS } from '../types';
import type { QScore } from '../types';

interface Props {
  qScore: QScore;
}

const AXIOMS = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;
export function AxiomChart({ qScore: _qScore }: Props) {
  return <div style={{ color: '#C9A84C', padding: 20 }}>AxiomChart disabled for debugging.</div>;
}

interface BarsProps {
  qScore: QScore;
}

export function AxiomBars({ qScore: _qScore }: BarsProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {AXIOMS.map((key) => {
        return (
          <div key={key}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontSize: 12, color: '#aaa' }}>
                {AXIOM_ICONS[key]} {key.toUpperCase()}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
