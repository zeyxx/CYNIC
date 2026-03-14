import { VERDICT_COLORS, VERDICT_LABELS, AXIOM_ICONS } from '../types';
import type { Verdict } from '../types';
import { AxiomChart, AxiomBars } from './AxiomChart';

interface Props {
  verdict: Verdict;
}

export function VerdictDisplay({ verdict }: Props) {
  const color = VERDICT_COLORS[verdict.verdict];
  const label = VERDICT_LABELS[verdict.verdict];
  const axioms = ['fidelity', 'phi', 'verify', 'culture', 'burn', 'sovereignty'] as const;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{
        textAlign: 'center',
        padding: '20px',
        border: `2px solid ${color}`,
        borderRadius: 12,
        background: `${color}11`,
      }}>
        <div style={{ fontSize: 48, fontWeight: 900, color, letterSpacing: 4, fontFamily: 'monospace' }}>
          {verdict.verdict.toUpperCase()}
        </div>
        <div style={{ color: '#888', marginTop: 4 }}>{label}</div>
        <div style={{ marginTop: 12, fontSize: 32, fontWeight: 700, color, fontFamily: 'monospace' }}>
          Q: {verdict.q_score.total.toFixed(4)}
        </div>
        <div style={{ fontSize: 12, color: '#555', marginTop: 4 }}>
          φ-max: {verdict.phi_max.toFixed(6)} · Dogs: {verdict.dogs_used}
        </div>
        {verdict.anomaly_detected && (
          <div style={{
            marginTop: 10,
            padding: '6px 14px',
            background: '#FF980022',
            border: '1px solid #FF9800',
            borderRadius: 6,
            color: '#FF9800',
            fontSize: 12,
            display: 'inline-block',
          }}>
            ⚠ DISCOVERY SIGNAL — Dogs disagree on {verdict.anomaly_axiom || 'unknown axiom'}
          </div>
        )}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20 }}>
        <div>
          <h3 style={{ color: '#C9A84C', margin: '0 0 12px', fontSize: 14, letterSpacing: 1 }}>AXIOM RADAR</h3>
          <AxiomChart qScore={verdict.q_score} />
        </div>
        <div>
          <h3 style={{ color: '#C9A84C', margin: '0 0 12px', fontSize: 14, letterSpacing: 1 }}>AXIOM SCORES</h3>
          <AxiomBars qScore={verdict.q_score} />
        </div>
      </div>

      <div>
        <h3 style={{ color: '#C9A84C', margin: '0 0 12px', fontSize: 14, letterSpacing: 1 }}>REASONING</h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {axioms.map((key) => (
            <div key={key} style={{
              padding: '10px 14px',
              background: '#111',
              border: '1px solid #222',
              borderRadius: 8,
            }}>
              <div style={{ fontSize: 12, color: '#C9A84C', marginBottom: 4, fontWeight: 600 }}>
                {AXIOM_ICONS[key]} {key.toUpperCase()}
              </div>
              <div style={{ fontSize: 13, color: '#ccc', lineHeight: 1.5 }}>
                {verdict.reasoning[key]}
              </div>
            </div>
          ))}
        </div>
      </div>

      {verdict.dog_scores && verdict.dog_scores.length > 1 && (
        <div>
          <h3 style={{ color: '#C9A84C', margin: '0 0 12px', fontSize: 14, letterSpacing: 1 }}>
            DOG COMPARISON
          </h3>
          <div style={{ display: 'grid', gridTemplateColumns: `repeat(${verdict.dog_scores.length}, 1fr)`, gap: 12 }}>
            {verdict.dog_scores.map((dog) => (
              <div key={dog.dog_id} style={{
                padding: '12px',
                background: '#111',
                border: '1px solid #222',
                borderRadius: 8,
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
                  <div style={{ fontSize: 12, color: '#888', fontWeight: 600 }}>
                    🐕 {dog.dog_id}
                  </div>
                  <div style={{ fontSize: 10, color: '#555', fontFamily: 'monospace' }}>
                    ⚡ {dog.latency_ms}ms
                  </div>
                </div>
                {axioms.map((key) => (
                  <div key={key} style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: '#aaa', marginBottom: 2 }}>
                    <span>{AXIOM_ICONS[key]}</span>
                    <span style={{ fontFamily: 'monospace' }}>{dog[key].toFixed(3)}</span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
