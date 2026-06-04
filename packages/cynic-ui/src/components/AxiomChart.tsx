import { AXIOM_COLORS, AXIOM_ICONS } from '../types';
import type { QScore } from '../types';

interface Props {
  qScore: QScore;
}
// ...

const AXIOMS = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;
export function AxiomChart({ qScore }: Props) {
  return <div style={{ color: '#C9A84C', padding: 20 }}>AxiomChart disabled for debugging.</div>;
}

interface BarsProps {
  qScore: QScore;
}

export function AxiomBars({ qScore }: BarsProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {AXIOMS.map((key) => {
        const value = qScore[key];
        const pct = Math.min((value / 0.618) * 100, 100);
        const color = AXIOM_COLORS[key];
        return (
          <div key={key}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontSize: 12, color: '#aaa' }}>
                {AXIOM_ICONS[key]} {key.toUpperCase()}
              </span>
              <span style={{ fontSize: 12, color, fontFamily: 'monospace' }}>
                {value.toFixed(3)}
              </span>
            </div>
            <div style={{ height: 6, background: '#2a2a2a', borderRadius: 3 }}>
              <div style={{
                height: '100%',
                width: `${pct}%`,
                background: color,
                borderRadius: 3,
                transition: 'width 0.6s ease',
              }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
